# gf-linalg

Библиотека векторов и прямоугольных матриц над конечными полями.

Основные типы — [`Vector<F>`](Vector) и [`Matrix<F>`](Matrix). Поле по умолчанию — `gf2m::Gf256`.

Rust 1.81+ · `std` · `unsafe` запрещён.

## Быстрый старт

Приложение в примере лежит рядом с каталогом `codelib`. Пути считаются относительно его `Cargo.toml`.

```toml
[dependencies]
gf-linalg = { path = "../codelib/gf-linalg" }
gf2m = { path = "../codelib/gf2m" }
```

Матрица хранит элементы по строкам: `[1, 2, 3, 4]` означает строки `[1, 2]` и `[3, 4]`.

```rust
use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, Vector};

fn main() -> Result<(), LinalgError> {
    let matrix = Matrix::try_new(2, 2, vec![
        Gf256::new(1), Gf256::new(2),
        Gf256::new(3), Gf256::new(4),
    ])?;
    let vector = Vector::new(vec![Gf256::one(), Gf256::new(2)]);
    let product = matrix.try_mul_vector(&vector)?;

    assert_eq!(product.as_slice(), &[Gf256::new(5), Gf256::new(11)]);
    Ok(())
}
```

## Операции

| Что | API | Условие |
|---|---|---|
| Векторы: сложение и вычитание | `try_add`, `try_sub`, `+`, `-` | одинаковая длина |
| Умножение вектора на скаляр | `scale`, `Vector * F` | скаляр из того же поля |
| Матрицы: сложение и вычитание | `try_add`, `try_sub`, `+`, `-` | одинаковая форма |
| Матрица × матрица | `try_mul`, `*` | столбцы слева = строки справа |
| Матрица × вектор | `try_mul_vector`, `*` | длина вектора = числу столбцов |
| Транспонирование | `transpose` | любая форма |
| Приведённый ступенчатый вид (RREF) и ранг | `rref`, `rank` | любая форма |
| Определитель и обратная матрица | `try_determinant`, `try_inverse` | квадратная матрица |

Операции с несовместимыми размерами возвращают ошибку в `Result`; `?` передаёт её вызывающему коду.
Запись со ссылками, например `&a + &b`, оставляет исходные значения доступными.

## Другие поля

`Matrix<F>` и `Vector<F>` работают с типом `F`, реализующим трейт [`FieldElement`] — контракт операций поля.
Поддержка статических `gf2m::Gf` и `gfpm::Gf` уже встроена.

Для примера ниже добавьте к зависимостям локальный путь `gfpm = { path = "../codelib/gfpm" }`.

```rust
use gf_linalg::{LinalgError, Matrix};
use gfpm::Gf;

type Gf2 = Gf<2, 1, 1>;

fn main() -> Result<(), LinalgError> {
    let matrix = Matrix::<Gf2>::try_new(2, 3, [1, 1, 0, 1, 0, 1].map(Gf2::new).to_vec())?;
    let reduced = matrix.rref();

    assert_eq!(reduced.matrix().rows(), 2);
    assert_eq!(reduced.pivot_columns(), &[0, 1]);
    assert_eq!(reduced.rank(), 2);
    Ok(())
}
```

RREF сохраняет форму и порядок столбцов, не изменяя исходную матрицу.

## Текст и коэффициенты

### Текстовый формат

`Display` и `FromStr` доступны для полей с [`FieldText`]. Формат элементов зависит от поля:

- `gf2m::Gf`: `0x` и фиксированное число шестнадцатеричных цифр (для `Gf256` — две).
- `gfpm::Gf`: упакованный код `value()` в десятичной записи; значение меньше `ORDER`.

```rust
use gf_linalg::{LinalgError, Matrix, Vector};

fn main() -> Result<(), LinalgError> {
    let matrix_text = "matrix 2 2\n0x01 0x02\n0x03 0x04";
    let matrix = matrix_text.parse::<Matrix>()?;
    assert_eq!(matrix.to_string(), matrix_text);

    let vector_text = "vector 3 0x07 0x00 0x00";
    let vector = vector_text.parse::<Vector>()?;
    assert_eq!(vector.to_string(), vector_text);
    Ok(())
}
```

### Коэффициенты

| Контейнер | В коэффициенты | Из коэффициентов |
|---|---|---|
| `Vector<F>` | `try_to_polynomial_coefficients` → `Result<Box<[F]>, _>` | `from_polynomial_coefficients` |
| `Matrix<F>` | `try_to_polynomial_rows` → `Result<Vec<Box<[F]>>, _>` | `try_from_polynomial_rows` |

Индекс `i` соответствует `x^i`; длина и хвостовые нули сохраняются. Отдельного типа `Polynomial`
и операций над многочленами в крейте нет.

## Ограничения

- У матрицы каждая ось в диапазоне `1..=4096` ([`MAX_MATRIX_DIM`]); длина данных — ровно `rows * cols`.
- Вектор может быть пустым; отдельного ограничения длины нет.
- `get`, `get_mut` и `row` возвращают `Option`: значение или ссылку в `Some`, а за границей — `None`.
- Определитель и обращение требуют квадратную матрицу. У вырожденной определитель равен нулю, обращение возвращает `SingularMatrix`.
- Каждый выходной массив коэффициентов ограничен 128 КиБ ([`MAX_POLYNOMIAL_BYTES`]), для матрицы — каждая строка отдельно. При импорте проверяется форма матрицы, лимит байтов не применяется.
- Скалярное произведение `try_dot` всегда возвращает `NotImplemented`.
- Операнды одной арифметической операции должны иметь одинаковый тип поля.

## Проверки

```text
cargo test -p gf-linalg
cargo doc -p gf-linalg --no-deps
```

Лицензия: MIT OR Apache-2.0.
