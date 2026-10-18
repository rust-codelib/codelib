//! Восстановление данных из `k` узлов (раздел VII, алгоритм 2) и поиск
//! испорченных узлов при избыточном числе переданных узлов.

use gf_linalg::Matrix;

use crate::code::NodeShare;
use crate::field::signed;
use crate::observer::{emit, CascadeEvent, CascadeObserver};
use crate::structure::{Origin, Term};
use crate::subsets::Combinations;
use crate::{CascadeCode, CascadeError, CodeField};

/// Настройки восстановления данных.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecoderConfig {
    /// Сколько подмножеств из `k` узлов можно перебрать, если первая попытка
    /// не согласуется со всеми переданными узлами. `1` отключает перебор.
    pub max_subset_trials: usize,
}

impl Default for DecoderConfig {
    /// 256 попыток.
    fn default() -> Self {
        Self {
            max_subset_trials: 256,
        }
    }
}

/// Итог восстановления данных.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeStatus {
    /// Все переданные узлы совпали с перекодированным результатом.
    Consistent,
    /// Часть узлов не совпала, но результат согласован с таким числом узлов
    /// `a`, что `a >= k + e`, где `e` — число несовпавших. Другой файл не может
    /// совпасть с таким числом узлов, поэтому результат однозначен.
    Corrected,
    /// Однозначного согласованного результата не найдено: возвращён лучший
    /// из проверенных вариантов.
    Inconsistent,
}

/// Результат восстановления данных.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeResult<F> {
    pub(crate) message: Vec<F>,
    pub(crate) status: DecodeStatus,
    pub(crate) used_nodes: Vec<usize>,
    pub(crate) suspect_nodes: Vec<usize>,
    pub(crate) mismatched_symbols: usize,
    pub(crate) subsets_tried: usize,
}

impl<F> DecodeResult<F> {
    /// Восстановленный блок из `F` символов.
    #[must_use]
    pub fn message(&self) -> &[F] {
        &self.message
    }

    /// Забирает восстановленный блок.
    #[must_use]
    pub fn into_message(self) -> Vec<F> {
        self.message
    }

    /// Статус согласованности.
    #[must_use]
    pub fn status(&self) -> DecodeStatus {
        self.status
    }

    /// Узлы, по которым получен результат.
    #[must_use]
    pub fn used_nodes(&self) -> &[usize] {
        &self.used_nodes
    }

    /// Узлы, чьё содержимое не совпало с результатом (предполагаемые места
    /// ошибок), по возрастанию номера.
    #[must_use]
    pub fn suspect_nodes(&self) -> &[usize] {
        &self.suspect_nodes
    }

    /// Число несовпавших символов во всех подозрительных узлах.
    #[must_use]
    pub fn mismatched_symbols(&self) -> usize {
        self.mismatched_symbols
    }

    /// Сколько подмножеств из `k` узлов было проверено.
    #[must_use]
    pub fn subsets_tried(&self) -> usize {
        self.subsets_tried
    }
}

/// Декодер данных: восстанавливает блок по содержимому узлов.
pub trait DataDecoder<F> {
    /// Восстанавливает блок по содержимому не менее `k` различных узлов.
    ///
    /// # Ошибки
    ///
    /// Ошибки проверки входа: неверный номер узла, повтор, длина, мало узлов.
    fn decode(
        &self,
        shares: &[NodeShare<F>],
        observer: Option<&mut dyn CascadeObserver>,
    ) -> Result<DecodeResult<F>, CascadeError>;
}

impl<F: CodeField> DataDecoder<F> for CascadeCode<F> {
    fn decode(
        &self,
        shares: &[NodeShare<F>],
        observer: Option<&mut dyn CascadeObserver>,
    ) -> Result<DecodeResult<F>, CascadeError> {
        self.decode_with(shares, self.decoder_config, observer)
    }
}

/// Кандидат: сообщение, использованные узлы и число согласных узлов.
struct Candidate<F> {
    message: Vec<F>,
    used: Vec<usize>,
    mismatches: Vec<(usize, usize)>,
}

