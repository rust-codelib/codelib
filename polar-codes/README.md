# polar-codes

Двоичные полярные коды: преобразование Арикана, выбор информационных позиций по каналу со стираниями и декодирование последовательным исключением.

Основной источник — лекция Сколтеха [Lecture 4. Polar Codes](https://sites.skoltech.ru/app/data/uploads/sites/52/2020/09/Lecture-4.-Polar-Codes.pdf).

Rust 1.81+ · `std` · `unsafe` запрещён.

Длина блока `N` — степень двойки от 1 до 4096. Порождающая матрица равна `G_N = F^{\otimes m}` над GF(2), где

```text
F = | 1 0 |
    | 1 1 |
```

Разворот битов `B_N` в произведение не входит: индексы `u` идут в естественном порядке фактора. Если в конспекте матрица записана как `B_N F^{\otimes m}`, индексы переводятся функцией `bit_reverse`. Замороженные биты источника равны нулю. Полярное преобразование обратно само себе.

Декодер оценивает `u` по канальным LLR. Положительное значение, включая `+0` и `-0`, означает бит 0; отрицательное — бит 1. f-функция по умолчанию точная, вариант `Combine::MinSum` — приближение из лекции. g-функция равна `L2 + (1 - 2û) L1`. Абсолютные LLR внутри декодера насыщаются на `10^6`.

## Быстрый старт

```rust
use polar_codes::{Bit, Combine, PolarCode, PolarError};

fn main() -> Result<(), PolarError> {
    let code = PolarCode::from_bec(4, 2, 0.5)?;
    let message = [Bit::One, Bit::Zero];
    let word = code.encode(&message)?;
    let llrs: Vec<f64> = word
        .iter()
        .map(|bit| match bit {
            Bit::Zero => 4.0,
            Bit::One => -4.0,
        })
        .collect();
    let decision = code.decode(&llrs, Combine::MinSum)?;
    assert_eq!(decision.message(), &message);
    assert_eq!(code.information_positions(), &[2, 3]);
    Ok(())
}
```

Для канала со стиранием вероятности `ε = 1/2` и `K = 2` надёжнее всего синтетические каналы `u2` и `u3`. Параметры Бхаттачарии считает `bhattacharyya_bec`: меньшее `Z` — более надёжный бит.

Кодирование и извлечение сообщения не обращаются к каналу. `extract_message` принимает слово только если после обратного преобразования замороженные биты равны нулю.

```sh
cargo run -p polar-codes --example round_trip
cargo run -p polar-codes --example recover
```
