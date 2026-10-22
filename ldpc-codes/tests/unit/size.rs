use super::check_buffer_bytes;
use crate::LdpcError;

#[test]
fn buffer_byte_check_detects_overflow() {
    assert_eq!(
        check_buffer_bytes::<u64>(usize::MAX),
        Err(LdpcError::SizeOverflow)
    );
}
