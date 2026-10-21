# Двоичные коды Кердока

`kerdock-codes` — параметризованный нелинейный блоковый код с жёстким
декодированием по расстоянию Хэмминга. Требуются Rust 1.81+ и `std`;
зависимости времени выполнения — локальные `gf2m` и `coding-core`.

## Быстрый старт

Из корня workspace:

```sh
cargo run -p kerdock-codes --example round_trip
```

Пример [`examples/round_trip.rs`](examples/round_trip.rs) кодирует ровно одно
12-битное сообщение и исправляет 13 различных позиций в 64-битном блоке:

```rust
use kerdock_codes::{Kerdock, KerdockError, KerdockSize};

fn main() -> Result<(), KerdockError> {
    let codec = Kerdock::new(KerdockSize::Bits64);
    // a = 5 (5 bits), b = 19 (6 bits), c = 1; least-significant bit first.
    let message = [
        true, false, true, false, false, true, true, false, false, true, false, true,
    ];
    let word = codec.encode(&message)?;
    assert_eq!(word.len(), 64);
    assert_eq!(codec.extract_message(&word)?, message);

    let positions = [0, 3, 7, 11, 16, 20, 25, 31, 38, 44, 50, 57, 63];
    assert_eq!(positions.len(), codec.correction_radius());
    let mut received = word.clone();
    for position in positions {
        received[position] = !received[position];
    }

    let decoded = codec.decode(&received)?;
    assert_eq!(decoded.message(), message);
    assert_eq!(decoded.word(), word);
    assert_eq!(decoded.distance(), positions.len());
    println!("Восстановлены 12 битов сообщения; исправлено 13 ошибок в блоке из 64 битов.");
    Ok(())
}
```

Один вызов обрабатывает **один битовый блок**, а не поток байтов. Вход —
заимствованный `&[bool]` (`false = 0`, `true = 1`), выход владеет данными.
Входные срезы не изменяются. Упаковки в байты, разбиения потока, дополнения,
мягких решений и стираний в API нет.

## Параметры и API

Поддерживаются только чётные `m` из `4..=10`. Здесь `n = 2^m`, `k = 2m`,
`d = (n - 2^(m/2))/2`, `t = floor((d-1)/2)`:

| `m` | Длина слова `n`, бит | Сообщение `k`, бит | Число слов `n²` | Расстояние `d` | Радиус `t` |
| --- | --- | --- | --- | --- | --- |
| 4 | 16 | 8 | 256 | 6 | 2 |
| 6 | 64 | 12 | 4096 | 28 | 13 |
| 8 | 256 | 16 | 65536 | 120 | 59 |
| 10 | 1024 | 20 | 1048576 | 496 | 247 |

`k` — информационная ёмкость, **не размерность двоичного линейного пространства**.
Кодирование не систематическое: сообщение нельзя просто прочитать из префикса
слова. Для извлечения служит `extract_message`.

Размер задаётся вариантами `KerdockSize::{Bits16, Bits64, Bits256, Bits1024}`.
Для длины, известной во время выполнения, используйте
`Kerdock::new(KerdockSize::try_from(bit_length)?)`: преобразование принимает
длину кодового слова **в битах**, не показатель `m` и не число байтов.
Конструктор `Kerdock::new` не возвращает ошибку параметра.

| API | Назначение |
| --- | --- |
| `Kerdock::new(size: KerdockSize) -> Kerdock` | Подготовить неизменяемый кодек |
| `KerdockSize::try_from(bit_length: usize) -> Result<KerdockSize, KerdockError>` | Проверить длину кодового слова в битах |
| `exponent() -> u32` | Показатель `m` |
| `message_len()`, `codeword_len() -> usize` | Длины `k`, `n` |
| `minimum_distance()`, `correction_radius() -> usize` | Параметры `d`, `t` |
| `encode(&[bool]) -> Result<Vec<bool>, KerdockError>` | Закодировать ровно `k` битов |
| `extract_message(&[bool]) -> Result<Vec<bool>, KerdockError>` | Извлечь сообщение только из точного кодового слова длины `n` |
| `decode(&[bool]) -> Result<DecodeResult, KerdockError>` | Найти ближайшее слово, принять только на расстоянии не больше `t` |
| `DecodeResult::message() -> &[bool]` | Восстановленное сообщение |
| `DecodeResult::word() -> &[bool]` | Восстановленное кодовое слово |
| `DecodeResult::distance() -> usize` | Расстояние от принятого до возвращённого слова |

Результат декодирования владеет сообщением и словом; последующие вызовы кодека
его не меняют. Методы работают через `&self` и используют локальные буферы.

## Shared coding-core block interfaces

`Kerdock` also implements `coding_core::Encoder<&[bool]>` and
`coding_core::Decoder<&[bool]>`, preserving the inherent methods above.
Trait and inherent method names collide: importing the traits alone is not
enough to select them. Use a generic trait bound or fully-qualified calls:

