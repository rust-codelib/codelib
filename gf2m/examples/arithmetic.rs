//! Арифметика GF(256): редукция из спецификации QR, степени
//! примитивного элемента, корни и следы.

use gf2m::Gf256;

fn main() {
    println!("GF(256), образующий многочлен 0x11D (Рид–Соломон, QR)");

    // Редукция из спецификации QR: t^8 = t^4 + t^3 + t^2 + 1
    let a = Gf256::new(0x80);
    println!("0x80 · 0x02 = {}", a * Gf256::new(0x02));
    println!("0x80⁻¹      = {}", a.inv());

    // Степени примитивного элемента α = t
    let alpha = Gf256::alpha();
    let mut p = Gf256::new(1);
    print!("степени α: {p}");
    for _ in 0..11 {
        p *= alpha;
        print!(" {p}");
    }
    println!(" …");
    println!("α^255 = {}", alpha.pow(255));

    // Квадратные корни и следы
    println!("{:>8} {:>8} {:>8} {:>8}", "x", "sqrt(x)", "sqrt^2", "trace");
    for v in [3u16, 7, 53, 128, 200, 255] {
        let x = Gf256::new(v);
        let r = x.sqrt();
        println!(
            "{:>8} {:>8} {:>8} {:>8}",
            x,
            r,
            r.sqr(),
            if x.trace().is_zero() { "0" } else { "1" }
        );
    }
}
