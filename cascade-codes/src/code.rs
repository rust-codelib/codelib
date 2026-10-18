//! Каскадный код: матрица кодера, супер-матрица сообщения и кодирование.

use gf_linalg::Matrix;

use crate::field::signed;
use crate::structure::{Origin, Structure, Term};
#[cfg(doc)]
use crate::DataDecoder;
use crate::{CascadeError, CascadeParams, CodeField, DecoderConfig};

/// Содержимое одного узла хранения: номер узла и его `alpha` символов.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeShare<F> {
    /// Номер узла в диапазоне `0..n`.
    pub node: usize,
    /// Закодированные символы узла.
    pub symbols: Vec<F>,
}

/// Каскадный регенерирующий код `(n, k, d; mu)` над полем `F`.
///
/// Код хранит параметры, матрицу кодера Вандермонда `Ψ` размера `n × d` и
/// символический шаблон супер-матрицы сообщения `M` размера `d × alpha`.
/// Узел `i` хранит строку `Ψ[i, :] · M`.
///
/// ```rust
/// use cascade_codes::{CascadeCode, CascadeParams, DataDecoder, Encoder};
/// use gfpm::Gf13;
///
/// let params = CascadeParams::try_new(6, 3, 4, 2)?;
/// let code = CascadeCode::<Gf13>::try_new(params)?;
/// let message: Vec<Gf13> = (0..params.file_size() as u64).map(|i| Gf13::new(i % 13)).collect();
/// let nodes = code.encode(&message)?;
/// // Любые k = 3 узла восстанавливают сообщение.
/// let result = code.decode(&[nodes[5].clone(), nodes[0].clone(), nodes[2].clone()], None)?;
/// assert_eq!(result.message(), &message[..]);
/// # Ok::<(), cascade_codes::CascadeError>(())
/// ```
#[derive(Debug, Clone)]
pub struct CascadeCode<F: CodeField> {
    pub(crate) params: CascadeParams,
    pub(crate) structure: Structure,
    /// Строки матрицы кодера `Ψ`.
    pub(crate) encoder: Vec<Vec<F>>,
    pub(crate) decoder_config: DecoderConfig,
}

impl<F: CodeField> CascadeCode<F> {
    /// Строит код с матрицей кодера Вандермонда `Ψ[i][j] = a_i^j`.
    ///
    /// Точка `a_i` — элемент поля с номером `i` ([`CodeField::from_index`]).
    /// Матрица Вандермонда с различными точками удовлетворяет условиям (E1)
    /// и (E2) определения 4: любые `k` строк первых `k` столбцов и любые `d`
    /// строк всей матрицы линейно независимы.
    ///
    /// # Ошибки
    ///
    /// [`CascadeError::FieldTooSmall`], если `q < n`, а также ошибки
    /// пределов размера при построении структуры кода.
    pub fn try_new(params: CascadeParams) -> Result<Self, CascadeError> {
        let order = F::order();
        if order < params.n() as u128 {
            return Err(CascadeError::FieldTooSmall {
                n: params.n(),
                order,
            });
        }
        let structure = Structure::build(&params)?;
        let encoder = (0..params.n())
            .map(|i| {
                let point = F::from_index(i as u64)
                    .ok_or(CascadeError::Internal("номер точки Вандермонда вне поля"))?;
                let mut row = Vec::with_capacity(params.d());
                let mut power = F::one();
                for _ in 0..params.d() {
                    row.push(power);
                    power = power * point;
                }
                Ok(row)
            })
            .collect::<Result<Vec<_>, CascadeError>>()?;
        Ok(Self {
            params,
            structure,
            encoder,
            decoder_config: DecoderConfig::default(),
        })
    }

    /// Заменяет настройки, которые использует [`DataDecoder::decode`].
    #[must_use]
    pub fn with_decoder_config(mut self, config: DecoderConfig) -> Self {
        self.decoder_config = config;
        self
    }

    /// Настройки восстановления данных.
    #[must_use]
    pub fn decoder_config(&self) -> DecoderConfig {
        self.decoder_config
    }

    /// Параметры кода.
    #[must_use]
    pub fn params(&self) -> &CascadeParams {
        &self.params
    }

    /// Матрица кодера `Ψ` размера `n × d`.
    ///
    /// # Ошибки
    ///
    /// Ошибка линейной алгебры при превышении её пределов размера.
    pub fn encoder_matrix(&self) -> Result<Matrix<F>, CascadeError> {
        let data = self.encoder.iter().flatten().copied().collect();
        Ok(Matrix::try_new(self.params.n(), self.params.d(), data)?)
    }

