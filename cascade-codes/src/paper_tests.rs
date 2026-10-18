//! Сверка построения с явными примерами статьи для `(n, k, d; mu) = (n, 4, 6; 4)`:
//! примеры 1–8, матрицы (10), (16)–(21), группы (20), дерево рис. 4 и
//! пространство восстановления (24)–(25).
//!
//! Символы печатаются в нотации статьи: нумерация с единицы, `v`/`w` по (6),
//! знак `-` для `(-1)^σ(x)`, ноль для группы `G2`. Инъецированные символы
//! помечаются номером сегмента статьи `<i>`.

use std::collections::{BTreeMap, BTreeSet};

use crate::repair::{prop3_neg, xi_pattern};
use crate::structure::{Origin, Structure, Term};
use crate::subsets::with;
use crate::CascadeParams;

fn example() -> Structure {
    Structure::build(&CascadeParams::try_new(8, 4, 6, 4).unwrap()).unwrap()
}

fn fmt_set(set: &[usize]) -> String {
    let items: Vec<String> = set.iter().map(|x| (x + 1).to_string()).collect();
    format!("{{{}}}", items.join(","))
}

/// Множество из нумерации статьи (с единицы) во внутреннюю.
fn set0(paper: &[usize]) -> Vec<usize> {
    paper.iter().map(|x| x - 1).collect()
}

/// Метка символа в позиции `(x, I)` по формуле (6) без знака.
fn label(set: &[usize], x: usize) -> String {
    if set.contains(&x) {
        format!("v{},{}", x + 1, fmt_set(set))
    } else {
        format!("w{},{}", x + 1, fmt_set(&with(set, x)))
    }
}

fn column(s: &Structure, seg: usize, c: usize) -> Vec<usize> {
    s.table(s.segments[seg].mode)
        .iter()
        .nth(c)
        .unwrap()
        .to_vec()
}

fn col_of(s: &Structure, seg: usize, paper_set: &[usize]) -> usize {
    s.table(s.segments[seg].mode)
        .position(&set0(paper_set))
        .unwrap()
}

/// Сегмент-ребёнок `parent` с парой инъекции `(x, B)` в нумерации статьи.
fn child(s: &Structure, parent: usize, x: usize, b: &[usize]) -> usize {
    s.segments
        .iter()
        .position(|seg| {
            seg.parent
                .as_ref()
                .is_some_and(|p| p.index == parent && p.x == x - 1 && p.b == set0(b))
        })
        .unwrap()
}

/// Номера сегментов статьи (таблица III, пример 7) для наших индексов.
fn paper_names(s: &Structure) -> BTreeMap<usize, usize> {
    let mut names = BTreeMap::from([(0, 0)]);
    for (t, x, b) in [
        (1, 5, &[5][..]),
        (2, 6, &[6]),
        (3, 5, &[6]),
        (4, 5, &[5, 6]),
        (5, 6, &[5, 6]),
    ] {
        names.insert(child(s, 0, x, b), t);
    }
    let t2 = child(s, 0, 6, &[6]);
    for (t, x, b) in [(9, 5, &[5][..]), (10, 6, &[6]), (11, 5, &[6])] {
        names.insert(child(s, t2, x, b), t);
    }
    names
}

fn find_paper(s: &Structure, t: usize) -> usize {
    *paper_names(s).iter().find(|(_, &v)| v == t).unwrap().0
}

/// Исходная часть элемента в записи статьи: `±v…`/`±w…` или `0` для `G2`.
fn base(s: &Structure, seg: usize, x: usize, c: usize) -> String {
    let segment = &s.segments[seg];
    match segment.entry(x, c).origin {
        Origin::Zero => "0".to_string(),
        _ => {
            let sign = if segment.sigma[x] { "-" } else { "" };
            format!("{sign}{}", label(&column(s, seg, c), x))
        }
    }
}

/// Метка сырого символа (без знака) с номером сегмента статьи.
fn raw_label(s: &Structure, id: usize) -> String {
    let names = paper_names(s);
    for (seg, segment) in s.segments.iter().enumerate() {
        for x in 0..s.d {
            for c in 0..segment.cols {
                if let Origin::Raw(t) = segment.entry(x, c).origin {
                    if t.id == id {
                        let tag = names.get(&seg).map_or("?".into(), ToString::to_string);
                        return format!("{}<{tag}>", label(&column(s, seg, c), x));
                    }
                }
            }
        }
    }
    panic!("символ {id} не найден");
}

