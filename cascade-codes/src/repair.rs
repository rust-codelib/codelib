//! Точное восстановление отказавшего узла (раздел VI, предложение 4).
//!
//! Помощник `h` для каждого сегмента `Q` режима `m` вычисляет
//! `Ψ[h, :] · Q · Ξ^{f,(m)}` по матрице (7). Ранг `Ξ` равен `C(d-1, m-1)`,
//! поэтому передаются только координаты в опорных столбцах приведённой
//! ступенчатой формы `Ξ` — ровно `beta` символов на помощника. Новый узел
//! восстанавливает полный вектор, решает систему с `Ψ[H, :]` и получает
//! пространства восстановления `R_f(Q)`, из которых по (23) собирает своё
//! содержимое.

use gf_linalg::Matrix;

use crate::code::NodeShare;
use crate::decode::rows_of;
use crate::field::signed;
use crate::observer::{emit, CascadeEvent, CascadeObserver};
use crate::structure::Structure;
use crate::subsets::{ind, union, without};
use crate::{CascadeCode, CascadeError, CodeField};

/// Данные, которые помощник отправляет для восстановления узла.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairPayload<F> {
    /// Номер помощника.
    pub helper: usize,
    /// `beta` символов.
    pub symbols: Vec<F>,
}

/// Контракт точного восстановления узла.
pub trait NodeRepairer<F> {
    /// Число символов, которое каждый помощник передаёт (`beta`).
    fn repair_payload_len(&self) -> usize;

    /// Вычисляет пакет помощника для восстановления узла `failed`.
    ///
    /// Пакет зависит только от содержимого помощника и номера отказавшего
    /// узла, но не от состава остальных помощников.
    ///
    /// # Ошибки
    ///
    /// Неверные номера узлов, совпадение помощника с отказавшим узлом,
    /// неверная длина содержимого.
    fn repair_payload(
        &self,
        helper: &NodeShare<F>,
        failed: usize,
    ) -> Result<RepairPayload<F>, CascadeError>;

    /// Восстанавливает содержимое узла `failed` по пакетам ровно `d`
    /// различных помощников.
    ///
    /// # Ошибки
    ///
    /// Неверное число помощников, повторы, неверные длины пакетов.
    fn repair(
        &self,
        failed: usize,
        payloads: &[RepairPayload<F>],
        observer: Option<&mut dyn CascadeObserver>,
    ) -> Result<NodeShare<F>, CascadeError>;
}

/// Ненулевой элемент `Ξ`: `(-1)^neg · ψ_{f,y}` в строке `I` и столбце `I \ {y}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct XiTerm {
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) y: usize,
    pub(crate) neg: bool,
}

/// Структура матрицы (7) сегмента режима `m >= 1`: ненулевые элементы
/// `Ξ_{I,J} = (-1)^{σ(y) + ind_I(y)} ψ_{f,y}` при `I \ J = {y}`.
pub(crate) fn xi_pattern(s: &Structure, segment: usize) -> Result<Vec<XiTerm>, CascadeError> {
    let seg = &s.segments[segment];
    let rows_table = s.table(seg.mode);
    let cols_table = s.table(seg.mode - 1);
    let mut terms = Vec::new();
    for (row, set) in rows_table.iter().enumerate() {
        for &y in set {
            let col = cols_table
                .position(&without(set, y))
                .ok_or(CascadeError::Internal("нет столбца Ξ"))?;
            terms.push(XiTerm {
                row,
                col,
                y,
                neg: prop3_neg(&seg.sigma, set, y),
            });
        }
    }
    Ok(terms)
}

/// Знак `(-1)^{σ(i) + ind_I(i)}` слагаемого формулы (9) / (23).
pub(crate) fn prop3_neg(sigma: &[bool], set: &[usize], i: usize) -> bool {
    sigma[i] ^ (ind(set, i) % 2 == 1)
}

