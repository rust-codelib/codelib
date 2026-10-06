use super::graph::{checked_workspace_sizes, SparseGraph};
use super::SpaStepResult;
use crate::bit::bit_parity;
use crate::decode_input::prepare_channel;
use crate::{Bit, DecodeInput, DecoderConfig, LdpcError, ParityCheckMatrix};

#[derive(Debug)]
pub(super) struct SpaState {
    pub(super) graph: SparseGraph,
    /// Канальные LLR каждого бита.
    pub(super) channel: Vec<f64>,
    /// Переиспользуемая маска для проверки повторов позиций стирания.
    ///
    /// Это служебный буфер: проверка неверного списка стираний может оставить
    /// его очищенным или частично заполненным, не меняя содержательные буферы.
    pub(super) erased_positions: Vec<bool>,
    /// Сообщения `q`: бит → проверка, в порядке рёбер графа.
    pub(super) q: Vec<f64>,
    /// Сообщения `r`: проверка → бит, в порядке рёбер графа.
    pub(super) r: Vec<f64>,
    /// Апостериорный LLR каждого бита после обновления.
    pub(super) posterior: Vec<f64>,
    /// Жёсткие решения по текущим апостериорным LLR.
    pub(super) word: Vec<Bit>,
    /// Переиспользуемые значения `tanh(q / 2)` одной проверки.
    pub(super) tanh: Vec<f64>,
    /// Переиспользуемые префиксные произведения одной проверки.
    pub(super) prefix: Vec<f64>,
    /// Переиспользуемые суффиксные произведения одной проверки.
    pub(super) suffix: Vec<f64>,
}

impl SpaState {
    pub(super) fn try_new(checks: &ParityCheckMatrix) -> Result<Self, LdpcError> {
        let sizes = checked_workspace_sizes(checks)?;
        let graph = SparseGraph::from_checked_sizes(checks, sizes);
        let scratch_len = sizes.scratch_len;

        Ok(Self {
            graph,
            channel: vec![0.0; sizes.bits],
            erased_positions: vec![false; sizes.bits],
            q: vec![0.0; sizes.edges],
            r: vec![0.0; sizes.edges],
            posterior: vec![0.0; sizes.bits],
            word: vec![Bit::Zero; sizes.bits],
            tanh: vec![0.0; scratch_len],
            prefix: vec![1.0; scratch_len],
            suffix: vec![1.0; scratch_len],
        })
    }

    pub(super) fn prepare_block(
        &mut self,
        input: DecodeInput<'_>,
        config: DecoderConfig,
    ) -> Result<(), LdpcError> {
        // Проверки содержимого завершатся до записи в содержательные буферы.
        // При повторе стирания может измениться только служебная маска.
        prepare_channel(input, &mut self.channel, &mut self.erased_positions, config)?;

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

    pub(super) fn update_check_messages(&mut self, config: DecoderConfig) {
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

    pub(super) fn update_bit_messages(&mut self, config: DecoderConfig) {
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

    pub(super) fn step(&mut self, config: DecoderConfig) {
        self.update_check_messages(config);
        self.update_bit_messages(config);
    }

    pub(super) fn fill_syndrome(&self, syndrome: &mut [Bit]) {
        debug_assert_eq!(syndrome.len(), self.graph.check_edges.len());
        for (check, edges) in self.graph.check_edges.iter().enumerate() {
            syndrome[check] = bit_parity(
                edges
                    .iter()
                    .map(|&edge| self.word[self.graph.edge_bits[edge]]),
            );
        }
    }

    pub(super) fn into_result(self) -> SpaStepResult {
        SpaStepResult {
            word: self.word,
            posterior_llrs: self.posterior,
        }
    }

    #[cfg(test)]
    pub(super) fn snapshot(&self) -> StateSnapshot {
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

fn hard_decision(llr: f64) -> Bit {
    if llr >= 0.0 {
        Bit::Zero
    } else {
        Bit::One
    }
}

pub(super) fn llr_from_product(product: f64, config: DecoderConfig) -> f64 {
    if product == 0.0 {
        return 0.0;
    }

    let product_limit = 1.0 - f64::EPSILON;
    let product = product.clamp(-product_limit, product_limit);
    (product.ln_1p() - (-product).ln_1p()).clamp(-config.llr_limit(), config.llr_limit())
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
/// Snapshot of semantic state; the validation-only erasure mask is excluded.
pub(super) struct StateSnapshot {
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
pub(super) fn float_bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}
