//! Публичный контракт декодирования LDPC-кода.
//!
//! Содержит канальные данные для шага или полного декодирования SPA,
//! настройки декодирования, подготовку канала и результат блока.

use crate::bit::{bit_distance, count_ones};
use crate::{Bit, LdpcError, ParityCheckMatrix};

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

const MAX_LLR_LIMIT: f64 = 20.0;

/// Неизменяемые параметры SPA.
///
/// Значения по умолчанию: бюджет 50 итераций и предел LLR `20.0`. Бюджет
/// может быть равен нулю. Предел должен быть конечным числом из диапазона
/// `(0, 20]`. Публичный [`crate::spa_step`] выполняет ровно один шаг при любом
/// бюджете; `max_iterations` предназначен для цикла полного декодера.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecoderConfig {
    max_iterations: usize,
    llr_limit: f64,
}

impl DecoderConfig {
    /// Создаёт конфигурацию с бюджетом итераций и пределом LLR.
    ///
    /// Возвращает [`LdpcError::InvalidLlrLimit`], если `llr_limit` не является
    /// конечным числом из диапазона `(0, 20]`. Значение `max_iterations = 0`
    /// допустимо.
    pub fn try_new(max_iterations: usize, llr_limit: f64) -> Result<Self, LdpcError> {
        if !llr_limit.is_finite() || llr_limit <= 0.0 || llr_limit > MAX_LLR_LIMIT {
            return Err(LdpcError::InvalidLlrLimit);
        }

        Ok(Self {
            max_iterations,
            llr_limit,
        })
    }

    /// Возвращает бюджет итераций полного декодера.
    ///
    /// [`crate::spa_step`] этот бюджет не применяет и всегда выполняет один шаг.
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    /// Возвращает предел насыщения канальных LLR и сообщений SPA.
    pub fn llr_limit(&self) -> f64 {
        self.llr_limit
    }
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            llr_limit: MAX_LLR_LIMIT,
        }
    }
}

/// Причина завершения полного декодирования.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeStatus {
    /// Итоговое слово удовлетворяет всем проверкам `H`.
    ParitySatisfied,
    /// Бюджет итераций исчерпан до получения нулевого синдрома.
    IterationLimit,
}

/// Результат декодирования одного блока.
///
/// Результат владеет словом, итоговыми LLR и синдромом. Его данные не
/// изменяются при следующих вызовах декодера. Нулевой синдром означает, что
/// слово удовлетворяет проверочной матрице, но не доказывает совпадение с
/// переданным словом. SPA строит начальное жёсткое решение по LLR после
/// насыщения пределом конфигурации и применения стираний.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodeResult {
    pub(crate) word: Vec<Bit>,
    pub(crate) posterior_llrs: Vec<f64>,
    pub(crate) syndrome: Vec<Bit>,
    pub(crate) status: DecodeStatus,
    pub(crate) iterations: usize,
    pub(crate) changed_bits: usize,
    pub(crate) initial_unsatisfied_checks: usize,
    pub(crate) final_unsatisfied_checks: usize,
}

