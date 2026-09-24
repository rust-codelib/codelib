# gf2m — конечные поля GF(2^m)

Библиотека арифметики конечных полей GF(2^m) для m = 2..=16 на
чистом `core`: фундамент для кодов Рида–Соломона (крейт
[`rs-codes`](../rs-codes/)), BCH и другой теории кодирования.

## Слои

| Слой | Что даёт |
|---|---|
| `Gf<N, POLY>` | const-generic ядро: тип поля — параметры типа, операции `const fn` |
| `tables` | компилируемые log/exp-таблицы GF(256) и GF(65536): умножение за O(1) |
| `poly` | многочлены над GF(2) в машинном слове: `clmul`, `poly_mod`, `poly_degree` |
| `GfRuntime` | рантайм-слой: степень поля — параметр выполнения, обращение по Ито–Цудзии |

Коды Рида–Соломона поверх поля — отдельный крейт
[`rs-codes`](../rs-codes/) этого воркспейса.

## Быстрый старт

```rust
use gf2m::{Gf256, GfRuntime};

// Поле Рида–Соломона 0x11D (QR-коды)
let a = Gf256::new(0x80);
assert_eq!(a * Gf256::new(0x02), Gf256::new(0x1D));

// Степень поля во время выполнения
let b = GfRuntime::new(8, 0x57).unwrap();
let c = GfRuntime::new(8, 0x02).unwrap();
assert_eq!(b * c, GfRuntime::new(8, 0xAE).unwrap());
```

## Инварианты

- `#![no_std]` — только `core`, работает на микроконтроллерах;
- `#![forbid(unsafe_code)]` — ни одной строчки `unsafe`;
- без аллокаций — все структуры фиксированного размера;
- MSRV 1.81;
- ошибки через `Result`, арифметика не паникует
  (соглашения: `inv(0) = 0`, `x / 0 = 0`, `0^0 = 1`).

## Сборка и тесты

    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo doc --workspace --no-deps

## Лицензия

MIT OR Apache-2.0.
