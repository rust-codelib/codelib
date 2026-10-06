# gf-linalg

`gf-linalg` — пакет с библиотечным крейтом (библиотекой Rust). В исходном коде библиотеку импортируют как `gf_linalg`. Он предоставляет `Vector<F>` и прямоугольную `Matrix<F>` над элементами одного статического конечного поля `F`, безопасное чтение и замену элементов, операции над векторами и матрицами, RREF и ранг, а также определитель и обратную матрицу для квадратных матриц. Тип поля по умолчанию — `gf2m::Gf256`.

## Выбор поля

Параметр типа `F` задаёт тип каждого элемента. `Vector<Gf9>` содержит только
элементы `Gf9`, а `Matrix<Gf256>` — только `Gf256`; Rust проверяет это при
компиляции и отклоняет смешивание полей в одной операции. В аннотации типа без
`<F>` `Vector` и `Matrix` используют `Gf256` для совместимости с прежним API.
Непустой конструктор может вывести `F` из типа элементов `Vec`. Пустой `Vec` не
сообщает компилятору тип элемента, поэтому для пустой коллекции нужно явно
указать поле в аннотации или turbofish:

```rust
use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, Vector};
use gfpm::Gf9;

fn main() -> Result<(), LinalgError> {
    let default_field: Vector = Vector::new(vec![Gf256::new(7)]);
    let inferred_gf9 = Vector::new(vec![Gf9::one()]);
    let empty_gf9 = Vector::<Gf9>::new(Vec::new());
    assert!(empty_gf9.is_empty());
    assert_eq!(default_field.as_slice(), &[Gf256::new(7)]);
    assert_eq!(inferred_gf9.as_slice(), &[Gf9::one()]);

    // В характеристике 3 вычитание отличается от сложения.
    let left = Vector::<Gf9>::new(vec![Gf9::new(1), Gf9::new(2)]);
    let right = Vector::<Gf9>::new(vec![Gf9::new(2), Gf9::new(1)]);
    let difference = left.try_sub(&right)?;
    assert_eq!(difference.as_slice(), &[Gf9::new(2), Gf9::new(1)]);
    assert_eq!((&left - &right)?, difference);

    let matrix = Matrix::<Gf9>::try_new(
        2,
        2,
        vec![Gf9::one(), Gf9::one(), Gf9::one(), Gf9::zero()],
    )?;
    assert_eq!(matrix.try_determinant()?, Gf9::new(2));
    let inverse = matrix.try_inverse()?;
    let identity = Matrix::<Gf9>::try_new(
        2,
        2,
        vec![Gf9::one(), Gf9::zero(), Gf9::zero(), Gf9::one()],
    )?;
    assert_eq!(matrix.try_mul(&inverse)?, identity);
    assert_eq!(inverse.try_mul(&matrix)?, identity);

    // Формат элемента выбирается по FieldText для указанного типа поля.
    let text = matrix.to_string();
    assert_eq!(text, "matrix 2 2\n1 1\n1 0");
    let parsed: Matrix<Gf9> = text.parse()?;
    assert_eq!(parsed, matrix);

    // Это массив данных, а не отдельный тип Polynomial.
    let rows = matrix.try_to_polynomial_rows()?;
    assert_eq!(rows[0].as_ref(), &[Gf9::one(), Gf9::one()]);
    assert_eq!(Matrix::try_from_polynomial_rows(rows)?, matrix);
    Ok(())
}
```

`FieldElement` задаёт арифметику, необходимую общим алгоритмам: ноль, единицу,
обратный элемент, сравнение с нулём и операторы `+`, `-`, `*`, унарный `-`.
Реализации обязаны соблюдать законы поля; компилятор проверяет наличие
операций, но не доказывает их математические свойства. Обе готовые статические
семьи автоматически поддерживаются для типов `gf2m::Gf<N, POLY>` и
`gfpm::Gf<P, M, POLY>`, чьи параметры задают поля с неприводимым модулем.
Адаптер для каждого порядка отдельно не нужен. `Gf256` и `Gf9` в примерах
показывают разную характеристику; тесты также используют другие параметры.

Типы с полем, параметры которого выбираются во время выполнения, сюда не
входят: для каждого типа `F` нужен один фиксированный набор параметров.
Например, `gf2m::GfRuntime` требует отдельного контекста и проверки
совместимости параметров.