/// Сжатое представление строкового пространства `Ξ` одного сегмента.
struct SegmentPlan<F> {
    /// Столбцы `Ξ`, оставленные в пакете помощника.
    xi_pivot_cols: Vec<Vec<F>>,
    /// Ненулевые строки RREF(`Ξ`): `y = payload · basis`.
    basis: Vec<Vec<F>>,
}

impl<F: CodeField> CascadeCode<F> {
    /// Матрица восстановления `Ξ^{f,(m)}` сегмента, формула (7).
    fn repair_encoder(&self, segment: usize, failed: usize) -> Result<Matrix<F>, CascadeError> {
        let s = &self.structure;
        let m = s.segments[segment].mode;
        let (rows, cols) = (s.table(m).len(), s.table(m - 1).len());
        let psi = &self.encoder[failed];
        let mut data = vec![F::zero(); rows * cols];
        for term in xi_pattern(s, segment)? {
            data[term.row * cols + term.col] = signed(term.neg, psi[term.y]);
        }
        Ok(Matrix::try_new(rows, cols, data)?)
    }

    fn repair_plan(&self, failed: usize) -> Result<Vec<Option<SegmentPlan<F>>>, CascadeError> {
        (0..self.structure.segments.len())
            .map(|index| {
                if self.structure.segments[index].mode == 0 {
                    return Ok(None); // Ξ имеет ноль столбцов
                }
                let xi = self.repair_encoder(index, failed)?;
                let rref = xi.rref();
                let reduced = rows_of(rref.matrix());
                let basis = reduced[..rref.rank()].to_vec();
                let xi_pivot_cols = rref
                    .pivot_columns()
                    .iter()
                    .map(|&c| {
                        (0..xi.rows())
                            .map(|r| xi.get(r, c).unwrap_or_else(F::zero))
                            .collect()
                    })
                    .collect();
                Ok(Some(SegmentPlan {
                    xi_pivot_cols,
                    basis,
                }))
            })
            .collect()
    }

    /// Проверяет отказавший узел и пакеты: ровно `d` различных помощников,
    /// не совпадающих с отказавшим узлом, по `len` символов в каждом пакете.
    pub(crate) fn check_payloads(
        &self,
        failed: usize,
        payloads: &[RepairPayload<F>],
        len: usize,
    ) -> Result<(), CascadeError> {
        let d = self.params.d();
        self.check_node(failed)?;
        if payloads.len() != d {
            return Err(CascadeError::WrongHelperCount {
                expected: d,
                actual: payloads.len(),
            });
        }
        let mut seen = vec![false; self.params.n()];
        for payload in payloads {
            self.check_node(payload.helper)?;
            if payload.helper == failed {
                return Err(CascadeError::HelperIsFailedNode { node: failed });
            }
            if std::mem::replace(&mut seen[payload.helper], true) {
                return Err(CascadeError::DuplicateNode {
                    node: payload.helper,
                });
            }
            if payload.symbols.len() != len {
                return Err(CascadeError::ShareLengthMismatch {
                    node: payload.helper,
                    expected: len,
                    actual: payload.symbols.len(),
                });
            }
        }
        Ok(())
    }

    fn check_node(&self, node: usize) -> Result<(), CascadeError> {
        if node >= self.params.n() {
            return Err(CascadeError::NodeOutOfRange {
                node,
                n: self.params.n(),
            });
        }
        Ok(())
    }
}

impl<F: CodeField> NodeRepairer<F> for CascadeCode<F> {
    fn repair_payload_len(&self) -> usize {
        self.params.beta()
    }

    fn repair_payload(
        &self,
        helper: &NodeShare<F>,
        failed: usize,
    ) -> Result<RepairPayload<F>, CascadeError> {
        self.check_node(failed)?;
        self.check_shares(core::slice::from_ref(helper), self.params.alpha())?;
        if helper.node == failed {
            return Err(CascadeError::HelperIsFailedNode { node: failed });
        }
        let plan = self.repair_plan(failed)?;
        let mut symbols = Vec::with_capacity(self.params.beta());
        for (seg, plan) in self.structure.segments.iter().zip(&plan) {
            let Some(plan) = plan else { continue };
            let content = &helper.symbols[seg.offset..seg.offset + seg.cols];
            for column in &plan.xi_pivot_cols {
                symbols.push(
                    content
                        .iter()
                        .zip(column)
                        .fold(F::zero(), |acc, (&u, &xi)| acc + u * xi),
                );
            }
        }
        Ok(RepairPayload {
            helper: helper.node,
            symbols,
        })
    }