    /// Число сегментов (знаковых детерминантных кодов) в супер-матрице.
    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.structure.segments.len()
    }

    /// Режимы сегментов в порядке столбцов супер-матрицы.
    #[must_use]
    pub fn segment_modes(&self) -> Vec<usize> {
        self.structure.segments.iter().map(|s| s.mode).collect()
    }

    /// Вычисляет супер-матрицу сообщения `M` (`d × alpha`) для блока данных.
    ///
    /// # Ошибки
    ///
    /// [`CascadeError::MessageLengthMismatch`], если длина блока не равна `F`.
    pub fn message_matrix(&self, message: &[F]) -> Result<Matrix<F>, CascadeError> {
        let rows = self.message_rows(message)?;
        let data = rows.into_iter().flatten().collect();
        Ok(Matrix::try_new(self.params.d(), self.params.alpha(), data)?)
    }

    pub(crate) fn message_rows(&self, message: &[F]) -> Result<Vec<Vec<F>>, CascadeError> {
        if message.len() != self.params.file_size() {
            return Err(CascadeError::MessageLengthMismatch {
                expected: self.params.file_size(),
                actual: message.len(),
            });
        }
        let s = &self.structure;
        let mut rows = vec![vec![F::zero(); s.alpha]; s.d];
        for seg in &s.segments {
            for (x, row) in rows.iter_mut().enumerate() {
                for c in 0..seg.cols {
                    let entry = seg.entry(x, c);
                    let mut value = match &entry.origin {
                        Origin::Zero => F::zero(),
                        Origin::Raw(term) => term_value(*term, message),
                        Origin::Parity(terms) => form_value(terms, message),
                    };
                    if let Some(term) = entry.injection {
                        value = value + term_value(term, message);
                    }
                    row[seg.offset + c] = value;
                }
            }
        }
        Ok(rows)
    }

    /// Содержимое узла `node` для готовой супер-матрицы.
    pub(crate) fn node_symbols(&self, rows: &[Vec<F>], node: usize) -> Vec<F> {
        let psi = &self.encoder[node];
        (0..self.params.alpha())
            .map(|c| {
                psi.iter()
                    .zip(rows)
                    .fold(F::zero(), |acc, (&p, row)| acc + p * row[c])
            })
            .collect()
    }

    /// Проверяет номера и длины узлов: номера в `0..n` без повторов.
    pub(crate) fn check_shares(
        &self,
        shares: &[NodeShare<F>],
        expected_len: usize,
    ) -> Result<(), CascadeError> {
        let n = self.params.n();
        let mut seen = vec![false; n];
        for share in shares {
            if share.node >= n {
                return Err(CascadeError::NodeOutOfRange {
                    node: share.node,
                    n,
                });
            }
            if std::mem::replace(&mut seen[share.node], true) {
                return Err(CascadeError::DuplicateNode { node: share.node });
            }
            if share.symbols.len() != expected_len {
                return Err(CascadeError::ShareLengthMismatch {
                    node: share.node,
                    expected: expected_len,
                    actual: share.symbols.len(),
                });
            }
        }
        Ok(())
    }

    /// Подматрица `Ψ[nodes, cols]` для набора узлов и диапазона столбцов.
    pub(crate) fn encoder_block(
        &self,
        nodes: &[usize],
        cols: core::ops::Range<usize>,
    ) -> Result<Matrix<F>, CascadeError> {
        let width = cols.len();
        let data = nodes
            .iter()
            .flat_map(|&node| self.encoder[node][cols.clone()].iter().copied())
            .collect();
        Ok(Matrix::try_new(nodes.len(), width, data)?)
    }
}

/// Кодер: блок из `F` символов в содержимое всех `n` узлов.
pub trait Encoder<F> {
    /// Число информационных символов в блоке.
    fn message_len(&self) -> usize;

    /// Число символов, хранимых одним узлом.
    fn node_len(&self) -> usize;

    /// Кодирует блок и возвращает содержимое всех узлов по порядку номеров.
    ///
    /// # Ошибки
    ///
    /// [`CascadeError::MessageLengthMismatch`] при неверной длине блока.
    fn encode(&self, message: &[F]) -> Result<Vec<NodeShare<F>>, CascadeError>;

    /// Кодирует блок и возвращает содержимое одного узла.
    ///
    /// # Ошибки
    ///
    /// Ошибка длины блока или [`CascadeError::NodeOutOfRange`].
    fn encode_node(&self, message: &[F], node: usize) -> Result<NodeShare<F>, CascadeError>;
}

impl<F: CodeField> Encoder<F> for CascadeCode<F> {
    fn message_len(&self) -> usize {
        self.params.file_size()
    }

    fn node_len(&self) -> usize {
        self.params.alpha()
    }

    fn encode(&self, message: &[F]) -> Result<Vec<NodeShare<F>>, CascadeError> {
        let rows = self.message_rows(message)?;
        Ok((0..self.params.n())
            .map(|node| NodeShare {
                node,
                symbols: self.node_symbols(&rows, node),
            })
            .collect())
    }

    fn encode_node(&self, message: &[F], node: usize) -> Result<NodeShare<F>, CascadeError> {
        if node >= self.params.n() {
            return Err(CascadeError::NodeOutOfRange {
                node,
                n: self.params.n(),
            });
        }
        let rows = self.message_rows(message)?;
        Ok(NodeShare {
            node,
            symbols: self.node_symbols(&rows, node),
        })
    }
}

pub(crate) fn term_value<F: CodeField>(term: Term, symbols: &[F]) -> F {
    signed(term.neg, symbols[term.id])
}

pub(crate) fn form_value<F: CodeField>(terms: &[Term], symbols: &[F]) -> F {
    terms
        .iter()
        .fold(F::zero(), |acc, &t| acc + term_value(t, symbols))
}
