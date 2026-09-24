//! Скан источников rs-codes: инварианты no_std, отсутствие unsafe,
//! аллокаций и явных паник; каждый модуль обязан содержать
//! `todo!()`-заглушку — контракты заморожены до реализации.
//! Сканируется только код: doc-комментарии и `//` исключаются,
//! атрибут `#![forbid(unsafe_code)]` — разрешён.

static SOURCES: &[(&str, &str)] = &[
    ("src/lib.rs", include_str!("../src/lib.rs")),
    (
        "src/berlekamp_massey.rs",
        include_str!("../src/berlekamp_massey.rs"),
    ),
    ("src/chien.rs", include_str!("../src/chien.rs")),
    ("src/decode.rs", include_str!("../src/decode.rs")),
    ("src/encode.rs", include_str!("../src/encode.rs")),
    ("src/euclid.rs", include_str!("../src/euclid.rs")),
    ("src/forney.rs", include_str!("../src/forney.rs")),
    ("src/generator.rs", include_str!("../src/generator.rs")),
    ("src/syndrome.rs", include_str!("../src/syndrome.rs")),
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
fn no_explicit_panics() {
    // Допустим только todo!(); явные паники запрещены.
    for (name, src) in SOURCES {
        let code = code_of(src);
        for pat in ["unimplemented!", "panic!(", "unreachable!("] {
            assert!(!code.contains(pat), "{pat} в {name}");
        }
    }
}

#[test]
fn every_module_keeps_its_todo_stub() {
    // Заморозка контрактов: каждый алгоритмический модуль содержит
    // ровно заглушку, сигнатуры не должны исчезнуть.
    for (name, src) in &SOURCES[1..] {
        assert!(
            code_of(src).contains("todo!()"),
            "todo!() отсутствует в {name}: контракт потерян"
        );
    }
}