### Подключение собственного типа поля

Для своего типа нужно реализовать `FieldElement` и требуемые им арифметические
операторы. Обычно для этого используют обёртку newtype — собственную структуру
с одним внутренним значением. Одной реализации `FieldElement` достаточно для
векторов, матриц и массивов коэффициентов; `Display`, `FieldText` и операторы
присваивания вроде `AddAssign` для этого не нужны. Строковый ввод и вывод
появятся только после отдельной реализации `FieldText`.

```rust
use core::ops::{Add, Mul, Neg, Sub};
use gf_linalg::{FieldElement, LinalgError, Vector};
use gfpm::Gf9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MyField(Gf9);

impl Add for MyField {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { Self(self.0 + rhs.0) }
}
impl Sub for MyField {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Self(self.0 - rhs.0) }
}
impl Mul for MyField {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self { Self(self.0 * rhs.0) }
}
impl Neg for MyField {
    type Output = Self;
    fn neg(self) -> Self { Self(-self.0) }
}
impl FieldElement for MyField {
    fn zero() -> Self { Self(Gf9::zero()) }
    fn one() -> Self { Self(Gf9::one()) }
    fn inv(self) -> Self {
        assert!(!self.0.is_zero());
        Self(self.0.inv())
    }
}

fn main() -> Result<(), LinalgError> {
    let vector = Vector::<MyField>::new(vec![MyField(Gf9::one())]);
    let sum = vector.try_add(&vector)?;
    let coefficients = sum.try_to_polynomial_coefficients()?;
    assert_eq!(coefficients.as_ref(), &[MyField(Gf9::new(2))]);
    Ok(())
}
```

Реализация `inv` должна возвращать обратный элемент только для ненулевого
входа. Алгоритмы `gf-linalg` проверяют ведущий элемент до вызова `inv`; поведение
пользовательского поля для `inv(0)` не используется.

```rust
use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, Vector};

fn main() -> Result<(), LinalgError> {
    // Vector::new принимает Vec во владение без копирования элементов.
    // После передачи vector_data больше нельзя использовать.
    let vector_data = vec![Gf256::new(7), Gf256::zero(), Gf256::zero()];
    let vector = Vector::new(vector_data);
    assert_eq!(vector.len(), 3); // Хвостовые нули входят в длину.
    assert_eq!(vector.as_slice()[2], Gf256::zero());

    // Для матрицы 2 × 3 передаём ровно шесть элементов по строкам.
    let data = vec![
        Gf256::new(1), Gf256::new(2), Gf256::new(3),
        Gf256::new(4), Gf256::new(5), Gf256::zero(),
    ];
    let mut matrix = Matrix::try_new(2, 3, data)?;

    assert_eq!(
        matrix.row(1),
        Some(&[Gf256::new(4), Gf256::new(5), Gf256::zero()][..])
    );
    assert_eq!(matrix.get(1, 1), Some(Gf256::new(5)));
    assert_eq!(matrix.get(2, 0), None); // Строка вне матрицы.
    assert_eq!(matrix.get(usize::MAX, 0), None);

    // get_mut даёт временную изменяемую ссылку на один элемент.
    if let Some(element) = matrix.get_mut(1, 1) {
        *element = Gf256::new(9);
    }
    assert_eq!(matrix.get(1, 1), Some(Gf256::new(9)));
    assert_eq!(matrix.as_slice().len(), 6); // Замена не меняет размеры.

    // Неверная длина возвращается как Err(LinalgError), не создавая матрицу.
    let wrong_length = Matrix::try_new(2, 3, vec![Gf256::zero(); 5]);
    assert!(matches!(
        wrong_length,
        Err(LinalgError::ElementCountMismatch {
            expected: 6,
            actual: 5,
        })
    ));

    // Размеры проверяются до длины данных; нулевая ось запрещена.
    let invalid_shape = Matrix::<Gf256>::try_new(0, 3, Vec::new());
    assert!(matches!(
        invalid_shape,
        Err(LinalgError::InvalidDimensions { rows: 0, cols: 3 })
    ));

    // Сложение доступно как именованный метод и оператор; оба возвращают Result.
    let other = Vector::new(vec![Gf256::new(3), Gf256::zero(), Gf256::zero()]);
    let sum = vector.try_add(&other)?;
    let operator_sum = (&vector + &other)?;
    assert_eq!(sum, operator_sum);
    assert_eq!(sum.as_slice(), &[Gf256::new(4), Gf256::zero(), Gf256::zero()]);

    // Умножение создаёт новый вектор той же длины.
    let scaled = vector.scale(Gf256::new(2));
    let operator_product = &vector * Gf256::new(2);
    assert_eq!(scaled, operator_product);
    assert_eq!(vector, Vector::new(vec![Gf256::new(7), Gf256::zero(), Gf256::zero()]));
    assert_ne!(vector, other);

    // try_dot пока сообщает об отсутствии реализации вместо фиктивного значения.
    assert!(matches!(
        vector.try_dot(&other),
        Err(LinalgError::NotImplemented {
            operation: "скалярное произведение векторов",
        })
    ));

    Ok(())
}
```

