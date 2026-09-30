//! Систематическое кодирование.

use gf2m::Gf;

use super::RsError;

/// Вычисляет проверочные символы систематического кода РС:
/// `parity(x) = msg(x) · x^nsym mod g(x)`.
///
/// `gen` — порождающий многочлен из
/// [`generator_poly`](super::generator_poly) (коэффициенты по
/// убыванию степеней, `gen.len() = nsym + 1`);
/// `parity_out.len() == gen.len() − 1`. Кодовое слово — конкатенация
/// `msg || parity`.
///
/// # Ошибки
/// [`RsError::InvalidParameters`] — если `gen` пуст или
/// `parity_out.len() + 1 != gen.len()`.
///
/// # Паника
/// Пока не реализовано: тело функции — `todo!()`.
#[allow(unused_variables)]
pub fn encode_parity<const N: usize, const POLY: u32>(
    msg: &[Gf<N, POLY>],
    gen: &[Gf<N, POLY>],
    parity_out: &mut [Gf<N, POLY>],
) -> Result<(), RsError> {
    todo!()
}