impl DecodeResult {
    /// Создаёт проверяемый результат декодирования для реализации
    /// [`coding_core::Decoder`].
    ///
    /// Итоговое жёсткое слово строится из знаков `posterior_llrs`: значение
    /// `>= 0.0`, включая оба нуля, становится [`Bit::Zero`], отрицательное —
    /// [`Bit::One`]. Синдром и все счётчики вычисляются по `checks`,
    /// `initial_word` и полученному слову. Результат забирает владение
    /// `posterior_llrs`; `initial_word` используется только для расчёта
    /// диагностики и не сохраняется.
    ///
    /// `initial_word` — заимствованное начальное жёсткое решение конкретного
    /// алгоритма. Для SPA это решение канальных LLR после насыщения и
    /// применения стираний.
    /// Предел `iteration_limit` задаётся вызывающим декодером и проверяется
    /// относительно `iterations` и `status`.
    ///
    /// Конструктор проверяет форму результата, конечность итоговых LLR,
    /// соответствие статуса синдрому и пределу итераций. Он не может подтвердить
    /// историю стороннего алгоритма: соответствие `initial_word` исходному
    /// каналу и факт выполнения каждой заявленной итерации остаются на
    /// ответственности реализации [`coding_core::Decoder`]. При нуле итераций итоговое
    /// решение обязано совпадать с `initial_word`; конструктор проверяет это.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`LdpcError::WordLengthMismatch`] или
    /// [`LdpcError::LlrLengthMismatch`] для неверных длин,
    /// [`LdpcError::NonFiniteLlr`] для NaN или бесконечности,
    /// [`LdpcError::DecodeIterationsExceedLimit`] если итераций больше предела,
    /// и [`LdpcError::DecodeResultStatusMismatch`] если статус не согласуется с
    /// синдромом либо бюджетом. `ParitySatisfied` требует нулевой синдром;
    /// `IterationLimit` требует ненулевой синдром и `iterations == iteration_limit`.
    /// [`LdpcError::DecodeResultZeroIterationsMismatch`] возвращается, если
    /// итоговое слово изменилось при нуле итераций.
    pub fn try_new(
        checks: &ParityCheckMatrix,
        initial_word: &[Bit],
        posterior_llrs: Vec<f64>,
        status: DecodeStatus,
        iterations: usize,
        iteration_limit: usize,
    ) -> Result<Self, LdpcError> {
        let expected = checks.cols();
        if initial_word.len() != expected {
            return Err(LdpcError::WordLengthMismatch {
                expected,
                actual: initial_word.len(),
            });
        }
        if posterior_llrs.len() != expected {
            return Err(LdpcError::LlrLengthMismatch {
                expected,
                actual: posterior_llrs.len(),
            });
        }
        if let Some(index) = posterior_llrs.iter().position(|llr| !llr.is_finite()) {
            return Err(LdpcError::NonFiniteLlr { index });
        }
        if iterations > iteration_limit {
            return Err(LdpcError::DecodeIterationsExceedLimit {
                iterations,
                limit: iteration_limit,
            });
        }

        let word: Vec<_> = posterior_llrs
            .iter()
            .map(|&llr| if llr >= 0.0 { Bit::Zero } else { Bit::One })
            .collect();
        let syndrome = checks.syndrome(&word)?;
        let initial_syndrome = checks.syndrome(initial_word)?;
        let final_unsatisfied_checks = count_ones(&syndrome);
        let initial_unsatisfied_checks = count_ones(&initial_syndrome);

        if iterations == 0 && initial_word != word.as_slice() {
            return Err(LdpcError::DecodeResultZeroIterationsMismatch);
        }

        let status_is_consistent = match status {
            DecodeStatus::ParitySatisfied => final_unsatisfied_checks == 0,
            DecodeStatus::IterationLimit => {
                final_unsatisfied_checks != 0 && iterations == iteration_limit
            }
        };
        if !status_is_consistent {
            return Err(LdpcError::DecodeResultStatusMismatch);
        }

        let changed_bits = bit_distance(initial_word, &word);

        Ok(Self {
            word,
            posterior_llrs,
            syndrome,
            status,
            iterations,
            changed_bits,
            initial_unsatisfied_checks,
            final_unsatisfied_checks,
        })
    }

    /// Возвращает итоговое жёсткое слово в порядке столбцов исходной `H`.
    #[must_use]
    pub fn word(&self) -> &[Bit] {
        &self.word
    }

    /// Возвращает итоговые LLR в порядке столбцов исходной `H`.
    ///
    /// Все значения конечны. SPA дополнительно ограничивает их пределом из
    /// [`crate::DecoderConfig`].
    #[must_use]
    pub fn posterior_llrs(&self) -> &[f64] {
        &self.posterior_llrs
    }

    /// Возвращает синдром итогового слова в исходном порядке проверок.
    #[must_use]
    pub fn syndrome(&self) -> &[Bit] {
        &self.syndrome
    }

    /// Возвращает причину остановки декодера.
    #[must_use]
    pub fn status(&self) -> DecodeStatus {
        self.status
    }

    /// Возвращает число полностью выполненных итераций, заявленное декодером.
    ///
    /// [`try_new`](Self::try_new) проверяет число относительно объявленного
    /// предела, но не может подтвердить историю выполнения стороннего алгоритма.
    #[must_use]
    pub fn iterations(&self) -> usize {
        self.iterations
    }

    /// Возвращает число позиций, изменившихся от начального решения к итоговому.
    ///
    /// Для SPA начальное решение строится после насыщения LLR пределом
    /// конфигурации и зануления стёртых позиций. В стороннем декодере оно
    /// задаётся при создании результата. Это число не является числом
    /// исправленных ошибок: без эталонного слова декодер не знает, какие
    /// изменения верны.
    #[must_use]
    pub fn changed_bits(&self) -> usize {
        self.changed_bits
    }