fn render_term(s: &Structure, term: Term) -> String {
    format!(
        "{}{}",
        if term.neg { "-" } else { "" },
        raw_label(s, term.id)
    )
}

/// Ненулевые элементы матрицы инъекции сегмента: `(строка, столбец) -> символ`.
fn injections(s: &Structure, seg: usize) -> BTreeMap<(usize, String), String> {
    let segment = &s.segments[seg];
    let mut out = BTreeMap::new();
    for x in 0..s.d {
        for c in 0..segment.cols {
            if let Some(term) = segment.entry(x, c).injection {
                out.insert((x + 1, fmt_set(&column(s, seg, c))), render_term(s, term));
            }
        }
    }
    out
}

fn expected_injections(items: &[(usize, &[usize], &str)]) -> BTreeMap<(usize, String), String> {
    items
        .iter()
        .map(|&(x, set, sym)| ((x, fmt_set(&set0(set))), sym.to_string()))
        .collect()
}

/// Группы нижних строк по определению 8 (метки со знаком, без номера).
fn groups(s: &Structure, seg: usize) -> [BTreeSet<String>; 3] {
    let segment = &s.segments[seg];
    let mut g: [BTreeSet<String>; 3] = Default::default();
    for x in s.k..s.d {
        for c in 0..segment.cols {
            let sign = if segment.sigma[x] { "-" } else { "" };
            let name = format!("{sign}{}", label(&column(s, seg, c), x));
            let index = match segment.entry(x, c).origin {
                Origin::Raw(_) => 0,
                Origin::Zero => 1,
                Origin::Parity(_) => 2,
            };
            g[index].insert(name);
        }
    }
    g
}

fn set_of(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(ToString::to_string).collect()
}

fn check_rows(s: &Structure, seg: usize, cols: &[&[usize]], rows: &[&[&str]]) {
    for (x, row) in rows.iter().enumerate() {
        for (paper_set, expected) in cols.iter().zip(row.iter()) {
            let c = col_of(s, seg, paper_set);
            assert_eq!(
                base(s, seg, x, c),
                *expected,
                "строка {}, столбец {}",
                x + 1,
                fmt_set(&set0(paper_set))
            );
        }
    }
}

#[test]
fn equation_10_root_matrix() {
    let s = example();
    let cols: [&[usize]; 5] = [
        &[1, 2, 3, 4],
        &[1, 2, 3, 5],
        &[1, 2, 3, 6],
        &[1, 2, 4, 5],
        &[1, 2, 4, 6],
    ];
    let rows: [&[&str]; 6] = [
        &[
            "v1,{1,2,3,4}",
            "v1,{1,2,3,5}",
            "v1,{1,2,3,6}",
            "v1,{1,2,4,5}",
            "v1,{1,2,4,6}",
        ],
        &[
            "v2,{1,2,3,4}",
            "v2,{1,2,3,5}",
            "v2,{1,2,3,6}",
            "v2,{1,2,4,5}",
            "v2,{1,2,4,6}",
        ],
        &[
            "v3,{1,2,3,4}",
            "v3,{1,2,3,5}",
            "v3,{1,2,3,6}",
            "w3,{1,2,3,4,5}",
            "w3,{1,2,3,4,6}",
        ],
        &[
            "v4,{1,2,3,4}",
            "w4,{1,2,3,4,5}",
            "w4,{1,2,3,4,6}",
            "v4,{1,2,4,5}",
            "v4,{1,2,4,6}",
        ],
        &[
            "w5,{1,2,3,4,5}",
            "v5,{1,2,3,5}",
            "w5,{1,2,3,5,6}",
            "v5,{1,2,4,5}",
            "w5,{1,2,4,5,6}",
        ],
        &[
            "w6,{1,2,3,4,6}",
            "w6,{1,2,3,5,6}",
            "v6,{1,2,3,6}",
            "w6,{1,2,4,5,6}",
            "v6,{1,2,4,6}",
        ],
    ];
    check_rows(&s, 0, &cols, &rows);
    // Последний столбец (10).
    let cols: [&[usize]; 1] = [&[3, 4, 5, 6]];
    let rows: [&[&str]; 6] = [
        &["w1,{1,3,4,5,6}"],
        &["w2,{2,3,4,5,6}"],
        &["v3,{3,4,5,6}"],
        &["v4,{3,4,5,6}"],
        &["v5,{3,4,5,6}"],
        &["v6,{3,4,5,6}"],
    ];
    check_rows(&s, 0, &cols, &rows);
    assert!(
        injections(&s, 0).is_empty(),
        "в корень ничего не инъецируется"
    );
}

