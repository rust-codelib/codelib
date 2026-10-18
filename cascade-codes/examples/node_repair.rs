//! Точное восстановление отказавшего узла по d помощникам.

use cascade_codes::{CascadeConfigurator, CascadeError, Encoder, EventLog, NodeRepairer};
use gfpm::Gf13;

fn main() -> Result<(), CascadeError> {
    // Пример из статьи: (n, k, d; mu) = (8, 4, 6; 4), MSR-точка.
    let code = CascadeConfigurator::new(8, 4, 6).build::<Gf13>()?;
    let message: Vec<Gf13> = (0..code.message_len() as u64)
        .map(|i| Gf13::new(i % 13))
        .collect();
    let nodes = code.encode(&message)?;

    let failed = 2;
    let helpers = [0, 1, 3, 4, 6, 7];
    let payloads = helpers
        .iter()
        .map(|&h| code.repair_payload(&nodes[h], failed))
        .collect::<Result<Vec<_>, _>>()?;

    let mut log = EventLog::default();
    let repaired = code.repair(failed, &payloads, Some(&mut log))?;
    for line in log.lines() {
        println!("{line}");
    }
    assert_eq!(repaired, nodes[failed]);

    let p = code.params();
    println!(
        "узел {failed} восстановлен: скачано {} символов вместо {} при полном декодировании",
        p.d() * p.beta(),
        p.k() * p.alpha()
    );
    Ok(())
}
