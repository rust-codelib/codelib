//! Полный SPA-декодер и публичный помощник одного шага.
//!
//! Построение разреженного графа и численное состояние находятся в `spa/graph.rs`
//! и `spa/state.rs`.

mod graph;
mod state;

use coding_core::{Candidate, DecodeOutcome, Observer};

use crate::bit::{bit_distance, count_ones};
#[cfg(test)]
use crate::decode::prepared_channel;
use crate::{
    Bit, DecodeChecks, DecodeEvent, DecodeInput, DecodeResult, DecodeStatus, DecoderConfig,
    LdpcDecodeOutcome, LdpcError, ParityCheckMatrix,
};
use state::SpaState;

/// Жёсткое слово и итоговые LLR после одного шага SPA.
///
/// Оба массива принадлежат результату и имеют длину числа столбцов исходной
/// проверочной матрицы. Позиции соответствуют исходному порядку столбцов `H`;
/// итоговые LLR ограничены пределом конфигурации. Жёсткое слово содержит
/// [`Bit::Zero`] при LLR `>= 0` (в том числе при `+0.0` и `-0.0`) и
/// [`Bit::One`] при отрицательном LLR. Сообщения рёбер остаются внутренним
/// состоянием.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaStepResult {
    word: Vec<Bit>,
    posterior_llrs: Vec<f64>,
}

impl SpaStepResult {
    /// Возвращает жёсткое слово в порядке столбцов исходной матрицы.
    #[must_use]
    pub fn word(&self) -> &[Bit] {
        &self.word
    }

    /// Возвращает принадлежащие результату итоговые LLR в исходном порядке `H`.
    ///
    /// Значения ограничены заданным пределом LLR.
    #[must_use]
    pub fn posterior_llrs(&self) -> &[f64] {
        &self.posterior_llrs
    }
}

/// Переиспользуемый декодер по алгоритму sum-product (SPA).
#[derive(Debug)]
pub struct SpaDecoder {
    state: SpaState,
    config: DecoderConfig,
}

impl SpaDecoder {
    /// Создаёт SPA-декодер по исходной проверочной матрице и конфигурации.
    ///
    /// Любая допустимая матрица подходит для декодирования, включая матрицу
    /// нулевого или полного ранга. Граф связей и рабочие буферы создаются один
    /// раз и повторно используются для последующих блоков.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`LdpcError::SizeOverflow`], если размер внутреннего графа или
    /// рабочих буферов нельзя представить типом `usize`.
    pub fn try_new(checks: ParityCheckMatrix, config: DecoderConfig) -> Result<Self, LdpcError> {
        Self::try_new_borrowed(&checks, config)
    }

    pub(crate) fn try_new_borrowed(
        checks: &ParityCheckMatrix,
        config: DecoderConfig,
    ) -> Result<Self, LdpcError> {
        let state = SpaState::try_new(checks)?;
        Ok(Self { state, config })
    }

    /// Возвращает число LLR, ожидаемое для одного входного блока.
    #[must_use]
    pub fn codeword_len(&self) -> usize {
        self.state.graph.bit_edges.len()
    }
}

/// Декодирует один блок и синхронно передаёт события общему наблюдателю.
///
/// Перед жёстким решением конечные LLR насыщаются пределом конфигурации,
/// а стёртые позиции заменяются нулём. Решение равно [`Bit::Zero`] при LLR
/// `>= 0.0` (включая `+0.0` и `-0.0`) и [`Bit::One`] при отрицательном LLR.
/// Входной срез `DecodeInput` берётся по значению и не сохраняется после
/// вызова; [`DecodeResult`] и его массивы принадлежат кандидату.
///
/// Для корректного входа события идут в порядке `Started`, по одному
/// `IterationFinished` на полностью завершённую итерацию, затем `Finished`.
/// При ошибке входа событий нет. Проверки выполняются в порядке: длина LLR,
/// конечность значений, границы индексов стираний, повторы индексов.
/// Исчерпание бюджета итераций является обычным [`DecodeStatus`] в результате:
/// при `IterationLimit`, включая бюджет `0`, возвращается
/// [`DecodeOutcome::Candidate`] с сохранёнными словом, LLR и синдромом, а не
/// ошибка или [`DecodeOutcome::Stopped`].
///
/// Декодер обрабатывает каждый блок независимо и не накапливает канальные
/// данные между вызовами. Ранее возвращённые результаты не изменяются.
impl<'a> coding_core::Decoder<DecodeInput<'a>> for SpaDecoder {
    type Value = DecodeResult;
    type Checks = DecodeChecks;
    type NeedMore = core::convert::Infallible;
    type StopReason = core::convert::Infallible;
    type Error = LdpcError;
    type Event = DecodeEvent;

