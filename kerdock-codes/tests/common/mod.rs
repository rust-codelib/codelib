// Independent defining-form oracle: carryless polynomial multiplication with
// long division, no gf2m calls and no production quadratic tables.

pub fn message(m: u32, a: usize, b: usize, c: bool) -> Vec<bool> {
    (0..m - 1)
        .map(|i| a & (1 << i) != 0)
        .chain((0..m).map(|i| b & (1 << i) != 0))
        .chain([c])
        .collect()
}

fn multiply(x: usize, y: usize, s: u32, modulus: usize) -> usize {
    let mut product = 0;
    for i in 0..s {
        if y & (1 << i) != 0 {
            product ^= x << i;
        }
    }
    for i in (s..2 * s - 1).rev() {
        if product & (1 << i) != 0 {
            product ^= modulus << (i - s);
        }
    }
    product
}

fn trace(mut x: usize, s: u32, modulus: usize) -> bool {
    let mut sum = 0;
    for _ in 0..s {
        sum ^= x;
        x = multiply(x, x, s, modulus);
    }
    assert!(sum <= 1, "absolute trace must lie in GF(2)");
    sum == 1
}

pub fn reference_word(m: u32, message: &[bool]) -> Vec<bool> {
    let s = m - 1;
    let modulus = match s {
        3 => 0xB,
        5 => 0x25,
        7 => 0x83,
        9 => 0x211,
        _ => panic!("unsupported reference field"),
    };
    assert_eq!(message.len(), 2 * m as usize);
    let a = message[..s as usize]
        .iter()
        .rev()
        .fold(0, |value, &bit| (value << 1) | usize::from(bit));
    let field_mask = (1 << s) - 1;
    let bilinear = |left: usize, right: usize| {
        let ax = multiply(a, left & field_mask, s, modulus);
        let ay = multiply(a, right & field_mask, s, modulus);
        let tx = trace(ax, s, modulus);
        let ty = trace(ay, s, modulus);
        trace(multiply(ax, ay, s, modulus), s, modulus)
            ^ (tx & ty)
            ^ ((left >> s != 0) & ty)
            ^ ((right >> s != 0) & tx)
    };
    (0..1 << m)
        .map(|coordinate| {
            // Q(v + e_i) = Q(v) + B(v,e_i), since Q(e_i) = 0.
            // Accumulate selected basis vectors rather than storing coefficients.
            let mut prefix = 0;
            let mut bit = message[2 * m as usize - 1];
            for i in 0..m {
                let basis = 1 << i;
                if coordinate & basis != 0 {
                    bit ^= bilinear(prefix, basis);
                    bit ^= message[(s + i) as usize];
                    prefix ^= basis;
                }
            }
            bit
        })
        .collect()
}