`Vector::new` и `Matrix::try_new` принимают `Vec<F>` во владение: Rust передаёт коллекцию новому объекту, не копируя элементы. Срез, например `&[F]` из `as_slice` или `row`, — это ссылка на существующие данные для чтения. `get_mut` возвращает `Option<&mut F>`: `&mut` — временная изменяемая ссылка на отдельный элемент, через которую можно заменить его значение. Ни срез, ни такая ссылка не позволяют изменить длину вектора или матрицы.

Вектор может быть пустым и не имеет отдельного ограничения длины. Он сохраняет все элементы, в том числе нули в конце. Пределы матриц, многочленов и исходного сообщения не ограничивают создание вектора.

`Vector::try_add` и оператор `+` складывают, а `Vector::try_sub` и оператор `-` вычитают векторы одинаковой длины по правилам `F`. Все четыре формы возвращают `Result<Vector<F>, LinalgError>`; при несовпадении длин возникает `VectorLengthMismatch` с длинами левого и правого операндов. Форма `a + b` или `a - b` передаёт оба вектора оператору во владение; форма `&a + &b` или `&a - &b` временно заимствует их и оставляет доступными после операции. При успехе создаётся новый вектор, а исходные не меняются.

`Vector::scale` и оператор `*` умножают каждый элемент на скаляр того же поля `F` и создают новый вектор той же длины. Доступны варианты `vector * scalar` и `&vector * scalar`. Равенство через `==` сравнивает длину и все элементы; например, `[a]` и `[a, 0]` различаются, поскольку длины различны. Оператор `!=` является отрицанием `==`.

`Vector::try_dot` уже доступен с результатом типа `Result<F, LinalgError>`, но скалярное произведение пока не вычисляет: каждый вызов возвращает `NotImplemented { operation: "скалярное произведение векторов" }`, независимо от длин операндов.

## Операции с матрицами

Методы операций заимствуют матрицы и создают новый результат. В следующем примере `?` проверяет каждый `Result`: при ошибке функция `main` завершится с этой ошибкой, а при успехе выполнение продолжится со значением.

```rust
use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, Vector};

fn main() -> Result<(), LinalgError> {
    let a = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::new(1), Gf256::new(2), Gf256::new(3),
            Gf256::new(4), Gf256::new(5), Gf256::new(6),
        ],
    )?;
    let b = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::new(1), Gf256::new(1), Gf256::new(1),
            Gf256::zero(), Gf256::zero(), Gf256::zero(),
        ],
    )?;

    let sum = a.try_add(&b)?;
    assert_eq!(sum, (&a + &b)?);
    let difference = a.try_sub(&b)?;
    assert_eq!(difference, (&a - &b)?);
    assert_ne!(a, b);

    // Матрица 2 × 3 на матрицу 3 × 2 даёт результат 2 × 2.
    let right = Matrix::try_new(
        3,
        2,
        vec![
            Gf256::new(1), Gf256::zero(),
            Gf256::zero(), Gf256::new(1),
            Gf256::zero(), Gf256::zero(),
        ],
    )?;
    let product = a.try_mul(&right)?;
    assert_eq!((product.rows(), product.cols()), (2, 2));
    assert_eq!(
        product.as_slice(),
        &[Gf256::new(1), Gf256::new(2), Gf256::new(4), Gf256::new(5)]
    );
    assert_eq!(product, (&a * &right)?);

    let transposed = a.transpose();
    assert_eq!((transposed.rows(), transposed.cols()), (3, 2));
    assert_eq!(a, transposed.transpose());

    let vector = Vector::new(vec![Gf256::zero(), Gf256::zero(), Gf256::new(1)]);
    let vector_product = a.try_mul_vector(&vector)?;
    assert_eq!(vector_product.as_slice(), &[Gf256::new(3), Gf256::new(6)]);
    assert_eq!(vector_product, (&a * &vector)?);

    // a, b, right и vector доступны и после операций со ссылками.
    assert_eq!((a.rows(), a.cols()), (2, 3));
    assert_eq!(vector.len(), 3);
    Ok(())
}
```

