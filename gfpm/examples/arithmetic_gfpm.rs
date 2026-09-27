//! Арифметика GF(9) и GF(125): степени примитивного элемента,
//! обратные, след и норма, корни; рантайм-слой с найденным модулем.

use gfpm::{FieldParams, Gf9, GfRuntime};

fn main() {
    println!("GF(9) = GF(3)[x]/(x^2 + x + 2), примитивный модуль");

    // Степени примитивного элемента α = x: полный цикл порядка 8
    let alpha = Gf9::alpha();
    let mut p = Gf9::one();
    print!("степени α: {p}");
    for _ in 0..8 {
        p *= alpha;
        print!(" {p}");
    }
    println!(" …");
    println!("α^8 = {}", alpha.pow(8));

    // Обратные: x · (x + 1) = x^2 + x = 1
    println!("inv(x)     = {} (= x + 1)", alpha.inv());
    println!("x · inv(x) = {}", alpha * alpha.inv());

    // След и норма лежат в простом подполе GF(3)
    println!(
        "{:>8} {:>8} {:>8} {:>10} {:>8}",
        "x", "inv", "trace", "norm", "sqrt"
    );
    for v in [0u64, 1, 2, 3, 5, 7, 8] {
        let x = Gf9::new(v);
        let r = x
            .sqrt()
            .map(|r| r.to_string())
            .unwrap_or_else(|| "—".into());
        println!(
            "{:>8} {:>8} {:>8} {:>10} {:>8}",
            x,
            x.inv(),
            x.trace(),
            x.norm(),
            r
        );
    }

    // Рантайм-слой: GF(125), модуль находится автоматически
    let params = FieldParams::find(5, 3).unwrap();
    println!(
        "\nGF(125) = GF(5)[x]/({}): модуль найден поиском Рабина",
        params.poly()
    );
    let a = GfRuntime::new(params, 7).unwrap(); // x + 2
    let b = GfRuntime::new(params, 5).unwrap(); // x
    println!("{a} · {b} = {}", a * b);
    println!("inv({a}) = {}", a.inv());
    println!("norm({a}) = {}, trace({a}) = {}", a.norm(), a.trace());
    // коэффициенты и упакованное представление
    println!(
        "коэффициенты a: {:?}, упакованно: {}",
        a.coeffs(),
        a.value()
    );
}