#[test]
fn example_2_groups_of_root() {
    let s = example();
    let [g1, g2, g3] = groups(&s, 0);
    assert!(g2.is_empty());
    assert_eq!(
        g3,
        set_of(&[
            "w5,{1,2,3,4,5}",
            "w6,{1,2,3,4,6}",
            "w6,{1,2,3,5,6}",
            "w6,{1,2,4,5,6}",
            "w6,{1,3,4,5,6}",
            "w6,{2,3,4,5,6}",
        ])
    );
    // В (10) обведены 14 символов строки 5 и 10 символов строки 6.
    assert_eq!(g1.len(), 24);
    for boxed in [
        "v5,{1,2,3,5}",
        "w5,{1,2,3,5,6}",
        "v6,{1,2,3,6}",
        "v6,{1,3,5,6}",
        "w5,{2,3,4,5,6}",
    ] {
        assert!(g1.contains(boxed), "{boxed}");
    }
}

#[test]
fn table_iii_and_examples_4_5_children_of_root() {
    let s = example();
    let root_children: Vec<(usize, Vec<usize>, usize)> = s
        .segments
        .iter()
        .filter_map(|seg| {
            let p = seg.parent.as_ref()?;
            (p.index == 0).then(|| (p.x + 1, p.b.iter().map(|b| b + 1).collect(), seg.mode))
        })
        .collect();
    let mut sorted = root_children.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        vec![
            (5, vec![5], 2),
            (5, vec![5, 6], 1),
            (5, vec![6], 2),
            (6, vec![5, 6], 1),
            (6, vec![6], 2),
        ]
    );
    // Пример 5: σ(T2) = (2,2,2,2,2,2), σ(T5) = (2,2,2,2,2,3) — по чётности.
    assert_eq!(s.segments[find_paper(&s, 2)].sigma, vec![false; 6]);
    assert_eq!(
        s.segments[find_paper(&s, 5)].sigma,
        vec![false, false, false, false, false, true]
    );
}

