use core::mem::size_of;

use crate::LdpcError;

pub(crate) fn check_buffer_bytes<T>(len: usize) -> Result<(), LdpcError> {
    len.checked_mul(size_of::<T>())
        .ok_or(LdpcError::SizeOverflow)?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/size.rs"]
mod tests;
