# bch-codes

Библиотека двоичных примитивных узкосмысловых BCH-кодов: построение
порождающего многочлена, систематическое кодирование и ограниченное
декодирование с алгоритмом Берлекэмпа—Мэсси и поиском Ченя.

Rust 1.81+ · `no_std` · `unsafe` запрещён · аллокации доступны опционально.

## Быстрый старт

Если приложение находится рядом с каталогом `codelib`, добавьте локальную
зависимость:

```toml
[dependencies]
bch-codes = { path = "../codelib/bch-codes" }
coding-core = { path = "../codelib/coding-core" }
```

`OwnedBchCodec` — простой интерфейс с аллокациями. Пользователь задаёт длину
сообщения в битах и число исправляемых ошибок; поле, примитивный полином,
порождающий полином и укорочение библиотека выбирает сама.

```rust
use bch_codes::OwnedBchCodec;
use coding_core::DecodeOutcome;

let codec = OwnedBchCodec::new(4, 1)?;
let message = [0b1010];
let encoded = codec.encode(&message)?;

let mut received = encoded.clone();
received[0] ^= 1 << 5;

let outcome = codec.decode(&received)?;
let DecodeOutcome::Candidate(candidate) = outcome else {
    panic!("одна ошибка должна исправляться");
};

assert_eq!(candidate.value.0, message);
assert_eq!(candidate.checks.corrected_bits, 1);
# Ok::<(), bch_codes::BchError>(())
```

Биты упакованы в байты младшим битом вперёд: позиция $j$ соответствует
коэффициенту при $x^j$. Неиспользуемые старшие биты последнего байта должны
быть нулевыми.

## Два уровня API

| Уровень | Основные типы и функции | Управление памятью |
|---|---|---|
| Простой | `OwnedBchCodec`, `OwnedBchDecodeInput` | порождающий полином хранит кодек, результаты и workspace выделяются через `Vec` |
| Низкоуровневый | `BchCodec`, `build_generator`, `encode_systematic`, `correct` | все выходные и рабочие буферы предоставляет вызывающий код |

Feature `alloc` включён по умолчанию и открывает владеющий API. Без него крейт
остаётся полностью без аллокаций:

```sh
cargo check -p bch-codes --no-default-features
```

Низкоуровневый слой использует `Gf<N, POLY>` из [`gf2m`](../gf2m/). Он нужен,
когда поле выбирается явно, память должна быть заранее выделена или код работает
на embedded-цели без аллокатора.

## Автоматический выбор кода

Для сообщения длины $k$ и исправляющей способности $t$ `OwnedBchCodec`
перебирает стандартные поля от $GF(2^2)$ до $GF(2^{16})$. Для каждого поля
полная длина равна

$$
n_0 = 2^m - 1.
$$

После построения порождающего многочлена степени $r$ полная информационная
длина равна

$$
k_0 = n_0-r.
$$

Выбирается первое поле, для которого $k \leq k_0$. Код автоматически
укорачивается на

$$
s = k_0-k
$$

позиций, поэтому передаваемое слово содержит

$$
n = k+r
$$

битов. Удаляются старшие информационные позиции; пользователь не передаёт
параметр укорочения самостоятельно.

## Кодирование и декодирование

Код систематический: проверочные биты занимают младшие полиномиальные позиции,
а информационные следуют за ними. `encode` возвращает упакованное кодовое слово,
`decode` исправляет копию принятого слова и извлекает сообщение.

Декодирование выполняет следующий конвейер:

```text
синдромы → Берлекэмп—Мэсси → локатор ошибок → поиск Ченя → проверка синдромов
```

Успешный результат — `DecodeOutcome::Candidate` с `BchChecks`:

- `initial_syndromes_zero` показывает, было ли принятое слово уже кодовым;
- `corrected_bits` содержит число инвертированных позиций;
- `final_syndromes_zero` подтверждает повторную проверку после исправления.

Если согласованный кандидат не найден, декодер возвращает
`DecodeOutcome::Stopped(BchStopReason)`. Это обычный исход декодирования, а не
ошибка использования API. Некорректные параметры, размеры, padding и рабочая
память возвращаются через `Result::Err(BchError)`.

`Candidate` означает найденное допустимое кодовое слово, но не доказывает его
совпадение с отправленным оригиналом. BCH-код гарантированно исправляет все
ошибки веса не больше $t$; при большем числе ошибок возможны остановка,
неисправимый результат или принятие другого кодового слова.

## Унифицированный API

Оба уровня интегрированы с [`coding-core`](../coding-core/). `BchCodec`
принимает `BchDecodeInput` с рабочей памятью вызывающего кода.
`OwnedBchCodec` принимает `OwnedBchDecodeInput` и выделяет workspace внутри
каждого вызова.

```rust
use bch_codes::{BchEncodeInput, OwnedBchCodec, OwnedBchDecodeInput};
use coding_core::{Codeword, DecodeOutcome, Decoder, Encoder, NoopObserver};

let mut codec = OwnedBchCodec::new(4, 1)?;
let mut encoded = [0u8; 1];
let Codeword(word) = Encoder::encode(
    &mut codec,
    BchEncodeInput {
        message: &[0b1010],
        output: &mut encoded,
    },
    &mut NoopObserver,
)?;

word[0] ^= 1 << 5;
let outcome = Decoder::decode(
    &mut codec,
    OwnedBchDecodeInput { received: word },
    &mut NoopObserver,
)?;
assert!(matches!(outcome, DecodeOutcome::Candidate(_)));
# Ok::<(), bch_codes::BchError>(())
```

Унифицированный декодер исправляет переданный буфер на месте только при успехе.
Собственный метод `OwnedBchCodec::decode` входной срез не меняет.

## Ограничения

- Один экземпляр кодека обрабатывает блок с фиксированными $k$ и $t$.
- Поддерживаются стандартные поля $GF(2^m)$ для $m=2,\ldots,16$.
- Длина сообщения ограничена требованиями `coding-core`: не более 4096
  передаваемых байт.
- `correctable_errors` должен быть положительным и допустимым для хотя бы одного
  поддерживаемого поля.
- Библиотека обрабатывает один BCH-блок и не разбивает длинный поток на блоки.
- Декодер работает с жёсткими двоичными решениями; мягкие значения и стирания
  не входят в этот API.

## Проверки

```sh
cargo test -p bch-codes
cargo clippy -p bch-codes --all-targets --all-features -- -D warnings
cargo check -p bch-codes --no-default-features
cargo doc -p bch-codes --no-deps
```

Лицензия: MIT OR Apache-2.0.
