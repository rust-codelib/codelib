# Repairable Fountain Codes

Библиотечный модуль Rust (`std`, safe Rust, Rust 1.81 / edition 2021).
Сообщение до 4096 байт разбивается на исходные части. По любому дополнительному
ID создаётся проверочная часть — взвешенная сумма некоторых исходных частей.
Можно восстановить сообщение после потерь либо отдельно отремонтировать одну
часть по доступным помощникам, не восстанавливая весь блок.

Это реализация конструкции Asteris–Dimakis и точного локального
[профиля v1](../docs/FOUNTAIN-SPEC-V1.md). Генератор SplitMix64, выборы с возвращением,
нулевые коэффициенты и little-endian wire закреплены независимыми
[контрольными значениями](../docs/fixtures/fountain-v1.json).
Производственных зависимостей вне workspace нет. `serde_json` (MIT/Apache-2.0,
MSRV 1.71) используется только тестами для чтения сохранённого JSON.

## Быстрый старт

Параметры ниже экспериментальные, без гарантии заданной вероятности успеха.

```rust
use coding_core::{IncrementalDecoder, Message, Encoder, NoopObserver};
use fountain_codes::*;

let config = RepairableConfig::try_new(
    Profile::Gf256, 2, 5, 0x0123456789abcdef, ResourceLimits::default(),
)?;
let mut setup = FountainConfigurator::new(config);
let encoder = setup.encode(PrepareBlock {
    message: Message(&[0x80, 0xff, 0x34, 0x12, 0]), block_id: [0; 16],
}, &mut NoopObserver)?;
let descriptor = parse_descriptor(&encoder.descriptor().to_wire()?)?;
let mut decoder = RepairableDecoder::try_new(descriptor, ResourceLimits::default())?;
// Все исходные части (ID 0,1,2) потеряны; доступны только проверки.
for id in [3, 4, 5] {
    let received = parse_symbol(&encoder.symbol_unobserved(id)?.to_wire()?)?;
    decoder.push(&received, &mut NoopObserver)?;
}
assert_eq!(decoder.message(), Some([0x80, 0xff, 0x34, 0x12, 0].as_slice()));
# Ok::<(), FountainError>(())
```

Кодер владеет копией источника. Повтор ID воспроизводим независимо от порядка
вызовов. `symbols_from(u64::MAX)` выдаёт ровно один символ без переполнения.
Пустое сообщение немедленно восстановлено, но частей не имеет.
GF(65536) использует пары байтов LE и полином 0x1002D, GF(256) — один байт
и полином 0x11D. Нечётная длина и хвостовые нули сохраняются. Нули за исходной
длиной считаются padding — дополнением последней части.

## Общие интерфейсы

Трейт — общий договор вызовов. Все семь ролей используют существующий
[`coding-core`](../coding-core/README.md), включая `Observer` и `NoopObserver`.

| Тип | Трейт и результат |
|---|---|
| `FountainConfigurator` | `Encoder<PrepareBlock>` → `RepairableEncoder` |
| `RepairableEncoder` | `SymbolEncoder<u64>` → `EncodedSymbol` |
| `RepairableDecoder` | `IncrementalDecoder<&EncodedSymbol>` → `PushOutcome`; заимствованный `Candidate<Message<Vec<u8>>, RecoveryChecks>` |
| `RepairableBatchDecoder` | `Decoder<&[EncodedSymbol]>` → `Candidate` или `NeedMore`; `StopReason = Infallible` |
| `RepairCatalog` | `RepairPlanner<RepairRequest>` → `RepairPlan::Ready` или `Unavailable` |
| `StoredSymbol` | `RepairHelper<&HelperRequest>` → `HelperResponse` |
| `RepairCatalog` | `RepairExecutor<ExecuteRepair>` → `EncodedSymbol` |

Удобные методы с окончанием `_unobserved` вызывают эти же роли без логирования.
`repair_local` соединяет планировщик и исполнитель. `message_to_coefficients`
и `coefficients_to_message` переводят байты в существующий массив GF(256)
с сохранением длины и нулей; отдельный тип Polynomial не вводится.

## Приём и ошибки

Ранг — число независимых уравнений. Число принятых ID может быть больше ранга.
`NeedMoreSymbols` означает нехватку ранга; `Complete` означает полный ранг,
согласованность принятых частей и нулевой padding. Возвращается кандидат,
не доказательство совпадения с отправленным оригиналом. Проверять целостность
пакетов должен транспорт: модель канала — стирания, а не произвольные повреждения.
Wire v1 не содержит контрольной суммы или аутентификации.

Полный контекст проверяется раньше использования части. Обычные ошибки формы,
контекста, ресурсов и конфликт уже принятого ID сохраняют состояние.
Совпадающий дубликат разрешён даже при заполненном лимите. Противоречивый новый
ID или ненулевой padding полного решения переводит поток в `Invalid`;
`result()` и `message()` затем возвращают `None`. Части после `Complete`
также проверяются. Блочный декодер каждый раз начинает с нового состояния
и проверяет весь вход, в том числе после первого достижения полного ранга.

