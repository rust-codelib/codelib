use std::io::Write;

use serde::Serialize;

use crate::benchmark::BenchmarkReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    BitFlips,
    Erasures,
}

#[derive(Debug, Serialize)]
pub struct PointReport {
    pub codec: String,
    pub error_count: Option<usize>,
    pub error_unit: Option<&'static str>,
    pub n: Option<usize>,
    pub payload_bits: Option<usize>,
    pub requested_trials: usize,
    pub completed_trials: Option<usize>,
    pub recovered_blocks: Option<usize>,
    pub errors_before: Option<usize>,
    pub corrected: Option<usize>,
    pub introduced: Option<usize>,
    pub residual: Option<usize>,
    pub recovery_rate: Option<f64>,
    pub error_rate_before: Option<f64>,
    pub error_rate_after: Option<f64>,
    pub fer: Option<f64>,
    pub mean_iterations: Option<f64>,
    pub encode_bits_per_second: Option<f64>,
    pub decode_bits_per_second: Option<f64>,
    pub status: &'static str,
    pub error: Option<String>,
    pub channel: ChannelKind,
    pub erasure_count: Option<usize>,
}

impl PointReport {
    pub fn new(
        codec: &str,
        channel: ChannelKind,
        count: usize,
        requested_trials: usize,
        result: Result<BenchmarkReport, String>,
    ) -> Self {
        let r = result.as_ref().ok();
        Self {
            codec: codec.into(),
            error_count: (channel == ChannelKind::BitFlips).then_some(count),
            error_unit: r.map(|r| r.error_unit),
            n: r.map(|r| r.codeword_len),
            payload_bits: r.map(|r| r.payload_bits),
            requested_trials,
            completed_trials: r.map(|r| r.counts.blocks),
            recovered_blocks: r.map(|r| r.counts.recovered),
            errors_before: r.and_then(|r| r.counts.before),
            corrected: r.and_then(|r| r.counts.after.map(|a| a.corrected)),
            introduced: r.and_then(|r| r.counts.after.map(|a| a.introduced)),
            residual: r.and_then(|r| r.counts.after.map(|a| a.residual)),
            recovery_rate: r.and_then(|r| r.block_rate(r.counts.recovered)),
            error_rate_before: r.and_then(|r| r.error_rate(r.counts.before)),
            error_rate_after: r.and_then(|r| r.error_rate(r.counts.after.map(|a| a.residual))),
            fer: r.and_then(|r| r.block_rate(r.counts.blocks - r.counts.recovered)),
            mean_iterations: r.and_then(|r| r.mean_iterations),
            encode_bits_per_second: r.and_then(|r| r.encode_bps),
            decode_bits_per_second: r.and_then(|r| r.decode_bps),
            status: if r.is_some() { "passed" } else { "failed" },
            error: result.err(),
            channel,
            erasure_count: (channel == ChannelKind::Erasures).then_some(count),
        }
    }
}

pub fn series(
    codec: &str,
    channel: ChannelKind,
    errors: &[usize],
    requested_trials: usize,
    mut run_point: impl FnMut(usize) -> Result<BenchmarkReport, String>,
) -> Vec<PointReport> {
    let mut points = Vec::with_capacity(errors.len());
    for &error_count in errors {
        let point = PointReport::new(
            codec,
            channel,
            error_count,
            requested_trials,
            run_point(error_count),
        );
        let failed = point.error.is_some();
        points.push(point);
        if failed {
            break;
        }
    }
    points
}

pub fn write_csv(writer: impl Write, points: &[PointReport]) -> Result<(), csv::Error> {
    let mut writer = csv::Writer::from_writer(writer);
    for point in points {
        writer.serialize(point)?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::{Counts, WordErrors};

    #[test]
    fn csv_preserves_metrics_escaping_and_completed_points_before_failure() {
        let mut calls = 0;
        let mut points = series(
            "codec,\"name\"\n",
            ChannelKind::BitFlips,
            &[1, 2, 3],
            2,
            |errors| {
                calls += 1;
                if errors == 2 {
                    return Err("API,\"failure\"\n".into());
                }
                Ok(BenchmarkReport {
                    counts: Counts {
                        blocks: 2,
                        recovered: 1,
                        before: Some(2),
                        after: Some(WordErrors {
                            corrected: 1,
                            introduced: 0,
                            residual: 1,
                        }),
                    },
                    error_unit: "bits",
                    codeword_len: 3,
                    payload_bits: 1,
                    mean_iterations: Some(2.5),
                    encode_bps: Some(64.0),
                    decode_bps: None,
                })
            },
        );
        assert_eq!((calls, points.len()), (2, 2));
        assert_eq!(points[0].recovery_rate, Some(0.5));
        assert_eq!(points[0].fer, Some(0.5));
        assert_eq!(points[0].error_rate_after, Some(1.0 / 6.0));
        let empty = BenchmarkReport {
            counts: Counts::default(),
            error_unit: "bits",
            codeword_len: 0,
            payload_bits: 0,
            mean_iterations: None,
            encode_bps: None,
            decode_bps: None,
        };
        points.push(PointReport::new(
            "empty",
            ChannelKind::Erasures,
            0,
            0,
            Ok(empty),
        ));
        let mut bytes = Vec::new();
        write_csv(&mut bytes, &points).unwrap();
        let mut reader = csv::Reader::from_reader(bytes.as_slice());
        let header = reader.headers().unwrap().clone();
        let rows: Vec<_> = reader.records().collect::<Result<_, _>>().unwrap();
        let cell = |row: usize, name: &str| {
            rows[row]
                .get(header.iter().position(|h| h == name).unwrap())
                .unwrap()
        };
        assert_eq!(cell(0, "codec"), "codec,\"name\"\n");
        assert_eq!(
            cell(0, "error_rate_before").parse::<f64>().unwrap(),
            1.0 / 3.0
        );
        assert_eq!(cell(0, "mean_iterations"), "2.5");
        assert_eq!(cell(0, "encode_bits_per_second"), "64.0");
        assert_eq!(cell(0, "decode_bits_per_second"), "");
        assert_eq!(cell(1, "status"), "failed");
        assert_eq!(cell(1, "error"), "API,\"failure\"\n");
        assert_eq!(cell(1, "completed_trials"), "");
        assert_eq!(cell(1, "error_rate_after"), "");
        for (name, value) in [
            ("channel", "erasures"),
            ("error_count", ""),
            ("erasure_count", "0"),
            ("recovery_rate", ""),
            ("error_rate_after", ""),
        ] {
            assert_eq!(cell(2, name), value);
        }
    }
}