    fn decode(
        &mut self,
        input: DecodeInput<'a>,
        observer: &mut dyn Observer<DecodeEvent>,
    ) -> Result<LdpcDecodeOutcome, LdpcError> {
        self.state.prepare_block(input, self.config)?;

        let mut syndrome = vec![Bit::Zero; self.state.graph.check_edges.len()];
        self.state.fill_syndrome(&mut syndrome);
        let initial_unsatisfied_checks = count_ones(&syndrome);
        let initial_word = self.state.word.clone();
        observer.on_event(&DecodeEvent::Started {
            unsatisfied_checks: initial_unsatisfied_checks,
        });

        let mut iterations = 0;
        let mut status = (initial_unsatisfied_checks == 0).then_some(DecodeStatus::ParitySatisfied);

        while status.is_none() && iterations < self.config.max_iterations() {
            self.state.step(self.config);
            iterations += 1;
            self.state.fill_syndrome(&mut syndrome);

            let unsatisfied_checks = count_ones(&syndrome);
            let changed_bits = bit_distance(&initial_word, &self.state.word);
            observer.on_event(&DecodeEvent::IterationFinished {
                iteration: iterations,
                unsatisfied_checks,
                changed_bits,
            });

            if unsatisfied_checks == 0 {
                status = Some(DecodeStatus::ParitySatisfied);
            }
        }

        let status = status.unwrap_or(DecodeStatus::IterationLimit);
        let final_unsatisfied_checks = count_ones(&syndrome);
        let changed_bits = bit_distance(&initial_word, &self.state.word);
        observer.on_event(&DecodeEvent::Finished {
            iterations,
            status,
            unsatisfied_checks: final_unsatisfied_checks,
            changed_bits,
        });

        let result = DecodeResult {
            word: self.state.word.clone(),
            posterior_llrs: self.state.posterior.clone(),
            syndrome,
            status,
            iterations,
            changed_bits,
            initial_unsatisfied_checks,
            final_unsatisfied_checks,
        };
        let checks = result.checks();

        Ok(DecodeOutcome::Candidate(Candidate {
            value: result,
            checks,
        }))
    }
}

/// Выполняет ровно один flooding-шаг SPA по разреженной проверочной матрице.
///
/// LLR задаются как `ln(P(0) / P(1))`: неотрицательное значение предпочитает
/// [`Bit::Zero`], отрицательное — [`Bit::One`]. Конечные канальные значения
/// ограничиваются `±config.llr_limit()`; NaN и бесконечности отклоняются,
/// даже если позиция стёрта. Стирание заменяет канальное LLR на `0.0` и не
/// инвертирует решение. Сообщения `q`, `r` и итоговые posterior LLR также
/// насыщаются этим пределом, поэтому конечные входы вроде `±f64::MAX` не
/// передаются дальше без ограничения.
///
/// Сначала проверяются длина LLR, конечность значений, индексы стираний и
/// повторы индексов, затем выполняются фаза проверок и фаза битов. Результат
/// владеет новыми массивами слова и posterior LLR в порядке столбцов исходной
/// `H`; входные срезы не изменяются и не сохраняются. Каждое обращение создаёт
/// новое рабочее состояние и не продолжает предыдущий шаг.
///
/// `config.max_iterations()` не ограничивает эту функцию: она выполняет один
/// шаг даже при бюджете `0`. Функция не проверяет исходный синдром для ранней
/// остановки и не сообщает число итераций, статус остановки или диагностику
/// полного декодирования. Нулевой синдром результата означает только, что
/// слово удовлетворяет проверкам `H`.
///
/// Для `m` проверок, `n` битов и `E` единиц матрицы время и временная память
/// имеют порядок `O(m + n + E)`.
///
/// # Ошибки
///
/// Возвращает [`LdpcError::LlrLengthMismatch`], [`LdpcError::NonFiniteLlr`],
/// [`LdpcError::ErasureIndexOutOfBounds`] или [`LdpcError::DuplicateErasureIndex`]
/// при неверном входе, а также [`LdpcError::SizeOverflow`], если размер рабочих
/// буферов нельзя представить. Проверки содержимого выполняются именно в
/// указанном порядке: длина, первое не конечное LLR, первый выход индекса за
/// границы, первый повтор.
pub fn spa_step(
    checks: &ParityCheckMatrix,
    config: DecoderConfig,
    input: DecodeInput<'_>,
) -> Result<SpaStepResult, LdpcError> {
    let mut state = SpaState::try_new(checks)?;
    state.prepare_block(input, config)?;
    state.step(config);
    Ok(state.into_result())
}

#[cfg(test)]
#[path = "../tests/unit/spa/support.rs"]
mod support;

#[cfg(test)]
#[path = "../tests/unit/spa/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/spa/reference_tests.rs"]
mod reference_tests;

#[cfg(test)]
#[path = "../tests/unit/spa/decoder_tests.rs"]
mod decoder_tests;