```rust
use coding_core::{Decoder, Encoder, NoopObserver};
use kerdock_codes::{Kerdock, KerdockError, KerdockSize};

fn main() -> Result<(), KerdockError> {
    let mut codec = Kerdock::new(KerdockSize::Bits16);
    let message = [true, false, false, false, false, false, false, false];
    let word = <Kerdock as Encoder<&[bool]>>::encode(
        &mut codec, &message, &mut NoopObserver,
    )?;
    let outcome = <Kerdock as Decoder<&[bool]>>::decode(
        &mut codec, &word.0, &mut NoopObserver,
    )?;
    println!("{outcome:?}");
    Ok(())
}
```

| Shared associated type | Value |
| --- | --- |
| Encoder `Output` | `coding_core::Codeword<Vec<bool>>` |
| Decoder `Value` | `DecodeResult` (owns message and recovered word) |
| Decoder `Checks` | `KerdockChecks { distance: usize, radius: usize }` |
| Decoder `NeedMore` | `core::convert::Infallible` |
| Decoder `StopReason` | `KerdockStopReason` |
| Both `Error` | `KerdockError` |
| Both `Event` | `()` (no events emitted) |

The shared decoder returns
`Result<DecodeOutcome<DecodeResult, KerdockChecks, Infallible, KerdockStopReason>, KerdockError>`:

- `Candidate` contains the inherent result and the actual executed distance-bound
  check: `checks.distance == value.distance()` and `checks.radius` is the code's
  correction radius. Passing the check does not prove identity with the
  transmitted original or measure the unknown channel error count.
- `Stopped(OutsideCorrectionRadius { nearest_distance, radius })` means no word
  lies within the radius. The nearest distance is exact; outside-radius ties
  also stop. In contrast, direct `codec.decode(&input)` still returns
  `Err(KerdockError::NoCodewordWithinRadius { .. })` for the same block.
- Malformed input remains `Err` with the original length error fields; no
  partial candidate is returned. All errors other than radius rejection are
  preserved. `NeedMore` cannot be constructed: this is complete-block decoding,
  not streaming or accumulation.

Shared methods take `&mut self` only to satisfy the traits. Each call is
independent, retaining no per-call state. No observer events are emitted on
success, stop, or error; choosing an observer cannot change the result. Inputs
and previous owned outputs remain unchanged on all paths. Allocation and `std`
requirements are unchanged; using a `no_std` interface does not make this codec
`no_std`.

The fixed message capacities 8/12/16/20 bits occupy 1/2/2/3 **transmitted bytes**
(rounding the last byte up), not the Rust storage size of `bool` elements.
Every valid configuration therefore satisfies `coding_core::MAX_MESSAGE_BYTES`
(4096) for both original and recovered messages. Any other encoder length,
including more than 32768 bits, is rejected as `MessageLengthMismatch` before
output allocation; no extra packing or arbitrary message-length mode is added.

Run [`examples/shared_codec.rs`](examples/shared_codec.rs) for candidate, known
midpoint stop (`0x02c0`, distance 3, radius 2), and malformed-input error handling:

```sh
cargo run -p kerdock-codes --example shared_codec
```

## Порядок битов и координат

Пусть `s = m-1`, `F = GF(2^s)` в полиномиальном базисе
`1, z, ..., z^(s-1)`. Полные маски модулей закреплены:

| `s` | Маска | Многочлен |
| --- | --- | --- |
| 3 | `0xB` | `z³ + z + 1` |
| 5 | `0x25` | `z⁵ + z² + 1` |
| 7 | `0x83` | `z⁷ + z + 1` |
| 9 | `0x211` | `z⁹ + z⁴ + 1` |

Сообщение содержит последовательно:

1. `s` коэффициентов элемента `a` поля `F`, младший коэффициент первым;
2. `m` битов аффинной маски `b`, младший бит первым;
3. один бит константы `c`.

Координаты идут по индексам `j = 0, ..., n-1`: младшие `s` битов индекса
задают `x ∈ F`, бит `s` — `u ∈ GF(2)`. Упорядоченный базис пространства
`V = F × GF(2)` равен `(1,0), (z,0), ..., (z^(s-1),0), (0,1)`.
Нулевое сообщение даёт нулевое слово; смена только `c` инвертирует всё слово.
Это точный контракт данной реализации. Эквивалентные коды Magma/GUAVA могут
иметь другую нумерацию координат и сообщений: побитовая/байтовая совместимость
без отдельного отображения не заявляется.

## Конструкция и расстояние

Абсолютный след `Tr(q) = q + q² + ... + q^(2^(s-1))` принимает значения в
`GF(2)`. Для каждого `a ∈ F` задаём билинейную форму (все суммы — XOR):

```text
B_a((x,u),(y,v)) = Tr(a²xy) + Tr(ax)Tr(ay) + uTr(ay) + vTr(ax).
Q_a(w) = Σ_{i<j} B_a(e_i,e_j) w_i w_j.
word[j] = Q_a(j) XOR parity(b & j) XOR c.
```

У `Q_a` нет линейной части, `Q_0 = 0`. Добавление всех аффинных функций к
`n/2` квадратичным представителям даёт `(n/2) · n · 2 = n²` слов.

Почему расстояние действительно равно заявленному:

