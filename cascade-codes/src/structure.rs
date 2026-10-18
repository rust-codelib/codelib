//! Дерево сегментов и шаблон супер-матрицы сообщения (раздел V, алгоритм 1).
//!
//! Каждый элемент супер-матрицы `M` хранится как символическое выражение над
//! информационными символами: исходная часть (сырой символ, ноль группы `G2`
//! или проверочный символ (5)) плюс необязательная инъекция символа
//! родительского сегмента (14). Шаблон не зависит от поля и от `n`.

use std::collections::{BTreeSet, VecDeque};

use crate::params::MAX_ALPHA;
use crate::subsets::{above_max, binomial, ind, union, with, SubsetTable};
use crate::{CascadeError, CascadeParams};

/// Знаковая ссылка на информационный символ: `(-1)^neg · s[id]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Term {
    pub(crate) id: usize,
    pub(crate) neg: bool,
}

/// Исходная часть элемента знакового детерминантного кода, формула (6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Origin {
    /// Символ группы `G2`, обнулённый по определению 8.
    Zero,
    /// Сырой символ `v` или непроверочный символ `w`.
    Raw(Term),
    /// Проверочный символ `w_{max Y, Y}`, выраженный через сырые символы
    /// своей w-группы по уравнению (5).
    Parity(Vec<Term>),
}

/// Элемент сегмента после инъекции: `origin + injection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) origin: Origin,
    pub(crate) injection: Option<Term>,
}

/// Связь дочернего сегмента с родителем: пара инъекции `(x, B)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParentLink {
    pub(crate) index: usize,
    pub(crate) x: usize,
    pub(crate) b: Vec<usize>,
}

/// Один сегмент кода — сообщение знакового детерминантного кода.
#[derive(Debug, Clone)]
pub(crate) struct Segment {
    pub(crate) mode: usize,
    /// Чётность вектора подписи `σ(x)` для каждой строки.
    pub(crate) sigma: Vec<bool>,
    pub(crate) parent: Option<ParentLink>,
    /// Первый столбец сегмента в супер-матрице.
    pub(crate) offset: usize,
    /// Число столбцов `C(d, mode)`.
    pub(crate) cols: usize,
    /// Элементы по строкам: `entries[x * cols + c]`.
    pub(crate) entries: Vec<Entry>,
}

impl Segment {
    pub(crate) fn entry(&self, row: usize, col: usize) -> &Entry {
        &self.entries[row * self.cols + col]
    }
}

/// Полная структура каскадного кода для фиксированных `(k, d; mu)`.
#[derive(Debug, Clone)]
pub(crate) struct Structure {
    pub(crate) k: usize,
    pub(crate) d: usize,
    /// `tables[m]` — подмножества размера `m`, если они нужны коду.
    tables: Vec<Option<SubsetTable>>,
    pub(crate) segments: Vec<Segment>,
    pub(crate) alpha: usize,
    pub(crate) file_size: usize,
}

impl Structure {
    pub(crate) fn table(&self, m: usize) -> &SubsetTable {
        self.tables[m]
            .as_ref()
            .expect("таблица подмножеств построена для каждого нужного размера")
    }

    /// Строит дерево сегментов в порядке обхода в ширину (алгоритм 1).
    pub(crate) fn build(params: &CascadeParams) -> Result<Self, CascadeError> {
        let (k, d, mu) = (params.k(), params.d(), params.mu());

        let shapes = segment_shapes(k, d, mu);
        let mut needed = BTreeSet::new();
        for shape in &shapes {
            needed.insert(shape.mode);
            if shape.mode > 0 {
                needed.insert(shape.mode - 1); // столбцы матрицы Ξ
            }
        }
        let mut tables = vec![None; mu + 1];
        for &m in &needed {
            let size = binomial(d, m);
            if size > MAX_ALPHA as u128 {
                return Err(CascadeError::SizeLimitExceeded {
                    what: "число подмножеств C(d, m)",
                    actual: size,
                    max: MAX_ALPHA,
                });
            }
            tables[m] = Some(SubsetTable::new(d, m));
        }

        let mut structure = Self {
            k,
            d,
            tables,
            segments: Vec::with_capacity(shapes.len()),
            alpha: 0,
            file_size: 0,
        };
        for shape in shapes {
            structure.push_segment(shape)?;
        }

        if structure.alpha != params.alpha() || structure.file_size != params.file_size() {
            return Err(CascadeError::Internal(
                "размеры построенной супер-матрицы не совпали с формулами (3)",
            ));
        }
        Ok(structure)
    }

