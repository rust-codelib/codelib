//! Обратимое битовое представление байтового сообщения над любым полем.

use crate::polynomial::check_coefficient_limit;
use crate::{FieldElement, Polynomial, PolynomialError, MAX_MESSAGE_BYTES};

fn check_message_length(bytes: usize) -> Result<(), PolynomialError> {
    if bytes > MAX_MESSAGE_BYTES {
        return Err(PolynomialError::MessageTooLong { bytes });
    }
    Ok(())
}

impl<F: FieldElement> Polynomial<F> {
    /// Представляет байт восемью коэффициентами `0`/`1` поля, младший бит первым.
    ///
    /// Бит байта хранится в `coefficients[8 * byte_index + bit_index]`.
    /// Длина и нулевые байты сохраняются; пустому сообщению соответствует
    /// пустой массив. Представление работает над любым `F: FieldElement`.
    ///
    /// # Errors
    ///
    /// До выделения массива проверяет [`MAX_MESSAGE_BYTES`]
    /// ([`PolynomialError::MessageTooLong`]), затем объём коэффициентов
    /// ([`PolynomialError::CoefficientLimitExceeded`]). Для крупных `F`
    /// второй предел может быть строже.
    pub fn try_from_message(message: &[u8]) -> Result<Self, PolynomialError> {
        check_message_length(message.len())?;
        // После предела 4096 произведение не больше 32768 и входит в usize.
        let coefficient_count = message.len() * 8;
        check_coefficient_limit::<F>(coefficient_count)?;

        let zero = F::zero();
        let one = F::one();
        let mut coefficients = Vec::with_capacity(coefficient_count);
        for &byte in message {
            for bit in 0..8 {
                coefficients.push(if byte & (1 << bit) == 0 { zero } else { one });
            }
        }
        Self::try_new(coefficients)
    }

    /// Обратная операция к [`try_from_message`](Self::try_from_message).
    /// Использует весь массив, включая хвостовые нули: равные многочлены
    /// могут восстановить сообщения разной длины.
    ///
    /// # Errors
    ///
    /// До выделения буфера проверяет кратность длины восьми
    /// ([`PolynomialError::InvalidMessageCoefficientCount`]), предел сообщения
    /// ([`PolynomialError::MessageTooLong`]), затем коэффициенты `0`/`1`
    /// ([`PolynomialError::InvalidMessageCoefficient`] с первым неверным индексом).
    pub fn try_to_message(&self) -> Result<Vec<u8>, PolynomialError> {
        let coefficients = self.coefficients();
        if coefficients.len() % 8 != 0 {
            return Err(PolynomialError::InvalidMessageCoefficientCount {
                coefficients: coefficients.len(),
            });
        }
        let bytes = coefficients.len() / 8;
        check_message_length(bytes)?;

        let zero = F::zero();
        let one = F::one();
        if let Some(index) = coefficients
            .iter()
            .position(|&coefficient| coefficient != zero && coefficient != one)
        {
            return Err(PolynomialError::InvalidMessageCoefficient { index });
        }

        let mut message = Vec::with_capacity(bytes);
        for bits in coefficients.chunks_exact(8) {
            let mut byte = 0u8;
            for (bit, &coefficient) in bits.iter().enumerate() {
                if coefficient == one {
                    byte |= 1 << bit;
                }
            }
            message.push(byte);
        }
        Ok(message)
    }
}
