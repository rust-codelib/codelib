#![forbid(unsafe_code)]

mod config;
mod kerdock;
mod ldpc;
mod smoke;

use codelib_testkit::{
    report::{self, PointReport},
    ChannelKind,
};
use config::Config;
use std::{ffi::OsString, path::PathBuf, process::ExitCode};

struct Options {
    config: Config,
    csv: Option<PathBuf>,
}

fn load(mut args: impl Iterator<Item = OsString>) -> Result<Option<Options>, String> {
    let mut path = None;
    let mut csv = None;
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!("codelib-testkit [--config PATH] [--csv PATH]\nDefault: configs/smoke.toml\nRUST_LOG overrides log_level.");
            return Ok(None);
        }
        let slot = if arg == "--config" {
            &mut path
        } else if arg == "--csv" {
            &mut csv
        } else {
            return Err(format!("unknown argument: {}", arg.to_string_lossy()));
        };
        if slot.is_some() {
            return Err(format!("duplicate argument: {}", arg.to_string_lossy()));
        }
        *slot = Some(PathBuf::from(
            args.next()
                .filter(|path| !path.to_string_lossy().starts_with('-'))
                .ok_or_else(|| format!("{} requires a path", arg.to_string_lossy()))?,
        ));
    }
    let path = path.unwrap_or_else(|| PathBuf::from("configs/smoke.toml"));
    let source = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(csv) = &csv {
        match same_file::is_same_file(&path, csv) {
            Ok(true) => {
                return Err("CSV output must differ from config (including file aliases)".into())
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(format!("{}: {error}", csv.display()));
            }
            _ => {}
        }
    }
    Config::parse(&source)
        .map(|config| Some(Options { config, csv }))
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn show(name: &str, result: Result<(), String>) -> bool {
    match result {
        Ok(()) => {
            println!("{name:<16} passed");
            log::info!("{name}: passed");
            true
        }
        Err(error) => {
            println!("{name:<16} failed: {error}");
            log::error!("{name}: {error}");
            false
        }
    }
}

fn rate(value: Option<f64>) -> String {
    value
        .map(|bps| format!("{:.3}", bps / 1e6))
        .unwrap_or_else(|| "—".into())
}

fn number<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}

fn decimal(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v:.6}"))
        .unwrap_or_else(|| "—".into())
}

fn show_series(points: &[PointReport]) -> bool {
    if let Some(p) = points.first() {
        println!(
            "{}: unit={} n={} payload_bits={}",
            p.codec,
            p.error_unit.unwrap_or("—"),
            number(p.n),
            number(p.payload_bits)
        );
    }
    let count_name = if points
        .first()
        .is_some_and(|p| p.channel == ChannelKind::Erasures)
    {
        "erasures"
    } else {
        "errors"
    };
    println!("{count_name} trials recovered corrected BER/SER_after FER iterations encode_Mbit/s decode_Mbit/s status");
    for p in points {
        println!(
            "{} {} {} {} {} {} {} {} {} {}",
            number(p.error_count.or(p.erasure_count)),
            number(p.completed_trials),
            number(p.recovered_blocks),
            number(p.corrected),
            decimal(p.error_rate_after),
            decimal(p.fer),
            decimal(p.mean_iterations),
            rate(p.encode_bits_per_second),
            rate(p.decode_bits_per_second),
            p.status
        );
        if let Some(error) = &p.error {
            show(
                &format!(
                    "{} {count_name}={}",
                    p.codec,
                    number(p.error_count.or(p.erasure_count))
                ),
                Err(error.clone()),
            );
        } else {
            log::info!(
                "{} {count_name}={}: passed",
                p.codec,
                number(p.error_count.or(p.erasure_count))
            );
        }
    }
    points.iter().all(|p| p.error.is_none())
}

fn main() -> ExitCode {
    let Options { config, csv } = match load(std::env::args_os().skip(1)) {
        Ok(Some(options)) => options,
        Ok(None) => return ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("codelib-testkit: {error}");
            return ExitCode::from(2);
        }
    };
    if let Err(error) = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&config.log_level),
    )
    .try_init()
    {
        eprintln!("codelib-testkit: {error}");
        return ExitCode::from(2);
    }
    log::info!(
        "seed={}, warmup={}, repetitions={}",
        config.seed,
        config.warmup,
        config.repetitions
    );
    let mut failed = false;
    for (name, check) in [
        ("gf2m", smoke::gf2m as fn() -> Result<(), String>),
        ("gfpm", smoke::gfpm),
        ("gf-linalg", smoke::linalg),
        ("rs-config", smoke::rs_config),
    ] {
        failed |= !show(name, check());
    }
    println!("rs-codec         skipped: encode/decode not implemented");
    log::warn!("RS encode/decode algorithms are not implemented");
    let mut points = Vec::new();
    for (name, channel, counts, run) in [
        (
            "ldpc-codes",
            ChannelKind::BitFlips,
            &config.ldpc.error_counts,
            ldpc::run as fn(&Config) -> Result<Vec<PointReport>, String>,
        ),
        (
            "kerdock-codes",
            ChannelKind::BitFlips,
            &config.kerdock.error_counts,
            kerdock::run,
        ),
    ] {
        let series = run(&config).unwrap_or_else(|error| {
            vec![PointReport::new(
                name,
                channel,
                counts.points()[0],
                config.repetitions,
                Err(error),
            )]
        });
        failed |= !show_series(&series);
        points.extend(series);
    }
    if let Some(path) = csv {
        let result = std::fs::File::create(&path)
            .map_err(csv::Error::from)
            .and_then(|file| report::write_csv(file, &points));
        if let Err(error) = result {
            eprintln!("codelib-testkit: {}: {error}", path.display());
            return ExitCode::from(2);
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
