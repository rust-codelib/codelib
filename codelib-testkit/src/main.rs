#![forbid(unsafe_code)]

mod config;
mod ldpc;
mod measure;
mod smoke;

use config::Config;
use std::{path::PathBuf, process::ExitCode};

fn load() -> Result<Option<Config>, String> {
    let mut args = std::env::args_os().skip(1);
    let path = match args.next() {
        None => PathBuf::from("configs/smoke.toml"),
        Some(arg) if arg == "--help" || arg == "-h" => {
            println!("codelib-testkit [--config PATH]\nDefault: configs/smoke.toml\nRUST_LOG overrides log_level.");
            return Ok(None);
        }
        Some(arg) if arg == "--config" => args
            .next()
            .map(PathBuf::from)
            .ok_or("--config requires a path")?,
        Some(arg) => return Err(format!("unknown argument: {}", arg.to_string_lossy())),
    };
    if args.next().is_some() {
        return Err("unexpected extra arguments".into());
    }
    let source = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Config::parse(&source)
        .map(Some)
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

fn main() -> ExitCode {
    let config = match load() {
        Ok(Some(config)) => config,
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
    match ldpc::run(&config) {
        Ok(report) => {
            show("ldpc-codes", Ok(()));
            let c = report.counts;
            println!("blocks recovered errors_before corrected introduced residual encode_Mbit/s decode_Mbit/s");
            println!(
                "{} {} {} {} {} {} {} {}",
                c.blocks,
                c.recovered,
                c.before,
                c.corrected,
                c.introduced,
                c.residual,
                rate(report.encode_bps),
                rate(report.decode_bps)
            );
        }
        Err(error) => {
            show("ldpc-codes", Err(error));
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