    fn push_segment(&mut self, shape: Shape) -> Result<(), CascadeError> {
        let (k, d, m) = (self.k, self.d, shape.mode);
        let table = self.table(m).clone();
        let cols = table.len();
        let mut entries = vec![
            Entry {
                origin: Origin::Zero,
                injection: None,
            };
            d * cols
        ];

        // Проход 1: сырые символы и нули G2 (непроверочные позиции).
        for (c, set) in table.iter().enumerate() {
            let lower_only = set.iter().all(|&y| y >= k);
            for x in 0..d {
                if !set.contains(&x) && above_max(set, x) {
                    continue; // проверочный символ, проход 2
                }
                // x ∈ I или x < max I: символ v_{x,I} либо w_{x,I∪{x}}.
                let nulled = x >= k && lower_only; // группа G2, определение 8
                entries[x * cols + c].origin = if nulled {
                    Origin::Zero
                } else {
                    let id = self.file_size;
                    self.file_size += 1;
                    Origin::Raw(Term {
                        id,
                        neg: shape.sigma[x],
                    })
                };
            }
        }

        // Проход 2: проверочные символы по уравнению (5).
        for (c, set) in table.iter().enumerate() {
            for x in 0..d {
                if set.contains(&x) || !above_max(set, x) {
                    continue;
                }
                let group = with(set, x); // Y = I ∪ {x}, max Y = x
                let mut terms = Vec::with_capacity(m);
                for &y in set {
                    let col = table
                        .position(&crate::subsets::without(&group, y))
                        .ok_or(CascadeError::Internal("нет столбца w-группы"))?;
                    match &entries[y * cols + col].origin {
                        Origin::Zero => {}
                        Origin::Raw(term) => terms.push(Term {
                            id: term.id,
                            neg: shape.sigma[x] ^ (m % 2 == 1) ^ (ind(&group, y) % 2 == 1),
                        }),
                        Origin::Parity(_) => {
                            return Err(CascadeError::Internal(
                                "член w-группы оказался проверочным символом",
                            ))
                        }
                    }
                }
                entries[x * cols + c].origin = Origin::Parity(terms);
            }
        }

        // Проход 3: матрица инъекции (14) из родительского сегмента.
        if let Some(link) = &shape.parent {
            let parent = &self.segments[link.index];
            let parent_table = self.table(parent.mode);
            for (c, set) in table.iter().enumerate() {
                if set.iter().any(|y| link.b.contains(y)) {
                    continue; // I ∩ B ≠ ∅
                }
                for i in 0..d {
                    if !above_max(set, i) || link.b.contains(&i) {
                        continue;
                    }
                    let j = union(&with(set, i), &link.b);
                    let col = parent_table
                        .position(&j)
                        .ok_or(CascadeError::Internal("нет столбца родителя"))?;
                    entries[i * cols + c].injection = match &parent.entry(link.x, col).origin {
                        Origin::Zero => None,
                        Origin::Raw(term) => Some(Term {
                            id: term.id,
                            neg: term.neg ^ (parent.sigma[i] ^ (ind(&j, i) % 2 == 0)),
                        }),
                        Origin::Parity(_) => {
                            return Err(CascadeError::Internal(
                                "инъекция ссылается на проверочный символ родителя",
                            ))
                        }
                    };
                }
            }
        }

        self.segments.push(Segment {
            mode: m,
            sigma: shape.sigma,
            parent: shape.parent,
            offset: self.alpha,
            cols,
            entries,
        });
        self.alpha += cols;
        Ok(())
    }
}

/// Режим, подпись и родитель сегмента — без элементов.
#[derive(Debug, Clone)]
struct Shape {
    mode: usize,
    sigma: Vec<bool>,
    parent: Option<ParentLink>,
}

