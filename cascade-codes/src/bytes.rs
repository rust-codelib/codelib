//! Байтовый слой: сообщение до 4096 байт, упаковка в символы и разбиение на
//! блоки (страйпы) по `F` символов.
//!
//! Каждый символ несёт `b = floor(log2 q)` бит сообщения (младшие биты байта
//! идут первыми). Последний блок дополняется нулями. Узел хранит блоки подряд:
//! `stripes · alpha` символов. Пакет помощника — `stripes · beta` символов.

use crate::code::NodeShare;
use crate::observer::{reborrow, CascadeObserver};
use crate::{
    CascadeCode, CascadeError, CodeField, DataDecoder, DecodeResult, DecodeStatus, Encoder,
    NodeRepairer, RepairPayload,
};

/// Наибольшая длина исходного сообщения в байтах.
pub const MAX_MESSAGE_BYTES: usize = 4096;

/// Число бит сообщения в одном символе поля: `floor(log2 q)`.
#[must_use]
pub fn bits_per_symbol<F: CodeField>() -> u32 {
    let q = F::order();
    (127 - q.leading_zeros()).min(64)
}

/// Упаковывает байты в символы поля, сохраняя порядок бит.
///
/// ```rust
/// use cascade_codes::{pack_bytes, unpack_bytes};
/// use gfpm::Gf13; // 3 бита на символ
///
/// let symbols = pack_bytes::<Gf13>(b"Hi");
/// assert_eq!(symbols.len(), 6); // ceil(16 / 3)
/// assert_eq!(unpack_bytes(&symbols, 2)?, b"Hi");
/// # Ok::<(), cascade_codes::CascadeError>(())
/// ```
#[must_use]
pub fn pack_bytes<F: CodeField>(bytes: &[u8]) -> Vec<F> {
    let bits = bits_per_symbol::<F>() as usize;
    let total = bytes.len() * 8;
    (0..total.div_ceil(bits))
        .map(|s| {
            let value = (0..bits)
                .map(|j| s * bits + j)
                .filter(|&bit| bit < total && (bytes[bit / 8] >> (bit % 8)) & 1 == 1)
                .fold(0u64, |acc, bit| acc | 1u64 << (bit - s * bits));
            F::from_index(value).unwrap_or_else(F::zero)
        })
        .collect()
}

/// Распаковывает первые `byte_len` байт из символов.
///
/// # Ошибки
///
/// [`CascadeError::InvalidPackedSymbol`], если номер символа не помещается в
/// `bits_per_symbol` бит, и [`CascadeError::MessageLengthMismatch`], если
/// символов слишком мало.
pub fn unpack_bytes<F: CodeField>(symbols: &[F], byte_len: usize) -> Result<Vec<u8>, CascadeError> {
    let bits = bits_per_symbol::<F>() as usize;
    let needed = (byte_len * 8).div_ceil(bits);
    if symbols.len() < needed {
        return Err(CascadeError::MessageLengthMismatch {
            expected: needed,
            actual: symbols.len(),
        });
    }
    let mut bytes = vec![0u8; byte_len];
    for (position, symbol) in symbols.iter().enumerate() {
        let value = symbol.to_index();
        if bits < 64 && value >> bits != 0 {
            return Err(CascadeError::InvalidPackedSymbol { position });
        }
        for j in 0..bits {
            let bit = position * bits + j;
            if bit < byte_len * 8 && (value >> j) & 1 == 1 {
                bytes[bit / 8] |= 1 << (bit % 8);
            }
        }
    }
    Ok(bytes)
}

/// Закодированное байтовое сообщение.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedFile<F> {
    /// Исходная длина в байтах (нужна для декодирования).
    pub byte_len: usize,
    /// Число блоков по `F` символов.
    pub stripes: usize,
    /// Содержимое всех `n` узлов, по `stripes · alpha` символов.
    pub shares: Vec<NodeShare<F>>,
}

/// Результат декодирования байтового сообщения.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDecodeResult<F> {
    bytes: Vec<u8>,
    stripes: Vec<DecodeResult<F>>,
}

impl<F> FileDecodeResult<F> {
    /// Восстановленные байты.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Результаты по блокам.
    #[must_use]
    pub fn stripes(&self) -> &[DecodeResult<F>] {
        &self.stripes
    }