Сложение и вычитание требуют одинаковой формы: равными должны быть и число строк, и число столбцов. Они создают матрицу той же формы; для несовпадающей формы сложение возвращает `MatrixShapeMismatch`, вычитание — `MatrixSubtractionShapeMismatch`. Умножение матриц требует, чтобы число столбцов левой матрицы совпадало с числом строк правой; форма результата состоит из числа строк левой матрицы и числа столбцов правой. При умножении матрицы на вектор длина вектора должна совпадать с числом столбцов матрицы, а длина результата равна числу строк матрицы. `transpose` меняет строки и столбцы местами и возвращает новую матрицу.

| Операция | Именованный метод | Заимствованные операнды | Операнды во владении | Результат |
| --- | --- | --- | --- | --- |
| Сложение матриц | `a.try_add(&b)` | `&a + &b` | `a + b` | `Result<Matrix<F>, LinalgError>` |
| Вычитание матриц | `a.try_sub(&b)` | `&a - &b` | `a - b` | `Result<Matrix<F>, LinalgError>` |
| Умножение матриц | `a.try_mul(&b)` | `&a * &b` | `a * b` | `Result<Matrix<F>, LinalgError>` |
| Умножение матрицы на вектор | `a.try_mul_vector(&v)` | `&a * &v` | `a * v` | `Result<Vector<F>, LinalgError>` |
| Транспонирование | `a.transpose()` | — | — | `Matrix<F>` |
| Сравнение | `a == b`, `a != b` | Сравнивает ссылки внутри выражения | Не потребляет значения | `bool` |

Методы `try_add`, `try_sub`, `try_mul` и `try_mul_vector` принимают ссылки на операнды. Операторы в столбце «заимствованные операнды» также временно берут ссылки: значения остаются доступны после выражения и не меняются. Операторы с принадлежащими значениями перемещают их во входные параметры; после `a + b`, `a - b` или `a * b` исходные переменные использовать нельзя. `Result` содержит либо успешный новый результат (`Ok`), либо ошибку (`Err`). Несовпадение форм при сложении или вычитании возвращает соответствующий вариант ошибки, несовпадение внутренних размеров произведения матриц — `MatrixProductMismatch`, несовпадение длины вектора — `MatrixVectorLengthMismatch`.

Равенство `==` учитывает форму и элементы: одинаковые плоские данные с разными размерами не делают матрицы равными. `!=` является отрицанием `==`. Хранение остаётся по строкам: при умножении и транспонировании результат также располагается по строкам в соответствии с его новой формой.

## Приведение к RREF и ранг

`Matrix::rref` приводит матрицу к приведённому ступенчатому виду над её полем
`F` и возвращает `RrefResult<F>`. Метод работает с квадратными и
прямоугольными матрицами, включая нулевые и вырожденные; размеры самой матрицы
по-прежнему должны быть допустимыми. Преобразования строк не меняют порядок
столбцов и не меняют исходную матрицу. Результат имеет ту же форму, нулевые
строки расположены внизу, а опорные столбцы перечислены по возрастанию с
индексами от нуля.

`RrefResult::matrix` и `pivot_columns` дают ссылки для чтения, а `rank`
возвращает количество опорных столбцов. `into_matrix` передаёт приведённую
матрицу во владение вызывающему коду. `Matrix::rank` вызывает `rref()` и
возвращает ранг результата. Если нужны и приведённая матрица, и ранг,
достаточно один раз вызвать `rref()` и взять оба значения из результата.