- Формы `B_a` знакопеременны: `Tr(a²x²) = Tr(ax)` сокращается с
  `Tr(ax)²`, а два слагаемых с `u` на диагонали сокращаются между собой.
- При `a ≠ a'` положим `h = a+a'`. Вектор `(x,u)` из радикала
  `B_a+B_a'` удовлетворяет `Tr(hx)=0` (подстановка `(0,1)`), поэтому
  `Tr(ax)=Tr(a'x)=r`. Подстановка всех `(y,0)` и невырожденность следового
  спаривания дают `h²x+hr+uh=0`, то есть `hx=r+u ∈ GF(2)`.
  Так как `s` нечётно, след двоичного скаляра равен ему самому; значит,
  `hx=0`, затем `x=r=u=0`. Разность форм невырождена.
- Спектр Уолша невырожденной двоичной квадратичной формы имеет модули
  `sqrt(n)`. Поэтому расстояния между разными квадратичными классами равны
  `(n ± sqrt(n))/2`. Внутри класса ненулевая аффинная разность имеет вес
  `n/2` или `n`. Отсюда различность всех слов и минимальное расстояние `d`.

Это общий алгоритм для всех поддерживаемых размеров, а не отдельные кодовые
книги. Теория и альтернативные представления приведены в источниках ниже.

## Декодирование, ресурсы и границы гарантии

Для каждого `a` строится `f[j] = (-1)^(received[j] XOR Q_a(j))`.
Целочисленное быстрое преобразование Уолша–Адамара (FWHT) вычисляет
`W[b] = Σ_j f[j] (-1)^parity(b & j)`. Оптимальная константа — `c=0` при
`W[b] >= 0`, иначе `c=1`; расстояние равно `(n - |W[b]|)/2`.
Поиск по всем `a,b` даёт глобально ближайшее слово, которое принимается
только при расстоянии `<= t`. В этом радиусе оно единственно, поскольку `2t < d`.
Ничьи вне радиуса также отклоняются. `extract_message` использует тот же поиск,
но требует расстояние ровно ноль и не исправляет даже одиночную ошибку.

Квадратичные строки хранятся упакованными в `u64`; при `m=10` полезная
нагрузка таблицы — `512 · 1024` битов, то есть **64 КиБ**, без метаданных
и рабочих буферов. Полная книга из `n²` слов в производственном коде не
создаётся. Кодирование занимает `O(n)`, декодирование — `O(n² log n)`
операций и `O(n)` дополнительной памяти. FWHT использует `i32`, все значения
ограничены `n <= 1024`; вычислений с плавающей точкой нет. Подготовка
квадратичных коэффициентов и оценка форм требуют порядка `O(n²m²)` работы.
Предел `m=10` — ресурсное ограничение реализации, а не математическое;
лимит массивов `gf-linalg` к этой таблице не относится. Ошибка выделения
памяти следует обычному поведению `Vec`, отдельного fallible-allocation API нет.

При не более `t` инверсиях битов переданное сообщение восстанавливается точно.
Если ошибок больше, принятое слово может оказаться в радиусе другого кодового
слова, и декодирование успешно вернёт **другое сообщение**. Успех и `distance()`
не доказывают, что было передано; расстояние относится к возвращённому слову,
а не к неизвестному исходному. Отказ означает отсутствие слова в радиусе,
а не определение точного числа ошибок канала.

## Ошибки

`KerdockError` реализует `Debug`, `Clone`, `PartialEq`, `Eq`, `Display` и
`std::error::Error`:

| Вариант | Причина |
| --- | --- |
| `UnsupportedBlockLength { bits }` | Длина в битах не равна 16, 64, 256 или 1024; возвращается из `KerdockSize::try_from` |
| `MessageLengthMismatch { expected, actual }` | Длина сообщения не равна `k` |
| `WordLengthMismatch { expected, actual }` | Длина входа `decode`/`extract_message` не равна `n` |
| `InvalidCodeword` | Точное извлечение получило слово вне кода |
| `NoCodewordWithinRadius { nearest_distance, radius }` | Ближайшее слово дальше `t`; указано фактическое ближайшее расстояние |

Пустые входы также отклоняются по длине. Недопустимых значений битов у `bool` нет.

## Источники

- [Error Correction Zoo: Kerdock code](https://errorcorrectionzoo.org/c/kerdock) — параметры, квадратичные формы и декодирование преобразованием Адамара.
- [A. M. Kerdock (1972), A class of low-rate nonlinear binary codes](https://doi.org/10.1016/S0019-9958(72)90376-2).
- [Hammons et al., The Z4-Linearity of Kerdock, Preparata, Goethals and Related Codes](https://arxiv.org/abs/math/0207208) — альтернативная четверичная конструкция.
- [Magma handbook](https://www.math.uzh.ch/sepp/magma-2.20.4-cr/html/text1810.htm) — отображение Грея и октакод; показатель в `KerdockCode` следует другой конвенции.
- [GAP/GUAVA](https://docs.gap-system.org/pkg/guava/doc/chap5.html) — конструкция Нордстрома–Робинсона.
