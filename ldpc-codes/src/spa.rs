//! Разреженный граф и один flooding-шаг алгоритма SPA.
//!
//! Номер ребра общий для списка соответствующей проверки и списка
//! соответствующего бита.

use core::mem::size_of;

use crate::decode_input::prepare_channel;
use crate::{
    Bit, DecodeEvent, DecodeInput, DecodeObserver, DecodeResult, DecodeStatus, Decoder,
    DecoderConfig, LdpcError, ParityCheckMatrix,
};

#[derive(Debug, PartialEq, Eq)]
struct SparseGraph {
    check_edges: Vec<Vec<usize>>,
    bit_edges: Vec<Vec<usize>>,
    edge_bits: Vec<usize>,
}

impl SparseGraph {
    #[cfg(test)]
    fn try_new(checks: &ParityCheckMatrix) -> Result<Self, LdpcError> {
        let sizes = checked_workspace_sizes(checks)?;
        Ok(Self::from_checked_sizes(checks, sizes))
    }

    fn from_checked_sizes(checks: &ParityCheckMatrix, sizes: WorkspaceSizes) -> Self {
        let mut check_edges = Vec::with_capacity(sizes.checks);
        let mut bit_edges = Vec::with_capacity(sizes.bits);
        for bit in 0..sizes.bits {
            let degree = checks
                .bit_checks(bit)
                .expect("bit index is within the matrix shape")
                .len();
            bit_edges.push(Vec::with_capacity(degree));
        }

        let mut edge_bits = Vec::with_capacity(sizes.edges);
        for check in 0..sizes.checks {
            let bits = checks
                .check_bits(check)
                .expect("check index is within the matrix shape");
            let mut edges = Vec::with_capacity(bits.len());
            for &bit in bits {
                let edge = edge_bits.len();
                edge_bits.push(bit);
                edges.push(edge);
                bit_edges[bit].push(edge);
            }
            check_edges.push(edges);
        }

        debug_assert_eq!(edge_bits.len(), sizes.edges);
        Self {
            check_edges,
            bit_edges,
            edge_bits,
        }
    }
}

#[derive(Debug)]
struct SpaState {
    graph: SparseGraph,
    channel: Vec<f64>,
    q: Vec<f64>,
    r: Vec<f64>,
    posterior: Vec<f64>,
    word: Vec<Bit>,
    tanh: Vec<f64>,
    prefix: Vec<f64>,
    suffix: Vec<f64>,
}

impl SpaState {
    fn try_new(checks: &ParityCheckMatrix) -> Result<Self, LdpcError> {
        let sizes = checked_workspace_sizes(checks)?;
        let graph = SparseGraph::from_checked_sizes(checks, sizes);
        let scratch_len = sizes.scratch_len;

        Ok(Self {
            graph,
            channel: vec![0.0; sizes.bits],
            q: vec![0.0; sizes.edges],
            r: vec![0.0; sizes.edges],
            posterior: vec![0.0; sizes.bits],
            word: vec![Bit::Zero; sizes.bits],
            tanh: vec![0.0; scratch_len],
            prefix: vec![1.0; scratch_len],
            suffix: vec![1.0; scratch_len],
        })
    }

    fn prepare_block(
        &mut self,
        input: DecodeInput<'_>,
        config: DecoderConfig,
    ) -> Result<(), LdpcError> {
        // До этой точки self не меняется: prepare_channel проверяет вход целиком.
        let channel = prepare_channel(input, self.graph.bit_edges.len(), config)?;

        self.channel = channel;
        self.posterior.copy_from_slice(&self.channel);
        self.r.fill(0.0);
        self.tanh.fill(0.0);
        self.prefix.fill(1.0);
        self.suffix.fill(1.0);
        for (edge, &bit) in self.graph.edge_bits.iter().enumerate() {
            self.q[edge] = self.channel[bit];
        }
        for (word_bit, &llr) in self.word.iter_mut().zip(&self.channel) {
            *word_bit = hard_decision(llr);
        }

        Ok(())
    }

