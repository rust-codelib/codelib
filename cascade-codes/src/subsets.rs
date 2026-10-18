//! Подмножества `[d]` фиксированного размера в лексикографическом порядке.
//!
//! Строки и столбцы матриц статьи помечаются подмножествами. Внутри крейта
//! элементы нумеруются с нуля: строка `x` статьи соответствует `x - 1`.
//! Функция `ind_I(x)` и сравнение элементов от сдвига нумерации не зависят.

use std::collections::HashMap;

/// Все `m`-элементные подмножества `[0, d)` в лексикографическом порядке.
#[derive(Debug, Clone)]
pub(crate) struct SubsetTable {
    list: Vec<Vec<usize>>,
    index: HashMap<Vec<usize>, usize>,
}

impl SubsetTable {
    /// Перечисляет подмножества размера `m`; при `m > d` таблица пуста.
    pub(crate) fn new(d: usize, m: usize) -> Self {
        let list: Vec<Vec<usize>> = Combinations::new(d, m).collect();
        let index = list
            .iter()
            .enumerate()
            .map(|(i, set)| (set.clone(), i))
            .collect();
        Self { list, index }
    }

    pub(crate) fn len(&self) -> usize {
        self.list.len()
    }

    /// Позиция отсортированного подмножества в таблице.
    pub(crate) fn position(&self, set: &[usize]) -> Option<usize> {
        self.index.get(set).copied()
    }

    pub(crate) fn iter(&self) -> impl DoubleEndedIterator<Item = &[usize]> {
        self.list.iter().map(Vec::as_slice)
    }
}

/// Ленивый перебор `m`-элементных подмножеств `[0, d)` в лексикографическом
/// порядке; ничего не выделяет заранее.
#[derive(Debug, Clone)]
pub(crate) struct Combinations {
    d: usize,
    current: Option<Vec<usize>>,
}

impl Combinations {
    pub(crate) fn new(d: usize, m: usize) -> Self {
        Self {
            d,
            current: (m <= d).then(|| (0..m).collect()),
        }
    }
}

impl Iterator for Combinations {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Vec<usize>> {
        let out = self.current.take()?;
        let m = out.len();
        // Ищем самый правый элемент, который ещё можно увеличить.
        if let Some(pos) = (0..m).rev().find(|&i| out[i] < self.d - m + i) {
            let mut next = out.clone();
            next[pos] += 1;
            for i in pos + 1..m {
                next[i] = next[i - 1] + 1;
            }
            self.current = Some(next);
        }
        Some(out)
    }
}

/// `ind_I(x) = |{y ∈ I : y ≤ x}|`, формула (2) статьи.
pub(crate) fn ind(set: &[usize], x: usize) -> usize {
    set.iter().filter(|&&y| y <= x).count()
}

/// Объединение отсортированного множества с элементом, которого в нём нет.
pub(crate) fn with(set: &[usize], x: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(set.len() + 1);
    out.extend(set.iter().copied().filter(|&y| y < x));
    out.push(x);
    out.extend(set.iter().copied().filter(|&y| y > x));
    out
}

/// Отсортированное объединение двух непересекающихся множеств.
pub(crate) fn union(a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = a.iter().chain(b).copied().collect();
    out.sort_unstable();
    out
}

/// Множество без одного элемента.
pub(crate) fn without(set: &[usize], x: usize) -> Vec<usize> {
    set.iter().copied().filter(|&y| y != x).collect()
}

/// `true`, если `x > max(set)`; `max ∅ = −∞`.
pub(crate) fn above_max(set: &[usize], x: usize) -> bool {
    set.last().map_or(true, |&max| x > max)
}

/// Биномиальный коэффициент; ноль при `m > n`, `u128::MAX` при переполнении.
pub(crate) fn binomial(n: usize, m: usize) -> u128 {
    if m > n {
        return 0;
    }
    let m = m.min(n - m);
    let mut acc: u128 = 1;
    for i in 0..m {
        // acc * (n - i) / (i + 1) всегда целое на каждом шаге.
        let Some(next) = acc.checked_mul((n - i) as u128) else {
            return u128::MAX;
        };
        acc = next / (i as u128 + 1);
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexicographic_order_matches_paper() {
        // {1,2,3} ≺ {1,2,4} ≺ {1,2,5} ≺ {1,3,4} ≺ ... в нумерации с нуля.
        let table = SubsetTable::new(5, 3);
        let first: Vec<&[usize]> = table.iter().take(4).collect();
        assert_eq!(
            first,
            vec![&[0, 1, 2][..], &[0, 1, 3], &[0, 1, 4], &[0, 2, 3]]
        );
        assert_eq!(table.len(), 10);
        assert_eq!(table.position(&[2, 3, 4]), Some(9));
    }

    #[test]
    fn empty_and_full_sizes() {
        assert_eq!(SubsetTable::new(4, 0).len(), 1);
        assert_eq!(SubsetTable::new(4, 0).position(&[]), Some(0));
        assert_eq!(SubsetTable::new(4, 4).len(), 1);
        assert_eq!(SubsetTable::new(3, 4).len(), 0);
    }

    #[test]
    fn table_sizes_are_binomials() {
        for d in 0..9 {
            for m in 0..=d + 1 {
                assert_eq!(SubsetTable::new(d, m).len() as u128, binomial(d, m));
            }
        }
    }

    #[test]
    fn combinations_are_lazy() {
        // C(200, 100) не помещается в память, но первые элементы доступны сразу.
        let first: Vec<Vec<usize>> = Combinations::new(200, 100).take(2).collect();
        assert_eq!(first[0], (0..100).collect::<Vec<_>>());
        assert_eq!(first[1][99], 100);
        assert_eq!(Combinations::new(3, 4).count(), 0);
        assert_eq!(
            Combinations::new(5, 0).collect::<Vec<_>>(),
            vec![Vec::<usize>::new()]
        );
    }

    #[test]
    fn set_helpers() {
        assert_eq!(ind(&[1, 3, 5], 3), 2);
        assert_eq!(ind(&[1, 3, 5], 0), 0);
        assert_eq!(with(&[1, 5], 3), vec![1, 3, 5]);
        assert_eq!(union(&[4, 6], &[0, 5]), vec![0, 4, 5, 6]);
        assert_eq!(without(&[1, 3, 5], 3), vec![1, 5]);
        assert!(above_max(&[], 0));
        assert!(above_max(&[1, 2], 3));
        assert!(!above_max(&[1, 4], 3));
    }

    #[test]
    fn binomial_values() {
        assert_eq!(binomial(6, 4), 15);
        assert_eq!(binomial(7, 0), 1);
        assert_eq!(binomial(3, 5), 0);
        assert_eq!(binomial(60, 30), 118_264_581_564_861_424);
    }
}
