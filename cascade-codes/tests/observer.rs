//! Журналирование событий кодека.

mod common;

use cascade_codes::{
    CascadeEvent, CascadeObserver, DataDecoder, DecodeStatus, Encoder, EventLog, NodeRepairer,
};
use common::{code, message};
use gfpm::Gf13;

#[derive(Default)]
struct Recorder {
    segments: Vec<(usize, usize)>,
    finished: Option<(DecodeStatus, Vec<usize>)>,
    repair_symbols: usize,
    downloaded: Option<usize>,
}

impl CascadeObserver for Recorder {
    fn on_event(&mut self, event: &CascadeEvent<'_>) {
        match event {
            CascadeEvent::SegmentRecovered { segment, mode } => {
                self.segments.push((*segment, *mode))
            }
            CascadeEvent::DecodeFinished {
                status,
                suspect_nodes,
            } => {
                self.finished = Some((*status, suspect_nodes.to_vec()));
            }
            CascadeEvent::RepairSegment {
                symbols_per_helper, ..
            } => {
                self.repair_symbols += symbols_per_helper;
            }
            CascadeEvent::RepairFinished { downloaded, .. } => self.downloaded = Some(*downloaded),
            _ => {}
        }
    }
}

#[test]
fn decode_reports_segments_bottom_up() {
    let code = code::<Gf13>(8, 4, 6, 4);
    let msg = message::<Gf13>(code.message_len(), 1);
    let nodes = code.encode(&msg).unwrap();
    let mut rec = Recorder::default();
    code.decode(&nodes[..4], Some(&mut rec)).unwrap();

    assert_eq!(rec.segments.len(), code.segment_count());
    // Алгоритм 2 идёт от режима 0 к корню.
    let modes: Vec<usize> = rec.segments.iter().map(|&(_, m)| m).collect();
    assert!(modes.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(rec.segments.last(), Some(&(0, 4)));
    assert_eq!(rec.finished, Some((DecodeStatus::Consistent, vec![])));
}

#[test]
fn repair_reports_bandwidth() {
    let code = code::<Gf13>(8, 4, 6, 3);
    let msg = message::<Gf13>(code.message_len(), 2);
    let nodes = code.encode(&msg).unwrap();
    let payloads: Vec<_> = (1..=6)
        .map(|h| code.repair_payload(&nodes[h], 0).unwrap())
        .collect();
    let mut rec = Recorder::default();
    code.repair(0, &payloads, Some(&mut rec)).unwrap();
    assert_eq!(rec.repair_symbols, code.params().beta());
    assert_eq!(rec.downloaded, Some(6 * code.params().beta()));
}

#[test]
fn event_log_collects_text_lines() {
    let code = code::<Gf13>(6, 3, 4, 2);
    let msg = message::<Gf13>(code.message_len(), 3);
    let nodes = code.encode(&msg).unwrap();
    let mut log = EventLog::default();
    code.decode(&nodes, Some(&mut log)).unwrap();
    // Одна строка на событие: начало, сегменты, попытка, итог.
    let mut rec = Recorder::default();
    code.decode(&nodes, Some(&mut rec)).unwrap();
    assert_eq!(log.lines().len(), rec.segments.len() + 3);
    assert!(log.lines().iter().all(|line| !line.is_empty()));
}

#[test]
fn no_events_on_invalid_input() {
    let code = code::<Gf13>(6, 3, 4, 2);
    let msg = message::<Gf13>(code.message_len(), 4);
    let nodes = code.encode(&msg).unwrap();
    let mut log = EventLog::default();
    assert!(code.decode(&nodes[..1], Some(&mut log)).is_err());
    assert!(log.lines().is_empty());
}
