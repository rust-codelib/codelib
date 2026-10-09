use core::mem::size_of;

use crate::LdpcError;

pub(crate) fn check_buffer_bytes<T>(len: usize) -> Result<(), LdpcError> {
    len.checked_mul(size_of::<T>())
        .ok_or(LdpcError::SizeOverflow)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_buffer_bytes;
    use crate::LdpcError;

    #[test]
    fn buffer_byte_check_detects_overflow() {
        assert_eq!(
            check_buffer_bytes::<u64>(usize::MAX),
            Err(LdpcError::SizeOverflow)
        );
    }
}
