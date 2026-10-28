//! Размер сообщения в передаваемом представлении и единицы измерения.

/// Максимальная длина исходного сообщения в байтах передаваемого представления.
pub const MAX_MESSAGE_BYTES: usize = 4096;

/// Единица измерения размера.
///
/// Этот тип не задаёт ограничений размеров матриц и векторов.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeUnit {
    /// Биты.
    Bits,
    /// Байты передаваемого представления, независимо от размера объекта Rust в памяти.
    Bytes,
    /// Символы поля или алфавита, определённого реализацией.
    Symbols,
    /// Части, пакеты или узлы в смысле, определённом реализацией.
    Parts,
}

/// Количество с явно указанной единицей измерения.
///
/// Переменную длину выхода кодек может описывать собственным типом.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    /// Количество единиц.
    pub count: usize,
    /// Единица измерения.
    pub unit: SizeUnit,
}

/// Проверенная длина исходного сообщения в байтах передаваемого представления.
///
/// Нулевая длина допустима; конкретный кодек может задавать более строгие правила.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageBytes(usize);

/// Ошибка проверки размера; может включаться в собственный тип ошибки кодека.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeError {
    /// Длина исходного сообщения превышает [`MAX_MESSAGE_BYTES`].
    MessageTooLong {
        /// Переданная длина в байтах.
        actual: usize,
    },
    /// Переполнение при вычислении размера в реализации кодека.
    Overflow,
}

impl MessageBytes {
    /// Проверяет длину сообщения независимо от размера элементов Rust в памяти.
    ///
    /// # Errors
    ///
    /// Возвращает [`SizeError::MessageTooLong`], если `bytes` превышает
    /// [`MAX_MESSAGE_BYTES`].
    pub const fn try_new(bytes: usize) -> Result<Self, SizeError> {
        if bytes > MAX_MESSAGE_BYTES {
            Err(SizeError::MessageTooLong { actual: bytes })
        } else {
            Ok(Self(bytes))
        }
    }

    /// Проверяет длину упакованного сообщения из `bits` битов в байтах.
    ///
    /// Неполный последний байт учитывается целиком. Сначала выполняется деление,
    /// поэтому вычисление не переполняется даже для `usize::MAX`.
    ///
    /// # Errors
    ///
    /// Возвращает [`SizeError::MessageTooLong`], если полученная длина
    /// превышает [`MAX_MESSAGE_BYTES`].
    pub const fn from_bits(bits: usize) -> Result<Self, SizeError> {
        Self::try_new(bits / 8 + if bits % 8 == 0 { 0 } else { 1 })
    }

    /// Возвращает проверенную длину сообщения в байтах.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}