```rust
use gf_linalg::{LinalgError, Matrix};
use gfpm::Gf;

type Gf2 = Gf<2, 1, 1>;

fn main() -> Result<(), LinalgError> {
    let matrix = Matrix::<Gf2>::try_new(
        2,
        3,
        [1, 1, 0, 1, 0, 1].map(Gf2::new).to_vec(),
    )?;
    let result = matrix.rref();
    let expected = Matrix::<Gf2>::try_new(
        2,
        3,
        [1, 0, 1, 0, 1, 1].map(Gf2::new).to_vec(),
    )?;

    assert_eq!(result.matrix(), &expected);
    assert_eq!(result.pivot_columns(), &[0, 1]);
    assert_eq!(result.rank(), 2);
    assert_eq!(matrix.rank(), 2);
    assert_eq!(matrix.get(0, 1), Some(Gf2::one())); // Источник не изменился.
    Ok(())
}
```

Это общие операции линейной алгебры. RREF сохраняет исходное расположение
столбцов и возвращает индексы опорных столбцов; выбор перестановки столбцов и
систематизация конкретного LDPC-кода остаются задачей соответствующего кодера.

## Определитель и обратная матрица

`Matrix::try_determinant` и `Matrix::try_inverse` доступны только для квадратных матриц. Для прямоугольной матрицы оба метода возвращают `LinalgError::NonSquareMatrix` с фактическим числом строк и столбцов. Для вырожденной квадратной матрицы определитель равен нулю, а `try_inverse` возвращает `LinalgError::SingularMatrix`. Исключение строк в алгоритме Гаусса использует вычитание в `F`; каждая перестановка строк меняет знак определителя через унарный минус. Это важно, например, для полей характеристики 3, где `-a` не равно `a`.

Оба метода принимают `&self`: это общая ссылка на исходную матрицу. Они вычисляют результат, не изменяя и не забирая исходную матрицу. Каждый метод возвращает `Result`: `Ok(value)` содержит результат, а `Err(error)` — причину, по которой вычисление невозможно. Оператор `?` в функции, которая тоже возвращает `Result`, передаёт ошибку вызывающему коду; при успехе он извлекает значение из `Ok`.

```rust
use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix};

fn main() -> Result<(), LinalgError> {
    let matrix = Matrix::try_new(
        2,
        2,
        vec![Gf256::one(), Gf256::one(), Gf256::one(), Gf256::zero()],
    )?;

    let determinant = matrix.try_determinant()?;
    assert_eq!(determinant, Gf256::one());

    let inverse = matrix.try_inverse()?;
    let identity = Matrix::try_new(
        2,
        2,
        vec![Gf256::one(), Gf256::zero(), Gf256::zero(), Gf256::one()],
    )?;
    assert_eq!(matrix.try_mul(&inverse)?, identity);

    // Если операция вернёт Err, ? завершит main и передаст LinalgError.
    // matrix и inverse переданы по ссылке, поэтому исходная матрица доступна.
    assert_eq!(matrix.get(0, 0), Some(Gf256::one()));

    let singular = Matrix::try_new(
        2,
        2,
        vec![Gf256::one(), Gf256::one(), Gf256::one(), Gf256::one()],
    )?;
    assert_eq!(singular.try_determinant()?, Gf256::zero());
    assert_eq!(singular.try_inverse(), Err(LinalgError::SingularMatrix));

    Ok(())
}
```

## Текстовый формат

`Vector<F>` и `Matrix<F>` реализуют `Display` для вывода и `FromStr` для
разбора строки через `str::parse`, когда `F` реализует отдельный трейт
`FieldText`. Канонический формат содержит явные размеры:

```text
vector 3 0x01 0x00 0xff

matrix 2 3
0x01 0x00 0xff
0x02 0x03 0x00
```

Заголовки должны быть ровно `vector` или `matrix` в нижнем регистре. Размеры —
беззнаковые десятичные ASCII-цифры; ведущие нули разрешены. Вектор может иметь
нулевую длину, у матрицы обе оси должны быть от 1 до `MAX_MATRIX_DIM`
включительно.

