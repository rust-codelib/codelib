//! Бенчмарки полевого слоя (criterion).

use criterion::{criterion_group, criterion_main, Criterion};
use gfpm::{FieldParams, Gf25, Gf65537, GfRuntime};

fn field_benchmarks(c: &mut Criterion) {
    // GF(65537): константно-генериковое умножение (M = 1)
    c.bench_function("gf65537/const_mul", |b| {
        b.iter(|| {
            let mut acc = Gf65537::new(1);
            for i in 1..=255u64 {
                acc = acc.mul(Gf65537::new(i));
            }
            acc
        })
    });

    // GF(65537): инверсия по Ферма. Результат накапливается
    // умножением — измеряются все 255 инверсий, а не только последняя
    c.bench_function("gf65537/const_inv", |b| {
        b.iter(|| {
            let mut acc = Gf65537::new(1);
            for i in 1..=255u64 {
                acc = acc.mul(Gf65537::new(i).inv());
            }
            acc
        })
    });

    // GF(25): константно-генериковое умножение (M = 2, редукция)
    c.bench_function("gf25/const_mul", |b| {
        b.iter(|| {
            let mut acc = Gf25::new(1);
            for i in 1..=24u64 {
                acc = acc.mul(Gf25::new(i));
            }
            acc
        })
    });

    // Рантайм-слой, GF(9)
    c.bench_function("runtime/gf9_mul", |b| {
        let params = FieldParams::new(3, 2, 5).unwrap();
        b.iter(|| {
            let mut acc = GfRuntime::one(params);
            for i in 1..=8u64 {
                acc = acc.mul(GfRuntime::new(params, i).unwrap());
            }
            acc
        })
    });

    // Рантайм-слой, GF(65537): инверсия (с накоплением — см. const_inv)
    c.bench_function("runtime/gf65537_inv", |b| {
        let params = FieldParams::new(65537, 1, 65534).unwrap();
        b.iter(|| {
            let mut acc = GfRuntime::one(params);
            for i in 1..=255u64 {
                acc = acc.mul(GfRuntime::new(params, i).unwrap().inv());
            }
            acc
        })
    });
}

criterion_group!(benches, field_benchmarks);
criterion_main!(benches);
