# gf-linalg

`gf-linalg` — пакет с библиотечным крейтом (библиотекой Rust). В исходном коде библиотеку импортируют как `gf_linalg`. Сейчас она предоставляет `Vector` и прямоугольную `Matrix` над элементами поля `gf2m::Gf256`, а также безопасное чтение и замену отдельных элементов.

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

    Ok(())
}
```

`Vector::new` и `Matrix::try_new` принимают `Vec<Gf256>` во владение: Rust передаёт коллекцию новому объекту, не копируя элементы. Срез, например `&[Gf256]` из `as_slice` или `row`, — это ссылка на существующие данные для чтения. `get_mut` возвращает `Option<&mut Gf256>`: `&mut` — временная изменяемая ссылка на отдельный элемент, через которую можно заменить его значение. Ни срез, ни такая ссылка не позволяют изменить длину вектора или матрицы.

Вектор может быть пустым и не имеет отдельного ограничения длины. Он сохраняет все элементы, в том числе нули в конце. Пределы матриц, многочленов и исходного сообщения не ограничивают создание вектора.

У каждой оси матрицы размер от `1` до `MAX_MATRIX_DIM` включительно; значение `MAX_MATRIX_DIM` равно `4096`. Ограничение применяется отдельно к строкам и столбцам, а не к общему числу элементов: например, матрица `65 × 65` допустима. Нулевые оси запрещены. Длина переданного массива должна точно равняться `rows * cols`; произведение вычисляется с проверкой переполнения.

Элементы хранятся по строкам, индексы начинаются с нуля. Сначала идут все элементы первой строки, затем второй и так далее. Элемент с координатами `(row, col)` лежит по индексу `row * cols + col`.

`Matrix::try_new` возвращает `Result`: `Ok(matrix)` означает, что матрица создана, а `Err(error)` содержит `LinalgError`. `InvalidDimensions { rows, cols }` означает, что размеры недопустимы; `ElementCountMismatch { expected, actual }` — что длина данных не равна ожидаемому числу элементов. Сначала проверяются размеры, затем длина данных.

Методы доступа возвращают `Option`: `Some(value)` или ссылку на значение для верного индекса и `None` за границей. Выход за границу, включая `usize::MAX`, не вызывает панику. Замена через `get_mut` сохраняет размеры и длину хранилища.

Проверить пакет вместе с примерами из этого README и собрать его документацию можно командами:

```sh
cargo test -p gf-linalg --locked
cargo doc -p gf-linalg --no-deps --locked
```
