//! Подсчёт расхождений и поиск испорченного узла при избыточных узлах.

use cascade_codes::{CascadeConfigurator, CascadeError, DataDecoder, Encoder};
use gf2m::Gf256;

fn main() -> Result<(), CascadeError> {
    let code = CascadeConfigurator::new(8, 4, 6).mode(3).build::<Gf256>()?;
    let message: Vec<Gf256> = (0..code.message_len())
        .map(|i| Gf256::new((i * 37 % 256) as u16))
        .collect();
    let mut nodes = code.encode(&message)?;

    // Портим два символа узла 1 — он попадает в первую попытку декодирования.
    nodes[1].symbols[0] += Gf256::new(0x55);
    nodes[1].symbols[7] += Gf256::new(0x01);

    let result = code.decode(&nodes, None)?;
    println!("статус: {:?}", result.status());
    println!("подозрительные узлы: {:?}", result.suspect_nodes());
    println!("несовпавших символов: {}", result.mismatched_symbols());
    println!(
        "использованы узлы {:?}, попыток: {}",
        result.used_nodes(),
        result.subsets_tried()
    );
    assert_eq!(result.message(), &message[..]);
    Ok(())
}