/// Перечисляет сегменты дерева в порядке обхода в ширину.
///
/// Дети родителя `P` соответствуют парам `(x, B)` из замечания 5:
/// `B ⊆ [k, d)`, `1 <= |B| < mode(P)`, `x ∈ [k, d)`, `x <= max B`. Порядок:
/// по возрастанию `|B|`, затем `B` лексикографически, затем `x`.
fn segment_shapes(k: usize, d: usize, mu: usize) -> Vec<Shape> {
    let mut shapes = vec![Shape {
        mode: mu,
        sigma: vec![false; d],
        parent: None,
    }];
    let mut queue = VecDeque::from([0usize]);
    while let Some(index) = queue.pop_front() {
        let parent_mode = shapes[index].mode;
        for size in 1..parent_mode {
            if size > d - k {
                break;
            }
            for lower in SubsetTable::new(d - k, size).iter() {
                let b: Vec<usize> = lower.iter().map(|&y| y + k).collect();
                let max_b = *b.last().expect("|B| >= 1");
                for x in k..=max_b {
                    // σ_Q(i) = 1 + σ_P(i) + ind_{B ∪ {i}}(i), формула (12).
                    let sigma = (0..d)
                        .map(|i| {
                            let below = b.iter().filter(|&&y| y < i).count();
                            shapes[index].sigma[i] ^ (below % 2 == 1)
                        })
                        .collect();
                    shapes.push(Shape {
                        mode: parent_mode - size - 1,
                        sigma,
                        parent: Some(ParentLink {
                            index,
                            x,
                            b: b.clone(),
                        }),
                    });
                    queue.push_back(shapes.len() - 1);
                }
            }
        }
    }
    shapes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structure(n: usize, k: usize, d: usize, mu: usize) -> Structure {
        Structure::build(&CascadeParams::try_new(n, k, d, mu).unwrap()).unwrap()
    }

    #[test]
    fn running_example_tree() {
        // Пример 8: моды (4, 2, 2, 2, 1, 1, 0 × 9), всего 15 сегментов.
        let s = structure(8, 4, 6, 4);
        let modes: Vec<usize> = s.segments.iter().map(|seg| seg.mode).collect();
        let mut sorted = modes.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(sorted, vec![4, 2, 2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(s.alpha, 81);
        assert_eq!(s.file_size, 324);
    }

    #[test]
    fn running_example_signatures() {
        // Пример 5 (в нумерации статьи с единицы):
        // σ(T2) = (2,2,2,2,2,2) для (6, {6}); σ(T5) = (2,2,2,2,2,3) для (6, {5,6}).
        let s = structure(8, 4, 6, 4);
        let find = |x: usize, b: &[usize]| {
            s.segments
                .iter()
                .find(|seg| {
                    seg.parent
                        .as_ref()
                        .is_some_and(|p| p.index == 0 && p.x == x && p.b == b)
                })
                .unwrap()
        };
        assert_eq!(find(5, &[5]).sigma, vec![false; 6]);
        assert_eq!(
            find(5, &[4, 5]).sigma,
            vec![false, false, false, false, false, true]
        );
        assert_eq!(find(5, &[4, 5]).mode, 1);
    }

    #[test]
    fn determinant_code_when_k_equals_d() {
        // При k = d нижних строк нет: код — один знаковый детерминантный код.
        let s = structure(6, 4, 4, 2);
        assert_eq!(s.segments.len(), 1);
        assert_eq!(s.alpha, 6);
        assert_eq!(s.file_size, 2 * 10); // m · C(d + 1, m + 1)
    }

    #[test]
    fn injections_land_only_on_parities() {
        let s = structure(8, 4, 6, 4);
        for seg in &s.segments {
            for entry in &seg.entries {
                if entry.injection.is_some() {
                    assert!(matches!(entry.origin, Origin::Parity(_)));
                }
            }
        }
    }

    #[test]
    fn every_lower_raw_symbol_has_one_primary_injection() {
        // Каждый символ G1 инъецирован в верхнюю часть ровно одного ребёнка.
        let s = structure(9, 3, 6, 3);
        let mut primary = vec![0usize; s.file_size];
        for seg in &s.segments {
            for x in 0..s.k {
                for c in 0..seg.cols {
                    if let Some(term) = seg.entry(x, c).injection {
                        primary[term.id] += 1;
                    }
                }
            }
        }
        for seg in &s.segments {
            for x in s.k..s.d {
                for c in 0..seg.cols {
                    if let Origin::Raw(term) = seg.entry(x, c).origin {
                        assert_eq!(primary[term.id], 1, "символ {}", term.id);
                    }
                }
            }
        }
    }

    #[test]
    fn construction_matches_formulas() {
        // Все (k, d; mu) с d <= 12: построенные alpha и F совпадают с (3).
        let mut checked = 0;
        for n in [13usize] {
            for k in 1..=8 {
                for d in k..n {
                    for mu in 1..=k {
                        let Ok(params) = CascadeParams::try_new(n, k, d, mu) else {
                            continue;
                        };
                        let s = Structure::build(&params).unwrap();
                        assert_eq!(s.alpha, params.alpha());
                        assert_eq!(s.file_size, params.file_size());
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 150, "проверено {checked} наборов");
    }
}