#[test]
fn equation_16_child_t2() {
    let s = example();
    let t2 = find_paper(&s, 2);
    let cols: [&[usize]; 15] = [
        &[1, 2],
        &[1, 3],
        &[1, 4],
        &[1, 5],
        &[1, 6],
        &[2, 3],
        &[2, 4],
        &[2, 5],
        &[2, 6],
        &[3, 4],
        &[3, 5],
        &[3, 6],
        &[4, 5],
        &[4, 6],
        &[5, 6],
    ];
    let rows: [&[&str]; 6] = [
        &[
            "v1,{1,2}",
            "v1,{1,3}",
            "v1,{1,4}",
            "v1,{1,5}",
            "v1,{1,6}",
            "w1,{1,2,3}",
            "w1,{1,2,4}",
            "w1,{1,2,5}",
            "w1,{1,2,6}",
            "w1,{1,3,4}",
            "w1,{1,3,5}",
            "w1,{1,3,6}",
            "w1,{1,4,5}",
            "w1,{1,4,6}",
            "w1,{1,5,6}",
        ],
        &[
            "v2,{1,2}",
            "w2,{1,2,3}",
            "w2,{1,2,4}",
            "w2,{1,2,5}",
            "w2,{1,2,6}",
            "v2,{2,3}",
            "v2,{2,4}",
            "v2,{2,5}",
            "v2,{2,6}",
            "w2,{2,3,4}",
            "w2,{2,3,5}",
            "w2,{2,3,6}",
            "w2,{2,4,5}",
            "w2,{2,4,6}",
            "w2,{2,5,6}",
        ],
        &[
            "w3,{1,2,3}",
            "v3,{1,3}",
            "w3,{1,3,4}",
            "w3,{1,3,5}",
            "w3,{1,3,6}",
            "v3,{2,3}",
            "w3,{2,3,4}",
            "w3,{2,3,5}",
            "w3,{2,3,6}",
            "v3,{3,4}",
            "v3,{3,5}",
            "v3,{3,6}",
            "w3,{3,4,5}",
            "w3,{3,4,6}",
            "w3,{3,5,6}",
        ],
        &[
            "w4,{1,2,4}",
            "w4,{1,3,4}",
            "v4,{1,4}",
            "w4,{1,4,5}",
            "w4,{1,4,6}",
            "w4,{2,3,4}",
            "v4,{2,4}",
            "w4,{2,4,5}",
            "w4,{2,4,6}",
            "v4,{3,4}",
            "w4,{3,4,5}",
            "w4,{3,4,6}",
            "v4,{4,5}",
            "v4,{4,6}",
            "w4,{4,5,6}",
        ],
        &[
            "w5,{1,2,5}",
            "w5,{1,3,5}",
            "w5,{1,4,5}",
            "v5,{1,5}",
            "w5,{1,5,6}",
            "w5,{2,3,5}",
            "w5,{2,4,5}",
            "v5,{2,5}",
            "w5,{2,5,6}",
            "w5,{3,4,5}",
            "v5,{3,5}",
            "w5,{3,5,6}",
            "v5,{4,5}",
            "w5,{4,5,6}",
            "0",
        ],
        &[
            "w6,{1,2,6}",
            "w6,{1,3,6}",
            "w6,{1,4,6}",
            "w6,{1,5,6}",
            "v6,{1,6}",
            "w6,{2,3,6}",
            "w6,{2,4,6}",
            "w6,{2,5,6}",
            "v6,{2,6}",
            "w6,{3,4,6}",
            "w6,{3,5,6}",
            "v6,{3,6}",
            "w6,{4,5,6}",
            "v6,{4,6}",
            "0",
        ],
    ];
    check_rows(&s, t2, &cols, &rows);
}

#[test]
fn equation_17_injection_into_t2() {
    let s = example();
    let expected = expected_injections(&[
        (3, &[1, 2], "v6,{1,2,3,6}<0>"),
        (4, &[1, 2], "v6,{1,2,4,6}<0>"),
        (4, &[1, 3], "v6,{1,3,4,6}<0>"),
        (4, &[2, 3], "v6,{2,3,4,6}<0>"),
        // Вторичные инъекции в строку 5.
        (5, &[1, 2], "v6,{1,2,5,6}<0>"),
        (5, &[1, 3], "v6,{1,3,5,6}<0>"),
        (5, &[1, 4], "v6,{1,4,5,6}<0>"),
        (5, &[2, 3], "v6,{2,3,5,6}<0>"),
        (5, &[2, 4], "v6,{2,4,5,6}<0>"),
        (5, &[3, 4], "v6,{3,4,5,6}<0>"),
    ]);
    assert_eq!(injections(&s, find_paper(&s, 2)), expected);
}

#[test]
fn equation_18_child_t5() {
    let s = example();
    let t5 = find_paper(&s, 5);
    let cols: [&[usize]; 6] = [&[1], &[2], &[3], &[4], &[5], &[6]];
    let rows: [&[&str]; 6] = [
        &[
            "v1,{1}", "w1,{1,2}", "w1,{1,3}", "w1,{1,4}", "w1,{1,5}", "w1,{1,6}",
        ],
        &[
            "w2,{1,2}", "v2,{2}", "w2,{2,3}", "w2,{2,4}", "w2,{2,5}", "w2,{2,6}",
        ],
        &[
            "w3,{1,3}", "w3,{2,3}", "v3,{3}", "w3,{3,4}", "w3,{3,5}", "w3,{3,6}",
        ],
        &[
            "w4,{1,4}", "w4,{2,4}", "w4,{3,4}", "v4,{4}", "w4,{4,5}", "w4,{4,6}",
        ],
        &["w5,{1,5}", "w5,{2,5}", "w5,{3,5}", "w5,{4,5}", "0", "0"],
        &[
            "-w6,{1,6}",
            "-w6,{2,6}",
            "-w6,{3,6}",
            "-w6,{4,6}",
            "-w6,{5,6}",
            "0",
        ],
    ];
    check_rows(&s, t5, &cols, &rows);
}