impl<F: CodeField> CascadeCode<F> {
    /// Восстанавливает блок с явными настройками перебора.
    ///
    /// Первая попытка использует первые `k` узлов в порядке передачи. Результат
    /// перекодируется и сравнивается со всеми переданными узлами. Если есть
    /// расхождения, перебираются подмножества из `k` узлов (не больше
    /// `config.max_subset_trials`, включая первую попытку) до первого
    /// однозначного результата.
    ///
    /// # Ошибки
    ///
    /// См. [`DataDecoder::decode`].
    pub fn decode_with(
        &self,
        shares: &[NodeShare<F>],
        config: DecoderConfig,
        mut observer: Option<&mut dyn CascadeObserver>,
    ) -> Result<DecodeResult<F>, CascadeError> {
        let k = self.params.k();
        self.check_shares(shares, self.params.alpha())?;
        if shares.len() < k {
            return Err(CascadeError::NotEnoughShares {
                required: k,
                actual: shares.len(),
            });
        }
        emit(
            &mut observer,
            CascadeEvent::DecodeStarted {
                shares: shares.len(),
                required: k,
            },
        );

        let s = shares.len();
        let limit = config.max_subset_trials.max(1);
        let mut best: Option<Candidate<F>> = None;
        let mut tried = 0;
        for subset in Combinations::new(s, k).take(limit) {
            tried += 1;
            let chosen: Vec<&NodeShare<F>> = subset.iter().map(|&i| &shares[i]).collect();
            let message = self.recover(&chosen, &mut observer)?;
            let rows = self.message_rows(&message)?;
            let mismatches: Vec<(usize, usize)> = shares
                .iter()
                .filter_map(|share| {
                    let expected = self.node_symbols(&rows, share.node);
                    let wrong = expected
                        .iter()
                        .zip(&share.symbols)
                        .filter(|(a, b)| a != b)
                        .count();
                    (wrong > 0).then_some((share.node, wrong))
                })
                .collect();
            let used: Vec<usize> = chosen.iter().map(|share| share.node).collect();
            emit(
                &mut observer,
                CascadeEvent::SubsetTried {
                    nodes: &used,
                    agreeing: s - mismatches.len(),
                },
            );
            let unique = is_unique(s, mismatches.len(), k);
            let better = best
                .as_ref()
                .map_or(true, |b| mismatches.len() < b.mismatches.len());
            if better {
                best = Some(Candidate {
                    message,
                    used,
                    mismatches,
                });
            }
            if unique {
                break;
            }
        }

        let best = best.ok_or(CascadeError::Internal("ни одной попытки декодирования"))?;
        let errors = best.mismatches.len();
        let status = if errors == 0 {
            DecodeStatus::Consistent
        } else if is_unique(s, errors, k) {
            DecodeStatus::Corrected
        } else {
            DecodeStatus::Inconsistent
        };
        let mut suspect_nodes: Vec<usize> = best.mismatches.iter().map(|&(node, _)| node).collect();
        suspect_nodes.sort_unstable();
        emit(
            &mut observer,
            CascadeEvent::DecodeFinished {
                status,
                suspect_nodes: &suspect_nodes,
            },
        );
        Ok(DecodeResult {
            message: best.message,
            status,
            used_nodes: best.used,
            suspect_nodes,
            mismatched_symbols: best.mismatches.iter().map(|&(_, wrong)| wrong).sum(),
            subsets_tried: tried,
        })
    }

