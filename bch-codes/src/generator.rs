//! Построение и проверка порождающего многочлена.

use crate::bits::{bit, bytes_for, set};
use crate::config::{
    construction_workspace_len, generator_degree, generator_shape, validate_config,
};
use crate::{BchConfig, BchError, Gf};

fn primitive<const N: usize, const POLY: u32>() -> bool {
    let order = N - 1;
    if Gf::<N, POLY>::alpha().pow(order as u32) != Gf::one() {
        return false;
    }
    let mut q = order;
    let mut p = 2;
    while p * p <= q {
        if q % p == 0 {
            if Gf::<N, POLY>::alpha().pow((order / p) as u32) == Gf::one() {
                return false;
            }
            while q % p == 0 {
                q /= p;
            }
        }
        p += 1;
    }
    q == 1 || Gf::<N, POLY>::alpha().pow((order / q) as u32) != Gf::one()
}

const fn valid_field_parameters<const N: usize, const POLY: u32>() -> bool {
    N >= 4 && N <= 65536 && N.is_power_of_two() && POLY >= N as u32 && POLY < 2 * N as u32
}

/// Строит упакованный порождающий многочлен и возвращает его степень.
pub fn build_generator<const N: usize, const POLY: u32>(
    config: &BchConfig,
    output: &mut [u8],
    workspace: &mut [Gf<N, POLY>],
) -> Result<usize, BchError> {
    validate_config::<N>(config)?;
    if !valid_field_parameters::<N, POLY>() {
        return Err(BchError::InvalidParameters);
    }
    if !primitive::<N, POLY>() {
        return Err(BchError::NonPrimitiveField);
    }
    let degree = generator_degree::<N>(config)?;
    let expected = bytes_for(degree + 1);
    if output.len() != expected {
        return Err(BchError::BufferLength {
            expected,
            actual: output.len(),
        });
    }
    let required = construction_workspace_len::<N>(config)?;
    if workspace.len() < required {
        return Err(BchError::WorkspaceTooSmall {
            required,
            actual: workspace.len(),
        });
    }
    output.fill(0);
    let m1 = N.trailing_zeros() as usize + 1;
    let (g, rest) = workspace.split_at_mut(N);
    let (a, rest) = rest.split_at_mut(m1);
    let b = &mut rest[..m1];
    g.fill(Gf::zero());
    g[0] = Gf::one();
    let mut g_degree = 0;
    for leader in 1..=2 * config.correctable_errors {
        // Новый минимальный многочлен начинается только с лидера класса.
        let n = N - 1;
        let mut x = leader;
        let mut class_leader = leader;
        loop {
            x = (x * 2) % n;
            class_leader = class_leader.min(x);
            if x == leader {
                break;
            }
        }
        if class_leader != leader {
            continue;
        }
        a.fill(Gf::zero());
        a[0] = Gf::one();
        let mut f_degree = 0;
        let mut exponent = leader;
        loop {
            b.fill(Gf::zero());
            let root = Gf::<N, POLY>::alpha().pow(exponent as u32);
            for j in 0..=f_degree {
                b[j] += a[j] * root;
                b[j + 1] += a[j];
            }
            a.copy_from_slice(b);
            f_degree += 1;
            exponent = (exponent * 2) % (N - 1);
            if exponent == leader {
                break;
            }
        }
        if a[..=f_degree]
            .iter()
            .any(|v| *v != Gf::zero() && *v != Gf::one())
        {
            return Err(BchError::InvalidParameters);
        }
        for j in (0..=g_degree + f_degree).rev() {
            let mut value = Gf::zero();
            for q in 0..=f_degree {
                if q <= j && j - q <= g_degree {
                    value += g[j - q] * a[q];
                }
            }
            g[j] = value;
        }
        g_degree += f_degree;
    }
    if g_degree != degree {
        return Err(BchError::InvalidParameters);
    }
    for (i, value) in g[..=degree].iter().enumerate() {
        if *value == Gf::one() {
            set(output, i, true);
        } else if !value.is_zero() {
            return Err(BchError::InvalidParameters);
        }
    }
    Ok(degree)
}

pub(crate) fn validate_generator<const N: usize, const POLY: u32>(
    config: &BchConfig,
    generator: &[u8],
) -> Result<usize, BchError> {
    let degree = generator_shape::<N>(config, generator)?;
    if !valid_field_parameters::<N, POLY>() {
        return Err(BchError::InvalidParameters);
    }
    if !primitive::<N, POLY>() {
        return Err(BchError::NonPrimitiveField);
    }
    for root_power in 1..=2 * config.correctable_errors {
        let x = Gf::<N, POLY>::alpha().pow(root_power as u32);
        let mut acc = Gf::zero();
        for i in (0..=degree).rev() {
            acc = acc * x
                + if bit(generator, i) {
                    Gf::one()
                } else {
                    Gf::zero()
                };
        }
        if !acc.is_zero() {
            return Err(BchError::InvalidGenerator);
        }
    }
    Ok(degree)
}
