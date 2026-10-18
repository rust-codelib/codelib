//! Замер скорости кодирования, восстановления данных и узла.
//!
//! Запуск: `cargo run --release -p cascade-codes --example metrics`.

use std::time::Instant;

use cascade_codes::{CascadeCode, CascadeError, CascadeParams, DataDecoder, Encoder, NodeRepairer};
use gf2m::Gf256;

fn main() -> Result<(), CascadeError> {
    const ROUNDS: u32 = 20;
    println!(
        "{:<18} {:>6} {:>6} {:>6} {:>12} {:>12} {:>12}",
        "(n,k,d;mu)", "alpha", "beta", "F", "encode, мкс", "decode, мкс", "repair, мкс"
    );
    for (n, k, d, mu) in [
        (8, 4, 6, 1),
        (8, 4, 6, 2),
        (8, 4, 6, 4),
        (12, 5, 8, 3),
        (16, 4, 10, 4),
    ] {
        let params = CascadeParams::try_new(n, k, d, mu)?;
        let code = CascadeCode::<Gf256>::try_new(params)?;
        let message: Vec<Gf256> = (0..params.file_size())
            .map(|i| Gf256::new((i % 251) as u16))
            .collect();

        let start = Instant::now();
        let mut nodes = Vec::new();
        for _ in 0..ROUNDS {
            nodes = code.encode(&message)?;
        }
        let encode = start.elapsed() / ROUNDS;

        let start = Instant::now();
        for _ in 0..ROUNDS {
            let result = code.decode(&nodes[n - k..], None)?;
            assert_eq!(result.message(), &message[..]);
        }
        let decode = start.elapsed() / ROUNDS;

        let start = Instant::now();
        for _ in 0..ROUNDS {
            let payloads = nodes[1..=d]
                .iter()
                .map(|h| code.repair_payload(h, 0))
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(code.repair(0, &payloads, None)?, nodes[0]);
        }
        let repair = start.elapsed() / ROUNDS;

        println!(
            "{:<18} {:>6} {:>6} {:>6} {:>12} {:>12} {:>12}",
            format!("({n},{k},{d};{mu})"),
            params.alpha(),
            params.beta(),
            params.file_size(),
            encode.as_micros(),
            decode.as_micros(),
            repair.as_micros()
        );
    }
    Ok(())
}