#[test]
fn equation_19_injection_into_t5() {
    let s = example();
    let expected = expected_injections(&[
        (2, &[1], "-v6,{1,2,5,6}<0>"),
        (3, &[1], "-v6,{1,3,5,6}<0>"),
        (3, &[2], "-v6,{2,3,5,6}<0>"),
        (4, &[1], "-v6,{1,4,5,6}<0>"),
        (4, &[2], "-v6,{2,4,5,6}<0>"),
        (4, &[3], "-v6,{3,4,5,6}<0>"),
    ]);
    assert_eq!(injections(&s, find_paper(&s, 5)), expected);
}

#[test]
fn equation_20_and_example_7_groups() {
    let s = example();
    let [g1, g2, g3] = groups(&s, find_paper(&s, 2));
    assert_eq!(
        g1,
        set_of(&[
            "v5,{1,5}",
            "v5,{2,5}",
            "v5,{3,5}",
            "v5,{4,5}",
            "v6,{1,6}",
            "v6,{2,6}",
            "v6,{3,6}",
            "v6,{4,6}",
            "w5,{1,5,6}",
            "w5,{2,5,6}",
            "w5,{3,5,6}",
            "w5,{4,5,6}",
        ])
    );
    assert_eq!(g2, set_of(&["v5,{5,6}", "v6,{5,6}"]));
    assert_eq!(
        g3,
        set_of(&[
            "w5,{1,2,5}",
            "w5,{1,3,5}",
            "w5,{1,4,5}",
            "w5,{2,3,5}",
            "w5,{2,4,5}",
            "w5,{3,4,5}",
            "w6,{1,2,6}",
            "w6,{1,3,6}",
            "w6,{1,4,6}",
            "w6,{1,5,6}",
            "w6,{2,3,6}",
            "w6,{2,4,6}",
            "w6,{2,5,6}",
            "w6,{3,4,6}",
            "w6,{3,5,6}",
            "w6,{4,5,6}",
        ])
    );

    let [g1, g2, g3] = groups(&s, find_paper(&s, 5));
    assert!(g1.is_empty());
    assert_eq!(g2, set_of(&["v5,{5}", "-v6,{6}", "w5,{5,6}"]));
    assert_eq!(
        g3,
        set_of(&[
            "w5,{1,5}",
            "w5,{2,5}",
            "w5,{3,5}",
            "w5,{4,5}",
            "-w6,{1,6}",
            "-w6,{2,6}",
            "-w6,{3,6}",
            "-w6,{4,6}",
            "-w6,{5,6}",
        ])
    );
}

#[test]
fn equation_21_mode_zero_child_t11() {
    let s = example();
    let t11 = find_paper(&s, 11);
    assert_eq!(s.segments[t11].mode, 0);
    // Сама матрица режима 0 нулевая: все элементы — пустые проверочные суммы.
    for x in 0..s.d {
        assert_eq!(
            s.segments[t11].entry(x, 0).origin,
            Origin::Parity(Vec::new())
        );
    }
    let expected = expected_injections(&[
        (1, &[], "w5,{1,5,6}<2>"),
        (2, &[], "w5,{2,5,6}<2>"),
        (3, &[], "w5,{3,5,6}<2>"),
        (4, &[], "w5,{4,5,6}<2>"),
    ]);
    assert_eq!(injections(&s, t11), expected);
}