    /// Худший статус среди блоков; [`DecodeStatus::Consistent`] без блоков.
    #[must_use]
    pub fn status(&self) -> DecodeStatus {
        let rank = |s: DecodeStatus| match s {
            DecodeStatus::Consistent => 0,
            DecodeStatus::Corrected => 1,
            DecodeStatus::Inconsistent => 2,
        };
        self.stripes
            .iter()
            .map(DecodeResult::status)
            .max_by_key(|&s| rank(s))
            .unwrap_or(DecodeStatus::Consistent)
    }

    /// Объединение подозрительных узлов всех блоков по возрастанию.
    #[must_use]
    pub fn suspect_nodes(&self) -> Vec<usize> {
        let mut nodes: Vec<usize> = self
            .stripes
            .iter()
            .flat_map(|r| r.suspect_nodes().iter().copied())
            .collect();
        nodes.sort_unstable();
        nodes.dedup();
        nodes
    }

    /// Суммарное число несовпавших символов.
    #[must_use]
    pub fn mismatched_symbols(&self) -> usize {
        self.stripes
            .iter()
            .map(DecodeResult::mismatched_symbols)
            .sum()
    }
}

impl<F: CodeField> CascadeCode<F> {
    /// Число блоков для сообщения длиной `byte_len` байт.
    #[must_use]
    pub fn stripes_for(&self, byte_len: usize) -> usize {
        let bits = bits_per_symbol::<F>() as usize;
        (byte_len * 8)
            .div_ceil(bits)
            .div_ceil(self.params.file_size())
    }

    /// Кодирует байтовое сообщение.
    ///
    /// # Ошибки
    ///
    /// [`CascadeError::MessageTooLong`] для сообщения длиннее
    /// [`MAX_MESSAGE_BYTES`].
    pub fn encode_bytes(&self, bytes: &[u8]) -> Result<EncodedFile<F>, CascadeError> {
        if bytes.len() > MAX_MESSAGE_BYTES {
            return Err(CascadeError::MessageTooLong {
                max: MAX_MESSAGE_BYTES,
                actual: bytes.len(),
            });
        }
        let stripes = self.stripes_for(bytes.len());
        let f = self.params.file_size();
        let mut symbols = pack_bytes::<F>(bytes);
        symbols.resize(stripes * f, F::zero());
        let mut shares: Vec<NodeShare<F>> = (0..self.params.n())
            .map(|node| NodeShare {
                node,
                symbols: Vec::with_capacity(stripes * self.params.alpha()),
            })
            .collect();
        for block in symbols.chunks(f) {
            for (share, encoded) in shares.iter_mut().zip(self.encode(block)?) {
                share.symbols.extend(encoded.symbols);
            }
        }
        Ok(EncodedFile {
            byte_len: bytes.len(),
            stripes,
            shares,
        })
    }

    /// Декодирует байтовое сообщение по содержимому не менее `k` узлов.
    ///
    /// # Ошибки
    ///
    /// Ошибки длины, номеров узлов и [`CascadeError::MessageTooLong`].
    pub fn decode_bytes(
        &self,
        byte_len: usize,
        shares: &[NodeShare<F>],
        mut observer: Option<&mut dyn CascadeObserver>,
    ) -> Result<FileDecodeResult<F>, CascadeError> {
        if byte_len > MAX_MESSAGE_BYTES {
            return Err(CascadeError::MessageTooLong {
                max: MAX_MESSAGE_BYTES,
                actual: byte_len,
            });
        }
        let stripes = self.stripes_for(byte_len);
        let alpha = self.params.alpha();
        self.check_shares(shares, stripes * alpha)?;
        if shares.len() < self.params.k() {
            return Err(CascadeError::NotEnoughShares {
                required: self.params.k(),
                actual: shares.len(),
            });
        }
        let mut results = Vec::with_capacity(stripes);
        let mut symbols = Vec::with_capacity(stripes * self.params.file_size());
        for stripe in 0..stripes {
            let block: Vec<NodeShare<F>> = shares
                .iter()
                .map(|share| NodeShare {
                    node: share.node,
                    symbols: share.symbols[stripe * alpha..(stripe + 1) * alpha].to_vec(),
                })
                .collect();
            let result = self.decode(&block, reborrow(&mut observer))?;
            symbols.extend_from_slice(result.message());
            results.push(result);
        }
        Ok(FileDecodeResult {
            bytes: unpack_bytes(&symbols, byte_len)?,
            stripes: results,
        })
    }

