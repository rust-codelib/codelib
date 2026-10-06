//! Публичный контракт полного SPA-декодирования и его результат.

use crate::{Bit, DecodeInput, LdpcError};

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
/// переданным словом. Начальное жёсткое решение строится по LLR после
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
    /// Возвращает итоговое жёсткое слово в порядке столбцов исходной `H`.
    #[must_use]
    pub fn word(&self) -> &[Bit] {
        &self.word
    }

    /// Возвращает итоговые LLR в порядке столбцов исходной `H`.
    ///
    /// Все значения конечны и ограничены пределом из [`crate::DecoderConfig`].
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

    /// Возвращает число полностью выполненных итераций SPA.
    #[must_use]
    pub fn iterations(&self) -> usize {
        self.iterations
    }

    /// Возвращает число позиций, изменившихся от начального решения к итоговому.
    ///
    /// Начальное решение строится после насыщения LLR пределом конфигурации и
    /// зануления стёртых позиций. Это число не является числом исправленных
    /// ошибок: без эталонного слова декодер не знает, какие изменения верны.
    #[must_use]
    pub fn changed_bits(&self) -> usize {
        self.changed_bits
    }

    /// Возвращает число нарушенных проверок начального решения после насыщения
    /// LLR пределом конфигурации и применения стираний.
    #[must_use]
    pub fn initial_unsatisfied_checks(&self) -> usize {
        self.initial_unsatisfied_checks
    }

    /// Возвращает число нарушенных проверок итогового жёсткого решения.
    #[must_use]
    pub fn final_unsatisfied_checks(&self) -> usize {
        self.final_unsatisfied_checks
    }
}

/// Событие синхронного наблюдения за одним вызовом декодера.
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

/// Получатель событий одного вызова декодера.
pub trait DecodeObserver {
    /// Обрабатывает событие до продолжения декодирования.
    fn on_event(&mut self, event: &DecodeEvent);
}

/// Общий контракт декодера одного кодового блока.
pub trait Decoder {
    /// Возвращает число LLR, ожидаемое для одного входного блока.
    fn codeword_len(&self) -> usize;

    /// Декодирует один блок и при необходимости синхронно отправляет события.
    ///
    /// Перед жёстким решением конечные LLR насыщаются пределом конфигурации,
    /// а стёртые позиции заменяются нулём. Решение равно [`Bit::Zero`] при LLR
    /// `>= 0.0` (включая `+0.0` и `-0.0`) и [`Bit::One`] при отрицательном LLR.
    ///
    /// Для корректного входа события идут в порядке `Started`, по одному
    /// `IterationFinished` на полностью завершённую итерацию, затем `Finished`.
    /// При ошибке входа событий нет. Проверки выполняются в порядке: длина LLR,
    /// конечность значений, границы индексов стираний, повторы индексов.
    /// Исчерпание бюджета итераций является обычным [`DecodeStatus`] в результате.
    fn decode(
        &mut self,
        input: DecodeInput<'_>,
        observer: Option<&mut dyn DecodeObserver>,
    ) -> Result<DecodeResult, LdpcError>;
}
