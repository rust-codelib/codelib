//! Канальные данные для шага или полного декодирования SPA.

/// Заимствованные канальные LLR и позиции стираний для [`crate::spa_step`] и
/// [`coding_core::Decoder::decode`].
///
/// Значение `llrs[i]` относится к биту с индексом `i` в порядке столбцов
/// исходной проверочной матрицы. Соглашение о знаке —
/// [`coding_core::LlrConvention::PositiveForZero`]:
/// `L = ln(P(0) / P(1))`, поэтому неотрицательное значение предпочитает ноль,
/// отрицательное — единицу. При `+0.0` и `-0.0` выбирается [`crate::Bit::Zero`].
/// Знак не переключается автоматически; при необходимости вызывающий код
/// заранее преобразует значения [`coding_core::LlrConvention::convert`].
///
/// Срезы не копируются в этот тип и не изменяются им. Оба API не сохраняют
/// ссылки после вызова; результаты владеют своими массивами.
///
/// Перед вычислением оба API проверяют длину LLR, конечность всех значений (в
/// том числе на стираемых позициях), границы индексов стираний и повторы именно
/// в таком порядке. После успешной проверки значения ограничиваются пределом
/// конфигурации, а стираемые позиции заменяются на `0.0`. Стирание означает
/// отсутствие предпочтения между битами, а не инверсию значения.
///
/// При ошибке входа `SpaDecoder` не меняет содержательное состояние блока и не
/// отправляет события. Для проверки повторов он может очистить или частично
/// заполнить внутреннюю служебную маску стираний; следующий вызов сбрасывает
/// эту маску перед проверкой. Эта маска не влияет на результаты и недоступна
/// вызывающему коду.
#[derive(Debug, Clone, Copy)]
pub struct DecodeInput<'a> {
    /// Канальное LLR для каждого бита кодового слова в порядке столбцов `H`.
    pub llrs: &'a [f64],
    /// Индексы битов, которые следует считать стёртыми; повторы запрещены.
    pub erasures: &'a [usize],
}

/// Проверяет вход и записывает подготовленные канальные LLR в переиспользуемый
/// буфер.
///
/// Сначала проверяются все значения и индексы. Проверка повторов использует
/// переиспользуемую маску и может оставить её очищенной или частично заполненной
/// при ошибке; канал и остальное содержательное состояние до полного успеха не
/// меняются. После успеха в канал записываются ограниченные LLR и нули для
/// стираний. Выделений памяти при подготовке нет.
pub(crate) fn prepare_channel(
    input: DecodeInput<'_>,
    channel: &mut [f64],
    erased_positions: &mut [bool],
    config: crate::DecoderConfig,
) -> Result<(), crate::LdpcError> {
    let bits = channel.len();
    debug_assert_eq!(erased_positions.len(), bits);

    if input.llrs.len() != bits {
        return Err(crate::LdpcError::LlrLengthMismatch {
            expected: bits,
            actual: input.llrs.len(),
        });
    }

    if let Some((index, _)) = input
        .llrs
        .iter()
        .enumerate()
        .find(|(_, llr)| !llr.is_finite())
    {
        return Err(crate::LdpcError::NonFiniteLlr { index });
    }

    if let Some(&bit) = input.erasures.iter().find(|&&bit| bit >= bits) {
        return Err(crate::LdpcError::ErasureIndexOutOfBounds { bit, bits });
    }

    erased_positions.fill(false);
    for &bit in input.erasures {
        if erased_positions[bit] {
            return Err(crate::LdpcError::DuplicateErasureIndex { bit });
        }
        erased_positions[bit] = true;
    }

    let limit = config.llr_limit();
    for (prepared, &llr) in channel.iter_mut().zip(input.llrs) {
        *prepared = llr.clamp(-limit, limit);
    }
    for &bit in input.erasures {
        channel[bit] = 0.0;
    }

    Ok(())
}

// Keeps the independent reference tests focused on input semantics while the
// production decoder writes into its reusable buffers.
#[cfg(test)]
pub(crate) fn prepared_channel(
    input: DecodeInput<'_>,
    bits: usize,
    config: crate::DecoderConfig,
) -> Result<Vec<f64>, crate::LdpcError> {
    let mut channel = vec![0.0; bits];
    let mut erased_positions = vec![false; bits];
    prepare_channel(input, &mut channel, &mut erased_positions, config)?;
    Ok(channel)
}

#[cfg(test)]
#[path = "../tests/unit/decode.rs"]
mod tests;