    fn update_check_messages(&mut self, config: DecoderConfig) {
        for edges in &self.graph.check_edges {
            match edges.as_slice() {
                [] => {}
                [edge] => self.r[*edge] = config.llr_limit(),
                [first, second] => {
                    self.r[*first] = self.q[*second];
                    self.r[*second] = self.q[*first];
                }
                _ => {
                    let degree = edges.len();
                    let llr_limit = config.llr_limit();

                    for (index, &edge) in edges.iter().enumerate() {
                        let q = self.q[edge].clamp(-llr_limit, llr_limit);
                        self.tanh[index] = (q / 2.0).tanh();
                    }

                    self.prefix[0] = 1.0;
                    for index in 0..degree {
                        self.prefix[index + 1] = self.prefix[index] * self.tanh[index];
                    }

                    self.suffix[degree] = 1.0;
                    for index in (0..degree).rev() {
                        self.suffix[index] = self.tanh[index] * self.suffix[index + 1];
                    }

                    for (index, &edge) in edges.iter().enumerate() {
                        let product = self.prefix[index] * self.suffix[index + 1];
                        self.r[edge] = llr_from_product(product, config);
                    }
                }
            }
        }
    }

    fn update_bit_messages(&mut self, config: DecoderConfig) {
        let llr_limit = config.llr_limit();
        for (bit, edges) in self.graph.bit_edges.iter().enumerate() {
            if edges.is_empty() {
                self.posterior[bit] = self.channel[bit];
                self.word[bit] = hard_decision(self.channel[bit]);
                continue;
            }

            let incoming = edges.iter().map(|&edge| self.r[edge]).sum::<f64>();
            let total = self.channel[bit] + incoming;

            self.posterior[bit] = total.clamp(-llr_limit, llr_limit);
            self.word[bit] = hard_decision(self.posterior[bit]);
            if edges.len() == 1 {
                self.q[edges[0]] = self.channel[bit].clamp(-llr_limit, llr_limit);
            } else {
                for &edge in edges {
                    self.q[edge] = (total - self.r[edge]).clamp(-llr_limit, llr_limit);
                }
            }
        }
    }

    fn step(&mut self, config: DecoderConfig) {
        self.update_check_messages(config);
        self.update_bit_messages(config);
    }

    fn fill_syndrome(&self, syndrome: &mut [Bit]) {
        debug_assert_eq!(syndrome.len(), self.graph.check_edges.len());
        for (check, edges) in self.graph.check_edges.iter().enumerate() {
            let mut parity = false;
            for &edge in edges {
                let bit = self.graph.edge_bits[edge];
                parity ^= self.word[bit] == Bit::One;
            }
            syndrome[check] = if parity { Bit::One } else { Bit::Zero };
        }
    }

    fn into_result(self) -> SpaStepResult {
        SpaStepResult {
            word: self.word,
            posterior_llrs: self.posterior,
        }
    }

    #[cfg(test)]
    fn snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            channel: float_bits(&self.channel),
            q: float_bits(&self.q),
            r: float_bits(&self.r),
            posterior: float_bits(&self.posterior),
            word: self.word.clone(),
            tanh: float_bits(&self.tanh),
            prefix: float_bits(&self.prefix),
            suffix: float_bits(&self.suffix),
        }
    }
}

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
        let state = SpaState::try_new(&checks)?;
        Ok(Self { state, config })
    }
}

impl Decoder for SpaDecoder {
    fn codeword_len(&self) -> usize {
        self.state.graph.bit_edges.len()
    }