События декодера: `Accepted → RankIncreased → Completed` при завершении;
`Accepted → Dependent` для зависимой части; только `Duplicate` для повтора;
`Accepted → Verified` для новой согласованной части после завершения.
Противоречие даёт `Invalidated`; неверный padding решения перед ним уже даёт
`Accepted → RankIncreased`. Пустой блок не создаёт событий приёма.

## Локальный ремонт

В конечном `RepairCatalog` регистрируются проверочные ID. Для исходной цели
план включает одну доступную проверку и остальные исходные части её ненулевых
коэффициентов. Для проверочной цели нужны её исходные участники. Выбор
детерминирован: сначала меньше помощников, затем меньше ID проверки.
Порядок и повторы `available_ids` не влияют; сама цель не читается.

```rust
use coding_core::NoopObserver;
use fountain_codes::*;
let config = RepairableConfig::try_new(Profile::Gf256, 2, 5,
    0x0123456789abcdef, ResourceLimits::default())?;
let encoder = RepairableEncoder::try_new(&[0x80,0xff,0x34,0x12,0], [0;16], config)?;
let expected = encoder.symbol_unobserved(0)?; // Только проверяющему коду.
let mut catalog = RepairCatalog::try_new(encoder.descriptor().clone(), ResourceLimits::default())?;
catalog.register(4)?;
let store: std::collections::BTreeMap<_,_> = [1,2,4].into_iter()
    .map(|id| Ok((id, StoredSymbol::new(encoder.symbol_unobserved(id)?))))
    .collect::<Result<_, FountainError>>()?;
drop(encoder);
let repaired = repair_local(&catalog, RepairRequest { target: 0, available_ids: &[1,2,4] },
    |request| store[&request.helper_id()].respond_unobserved(request),
    &mut NoopObserver)?.unwrap();
assert_eq!(repaired, expected);
# Ok::<(), FountainError>(())
```

Исполнитель вызывает callback один раз на помощника в порядке ID плана.
Каждый ответ проверяется по полному descriptor, цели, помощнику и длине.
`HelperRead` учитывает фактически возвращённый payload, даже если затем он
отклонён. Ошибка callback не считается успешным чтением. `GroupSelected`
создаёт только исполнитель; `Unavailable` — только планировщик;
`HelperResponded` описывает вычисление ответа самим помощником, а не чтение
исполнителем. Нулевая проверка восстанавливается без чтений.
`Completed` содержит число помощников и сумму байтов payload; это не замер
физического сетевого трафика. `availability()` возвращает ID выбранных непересекающихся групп и итератор
`plans()` для получения планов по одному. Число найденных групп — нижняя
оценка, не гарантированный максимум.
Нет скрытого полного декодирования или рекурсивного ремонта.

## Формат и ресурсы

Descriptor занимает ровно 48 байт, пакет — `64 + T` байт, включая весь контекст.
Парсеры требуют точной длины, известных версии/профиля, правильного полинома
и нулевых reserved-полей. Сначала проверяются размеры, затем копируется payload.
Локальные лимиты в wire не передаются.

`ResourceLimits::try_new(draws, unique, entries, edges, bytes)` проверяет
положительность; стандартные значения: 256 / 16384 / 16384 / 1048576 /
67108864. Каждый компонент отдельно проверяет точный расчёт v1 через
`ResourceCharge::calculate`, до изменения состояния. Это расчёт содержательных
буферов, не RSS с аллокатором. GF(256) передаёт байт на элемент, но его Rust-тип
занимает два байта. Переполнение и превышение лимита — разные ошибки.
Полный базис и решение резервируются в расчёте уже при создании декодера.

Каждая ось матрицы до `gf_linalg::MAX_MATRIX_DIM`; матрица ширины K+L
не создаётся. 128 КиБ ограничивают только отдельное существующее преобразование
массива коэффициентов, не всю память кодека. Полное восстановление методом
Гаусса стоит до O(MK(K+L) + K²L) операций поля и O(K² + KL + MT) памяти;
при M порядка K это O(K³ + K²L). После завершения новые части проверяются
по решению без повторного исключения Гаусса.
При очень мелких частях большие K обходятся дорого; тест допустимости K=4096
не означает выполнения максимального исключения Гаусса или замера его скорости.

## Запуск

```sh
cargo run -p fountain-codes --example systematic_symbols
cargo run -p fountain-codes --example decode_with_losses
cargo run -p fountain-codes --example repair_one_symbol
cargo run -p fountain-codes --example repair_alternative_group
cargo run -p fountain-codes --example decode_with_log
```

[Воспроизводимые эксперименты и сохранённый проверочный запуск](../docs/FOUNTAIN-EXPERIMENTS.md).
Все числовые параметры примеров экспериментальные. Восстановление зависит
от ранга, а локальный ремонт — от доступности зарегистрированной группы.