    /// Пакет помощника для узла, хранящего несколько блоков.
    ///
    /// # Ошибки
    ///
    /// Длина содержимого не кратна `alpha`, а также ошибки
    /// [`NodeRepairer::repair_payload`].
    pub fn file_repair_payload(
        &self,
        helper: &NodeShare<F>,
        failed: usize,
    ) -> Result<RepairPayload<F>, CascadeError> {
        let alpha = self.params.alpha();
        if helper.symbols.len() % alpha != 0 {
            return Err(CascadeError::ShareLengthMismatch {
                node: helper.node,
                expected: (helper.symbols.len() / alpha + 1) * alpha,
                actual: helper.symbols.len(),
            });
        }
        let mut symbols = Vec::new();
        for block in helper.symbols.chunks(alpha) {
            let share = NodeShare {
                node: helper.node,
                symbols: block.to_vec(),
            };
            symbols.extend(self.repair_payload(&share, failed)?.symbols);
        }
        Ok(RepairPayload {
            helper: helper.node,
            symbols,
        })
    }

    /// Восстанавливает узел, хранящий `stripes` блоков.
    ///
    /// # Ошибки
    ///
    /// Длина пакета не равна `stripes · beta`, а также ошибки
    /// [`NodeRepairer::repair`].
    pub fn file_repair(
        &self,
        failed: usize,
        stripes: usize,
        payloads: &[RepairPayload<F>],
        mut observer: Option<&mut dyn CascadeObserver>,
    ) -> Result<NodeShare<F>, CascadeError> {
        let beta = self.params.beta();
        self.check_payloads(failed, payloads, stripes * beta)?;
        let mut symbols = Vec::with_capacity(stripes * self.params.alpha());
        for stripe in 0..stripes {
            let block: Vec<RepairPayload<F>> = payloads
                .iter()
                .map(|p| RepairPayload {
                    helper: p.helper,
                    symbols: p.symbols[stripe * beta..(stripe + 1) * beta].to_vec(),
                })
                .collect();
            symbols.extend(
                self.repair(failed, &block, reborrow(&mut observer))?
                    .symbols,
            );
        }
        Ok(NodeShare {
            node: failed,
            symbols,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gf2m::Gf256;
    use gfpm::{Gf13, Gf3};

    #[test]
    fn bits_per_symbol_values() {
        assert_eq!(bits_per_symbol::<Gf256>(), 8);
        assert_eq!(bits_per_symbol::<Gf13>(), 3);
        assert_eq!(bits_per_symbol::<Gf3>(), 1);
    }

    #[test]
    fn pack_round_trip() {
        let data: Vec<u8> = (0..=255).collect();
        assert_eq!(
            unpack_bytes(&pack_bytes::<Gf256>(&data), 256).unwrap(),
            data
        );
        assert_eq!(unpack_bytes(&pack_bytes::<Gf13>(&data), 256).unwrap(), data);
        assert_eq!(unpack_bytes(&pack_bytes::<Gf3>(&data), 256).unwrap(), data);
        assert!(pack_bytes::<Gf13>(&[]).is_empty());
    }

    #[test]
    fn gf256_packs_one_byte_per_symbol() {
        let symbols = pack_bytes::<Gf256>(&[0xAB, 0x01]);
        assert_eq!(symbols, vec![Gf256::new(0xAB), Gf256::new(0x01)]);
    }

    #[test]
    fn unpack_rejects_wide_symbol() {
        // В GF(13) упаковка использует 3 бита: номер 9 не помещается.
        let symbols = vec![Gf13::new(9), Gf13::new(0), Gf13::new(0)];
        assert_eq!(
            unpack_bytes(&symbols, 1),
            Err(CascadeError::InvalidPackedSymbol { position: 0 })
        );
    }

    #[test]
    fn unpack_rejects_short_input() {
        assert!(matches!(
            unpack_bytes(&[Gf13::new(1)], 1),
            Err(CascadeError::MessageLengthMismatch { .. })
        ));
    }
}
