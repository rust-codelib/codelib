//! Пример использования экспандерных кодов (LDPC на основе регулярного двудольного графа).
//!
//! Запуск: `cargo run --example expander_codes`

use ldpc_codes::{
    Bit, DecodeInput, DecodeStatus, DecoderConfig, Encoder, LdpcConfigurator, ParityCheckMatrix,
};
use rand::seq::SliceRandom;
use rand::Rng;

// Параметры экспандерного кода
const N: usize = 20; // Общее число битов (левых вершин)
const C: usize = 3; // Степень каждой левой вершины (сколько проверок затрагивает бит)
const D: usize = 5; // Степень каждой правой вершины (сколько битов входит в проверку)

/// Генерирует регулярный двудольный граф с заданными параметрами.
/// Возвращает список проверок: для каждой правой вершины — список индексов левых вершин.
/// Генерирует регулярный двудольный граф с заданными параметрами.
/// Делает несколько попыток, так как жадный алгоритм может зайти в тупик.
fn generate_regular_bipartite_graph(
    n: usize,
    c: usize,
    d: usize,
) -> Result<Vec<Vec<usize>>, String> {
    if (n * c) % d != 0 {
        return Err("Параметры не согласованы: n*c должно делиться на d".into());
    }
    let m = n * c / d; // Число правых вершин (проверок)

    // Пробуем сгенерировать граф до 100 раз
    for _ in 0..100 {
        let mut right_neighbors: Vec<Vec<usize>> = vec![Vec::new(); m];

        // Создаем список левых вершин и перемешиваем его
        let mut left_vertices: Vec<usize> = (0..n).collect();
        let mut rng = rand::rng(); // Для rand 0.9
        left_vertices.shuffle(&mut rng);

        let mut success = true;
        for &v in &left_vertices {
            // Ищем правые вершины, у которых еще есть свободные слоты
            let mut available: Vec<usize> =
                (0..m).filter(|&r| right_neighbors[r].len() < d).collect();

            if available.len() < c {
                // Если свободных слотов меньше, чем нужно левой вершине,
                // значит, мы зашли в тупик. Прерываем эту попытку.
                success = false;
                break;
            }

            // Перемешиваем доступные правые вершины и берем первые c
            available.shuffle(&mut rng);
            for &r in available.iter().take(c) {
                right_neighbors[r].push(v);
            }
        }

        if success {
            // Если все левые вершины успешно распределены, возвращаем результат
            return Ok(right_neighbors);
        }

        // Можно вывести в консоль для отладки, если интересно:
        // println!("Попытка {} не удалась, пробуем снова...", attempt + 1);
    }

    Err("Не удалось сгенерировать граф за 100 попыток.".into())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Демонстрация экспандерных кодов (Сипсер-Шпильмен) ---");

    // 1. Генерируем граф-экспандер
    let checks = generate_regular_bipartite_graph(N, C, D).expect("Не удалось сгенерировать граф.");
    println!(
        "Сгенерирован граф: {} левых и {} правых вершин.",
        N,
        checks.len()
    );

    // 2. Строим проверочную матрицу H
    // Примечание: если метод try_from_rows отличается, посмотрите в src/parity_check.rs
    let h = ParityCheckMatrix::try_from_rows(N, checks)?;

    // 3. Настраиваем кодер и декодер
    let decoder_config = DecoderConfig::try_new(50, 20.0)?;
    // Настраиваем конфигурацию LDPC
    let mut code = LdpcConfigurator::build(h, decoder_config)?;

    // 4. Формируем сообщение
    let k = code.encoder().message_len();
    println!("Длина сообщения k = {}", k);
    if k == 0 {
        return Err("k = 0, невозможно закодировать сообщение".into());
    }

    let mut rng = rand::rng();
    let message: Vec<Bit> = (0..k)
        .map(|_| {
            if rng.random_bool(0.5) {
                Bit::One
            } else {
                Bit::Zero
            }
        })
        .collect();

    // 5. Кодируем
    let encoded = code.encoder().encode(&message)?;
    println!(
        "Закодированное слово (длина {}): {:?}",
        encoded.len(),
        encoded
    );

    // 6. Добавляем шум (инвертируем 2 бита)
    let mut noisy_encoded = encoded.clone();
    let num_errors = 2;
    for _ in 0..num_errors {
        let pos = rng.random_range(0..noisy_encoded.len());
        noisy_encoded[pos] = match noisy_encoded[pos] {
            Bit::Zero => Bit::One,
            Bit::One => Bit::Zero,
        };
    }
    println!("Слово с {} ошибками: {:?}", num_errors, noisy_encoded);

    // 7. Декодируем через SPA (Sum-Product Algorithm)
    // Задаем LLR (логарифм отношения правдоподобия) для BSC канала.
    let llrs: Vec<f64> = noisy_encoded
        .iter()
        .map(|&bit| match bit {
            Bit::Zero => 2.0, // Уверены, что 0
            Bit::One => -2.0, // Уверены, что 1
        })
        .collect();

    let result = code.decode(
        DecodeInput {
            llrs: &llrs,
            erasures: &[],
        },
        None, // Без наблюдателя (observer)
    )?;

    println!("Статус декодирования: {:?}", result.status());

    if result.status() == DecodeStatus::ParitySatisfied {
        let recovered = code.encoder().extract_message(result.word())?;
        println!("Исходное сообщение:        {:?}", message);
        println!("Восстановленное сообщение: {:?}", recovered);
        assert_eq!(recovered, message, "Декодирование не удалось!");
        println!("✅ УСПЕХ: Сообщение восстановлено без ошибок.");
    } else {
        println!("❌ ОШИБКА: Декодер не смог удовлетворить проверки чётности.");
    }

    Ok(())
}
