//! Публичный контракт декодирования LDPC-кода.
//!
//! Содержит канальные данные для шага или полного декодирования SPA,
//! настройки декодирования, подготовку канала и результат блока.

use crate::bit::{bit_distance, count_ones};
use crate::{Bit, LdpcError, ParityCheckMatrix};

/// Заимствованные канальные LLR и позиции стираний для [`crate::spa_step`] и
/// [`coding_core::Decoder::decode`].
///
/// `llrs[i]` относится к биту `i` в порядке столбцов исходной `H`. Соглашение
/// о знаке — [`coding_core::LlrConvention::PositiveForZero`]:
/// `L = ln(P(0) / P(1))`, поэтому неотрицательное значение предпочитает ноль,
/// отрицательное — единицу. При `+0.0` и `-0.0` выбирается [`crate::Bit::Zero`].
/// Знак не переключается автоматически; при необходимости вызывающий код
/// заранее преобразует значения [`coding_core::LlrConvention::convert`].
///
/// Срезы не копируются и не изменяются. Оба API не сохраняют ссылки после
/// вызова; результаты владеют своими массивами.
///
/// Оба API проверяют вход в таком порядке: длина LLR
/// ([`LdpcError::LlrLengthMismatch`]), конечность всех значений
/// ([`LdpcError::NonFiniteLlr`], включая стираемые позиции), границы индексов
/// стираний ([`LdpcError::ErasureIndexOutOfBounds`]) и повторы индексов
/// ([`LdpcError::DuplicateErasureIndex`]). После успешной проверки значения
/// ограничиваются [`DecoderConfig::llr_limit`], а стираемые позиции заменяются
/// на `0.0`. Стирание означает отсутствие предпочтения между битами, а не
/// инверсию значения.
///
/// При ошибке входа `SpaDecoder` не меняет содержательное состояние блока и не
/// отправляет события; следующий корректный вызов работает как обычно. Порядок
/// событий описан в [`DecodeEvent`].
#[derive(Debug, Clone, Copy)]
pub struct DecodeInput<'a> {
    /// Канальное LLR для каждого бита кодового слова в порядке столбцов `H`.
    pub llrs: &'a [f64],
    /// Индексы битов, которые следует считать стёртыми; повторы запрещены.
    pub erasures: &'a [usize],
}

// Проверяет вход до изменения канала и заполняет переиспользуемый канал.
// Маска стираний служебная: при ошибке повтора она может остаться очищенной
// или частично заполненной, поэтому следующий вызов сбрасывает её заново.
// До полного успеха канал и содержательное состояние блока не меняются.
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

// `-0.0` тоже даёт Zero: сравнение с нулём истинно для обоих знаковых нулей.
pub(crate) fn hard_decision(llr: f64) -> Bit {
    if llr >= 0.0 {
        Bit::Zero
    } else {
        Bit::One
    }
}

const MAX_LLR_LIMIT: f64 = 20.0;

/// Неизменяемые параметры SPA-декодера.
///
/// [`Default`] задаёт бюджет 50 итераций и предел LLR `20.0`. Бюджет может быть
/// равен нулю. Предел должен быть конечным числом из диапазона `(0, 20]`.
/// Бюджет использует полный декодер; публичный [`crate::spa_step`] выполняет
/// ровно один шаг при любом бюджете.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecoderConfig {
    max_iterations: usize,
    llr_limit: f64,
}

impl DecoderConfig {
    /// Создаёт конфигурацию с бюджетом итераций и пределом LLR.
    ///
    /// `max_iterations = 0` допустим.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`LdpcError::InvalidLlrLimit`], если `llr_limit` не является
    /// конечным числом из диапазона `(0, 20]`.
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
///
/// Статус описывает только условие остановки, а не совпадение с переданным
/// словом; см. [`DecodeResult`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeStatus {
    /// Итоговое слово удовлетворяет всем проверкам `H`.
    ParitySatisfied,
    /// Бюджет итераций исчерпан до получения нулевого синдрома.
    IterationLimit,
}

/// Результат декодирования одного блока.
///
/// Владеет итоговым словом, LLR и синдромом; данные не меняются при следующих
/// вызовах декодера. Нулевой синдром означает только, что слово удовлетворяет
/// `H`, но не доказывает совпадение с переданным словом. [`DecodeStatus`]
/// описывает причину остановки, а [`changed_bits`](Self::changed_bits) — не
/// число исправленных ошибок.
///
/// SPA строит начальное жёсткое решение по LLR после насыщения пределом
/// конфигурации и применения стираний. Внешний декодер создаёт проверяемый
/// результат через [`try_new`](Self::try_new), передавая своё начальное решение
/// и проходя проверки формы, конечности LLR, статуса и бюджета.
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
    /// `checks` задаёт `H` и ожидаемую длину. `initial_word` заимствуется для
    /// диагностики и проверок и не сохраняется, а `posterior_llrs` передаются во
    /// владение результату. `status`, `iterations` и `iteration_limit` описывают
    /// остановку и бюджет декодера. Жёсткое слово строится из знаков итоговых
    /// LLR по правилу знака [`DecodeInput`]; см. также [`DecodeResult`].
    ///
    /// Конструктор проверяет форму, конечность LLR, статус и бюджет, но не может
    /// подтвердить историю стороннего алгоритма: соответствие `initial_word`
    /// исходному каналу и факт выполнения заявленных итераций остаются на
    /// ответственности реализации [`coding_core::Decoder`].
    ///
    /// # Ошибки
    ///
    /// Возвращает [`LdpcError::WordLengthMismatch`] или
    /// [`LdpcError::LlrLengthMismatch`] для неверных длин,
    /// [`LdpcError::NonFiniteLlr`] для NaN или бесконечности,
    /// [`LdpcError::DecodeIterationsExceedLimit`], если итераций больше предела,
    /// и [`LdpcError::DecodeResultStatusMismatch`], если статус не согласуется с
    /// синдромом либо бюджетом. `ParitySatisfied` требует нулевой синдром;
    /// `IterationLimit` требует ненулевой синдром и
    /// `iterations == iteration_limit`.
    /// [`LdpcError::DecodeResultZeroIterationsMismatch`] возвращается, если при
    /// нуле итераций итоговое слово не совпало с `initial_word`.
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
            .map(|&llr| hard_decision(llr))
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
    /// Для SPA начальное решение строится после насыщения LLR и применения
    /// стираний; сторонний декодер передаёт его в [`try_new`](Self::try_new).
    /// Это не число исправленных ошибок: без эталонного слова декодер не знает,
    /// какие изменения верны.
    #[must_use]
    pub fn changed_bits(&self) -> usize {
        self.changed_bits
    }

    /// Возвращает число нарушенных проверок начального решения.
    ///
    /// Начальное решение определено так же, как для
    /// [`changed_bits`](Self::changed_bits).
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
