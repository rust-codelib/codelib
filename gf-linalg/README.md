# gf-linalg

`gf-linalg` — пакет с библиотечным крейтом (библиотекой Rust). В исходном коде библиотеку импортируют как `gf_linalg`. Он предоставляет `Vector` и прямоугольную `Matrix` над элементами поля `gf2m::Gf256`, безопасное чтение и замену элементов, операции над векторами и матрицами, а также определитель и обратную матрицу для квадратных матриц.

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
    let invalid_shape = Matrix::try_new(0, 3, Vec::new());
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

`Vector::new` и `Matrix::try_new` принимают `Vec<Gf256>` во владение: Rust передаёт коллекцию новому объекту, не копируя элементы. Срез, например `&[Gf256]` из `as_slice` или `row`, — это ссылка на существующие данные для чтения. `get_mut` возвращает `Option<&mut Gf256>`: `&mut` — временная изменяемая ссылка на отдельный элемент, через которую можно заменить его значение. Ни срез, ни такая ссылка не позволяют изменить длину вектора или матрицы.

Вектор может быть пустым и не имеет отдельного ограничения длины. Он сохраняет все элементы, в том числе нули в конце. Пределы матриц, многочленов и исходного сообщения не ограничивают создание вектора.

`Vector::try_add` и оператор `+` складывают только векторы одинаковой длины и возвращают `Result<Vector, LinalgError>`. При несовпадении длин возникает `VectorLengthMismatch` с длинами левого и правого операндов. Форма `a + b` передаёт оба вектора оператору во владение; форма `&a + &b` временно заимствует их и оставляет доступными после операции. При успехе создаётся новый вектор, а исходные не меняются.

`Vector::scale` и оператор `*` умножают каждый элемент на скаляр `Gf256` и создают новый вектор той же длины. Доступны варианты `vector * scalar` и `&vector * scalar`. Равенство через `==` сравнивает длину и все элементы; например, `[a]` и `[a, 0]` различаются, поскольку длины различны. Оператор `!=` является отрицанием `==`.

`Vector::try_dot` уже доступен с результатом типа `Result<Gf256, LinalgError>`, но скалярное произведение пока не вычисляет: каждый вызов возвращает `NotImplemented { operation: "скалярное произведение векторов" }`, независимо от длин операндов.

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

Сложение требует одинаковой формы: равными должны быть и число строк, и число столбцов. Оно создаёт матрицу той же формы. Умножение матриц требует, чтобы число столбцов левой матрицы совпадало с числом строк правой; форма результата состоит из числа строк левой матрицы и числа столбцов правой. При умножении матрицы на вектор длина вектора должна совпадать с числом столбцов матрицы, а длина результата равна числу строк матрицы. `transpose` меняет строки и столбцы местами и возвращает новую матрицу.

| Операция | Именованный метод | Заимствованные операнды | Операнды во владении | Результат |
| --- | --- | --- | --- | --- |
| Сложение матриц | `a.try_add(&b)` | `&a + &b` | `a + b` | `Result<Matrix, LinalgError>` |
| Умножение матриц | `a.try_mul(&b)` | `&a * &b` | `a * b` | `Result<Matrix, LinalgError>` |
| Умножение матрицы на вектор | `a.try_mul_vector(&v)` | `&a * &v` | `a * v` | `Result<Vector, LinalgError>` |
| Транспонирование | `a.transpose()` | — | — | `Matrix` |
| Сравнение | `a == b`, `a != b` | Сравнивает ссылки внутри выражения | Не потребляет значения | `bool` |

Методы `try_add`, `try_mul` и `try_mul_vector` принимают ссылки на операнды. Операторы в столбце «заимствованные операнды» также временно берут ссылки: значения остаются доступны после выражения и не меняются. Операторы с принадлежащими значениями перемещают их во входные параметры; после `a + b` или `a * b` исходные переменные использовать нельзя. `Result` содержит либо успешный новый результат (`Ok`), либо ошибку (`Err`). Несовпадение форм при сложении возвращает `MatrixShapeMismatch`, несовпадение внутренних размеров произведения матриц — `MatrixProductMismatch`, несовпадение длины вектора — `MatrixVectorLengthMismatch`.

Равенство `==` учитывает форму и элементы: одинаковые плоские данные с разными размерами не делают матрицы равными. `!=` является отрицанием `==`. Хранение остаётся по строкам: при умножении и транспонировании результат также располагается по строкам в соответствии с его новой формой.

## Определитель и обратная матрица

`Matrix::try_determinant` и `Matrix::try_inverse` доступны только для квадратных матриц. Для прямоугольной матрицы оба метода возвращают `LinalgError::NonSquareMatrix` с фактическим числом строк и столбцов. Для вырожденной квадратной матрицы определитель равен нулю, а `try_inverse` возвращает `LinalgError::SingularMatrix`.

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

Этапы 1–4 линейной алгебры реализованы. Ввод/вывод и связь с многочленами относятся к этапу 5 и пока не реализованы; обобщение типов по полю запланировано на этап 6.

У каждой оси матрицы размер от `1` до `MAX_MATRIX_DIM` включительно; значение `MAX_MATRIX_DIM` равно `4096`. Ограничение применяется отдельно к строкам и столбцам, а не к общему числу элементов: например, матрица `65 × 65` допустима. Нулевые оси запрещены. Длина переданного массива должна точно равняться `rows * cols`; произведение вычисляется с проверкой переполнения.

Элементы хранятся по строкам, индексы начинаются с нуля. Сначала идут все элементы первой строки, затем второй и так далее. Элемент с координатами `(row, col)` лежит по индексу `row * cols + col`.

`Matrix::try_new` возвращает `Result`: `Ok(matrix)` означает, что матрица создана, а `Err(error)` содержит `LinalgError`. `InvalidDimensions { rows, cols }` означает, что размеры недопустимы; `ElementCountMismatch { expected, actual }` — что длина данных не равна ожидаемому числу элементов. Сначала проверяются размеры, затем длина данных.

Методы доступа возвращают `Option`: `Some(value)` или ссылку на значение для верного индекса и `None` за границей. Выход за границу, включая `usize::MAX`, не вызывает панику. Замена через `get_mut` сохраняет размеры и длину хранилища.

Проверить пакет вместе с примерами из этого README и собрать его документацию можно командами:

```sh
cargo test -p gf-linalg --locked
cargo doc -p gf-linalg --no-deps --locked
```
