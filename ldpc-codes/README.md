# ldpc-codes

Библиотека для двоичных LDPC-кодов: разреженная проверочная матрица, систематическое кодирование и итерационное декодирование SPA.

Rust 1.81+ · `std` · `unsafe` запрещён.

## Быстрый старт

Если приложение находится рядом с каталогом `codelib`, добавьте локальную зависимость:

```toml
[dependencies]
ldpc-codes = { path = "../codelib/ldpc-codes" }
```

Этот пример строит согласованные кодер и декодер, кодирует один бит, декодирует LLR, проверяет статус и только затем извлекает сообщение:

```rust
use ldpc_codes::{
    Bit, DecodeInput, DecodeStatus, DecoderConfig, Encoder, LdpcConfigurator,
    LdpcError, ParityCheckMatrix,
};

fn main() -> Result<(), LdpcError> {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])?;
    let mut code = LdpcConfigurator::build(checks, DecoderConfig::try_new(1, 20.0)?)?;

    let message = [Bit::One];
    let encoded = code.encoder().encode(&message)?;
    let llrs = [-2.0, 1.0, -3.0];
    let result = code.decode(
        DecodeInput { llrs: &llrs, erasures: &[] },
        None,
    )?;

    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    let recovered = code.encoder().extract_message(result.word())?;
    assert_eq!(recovered, message);
    assert_eq!(result.word(), encoded);
    Ok(())
}
```

Методы `encode` и `extract_message` заданы трейтом [`Encoder`]. Пара `ConfiguredLdpc` предоставляет собственный `decode`, чтобы её внутренний декодер оставался согласован с кодером. `?` возвращает вызывающему коду ошибку из `Result`, если операция не удалась.

## Операции

| Задача | API |
|---|---|
| Создать бит и преобразовать биты в [`gf_linalg::Vector`] над [`Gf2`] | [`Bit`], [`bits_to_vector`], [`vector_to_bits`] |
| Создать `H`, получить смежности и вычислить синдром | `ParityCheckMatrix::try_from_rows`, `ParityCheckMatrix::check_bits`, `ParityCheckMatrix::bit_checks`, `ParityCheckMatrix::syndrome`, `ParityCheckMatrix::is_codeword` |
| Кодировать и извлекать сообщение | `Encoder::encode`, `Encoder::extract_message`, `SystematicEncoder::information_positions` |
| Создать согласованные кодер и декодер | `LdpcConfigurator::build`, `ConfiguredLdpc::encoder`, `ConfiguredLdpc::decode` |
| Декодировать блок или выполнить один flooding-шаг | `Decoder::decode`, `spa_step` |
| Получить статус, счётчики и события | [`DecodeResult`], [`DecodeStatus`], [`DecodeObserver`], [`DecodeEvent`] |
| Сравнить результат с известным эталоном | [`count_bit_errors`] |

В `ParityCheckMatrix::try_from_rows(n, rows)` параметр `n` задаёт число столбцов, а каждый список в `rows` перечисляет индексы единиц одной проверки. Здесь `m` — число строк, а `E` — общее число единиц матрицы.

Операции с некорректным входом возвращают `Err(LdpcError)` в `Result`; входные срезы не забираются и не меняются.

## LLR, стирания и остановка

Для LLR используется знак `L = ln(P(0) / P(1))`: `L >= 0` выбирает `Bit::Zero`, отрицательное значение — `Bit::One`. Ноль не задаёт предпочтения и даёт `Zero`. [`DecoderConfig::default()`] задаёт 50 итераций и предел LLR `20.0`; [`DecoderConfig::try_new`] принимает конечный предел из `(0, 20]` и бюджет от 0. Входные LLR и внутренние сообщения SPA насыщаются этим пределом. NaN и бесконечность отклоняются даже на стираемой позиции.

В [`DecodeInput`] позиции стирания передаются индексами. Стирание означает неизвестный бит: его LLR становится `0.0`; значение не инвертируется. Длина LLR, конечность значений, границы индексов и повторы проверяются до декодирования.

Декодер сначала проверяет синдром начального жёсткого решения. Нулевой синдром даёт [`DecodeStatus::ParitySatisfied`] с 0 итераций, даже при бюджете 0; иначе SPA работает до первого нулевого синдрома или исчерпания бюджета. [`DecodeStatus::IterationLimit`] — обычный результат, а не ошибка входа. `ParitySatisfied` говорит, что слово удовлетворяет `H`, но не гарантирует совпадение с переданным словом.