    fn decode(
        &mut self,
        input: DecodeInput<'_>,
        observer: Option<&mut dyn DecodeObserver>,
    ) -> Result<DecodeResult, LdpcError> {
        self.state.prepare_block(input, self.config)?;

        let mut observer = observer;
        let mut syndrome = vec![Bit::Zero; self.state.graph.check_edges.len()];
        self.state.fill_syndrome(&mut syndrome);
        let initial_unsatisfied_checks = count_unsatisfied_checks(&syndrome);
        let initial_word = self.state.word.clone();
        let started = DecodeEvent::Started {
            unsatisfied_checks: initial_unsatisfied_checks,
        };
        if let Some(observer) = observer.as_mut() {
            (**observer).on_event(&started);
        }

        let mut iterations = 0;
        let mut status = (initial_unsatisfied_checks == 0).then_some(DecodeStatus::ParitySatisfied);

        while status.is_none() && iterations < self.config.max_iterations() {
            self.state.step(self.config);
            iterations += 1;
            self.state.fill_syndrome(&mut syndrome);

            let unsatisfied_checks = count_unsatisfied_checks(&syndrome);
            let changed_bits = count_changed_bits(&initial_word, &self.state.word);
            let event = DecodeEvent::IterationFinished {
                iteration: iterations,
                unsatisfied_checks,
                changed_bits,
            };
            if let Some(observer) = observer.as_mut() {
                (**observer).on_event(&event);
            }

            if unsatisfied_checks == 0 {
                status = Some(DecodeStatus::ParitySatisfied);
            }
        }

        let status = status.unwrap_or(DecodeStatus::IterationLimit);
        let final_unsatisfied_checks = count_unsatisfied_checks(&syndrome);
        let changed_bits = count_changed_bits(&initial_word, &self.state.word);
        let finished = DecodeEvent::Finished {
            iterations,
            status,
            unsatisfied_checks: final_unsatisfied_checks,
            changed_bits,
        };
        if let Some(observer) = observer.as_mut() {
            (**observer).on_event(&finished);
        }

        Ok(DecodeResult {
            word: self.state.word.clone(),
            posterior_llrs: self.state.posterior.clone(),
            syndrome,
            status,
            iterations,
            changed_bits,
            initial_unsatisfied_checks,
            final_unsatisfied_checks,
        })
    }
}

fn count_unsatisfied_checks(syndrome: &[Bit]) -> usize {
    syndrome.iter().filter(|&&bit| bit == Bit::One).count()
}