    /// Возвращает число нарушенных проверок начального решения. Для SPA оно
    /// строится после насыщения LLR пределом конфигурации и применения стираний;
    /// сторонний декодер передаёт начальное решение в [`try_new`](Self::try_new).
    #[must_use]
    pub fn initial_unsatisfied_checks(&self) -> usize {
        self.initial_unsatisfied_checks
    }

    /// Возвращает число нарушенных проверок итогового жёсткого решения.
    #[must_use]
    pub fn final_unsatisfied_checks(&self) -> usize {
        self.final_unsatisfied_checks
    }

    /// Возвращает сводку выполненной проверки чётности итогового слова.
    ///
    /// Сводка строится из уже вычисленного
    /// [`final_unsatisfied_checks`](Self::final_unsatisfied_checks): синдром
    /// не копируется и повторно не подсчитывается. Успешная проверка не
    /// доказывает совпадение с оригиналом, поскольку декодер не знает
    /// переданного слова.
    #[must_use]
    pub fn checks(&self) -> DecodeChecks {
        DecodeChecks {
            unsatisfied_checks: self.final_unsatisfied_checks,
        }
    }
}

/// Сводка выполненной проверки чётности итогового слова.
///
/// Хранит только число нарушенных строк `H`. Нулевое значение означает, что
/// слово удовлетворяет проверочной матрице, но не доказывает совпадение с
/// оригиналом: декодер не знает переданного слова, а другое допустимое кодовое
/// слово также даст пройденную проверку.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeChecks {
    unsatisfied_checks: usize,
}

impl DecodeChecks {
    /// Возвращает число нарушенных проверок итогового слова.
    #[must_use]
    pub fn unsatisfied_checks(&self) -> usize {
        self.unsatisfied_checks
    }

    /// Возвращает `true`, если все проверки чётности итогового слова выполнены.
    #[must_use]
    pub fn parity_satisfied(&self) -> bool {
        self.unsatisfied_checks == 0
    }
}

/// Исход декодирования LDPC с богатым результатом и сводкой проверок.
///
/// Неиспользуемые варианты [`NeedMore`](coding_core::DecodeOutcome::NeedMore) и
/// [`Stopped`](coding_core::DecodeOutcome::Stopped) типизированы
/// [`Infallible`](core::convert::Infallible): текущий блочный декодер не может
/// запросить дополнительные наблюдения или остановиться без кандидата.
pub type LdpcDecodeOutcome = coding_core::DecodeOutcome<
    DecodeResult,
    DecodeChecks,
    core::convert::Infallible,
    core::convert::Infallible,
>;

/// Событие синхронного наблюдения за одним вызовом декодера.
///
/// Наблюдатель реализует [`coding_core::Observer<DecodeEvent>`] либо передаётся
/// замыкание `FnMut(&DecodeEvent)`; без журнала используют
/// [`coding_core::NoopObserver`]. События приходят синхронно: один `Started`,
/// по одному `IterationFinished` на полностью завершённую итерацию, затем один
/// `Finished`. При ошибке входа событий нет. Порядок и данные событий не
/// зависят от наблюдателя; паника обработчика не гарантирует восстановления.
///
/// Поля последнего события совпадают с [`DecodeResult`], а число нарушенных
/// проверок — с [`DecodeChecks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeEvent {
    /// Декодер подготовил начальное жёсткое решение.
    Started {
        /// Число нарушенных проверок начального решения.
        unsatisfied_checks: usize,
    },
    /// Полностью завершена одна итерация SPA.
    IterationFinished {
        /// Номер итерации, начиная с единицы.
        iteration: usize,
        /// Число нарушенных проверок после этой итерации.
        unsatisfied_checks: usize,
        /// Расстояние от начального решения до слова после этой итерации.
        changed_bits: usize,
    },
    /// Декодирование остановлено, показатели совпадают с результатом.
    Finished {
        /// Число полностью выполненных итераций.
        iterations: usize,
        /// Причина остановки декодера.
        status: DecodeStatus,
        /// Число нарушенных проверок итогового слова.
        unsatisfied_checks: usize,
        /// Расстояние от начального решения до итогового слова.
        changed_bits: usize,
    },
}

#[cfg(test)]
#[path = "../tests/unit/decode.rs"]
mod tests;