[`spa_step`] выполняет один шаг с новым рабочим состоянием при каждом вызове и не применяет бюджет итераций. Полный [`SpaDecoder`] выполняет цикл до остановки по синдрому или бюджету.

## Векторы и диагностика

[`bits_to_vector`] и [`vector_to_bits`] преобразуют биты в [`gf_linalg::Vector<Gf2>`] и обратно, сохраняя порядок, длину и хвостовые нули. `Gf2` — псевдоним `gfpm::Gf<2, 1, 1>`. Сообщение кодера — блок из `k = n - rank(H)` битов; `message[j]` помещается в `word[information_positions()[j]]`, и информационные позиции не обязательно идут первыми. Предел 4096 байт из требований относится к будущему байтовому слою: здесь API не упаковывает биты и не разбивает длинное сообщение на блоки.

[`DecodeResult`] предоставляет `word()`, `posterior_llrs()`, `syndrome()`, `status()`, `iterations()`, `initial_unsatisfied_checks()` и `final_unsatisfied_checks()`. `changed_bits()` считает отличия от начального жёсткого решения; это не число исправленных ошибок. Для собственного `impl Decoder` результат создаётся через `DecodeResult::try_new(&checks, initial_word, posterior_llrs, status, iterations, iteration_limit)`: итоговое слово выводится из знаков LLR, а синдром и счётчики вычисляются по `H`. Конструктор проверяет формы, конечность LLR, статус и бюджет итераций, но не может подтвердить историю внешнего алгоритма — например, соответствие `initial_word` канальным значениям или факт выполнения каждой заявленной итерации. [`count_bit_errors`] требует равные длины: кодовое слово длины `n` сравнивают с эталонным словом, а сообщение длины `k` — с эталонным сообщением. Декодер обычно эталона не знает.

События [`DecodeEvent`] приходят синхронно через [`DecodeObserver`]: начало, завершённая итерация и остановка. Передайте `Some(&mut observer)` в `decode`; `None` отключает события. При ошибке входа события не отправляются.

```rust
use ldpc_codes::{
    DecodeEvent, DecodeInput, DecodeObserver, Decoder, DecoderConfig, LdpcError,
    ParityCheckMatrix, SpaDecoder,
};

struct Log;
impl DecodeObserver for Log {
    fn on_event(&mut self, event: &DecodeEvent) {
        println!("{event:?}");
    }
}

fn main() -> Result<(), LdpcError> {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])?;
    let mut decoder = SpaDecoder::try_new(checks, DecoderConfig::default())?;
    let llrs = [2.0, -3.0, 4.0];
    let mut log = Log;
    let _result = decoder.decode(
        DecodeInput { llrs: &llrs, erasures: &[] },
        Some(&mut log),
    )?;
    Ok(())
}
```

## Ограничения

- Каждая ось [`ParityCheckMatrix`] имеет размер `1..=4096`. Пустые и повторяющиеся строки разрешены; повтор индекса внутри строки запрещён. Матрица хранит `O(m + n + E)` связей.
- Систематическому кодеру и конфигуратору нужен ранг `0 < rank(H) < n`. Зависимые строки учитываются через ранг; информационные позиции могут быть не первыми битами слова.
- [`SpaDecoder`] работает с любой допустимой матрицей, но SPA может не сойтись или вернуть другое допустимое кодовое слово.
- Общий тип полинома и его ещё не выбранный API, байтовая упаковка и предел исходного сообщения в байтах относятся к будущей интеграции. Сейчас кодек принимает только один блок битов.

## Примеры и проверки

Пять запускаемых примеров показывают синдром, кодирование, один шаг SPA, журнал и полный round-trip:

```sh
cargo run -p ldpc-codes --example syndrome
cargo run -p ldpc-codes --example systematic_encode
cargo run -p ldpc-codes --example spa_step
cargo run -p ldpc-codes --example decode_with_log
cargo run -p ldpc-codes --example decode_round_trip
```

Проверка doctest и документации workspace:

```sh
cargo test -p ldpc-codes
cargo test -p ldpc-codes --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Лицензия: MIT OR Apache-2.0.
