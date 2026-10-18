//! Кодирование текста, потеря n - k узлов и восстановление по оставшимся.

use cascade_codes::{CascadeConfigurator, CascadeError};
use gf2m::Gf256;

fn main() -> Result<(), CascadeError> {
    let code = CascadeConfigurator::new(8, 4, 6).mode(2).build::<Gf256>()?;
    let p = code.params();
    println!(
        "(n, k, d; mu) = ({}, {}, {}; {}): alpha = {}, beta = {}, F = {}",
        p.n(),
        p.k(),
        p.d(),
        p.mu(),
        p.alpha(),
        p.beta(),
        p.file_size()
    );

    let text = "Каскадные коды: любые k узлов восстанавливают файл.".as_bytes();
    let file = code.encode_bytes(text)?;
    println!(
        "{} байт -> {} блок(а), по {} символов на узел",
        text.len(),
        file.stripes,
        file.shares[0].symbols.len()
    );

    // Узлы 0, 3, 4 и 6 отказали: остались 1, 2, 5, 7.
    let survivors: Vec<_> = [1, 2, 5, 7]
        .iter()
        .map(|&i| file.shares[i].clone())
        .collect();
    let result = code.decode_bytes(file.byte_len, &survivors, None)?;
    println!("статус: {:?}", result.status());
    println!("восстановлено: {}", String::from_utf8_lossy(result.bytes()));
    assert_eq!(result.bytes(), text);
    Ok(())
}