    /// Алгоритм 2: восстановление супер-матрицы по ровно `k` узлам.
    ///
    /// Сегменты обходятся по возрастанию режима, столбцы — в обратном
    /// лексикографическом порядке. Для столбца сначала восстанавливаются
    /// нижние `d - k` строк (символы `G1` из детей, `G2` — нули, `G3` — из
    /// уравнения (5) и инъекции соседа), затем верхние строки по (53).
    /// Инъецированный символ извлекается из верхнего проверочного символа
    /// вычитанием его исходной части.
    pub(crate) fn recover(
        &self,
        shares: &[&NodeShare<F>],
        observer: &mut Option<&mut dyn CascadeObserver>,
    ) -> Result<Vec<F>, CascadeError> {
        let s = &self.structure;
        let (k, d) = (s.k, s.d);
        let nodes: Vec<usize> = shares.iter().map(|share| share.node).collect();

        // Γ[K, :]^{-1}, затем X = Γ^{-1} · Ψ[K, :] · M и L = Γ^{-1} · Υ[K, :].
        let gamma_inv = rows_of(&self.encoder_block(&nodes, 0..k)?.try_inverse()?);
        let x_rows: Vec<Vec<F>> = (0..k)
            .map(|r| {
                (0..s.alpha)
                    .map(|c| {
                        (0..k).fold(F::zero(), |acc, j| {
                            acc + gamma_inv[r][j] * shares[j].symbols[c]
                        })
                    })
                    .collect()
            })
            .collect();
        let upsilon = if d > k {
            rows_of(&self.encoder_block(&nodes, k..d)?)
        } else {
            vec![Vec::new(); k]
        };
        let l_rows: Vec<Vec<F>> = (0..k)
            .map(|r| {
                (0..d - k)
                    .map(|c| (0..k).fold(F::zero(), |acc, j| acc + gamma_inv[r][j] * upsilon[j][c]))
                    .collect()
            })
            .collect();

        let mut table: Vec<Option<F>> = vec![None; s.file_size];
        let mut order: Vec<usize> = (0..s.segments.len()).collect();
        order.sort_by_key(|&i| s.segments[i].mode);

        let mut bottom = vec![F::zero(); d - k];
        for index in order {
            let seg = &s.segments[index];
            for c in (0..seg.cols).rev() {
                for x in k..d {
                    let entry = seg.entry(x, c);
                    let mut value = match &entry.origin {
                        Origin::Zero => F::zero(),
                        Origin::Raw(term) => lookup(&table, *term)?,
                        Origin::Parity(terms) => lookup_form(&table, terms)?,
                    };
                    if let Some(term) = entry.injection {
                        value = value + lookup(&table, term)?;
                    }
                    bottom[x - k] = value;
                }
                for (i, (x_row, l_row)) in x_rows.iter().zip(&l_rows).enumerate() {
                    let top = l_row
                        .iter()
                        .zip(&bottom)
                        .fold(x_row[seg.offset + c], |acc, (&l, &b)| acc - l * b);
                    let entry = seg.entry(i, c);
                    let original = match &entry.origin {
                        Origin::Raw(term) => {
                            table[term.id] = Some(signed(term.neg, top));
                            continue;
                        }
                        Origin::Zero => F::zero(),
                        Origin::Parity(terms) => lookup_form(&table, terms)?,
                    };
                    // Первичная инъекция: извлекаем символ родителя. Без инъекции
                    // верхний проверочный символ избыточен; его расхождение
                    // выявит перекодирование.
                    if let Some(term) = entry.injection {
                        table[term.id] = Some(signed(term.neg, top - original));
                    }
                }
            }
            emit(
                observer,
                CascadeEvent::SegmentRecovered {
                    segment: index,
                    mode: seg.mode,
                },
            );
        }

        table
            .into_iter()
            .collect::<Option<Vec<F>>>()
            .ok_or(CascadeError::Internal("не все символы восстановлены"))
    }
}

/// `true`, если `s` узлов при `e` несовпадениях однозначно задают файл.
fn is_unique(s: usize, e: usize, k: usize) -> bool {
    s >= k + 2 * e
}

pub(crate) fn rows_of<F: CodeField>(matrix: &Matrix<F>) -> Vec<Vec<F>> {
    (0..matrix.rows())
        .map(|r| matrix.row(r).map(<[F]>::to_vec).unwrap_or_default())
        .collect()
}

fn lookup<F: CodeField>(table: &[Option<F>], term: Term) -> Result<F, CascadeError> {
    table[term.id]
        .map(|value| signed(term.neg, value))
        .ok_or(CascadeError::Internal(
            "символ нужен раньше, чем восстановлен",
        ))
}

fn lookup_form<F: CodeField>(table: &[Option<F>], terms: &[Term]) -> Result<F, CascadeError> {
    terms
        .iter()
        .try_fold(F::zero(), |acc, &term| Ok(acc + lookup(table, term)?))
}
