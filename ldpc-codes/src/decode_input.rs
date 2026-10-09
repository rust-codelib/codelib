//! Канальные данные для шага или полного декодирования SPA.

/// Заимствованные канальные LLR и позиции стираний для [`crate::spa_step`] и
/// [`crate::Decoder::decode`].
///
/// Значение `llrs[i]` относится к биту с индексом `i` в порядке столбцов
/// исходной проверочной матрицы. Срезы не копируются в этот тип и не изменяются
/// им. Оба API не сохраняют ссылки после вызова.
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
mod tests {
    use super::{prepare_channel, DecodeInput};
    use crate::{DecoderConfig, LdpcError};

    fn prepare(
        input: DecodeInput<'_>,
        bits: usize,
        config: DecoderConfig,
    ) -> Result<Vec<f64>, LdpcError> {
        let mut channel = vec![0.0; bits];
        let mut erased_positions = vec![false; bits];
        prepare_channel(input, &mut channel, &mut erased_positions, config)?;
        Ok(channel)
    }

    #[test]
    fn preparation_clips_extreme_finite_values_and_zeroes_erasures() {
        let llrs = [f64::MAX, -f64::MAX, 1.25, -2.5];
        let erasures = [2];

        let channel = prepare(
            DecodeInput {
                llrs: &llrs,
                erasures: &erasures,
            },
            4,
            DecoderConfig::try_new(0, 3.0).expect("valid limit"),
        )
        .expect("finite LLRs and valid erasures should be prepared");

        assert_eq!(channel, [3.0, -3.0, 0.0, -2.5]);
        assert_eq!(llrs, [f64::MAX, -f64::MAX, 1.25, -2.5]);
        assert_eq!(erasures, [2]);
    }

    #[test]
    fn preparation_accepts_no_erasures_and_erasing_every_bit() {
        let llrs = [-30.0, 4.0, 30.0];
        let config = DecoderConfig::default();
        let unerased = prepare(
            DecodeInput {
                llrs: &llrs,
                erasures: &[],
            },
            llrs.len(),
            config,
        )
        .expect("an empty erasure list is valid");
        let fully_erased = prepare(
            DecodeInput {
                llrs: &llrs,
                erasures: &[0, 1, 2],
            },
            llrs.len(),
            config,
        )
        .expect("all bits may be erased");

        assert_eq!(unerased, [-20.0, 4.0, 20.0]);
        assert_eq!(fully_erased, [0.0, 0.0, 0.0]);
        assert_eq!(llrs, [-30.0, 4.0, 30.0]);
    }

    #[test]
    fn length_mismatch_is_reported_before_other_input_errors() {
        let llrs = [f64::NAN];
        let erasures = [9, 9];

        let error = prepare(
            DecodeInput {
                llrs: &llrs,
                erasures: &erasures,
            },
            2,
            DecoderConfig::default(),
        )
        .expect_err("wrong LLR length must take precedence");

        assert_eq!(
            error,
            LdpcError::LlrLengthMismatch {
                expected: 2,
                actual: 1,
            }
        );
    }

    #[test]
    fn non_finite_erased_llr_is_reported_before_erasure_errors() {
        let llrs = [1.0, f64::INFINITY, f64::NAN];
        let erasures = [3, 1];

        let error = prepare(
            DecodeInput {
                llrs: &llrs,
                erasures: &erasures,
            },
            3,
            DecoderConfig::default(),
        )
        .expect_err("every LLR must be finite even when its bit is erased");

        assert_eq!(error, LdpcError::NonFiniteLlr { index: 1 });
        assert_eq!(llrs[0], 1.0);
        assert_eq!(llrs[1], f64::INFINITY);
        assert!(llrs[2].is_nan());
        assert_eq!(erasures, [3, 1]);
    }

    #[test]
    fn erasure_bounds_are_checked_before_duplicate_positions() {
        let llrs = [0.0; 3];
        let erasures = [1, 1, 3];

        let error = prepare(
            DecodeInput {
                llrs: &llrs,
                erasures: &erasures,
            },
            3,
            DecoderConfig::default(),
        )
        .expect_err("all bounds errors precede duplicate errors");

        assert_eq!(
            error,
            LdpcError::ErasureIndexOutOfBounds { bit: 3, bits: 3 }
        );
    }

    #[test]
    fn the_first_repeated_erasure_is_reported() {
        let llrs = [0.0; 3];
        let erasures = [2, 1, 2, 1];

        let error = prepare(
            DecodeInput {
                llrs: &llrs,
                erasures: &erasures,
            },
            3,
            DecoderConfig::default(),
        )
        .expect_err("the first duplicate in input order should be reported");

        assert_eq!(error, LdpcError::DuplicateErasureIndex { bit: 2 });
    }
}
