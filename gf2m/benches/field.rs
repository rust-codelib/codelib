//! Бенчмарки полевого слоя (criterion).

use criterion::{criterion_group, criterion_main, Criterion};
use gf2m::{Gf256, GfRuntime};

fn field_benchmarks(c: &mut Criterion) {
    // GF(256): константно-генериковое умножение (clmul + редукция)
    c.bench_function("gf256/const_mul", |b| {
        b.iter(|| {
            let mut acc = Gf256::new(1);
            for i in 1..=255u16 {
                acc = acc.mul(Gf256::new(i));
            }
            acc
        })
    });

    // GF(256): табличное умножение
    c.bench_function("gf256/table_mul", |b| {
        b.iter(|| {
            let mut acc: u8 = 1;
            for i in 1..=255u8 {
                acc = gf2m::tables::gf256::mul(acc, i);
            }
            acc
        })
    });

    // Рантайм-слой, m = 8 (таблицы через диспетчер)
    c.bench_function("runtime/mul_m8", |b| {
        b.iter(|| {
            let mut acc = GfRuntime::new(8, 1).unwrap();
            for i in 1..=255u16 {
                acc = acc.mul(GfRuntime::new(8, i).unwrap());
            }
            acc
        })
    });

    // Инверсия: Ферма в константном ядре
    c.bench_function("gf256/const_inv", |b| {
        b.iter(|| {
            let mut acc = Gf256::new(1);
            for i in 1..=255u16 {
                acc = Gf256::new(i).inv().mul(acc);
            }
            acc
        })
    });

    // Инверсия: Ито–Цудзии в рантайм-слое
    c.bench_function("runtime/inv_m8", |b| {
        b.iter(|| {
            let mut acc = GfRuntime::new(8, 1).unwrap();
            for i in 1..=255u16 {
                acc = GfRuntime::new(8, i).unwrap().inv().mul(acc);
            }
            acc
        })
    });

    // Многочлены над GF(2): clmul и poly_mod
    c.bench_function("poly/clmul", |b| {
        b.iter(|| {
            let mut acc: u32 = 1;
            for i in 1..255u32 {
                acc = gf2m::clmul(acc, i);
            }
            acc
        })
    });

    // GF(65536): большое поле — только на целях с указателем >= 32 бит
    #[cfg(not(target_pointer_width = "16"))]
    c.bench_function("gf65536/const_mul", |b| {
        use gf2m::Gf65536;
        b.iter(|| {
            let mut acc = Gf65536::new(1);
            for i in 1..=4096u16 {
                acc = acc.mul(Gf65536::new(i));
            }
            acc
        })
    });
}

criterion_group!(benches, field_benchmarks);
criterion_main!(benches);