Формат токена элемента задаётся `FieldText`. Для `gf2m::Gf<N, POLY>` токен
содержит префикс `0x` и фиксированное число шестнадцатеричных цифр: ширина
равна `max(2, ceil(M / 4))`, где `M` — степень расширения. Префикс `0x`
чувствителен к регистру; цифры на входе могут быть строчными или прописными,
вывод всегда использует строчные цифры. Для `gfpm::Gf<P, M, POLY>` элемент
представляется десятичным упакованным значением `u64` из `value()`, меньшим
`ORDER`; это код коэффициентов элемента, а не его числовое значение в поле.
Разбор принимает только ASCII-цифры `0`–`9` и разрешает ведущие нули; вывод
использует обычную десятичную запись без ведущих нулей. Упакованное значение
проверяется как `u128` на условие `< ORDER` до создания элемента.
Токены разделяются пробелом,
табуляцией, переводом строки, вертикальной табуляцией, переводом страницы или
возвратом каретки (ASCII `U+0009`–`U+000D` и `U+0020`). Границы строк входной
матрицы не влияют на порядок элементов: он определяется размерами и идёт
сначала по первой строке, затем по второй и далее. Вывод матрицы разделяет
строки переводами строки и не добавляет перевод после последней строки.

Разбор проверяет заголовок, размеры, допустимость осей матрицы, точное число
токенов элементов и затем формат самих элементов. Ошибка количества
элементных токенов предшествует ошибке значения элемента. Индексы токенов
размеров и плоские индексы элементов начинаются с нуля. Ошибки возвращаются
как `LinalgError` через `Result`:

```rust
use gfpm::Gf9;
use gf_linalg::{LinalgError, Matrix, Vector};

fn main() -> Result<(), LinalgError> {
    let vector = Vector::<Gf9>::new(vec![Gf9::new(1), Gf9::zero(), Gf9::zero()]);
    let vector_text = vector.to_string();
    assert_eq!(vector_text, "vector 3 1 0 0");
    let parsed_vector: Vector<Gf9> = vector_text.parse()?;
    assert_eq!(parsed_vector, vector); // Вектор сохранил длину и хвостовые нули.

    let matrix = Matrix::<Gf9>::try_new(
        2,
        2,
        vec![Gf9::new(1), Gf9::zero(), Gf9::new(2), Gf9::new(8)],
    )?;
    let matrix_text = matrix.to_string();
    assert_eq!(matrix_text, "matrix 2 2\n1 0\n2 8");
    let parsed_matrix: Matrix<Gf9> = matrix_text.parse()?;
    assert_eq!(parsed_matrix, matrix);

    let short_input = "vector 2 1".parse::<Vector<Gf9>>();
    assert_eq!(
        short_input,
        Err(LinalgError::TextElementCountMismatch {
            expected: 2,
            actual: 1,
        })
    );
    Ok(())
}
```

`FromStr` разбирает строку; чтение файлов и потоков остаётся за вызывающим
кодом. `Display` можно передать форматтеру или преобразовать в строку.

## Представление коэффициентов

У вектора нет отдельного максимума длины. Метод
`Vector::try_to_polynomial_coefficients` создаёт `Box<[F]>`, где элемент с
индексом `i` — коэффициент при `x^i`. Пустой вектор соответствует пустому
массиву; хвостовые нули сохраняются, так что `[a, 0, 0]` остаётся длины 3.
Это точность хранения коэффициентов, а не нормализация многочлена.

`MAX_POLYNOMIAL_BYTES` равен 131 072 байтам (128 КиБ). В лимит входит только
память элементов выходного массива, то есть
`len * size_of::<F>()`. Не учитываются метаданные значения `Box`, накладные
расходы аллокатора, внешний список строк и его ёмкость. Допустимое число
коэффициентов равно `MAX_POLYNOMIAL_BYTES / size_of::<F>()`; например,
`Gf256` размером 2 байта допускает 65 536 коэффициентов. Предел проверяется до
копирования через деление на размер элемента, без умножения. Исходная ёмкость
`Vec` не переносится в `Box<[F]>` и не учитывается.

При превышении `Vector::try_to_polynomial_coefficients` возвращает
`PolynomialCoefficientLimitExceeded`; сам вектор при этом остаётся допустимым.
`Vector::from_polynomial_coefficients` принимает `Box<[F]>` во владение и
не ограничивает длину результата. Произвольный `Box<[F]>`, переданный этому
обратному методу, не обязан укладываться в лимит.

