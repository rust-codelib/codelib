//! Таблица точек компромисса (alpha, beta, F) для системы (n, k, d).
//!
//! Запуск: `cargo run -p cascade-codes --example tradeoff -- 8 4 6`.

use cascade_codes::{CascadeError, CascadeParams};

fn main() -> Result<(), CascadeError> {
    let args: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let (n, k, d) = match args[..] {
        [n, k, d] => (n, k, d),
        _ => (8, 4, 6),
    };
    println!("(n, k, d) = ({n}, {k}, {d})");
    println!(
        "{:>3} {:>6} {:>6} {:>7} {:>10} {:>10}",
        "mu", "alpha", "beta", "F", "alpha/F", "d*beta/F"
    );
    for p in CascadeParams::tradeoff(n, k, d)? {
        let f = p.file_size() as f64;
        let tag = if p.is_mbr() {
            " MBR"
        } else if p.is_msr() {
            " MSR"
        } else {
            ""
        };
        println!(
            "{:>3} {:>6} {:>6} {:>7} {:>10.4} {:>10.4}{tag}",
            p.mu(),
            p.alpha(),
            p.beta(),
            p.file_size(),
            p.alpha() as f64 / f,
            (p.d() * p.beta()) as f64 / f
        );
    }
    Ok(())
}
