//! Решение линейной системы над GF(49) методом Гаусса–Жордана
//! на стековых массивах, без аллокаций — рантайм-слоем gfpm.

use gfpm::{FieldParams, GfRuntime};

/// Решает систему `A·x = b`; `a` — расширенная матрица 3×4
/// (последний столбец — правая часть). `None` — система вырождена.
fn solve(a: &mut [[GfRuntime; 4]; 3]) -> Option<[GfRuntime; 3]> {
    const N: usize = 3;
    for col in 0..N {
        // выбор ненулевого ведущего элемента
        let piv = (col..N).find(|&r| !a[r][col].is_zero())?;
        a.swap(col, piv);
        // нормировка строки
        let inv = a[col][col].inv();
        for v in &mut a[col][col..=N] {
            *v *= inv;
        }
        // исключение в остальных строках: row_r −= f·row_col
        for r in 0..N {
            if r != col && !a[r][col].is_zero() {
                let f = a[r][col];
                let pivot_row = a[col]; // Copy: маленькая строка
                for (dst, src) in a[r][col..=N].iter_mut().zip(&pivot_row[col..=N]) {
                    *dst -= *src * f;
                }
            }
        }
    }
    Some([a[0][N], a[1][N], a[2][N]])
}

fn main() {
    // GF(49) = GF(7)[x]/(x^2 + 1) — модуль найден поиском (Рабин)
    let params = FieldParams::find(7, 2).unwrap();
    let e = |v: u64| GfRuntime::new(params, v).unwrap();

    // Заготовка: известное решение x = [1, x, x + 3]
    let x = [e(1), e(7), e(10)];

    // Невырожденная матрица и правая часть A·x
    let rows: [[GfRuntime; 3]; 3] = [
        [e(1), e(2), e(3)],
        [e(7), e(11), e(13)], // [x, x+4, x+6]
        [e(4), e(5), e(6)],
    ];
    let mut a: [[GfRuntime; 4]; 3] = [[e(0); 4]; 3];
    for i in 0..3 {
        let mut acc = e(0);
        for j in 0..3 {
            a[i][j] = rows[i][j];
            acc += rows[i][j] * x[j];
        }
        a[i][3] = acc;
    }

    match solve(&mut a) {
        Some(sol) => {
            print!("решение:");
            for v in &sol {
                print!(" {v}");
            }
            println!();
            assert_eq!(sol, x, "решение должно совпасть с заготовкой");
            println!("сходится: A·x = b выполнено");
        }
        None => println!("система вырождена"),
    }
}