fn count_changed_bits(initial: &[Bit], current: &[Bit]) -> usize {
    debug_assert_eq!(initial.len(), current.len());
    initial
        .iter()
        .zip(current)
        .filter(|(initial, current)| initial != current)
        .count()
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

fn hard_decision(llr: f64) -> Bit {
    if llr >= 0.0 {
        Bit::Zero
    } else {
        Bit::One
    }
}

fn llr_from_product(product: f64, config: DecoderConfig) -> f64 {
    if product == 0.0 {
        return 0.0;
    }

    let product_limit = 1.0 - f64::EPSILON;
    let product = product.clamp(-product_limit, product_limit);
    (product.ln_1p() - (-product).ln_1p()).clamp(-config.llr_limit(), config.llr_limit())
}

#[derive(Clone, Copy)]
struct WorkspaceSizes {
    checks: usize,
    bits: usize,
    edges: usize,
    scratch_len: usize,
}

fn checked_vec_bytes<T>(len: usize) -> Result<usize, LdpcError> {
    len.checked_mul(size_of::<T>())
        .ok_or(LdpcError::SizeOverflow)
}

fn scratch_len_for_degree(max_degree: usize) -> Result<usize, LdpcError> {
    max_degree.checked_add(1).ok_or(LdpcError::SizeOverflow)
}

fn checked_workspace_sizes(checks: &ParityCheckMatrix) -> Result<WorkspaceSizes, LdpcError> {
    let check_count = checks.rows();
    let bit_count = checks.cols();
    let edge_count = checks.edge_count();

    checked_vec_bytes::<Vec<usize>>(check_count)?;
    checked_vec_bytes::<Vec<usize>>(bit_count)?;
    checked_vec_bytes::<usize>(edge_count)?;
    // Каждый массив указанного типа и длины имеет один и тот же размер в байтах.
    checked_vec_bytes::<Bit>(check_count)?;
    checked_vec_bytes::<f64>(bit_count)?;
    checked_vec_bytes::<Bit>(bit_count)?;
    checked_vec_bytes::<f64>(edge_count)?;

    let mut max_check_degree = 0;
    for check in 0..check_count {
        let degree = checks
            .check_bits(check)
            .expect("check index is within the matrix shape")
            .len();
        checked_vec_bytes::<usize>(degree)?;
        max_check_degree = max_check_degree.max(degree);
    }
    for bit in 0..bit_count {
        let degree = checks
            .bit_checks(bit)
            .expect("bit index is within the matrix shape")
            .len();
        checked_vec_bytes::<usize>(degree)?;
    }

    let scratch_len = scratch_len_for_degree(max_check_degree)?;
    checked_vec_bytes::<f64>(scratch_len)?;

    Ok(WorkspaceSizes {
        checks: check_count,
        bits: bit_count,
        edges: edge_count,
        scratch_len,
    })
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
struct StateSnapshot {
    channel: Vec<u64>,
    q: Vec<u64>,
    r: Vec<u64>,
    posterior: Vec<u64>,
    word: Vec<Bit>,
    tanh: Vec<u64>,
    prefix: Vec<u64>,
    suffix: Vec<u64>,
}

#[cfg(test)]
fn float_bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}

#[cfg(test)]
#[path = "spa/tests.rs"]
mod step_tests;

#[cfg(test)]
#[path = "spa/reference_tests.rs"]
mod reference_tests;

#[cfg(test)]
#[path = "spa/decoder_tests.rs"]
mod decoder_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bit, DecodeInput, DecoderConfig, LdpcError, ParityCheckMatrix};

    #[test]
    fn graph_numbers_edges_in_row_major_order_and_shares_indices() {
        let checks = ParityCheckMatrix::try_from_rows(
            6,
            vec![vec![4, 1], vec![], vec![3, 1], vec![4, 1], vec![]],
        )
        .expect("matrix is valid");

        let graph = SparseGraph::try_new(&checks).expect("graph sizes are representable");

        assert_eq!(graph.edge_bits, [1, 4, 1, 3, 1, 4]);
        assert_eq!(
            graph.check_edges,
            [vec![0, 1], vec![], vec![2, 3], vec![4, 5], vec![]]
        );
        assert_eq!(
            graph.bit_edges,
            [vec![], vec![0, 2, 4], vec![], vec![3], vec![1, 5], vec![]]
        );
        assert_eq!(graph.check_edges.iter().map(Vec::len).max(), Some(2));
    }

    #[test]
    fn graph_and_state_allow_no_edges_and_isolated_bits() {
        let checks = ParityCheckMatrix::try_from_rows(4, vec![vec![], vec![], vec![]])
            .expect("empty checks are valid");
        let mut state = SpaState::try_new(&checks).expect("empty graph is representable");

        assert_eq!(state.graph.edge_bits, []);
        assert_eq!(state.graph.check_edges, [vec![], vec![], vec![]]);
        assert_eq!(state.graph.bit_edges, [vec![], vec![], vec![], vec![]]);
        assert_eq!(state.tanh, [0.0]);
        assert_eq!(state.prefix, [1.0]);
        assert_eq!(state.suffix, [1.0]);

        state
            .prepare_block(
                DecodeInput {
                    llrs: &[1.0, -2.0, 0.0, 3.0],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("isolated bits can be prepared");

        assert_eq!(state.channel, [1.0, -2.0, 0.0, 3.0]);
        assert_eq!(state.posterior, [1.0, -2.0, 0.0, 3.0]);
        assert!(state.q.is_empty());
        assert!(state.r.is_empty());
        assert_eq!(state.word, [Bit::Zero, Bit::One, Bit::Zero, Bit::Zero]);
    }

    #[test]
    fn initial_hard_decision_treats_both_signed_zeros_as_zero() {
        let checks = ParityCheckMatrix::try_from_rows(4, vec![vec![]]).expect("valid matrix");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");

        state
            .prepare_block(
                DecodeInput {
                    llrs: &[-0.0, 0.0, -0.25, 0.25],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("finite LLRs can be prepared");

        state.step(DecoderConfig::default());

        assert_eq!(state.channel[0].to_bits(), (-0.0_f64).to_bits());
        assert_eq!(state.channel[1].to_bits(), 0.0_f64.to_bits());
        assert_eq!(state.word, [Bit::Zero, Bit::Zero, Bit::One, Bit::Zero]);
    }

    #[test]
    fn preparing_a_new_block_resets_every_workspace_buffer() {
        let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1, 2], vec![1, 2]])
            .expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[1.0, 2.0, 3.0],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("first block is valid");

        state.step(DecoderConfig::default());
        assert_ne!(state.r, [0.0; 5]);
        assert_ne!(state.tanh, [0.0; 4]);
        assert_ne!(state.prefix, [1.0; 4]);
        assert_ne!(state.suffix, [1.0; 4]);

        state
            .prepare_block(
                DecodeInput {
                    llrs: &[-4.0, 0.0, 5.0],
                    erasures: &[1],
                },
                DecoderConfig::default(),
            )
            .expect("second block is valid");

        assert_eq!(state.channel, [-4.0, 0.0, 5.0]);
        assert_eq!(state.posterior, [-4.0, 0.0, 5.0]);
        assert_eq!(state.word, [Bit::One, Bit::Zero, Bit::Zero]);
        assert_eq!(state.q, [-4.0, 0.0, 5.0, 0.0, 5.0]);
        assert_eq!(state.r, [0.0; 5]);
        assert_eq!(state.tanh, [0.0; 4]);
        assert_eq!(state.prefix, [1.0; 4]);
        assert_eq!(state.suffix, [1.0; 4]);
    }

    #[test]
    fn invalid_block_does_not_change_the_prepared_state() {
        let checks =
            ParityCheckMatrix::try_from_rows(2, vec![vec![0, 1]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[1.0, -2.0],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("initial block is valid");
        let previous = state.snapshot();

        let error = state
            .prepare_block(
                DecodeInput {
                    llrs: &[f64::NAN, 9.0],
                    erasures: &[usize::MAX],
                },
                DecoderConfig::default(),
            )
            .expect_err("non-finite input must fail before state replacement");

        assert_eq!(error, LdpcError::NonFiniteLlr { index: 0 });
        assert_eq!(state.snapshot(), previous);

        let error = state
            .prepare_block(
                DecodeInput {
                    llrs: &[3.0, 4.0],
                    erasures: &[1, 1],
                },
                DecoderConfig::default(),
            )
            .expect_err("duplicate erasures must fail before state replacement");

        assert_eq!(error, LdpcError::DuplicateErasureIndex { bit: 1 });
        assert_eq!(state.snapshot(), previous);
    }

    #[test]
    fn size_helpers_check_byte_multiplication_and_degree_increment() {
        assert_eq!(
            checked_vec_bytes::<f64>(3),
            Ok(3 * core::mem::size_of::<f64>())
        );
        assert_eq!(
            checked_vec_bytes::<usize>(usize::MAX),
            Err(LdpcError::SizeOverflow)
        );
        assert_eq!(scratch_len_for_degree(0), Ok(1));
        assert_eq!(scratch_len_for_degree(4), Ok(5));
        assert_eq!(
            scratch_len_for_degree(usize::MAX),
            Err(LdpcError::SizeOverflow)
        );
    }

    #[test]
    fn check_messages_handle_degrees_zero_one_and_two_exactly() {
        let checks = ParityCheckMatrix::try_from_rows(2, vec![vec![], vec![0], vec![0, 1]])
            .expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        let config = DecoderConfig::try_new(0, 7.0).expect("limit is valid");
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[3.0, -4.0],
                    erasures: &[],
                },
                config,
            )
            .expect("input is valid");
        let old_q = float_bits(&state.q);

        state.update_check_messages(config);

        assert_eq!(state.r, [7.0, -4.0, 3.0]);
        assert_eq!(float_bits(&state.q), old_q);
    }

    #[test]
    fn degree_three_messages_match_the_hand_calculation() {
        let checks =
            ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1, 2]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        let llr = 3.0_f64.ln();
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[llr, llr, llr],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("input is valid");
        let old_q = float_bits(&state.q);

        state.update_check_messages(DecoderConfig::default());

        for &message in &state.r {
            assert_close((5.0_f64 / 3.0).ln(), message);
        }
        assert_eq!(float_bits(&state.q), old_q);
    }

    #[test]
    fn check_messages_preserve_sign_and_handle_one_zero_multiplier() {
        let checks =
            ParityCheckMatrix::try_from_rows(4, vec![vec![0, 1, 2]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        let llr = 3.0_f64.ln();
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[llr, -llr, llr, 0.0],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("input is valid");

        state.update_check_messages(DecoderConfig::default());

        assert_close((3.0_f64 / 5.0).ln(), state.r[0]);
        assert_close((5.0_f64 / 3.0).ln(), state.r[1]);
        assert_close((3.0_f64 / 5.0).ln(), state.r[2]);

        let checks =
            ParityCheckMatrix::try_from_rows(4, vec![vec![0, 1, 2, 3]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[llr, -llr, 0.0, llr],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("input is valid");

        state.update_check_messages(DecoderConfig::default());

        assert_eq!(state.r[0], 0.0);
        assert_eq!(state.r[1], 0.0);
        assert_close((7.0_f64 / 9.0).ln(), state.r[2]);
        assert_eq!(state.r[3], 0.0);
    }

    #[test]
    fn two_zero_multipliers_make_every_excluded_product_zero() {
        let checks =
            ParityCheckMatrix::try_from_rows(4, vec![vec![0, 1, 2, 3]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[3.0_f64.ln(), 0.0, 0.0, -3.0_f64.ln()],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("input is valid");

        state.update_check_messages(DecoderConfig::default());

        assert_eq!(state.r, [0.0; 4]);
    }

    #[test]
    fn product_conversion_clamps_unit_boundaries_and_keeps_zero_exact() {
        let config = DecoderConfig::try_new(0, 6.5).expect("limit is valid");

        assert_eq!(llr_from_product(1.0, config), 6.5);
        assert_eq!(llr_from_product(-1.0, config), -6.5);
        assert_eq!(llr_from_product(0.0, config).to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn extreme_channel_values_produce_finite_bounded_messages() {
        let checks =
            ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1, 2]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        let config = DecoderConfig::default();
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[f64::MAX, -f64::MAX, f64::MAX],
                    erasures: &[],
                },
                config,
            )
            .expect("finite extreme inputs are saturated");
        let old_q = float_bits(&state.q);

        state.update_check_messages(config);

        assert_eq!(state.q, [20.0, -20.0, 20.0]);
        assert_eq!(float_bits(&state.q), old_q);
        assert_finite_in_range(&state.r, -20.0, 20.0);
        assert_finite_in_range(&state.tanh, -1.0, 1.0);
        assert_finite_in_range(&state.prefix, -1.0, 1.0);
        assert_finite_in_range(&state.suffix, -1.0, 1.0);
    }

    #[test]
    fn degree_4096_uses_finite_reusable_scratch_without_changing_q() {
        let degree = 4096;
        let bits: Vec<_> = (0..degree).collect();
        let checks = ParityCheckMatrix::try_from_rows(degree, vec![bits]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        let llrs = vec![f64::MAX; degree];
        let config = DecoderConfig::default();
        state
            .prepare_block(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &[],
                },
                config,
            )
            .expect("finite extreme inputs are saturated");
        let old_q = float_bits(&state.q);

        state.update_check_messages(config);

        assert_eq!(state.r.len(), degree);
        assert_eq!(state.tanh.len(), degree + 1);
        assert_eq!(state.prefix.len(), degree + 1);
        assert_eq!(state.suffix.len(), degree + 1);
        assert_eq!(float_bits(&state.q), old_q);
        assert_finite_in_range(&state.r, -20.0, 20.0);
        assert_finite_in_range(&state.tanh, -1.0, 1.0);
        assert_finite_in_range(&state.prefix, -1.0, 1.0);
        assert_finite_in_range(&state.suffix, -1.0, 1.0);
    }

    fn assert_close(expected: f64, actual: f64) {
        assert!(actual.is_finite(), "actual value is not finite: {actual}");
        let tolerance = 1e-10 + 1e-10 * expected.abs();
        assert!(
            (expected - actual).abs() <= tolerance,
            "expected {expected}, got {actual}, tolerance {tolerance}"
        );
    }

    fn assert_finite_in_range(values: &[f64], min: f64, max: f64) {
        for (index, &value) in values.iter().enumerate() {
            assert!(value.is_finite(), "value at index {index} is not finite");
            assert!(
                (min..=max).contains(&value),
                "value at index {index} is outside [{min}, {max}]: {value}"
            );
        }
    }
}
