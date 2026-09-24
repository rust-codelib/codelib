//! Скан источников gf2m: инварианты no_std, отсутствие unsafe,
//! аллокаций, явных паник и заглушек. Сканируется только код:
//! doc-комментарии и `//` исключаются, атрибут
//! `#![forbid(unsafe_code)]` — разрешён.

static SOURCES: &[(&str, &str)] = &[
    ("src/lib.rs", include_str!("../src/lib.rs")),
    ("src/error.rs", include_str!("../src/error.rs")),
    ("src/gf.rs", include_str!("../src/gf.rs")),
    ("src/poly.rs", include_str!("../src/poly.rs")),
    ("src/runtime.rs", include_str!("../src/runtime.rs")),
    ("src/tables.rs", include_str!("../src/tables.rs")),
];

/// Код без комментариев; атрибут запрета unsafe изымается.
fn code_of(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .map(|l| l.replace("#![forbid(unsafe_code)]", ""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn no_unsafe() {
    for (name, src) in SOURCES {
        assert!(!code_of(src).contains("unsafe"), "unsafe в {name}");
    }
}

#[test]
fn no_std_runtime() {
    for (name, src) in SOURCES {
        let code = code_of(src);
        assert!(
            !code.contains("std::") && !code.contains("use std"),
            "std в {name}"
        );
    }
}

#[test]
fn no_allocations() {
    for (name, src) in SOURCES {
        let code = code_of(src);
        for pat in [
            "Vec<",
            "Box<",
            "Rc<",
            "Arc<",
            "format!",
            ".collect()",
            ".to_vec()",
            "alloc::",
            "String",
        ] {
            assert!(!code.contains(pat), "{pat} в {name}");
        }
    }
}

#[test]
fn no_stubs_or_explicit_panics() {
    // Полевой слой завершён: заглушек и явных паник нет.
    // debug_assert! и const assert! допустимы: это контракты,
    // а не паники арифметики.
    for (name, src) in SOURCES {
        let code = code_of(src);
        for pat in ["todo!", "unimplemented!", "panic!(", "unreachable!("] {
            assert!(!code.contains(pat), "{pat} в {name}");
        }
    }
}
