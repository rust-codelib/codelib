//! Exact decimal arithmetic for the experiment's sample count, without changing
//! the codec's mathematical profile or adding a dependency to the library.

const INVALID_EPSILON: &str = "epsilon must be a decimal number in [0, 1]";

/// Compute K + ceil(epsilon * K) from the user's decimal text. Scientific
/// notation and a leading sign remain supported, as with the previous f64 CLI.
/// Only this count uses exact decimal arithmetic; probabilities are unchanged.
pub(super) fn requested_sample_size(k: usize, epsilon: &str) -> Result<usize, &'static str> {
    let (negative, unsigned) = match epsilon.as_bytes().first() {
        Some(b'-') => (true, &epsilon[1..]),
        Some(b'+') => (false, &epsilon[1..]),
        _ => (false, epsilon),
    };
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(at) => (&unsigned[..at], decimal_exponent(&unsigned[at + 1..])?),
        None => (unsigned, 0),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if (integer.is_empty() && fraction.is_empty())
        || !integer
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return Err(INVALID_EPSILON);
    }
    let digits: Vec<_> = integer.bytes().chain(fraction.bytes()).collect();
    let Some(first) = digits.iter().position(|&b| b != b'0') else {
        return Ok(k); // Includes -0, but not a negative nonzero decimal.
    };
    if negative {
        return Err(INVALID_EPSILON);
    }
    let digits = &digits[first..];
    let point = exponent
        .saturating_add(integer.len() as i128)
        .saturating_sub(first as i128);
    if point > 0 {
        if point == 1 && digits[0] == b'1' && digits[1..].iter().all(|&b| b == b'0') {
            return k.checked_mul(2).ok_or("sample size overflow");
        }
        return Err(INVALID_EPSILON);
    }

    // Grade-school multiplication, right to left. All written decimal digits
    // are fractional here. Retain only the carry (integer part) and whether
    // any discarded digit is nonzero, which determines exact rounding upward.
    let mut carry = 0u128;
    let mut remainder = false;
    for &digit in digits.iter().rev() {
        let product = (k as u128)
            .checked_mul((digit - b'0') as u128)
            .and_then(|v| v.checked_add(carry))
            .ok_or("sample size overflow")?;
        remainder |= product % 10 != 0;
        carry = product / 10;
    }
    // Leading fractional zeros contribute further divisions by ten. Stop once
    // the carry vanishes, even for inputs such as 1e-1000000000000000000000.
    for _ in 0..point.unsigned_abs() {
        if carry == 0 {
            break;
        }
        remainder |= carry % 10 != 0;
        carry /= 10;
    }
    let extra = usize::try_from(carry).map_err(|_| "sample size overflow")?;
    k.checked_add(extra)
        .and_then(|v| v.checked_add(usize::from(remainder)))
        .ok_or("sample size overflow")
}

fn decimal_exponent(text: &str) -> Result<i128, &'static str> {
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(INVALID_EPSILON);
    }
    // Saturation only classifies an exponent far beyond the input's length:
    // a nonzero value is then either >1 (invalid) or tiny but still positive.
    // Decimal digits themselves and rounding are never approximated.
    let magnitude = digits.bytes().fold(0i128, |v, b| {
        v.saturating_mul(10).saturating_add((b - b'0') as i128)
    });
    Ok(if negative { -magnitude } else { magnitude })
}