Матрица представляется `Vec<Box<[F]>>`, с одним массивом на каждую строку.
Лимит применяется отдельно к каждому массиву, поэтому суммарный размер строк
может превышать 128 КиБ. При максимальной ширине 4096 строка занимает
`4096 * size_of::<F>()` байт; например, для `Gf256` это 8192 байта. Допустимая
по размерам матрица с крупным `F` может превысить предел одной строки и получить
`PolynomialCoefficientLimitExceeded`. `Matrix::try_to_polynomial_rows` оставляет
источник неизменным. `Matrix::try_from_polynomial_rows` принимает входные строки
во владение и копирует их элементы в плоский буфер матрицы; обратное
преобразование проверяет только форму и не применяет предел 128 КиБ. Сначала
проверяется число строк, затем ширина первой строки, потом остальные строки по
порядку: недопустимая ширина (`InvalidDimensions`) возвращается сразу, а для
допустимой ширины проверяется совпадение с первой
(`PolynomialRowLengthMismatch`). Первая найденная ошибка возвращается до
выделения плоского буфера. Пустой набор и оси вне диапазона 1–4096 дают
`InvalidDimensions` при проверке соответствующего размера.

Эти методы задают обмен представлением с массивами коэффициентов; отдельный
тип многочлена над произвольным полем и арифметика над таким типом пока не
реализованы.

```rust
use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, Vector};

fn main() -> Result<(), LinalgError> {
    let vector = Vector::new(vec![Gf256::new(7), Gf256::zero(), Gf256::zero()]);
    let coefficients = vector.try_to_polynomial_coefficients()?;
    assert_eq!(coefficients.as_ref(), &[Gf256::new(7), Gf256::zero(), Gf256::zero()]);
    assert_eq!(vector.len(), 3); // Прямое преобразование заимствует источник.
    let restored = Vector::from_polynomial_coefficients(coefficients);
    assert_eq!(restored, vector); // Box перемещён в обратное преобразование.

    let matrix = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::new(1), Gf256::zero(), Gf256::zero(),
            Gf256::new(2), Gf256::new(3), Gf256::zero(),
        ],
    )?;
    let rows = matrix.try_to_polynomial_rows()?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].as_ref(), &[Gf256::new(2), Gf256::new(3), Gf256::zero()]);
    let restored = Matrix::try_from_polynomial_rows(rows)?;
    assert_eq!(restored, matrix);
    Ok(())
}
```

Этапы 1–6 линейной алгебры реализованы в `gf-linalg`; также выполнен этап 0
подготовки к LDPC — RREF и вычисление ранга. `Vector<F>` и `Matrix<F>`
работают с любым типом обоих статических семейств `gf2m::Gf<N, POLY>` и
`gfpm::Gf<P, M, POLY>`, если параметры задают корректное поле. Также можно
подключить собственный тип через `FieldElement`; для текстового ввода и вывода
ему отдельно нужен `FieldText`. Доступны сложение и вычитание векторов и
матриц, умножение на скаляр, матричные произведения, транспонирование, RREF,
опорные столбцы и ранг, определитель и обращение квадратных матриц. Скалярное произведение векторов
имеет объявленный метод, но пока возвращает явную ошибку `NotImplemented`.
Отдельного типа многочлена и арифметики над ним нет.

У каждой оси матрицы размер от `1` до `MAX_MATRIX_DIM` включительно; значение `MAX_MATRIX_DIM` равно `4096`. Ограничение применяется отдельно к строкам и столбцам, а не к общему числу элементов: например, матрица `65 × 65` допустима. Нулевые оси запрещены. Длина переданного массива должна точно равняться `rows * cols`; произведение вычисляется с проверкой переполнения.

Элементы хранятся по строкам, индексы начинаются с нуля. Сначала идут все элементы первой строки, затем второй и так далее. Элемент с координатами `(row, col)` лежит по индексу `row * cols + col`.

`Matrix::try_new` возвращает `Result`: `Ok(matrix)` означает, что матрица создана, а `Err(error)` содержит `LinalgError`. `InvalidDimensions { rows, cols }` означает, что размеры недопустимы; `ElementCountMismatch { expected, actual }` — что длина данных не равна ожидаемому числу элементов. Сначала проверяются размеры, затем длина данных.

Методы доступа возвращают `Option`: `Some(value)` или ссылку на значение для верного индекса и `None` за границей. Выход за границу, включая `usize::MAX`, не вызывает панику. Замена через `get_mut` сохраняет размеры и длину хранилища.

Проверить пакет вместе с примерами из этого README и собрать его документацию можно командами:

```sh
cargo test -p gf-linalg --locked
cargo doc -p gf-linalg --no-deps --locked
```
