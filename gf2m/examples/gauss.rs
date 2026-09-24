//! Решение линейной системы над GF(256) методом Гаусса–Жордана
//! на стековых массивах, без аллокаций.

use gf2m::Gf256;

/// Решает систему `A·x = b`, где `a` — расширенная матрица 4×5
/// (последний столбец — правая часть). Возвращает `None`, если
/// система вырождена.
fn solve(a: &mut [[Gf256; 5]; 4]) -> Option<[Gf256; 4]> {
    const N: usize = 4;
    for col in 0..N {
        // Выбор ненулевого ведущего элемента
        let piv = (col..N).find(|&r| !a[r][col].is_zero())?;
        a.swap(col, piv);
        // Нормализация строки
        let inv = a[col][col].inv();
        for v in &mut a[col][col..=N] {
            *v *= inv;
        }
        // Исключение в остальных строках
        for r in 0..N {
            if r != col && !a[r][col].is_zero() {
                let f = a[r][col];
                let pivot = a[col]; // Copy: строка маленькая и неизменна здесь
                for (dst, src) in a[r][col..=N].iter_mut().zip(&pivot[col..=N]) {
                    *dst += *src * f;
                }
            }
        }
    }
    Some([a[0][N], a[1][N], a[2][N], a[3][N]])
}

fn main() {
    // Заготовка: известное решение x = [1, 2, 3, 4]
    let x = [Gf256::new(1), Gf256::new(2), Gf256::new(3), Gf256::new(4)];

    // Матрица коэффициентов (невырожденная) и правая часть A·x
    let rows: [[Gf256; 4]; 4] = [
        [
            Gf256::new(0x01),
            Gf256::new(0x02),
            Gf256::new(0x03),
            Gf256::new(0x05),
        ],
        [
            Gf256::new(0x53),
            Gf256::new(0xCA),
            Gf256::new(0x57),
            Gf256::new(0x13),
        ],
        [
            Gf256::new(0x0F),
            Gf256::new(0x10),
            Gf256::new(0x20),
            Gf256::new(0x40),
        ],
        [
            Gf256::new(0x80),
            Gf256::new(0x81),
            Gf256::new(0x82),
            Gf256::new(0x83),
        ],
    ];
    let mut a: [[Gf256; 5]; 4] = [[Gf256::new(0); 5]; 4];
    for (i, row) in rows.iter().enumerate() {
        let mut acc = Gf256::new(0);
        for j in 0..4 {
            a[i][j] = row[j];
            acc += row[j] * x[j];
        }
        a[i][4] = acc;
    }

    match solve(&mut a) {
        Some(sol) => {
            print!("решение:");
            for v in sol {
                print!(" {v}");
            }
            println!();
            assert_eq!(sol, x, "решение должно совпасть с заготовкой");
            println!("сходится: A·x = b выполнено");
        }
        None => println!("система вырождена"),
    }
}