    fn repair(
        &self,
        failed: usize,
        payloads: &[RepairPayload<F>],
        mut observer: Option<&mut dyn CascadeObserver>,
    ) -> Result<NodeShare<F>, CascadeError> {
        let s = &self.structure;
        let d = s.d;
        self.check_payloads(failed, payloads, self.params.beta())?;
        let helpers: Vec<usize> = payloads.iter().map(|p| p.helper).collect();
        emit(
            &mut observer,
            CascadeEvent::RepairStarted {
                failed,
                helpers: &helpers,
            },
        );

        // R_f(Q) = Ψ[H, :]^{-1} · (стопка Ψ[h, :] · Q · Ξ), формула (22).
        let psi_h_inv = rows_of(&self.encoder_block(&helpers, 0..d)?.try_inverse()?);
        let plan = self.repair_plan(failed)?;
        let mut spaces: Vec<Vec<Vec<F>>> = Vec::with_capacity(s.segments.len());
        let mut cursor = 0;
        for (index, (seg, plan)) in s.segments.iter().zip(&plan).enumerate() {
            let Some(plan) = plan else {
                spaces.push(vec![Vec::new(); d]);
                continue;
            };
            let rank = plan.basis.len();
            let width = plan.basis.first().map_or(0, Vec::len);
            let stacked: Vec<Vec<F>> = payloads
                .iter()
                .map(|payload| {
                    let coords = &payload.symbols[cursor..cursor + rank];
                    (0..width)
                        .map(|c| {
                            coords
                                .iter()
                                .zip(&plan.basis)
                                .fold(F::zero(), |acc, (&a, row)| acc + a * row[c])
                        })
                        .collect()
                })
                .collect();
            cursor += rank;
            let space = (0..d)
                .map(|r| {
                    (0..width)
                        .map(|c| {
                            (0..d).fold(F::zero(), |acc, j| acc + psi_h_inv[r][j] * stacked[j][c])
                        })
                        .collect()
                })
                .collect();
            spaces.push(space);
            emit(
                &mut observer,
                CascadeEvent::RepairSegment {
                    segment: index,
                    mode: seg.mode,
                    symbols_per_helper: rank,
                },
            );
        }
        if cursor != self.params.beta() {
            return Err(CascadeError::Internal(
                "ранги матриц Ξ не совпали с beta из формулы (3)",
            ));
        }

        // Предложение 4, формула (23).
        let mut symbols = Vec::with_capacity(self.params.alpha());
        for (index, seg) in s.segments.iter().enumerate() {
            let table = s.table(seg.mode);
            for set in table.iter() {
                let mut value = F::zero();
                if seg.mode > 0 {
                    let sub = s.table(seg.mode - 1);
                    for &i in set {
                        let c = sub
                            .position(&without(set, i))
                            .ok_or(CascadeError::Internal("нет столбца R_f(Q)"))?;
                        value = value + signed(prop3_neg(&seg.sigma, set, i), spaces[index][i][c]);
                    }
                }
                if let Some(link) = &seg.parent {
                    if !set.iter().any(|y| link.b.contains(y)) {
                        let parent_mode = s.segments[link.index].mode;
                        let c = s
                            .table(parent_mode - 1)
                            .position(&union(set, &link.b))
                            .ok_or(CascadeError::Internal("нет столбца R_f(P)"))?;
                        value = value - spaces[link.index][link.x][c];
                    }
                }
                symbols.push(value);
            }
        }

        emit(
            &mut observer,
            CascadeEvent::RepairFinished {
                failed,
                downloaded: d * self.params.beta(),
            },
        );
        Ok(NodeShare {
            node: failed,
            symbols,
        })
    }
}