#[test]
fn example_8_and_fig_4_tree() {
    let s = example();
    let modes: Vec<usize> = s.segments.iter().map(|seg| seg.mode).collect();
    assert_eq!(modes, vec![4, 2, 2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    // Рис. 4: у T1, T2, T3 по три ребёнка (5,{5}), (6,{6}), (5,{6}); у T4, T5 детей нет.
    for t in 1..=5 {
        let parent = find_paper(&s, t);
        let mut pairs: Vec<(usize, Vec<usize>)> = s
            .segments
            .iter()
            .filter_map(|seg| {
                let p = seg.parent.as_ref()?;
                (p.index == parent).then(|| (p.x + 1, p.b.iter().map(|b| b + 1).collect()))
            })
            .collect();
        pairs.sort();
        let expected = if t <= 3 {
            vec![(5, vec![5]), (5, vec![6]), (6, vec![6])]
        } else {
            vec![]
        };
        assert_eq!(pairs, expected, "T{t}");
    }
    // Рис. 5: alpha = 1·C(6,4) + 3·C(6,2) + 2·C(6,1) + 9·C(6,0) = 81.
    assert_eq!(s.alpha, 15 + 3 * 15 + 2 * 6 + 9);
}

/// Символическая запись `[R_f(T0)]_{x,J} = Σ_I T0[x][I] · Ξ[I][J]` как
/// множество строк вида `+ψ4·v1,{1,2,3,4}`.
fn repair_space_entry(s: &Structure, x: usize, paper_col: &[usize]) -> BTreeSet<String> {
    let col = s.table(3).position(&set0(paper_col)).unwrap();
    xi_pattern(s, 0)
        .unwrap()
        .into_iter()
        .filter(|t| t.col == col)
        .map(|t| {
            let entry = base(s, 0, x, t.row);
            let (entry_neg, entry_label) = match entry.strip_prefix('-') {
                Some(rest) => (true, rest.to_string()),
                None => (false, entry),
            };
            let sign = if t.neg ^ entry_neg { '-' } else { '+' };
            format!("{sign}ψ{}·{entry_label}", t.y + 1)
        })
        .collect()
}

/// Слагаемое столбца (25): знак, индекс `y` у `ψ_{f,y}` и столбец `T0`.
type XiColumnTerm = (char, usize, &'static [usize]);

#[test]
fn equation_25_repair_space_of_root() {
    let s = example();
    // Шаблон знаков и индексов ψ по столбцам (25).
    let pattern: [(&[usize], [XiColumnTerm; 3]); 4] = [
        (
            &[1, 2, 3],
            [
                ('+', 4, &[1, 2, 3, 4]),
                ('+', 5, &[1, 2, 3, 5]),
                ('+', 6, &[1, 2, 3, 6]),
            ],
        ),
        (
            &[1, 2, 6],
            [
                ('-', 3, &[1, 2, 3, 6]),
                ('-', 4, &[1, 2, 4, 6]),
                ('-', 5, &[1, 2, 5, 6]),
            ],
        ),
        (
            &[1, 3, 6],
            [
                ('+', 2, &[1, 2, 3, 6]),
                ('-', 4, &[1, 3, 4, 6]),
                ('-', 5, &[1, 3, 5, 6]),
            ],
        ),
        (
            &[2, 3, 6],
            [
                ('-', 1, &[1, 2, 3, 6]),
                ('-', 4, &[2, 3, 4, 6]),
                ('-', 5, &[2, 3, 5, 6]),
            ],
        ),
    ];
    for (col, terms) in pattern {
        for x in 0..s.d {
            let expected: BTreeSet<String> = terms
                .iter()
                .map(|&(sign, y, set)| format!("{sign}ψ{y}·{}", label(&set0(set), x)))
                .collect();
            assert_eq!(
                repair_space_entry(&s, x, col),
                expected,
                "строка {}, столбец {col:?}",
                x + 1
            );
        }
    }
    // Несколько элементов, выписанных в (25) явно.
    assert_eq!(
        repair_space_entry(&s, 3, &[2, 3, 6]),
        set_of(&[
            "-ψ4·v4,{2,3,4,6}",
            "-ψ1·w4,{1,2,3,4,6}",
            "-ψ5·w4,{2,3,4,5,6}"
        ])
    );
    assert_eq!(
        repair_space_entry(&s, 4, &[1, 2, 3]),
        set_of(&[
            "+ψ4·w5,{1,2,3,4,5}",
            "+ψ5·v5,{1,2,3,5}",
            "+ψ6·w5,{1,2,3,5,6}"
        ])
    );
}

#[test]
fn equation_24_proposition_3_signs() {
    // [Ψf · T0]_{1,2,3,6} = −R_{1,{2,3,6}} + R_{2,{1,3,6}} − R_{3,{1,2,6}} + R_{6,{1,2,3}}.
    let s = example();
    let set = set0(&[1, 2, 3, 6]);
    let signs: Vec<bool> = set
        .iter()
        .map(|&i| prop3_neg(&s.segments[0].sigma, &set, i))
        .collect();
    assert_eq!(signs, vec![true, false, true, false]);
}
