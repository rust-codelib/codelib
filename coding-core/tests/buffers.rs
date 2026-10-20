#![no_std]
#![forbid(unsafe_code)]

#[path = "support/buffers.rs"]
mod buffers;
use buffers::*;
use coding_core::{DecodeOutcome, Decoder, Encoder, NoopObserver, SizeError};
use gf2m::Gf256;

#[test]
fn field_symbols_borrow_output_and_scratch_without_alloc() {
    assert_eq!(core::mem::size_of::<Gf256>(), 2);
    let message = [Gf256::new(9), Gf256::new(0), Gf256::new(0)];
    let mut word = [Gf256::new(0); 4];
    let mut scratch = [Gf256::new(0)];
    let encoded = FieldCodec
        .encode(
            EncodeInto {
                message: &message,
                output: &mut word,
                scratch: &mut scratch,
            },
            &mut NoopObserver,
        )
        .unwrap();
    let mut restored = [Gf256::new(7); 3];
    let DecodeOutcome::Candidate(candidate) = FieldCodec
        .decode(
            DecodeInto {
                word: encoded,
                original_len: 3,
                output: &mut restored,
            },
            &mut NoopObserver,
        )
        .unwrap()
    else {
        panic!("candidate expected")
    };
    assert_eq!(candidate.value.0, message);
}

#[test]
fn size_and_shape_errors_leave_caller_buffers_unchanged() {
    let oversized = [Gf256::new(0); 4097];
    let mut output = [Gf256::new(7); 1];
    let mut scratch = [Gf256::new(8)];
    assert_eq!(
        FieldCodec.encode(
            EncodeInto {
                message: &oversized,
                output: &mut output,
                scratch: &mut scratch
            },
            &mut NoopObserver
        ),
        Err(Error::Size(SizeError::MessageTooLong { actual: 4097 }))
    );
    assert_eq!(
        FieldCodec.encode(
            EncodeInto {
                message: &oversized[..1],
                output: &mut output,
                scratch: &mut scratch
            },
            &mut NoopObserver
        ),
        Err(Error::Shape)
    );
    assert_eq!(output, [Gf256::new(7)]);
    assert_eq!(scratch, [Gf256::new(8)]);
    let mut max_output = [Gf256::new(0); 4097];
    assert!(FieldCodec
        .encode(
            EncodeInto {
                message: &oversized[..4096],
                output: &mut max_output,
                scratch: &mut scratch
            },
            &mut NoopObserver
        )
        .is_ok());
}

#[test]
fn decode_validates_restored_size_and_parity_before_writing() {
    let mut output = [Gf256::new(7)];
    assert_eq!(
        FieldCodec.decode(
            DecodeInto {
                word: &[],
                original_len: 4097,
                output: &mut output
            },
            &mut NoopObserver
        ),
        Err(Error::Size(SizeError::MessageTooLong { actual: 4097 }))
    );
    assert_eq!(
        FieldCodec.decode(
            DecodeInto {
                word: &[],
                original_len: 1,
                output: &mut output
            },
            &mut NoopObserver
        ),
        Err(Error::Shape)
    );
    assert_eq!(
        FieldCodec.decode(
            DecodeInto {
                word: &[Gf256::new(1), Gf256::new(0)],
                original_len: 1,
                output: &mut output
            },
            &mut NoopObserver
        ),
        Ok(DecodeOutcome::Stopped(()))
    );
    assert_eq!(output, [Gf256::new(7)]);
    let mut empty = [];
    let DecodeOutcome::Candidate(result) = FieldCodec
        .decode(
            DecodeInto {
                word: &[Gf256::new(0)],
                original_len: 0,
                output: &mut empty,
            },
            &mut NoopObserver,
        )
        .unwrap()
    else {
        panic!("empty candidate expected")
    };
    assert!(result.value.0.is_empty());
}
