# rs-codes — коды Рида–Соломона над GF(2^m)

Коды Рида–Соломона над конечными полями GF(2^m), скелет v0.1.0.

## Статус

Контракты зафиксированы, тела алгоритмических функций помечены
`todo!()`, приёмочные тесты конвейера — в `tests/reed_solomon.rs`
под `#[ignore]`. Публичные сигнатуры, `RsError`, `RsConfig` и
`KeyEquationSolver` заморожены и изменению не подлежат; реализация
тела функций — следующие PR.

## Конвейер

    generator_poly → encode_parity
    syndromes → (berlekamp_massey | euclid) → error_positions →
    error_magnitudes → correct

- `generator_poly` строит порождающий многочлен
  `g(x) = Π (x − α^(first_root + i))`;
- `encode_parity` вычисляет проверочные символы систематического
  кода: `parity = msg · x^nsym mod g`;
- `syndromes` считает `S_j = R(α^(first_root + j))`;
- `berlekamp_massey` или `euclid` решают ключевое уравнение:
  локатор ошибок Λ(x) и вычислитель Ω(x);
- `error_positions` (поиск Ченя) находит корни Λ — позиции ошибок;
- `error_magnitudes` (формула Форни) даёт величины ошибок;
- `correct` прогоняет весь конвейер и исправляет слово на месте.

## Дизайн

- без аллокаций: результаты пишутся в срезы вызывающего,
  `correct` принимает рабочий буфер `3·nsym + 2` элементов;
- `no_std`, `#![forbid(unsafe_code)]`;
- обобщение по полю: `Gf<N, POLY>` из [`gf2m`](../gf2m/) — любой
  тип поля без изменений в коде;
- отдельный `RsError` (`InvalidParameters`, `TooManyErrors`);
- решатель ключевого уравнения выбирается в `RsConfig`
  (`KeyEquationSolver::BerlekampMassey` или `Euclid`).

## Quick Start

Включить после реализации (сейчас тела — `todo!()`):

```text
use gf2m::Gf256;
use rs_codes::{correct, encode_parity, generator_poly, RsConfig};

let cfg = RsConfig::new(4, 0).unwrap(); // t = 2 ошибки
let mut gen = [Gf256::new(0); 5];
generator_poly(cfg.nsym, cfg.first_root, &mut gen).unwrap();

let msg = [Gf256::new(1); 251];
let mut parity = [Gf256::new(0); 4];
encode_parity(&msg, &gen, &mut parity).unwrap();

// кодовое слово: msg || parity; после внесения ошибок:
// let mut workspace = [Gf256::new(0); 14]; // 3·nsym + 2
// let fixed = correct(&mut word, &cfg, &mut workspace).unwrap();
```

## Поле

Арифметика — из крейта [`gf2m`](../gf2m/): константно-генериковое
ядро `Gf<N, POLY>`, log/exp-таблицы и рантайм-слой. Имя крейта —
`rs-codes`: `reed-solomon` на crates.io занят (mersinvald, 2018).

## Лицензия

MIT OR Apache-2.0.
