use coding_core::{DecodeOutcome, Decoder, IncrementalDecoder, NoopObserver};
use fountain_codes::*;
fn encoder(profile: Profile, msg: &[u8], t: usize, d: u16, seed: u64) -> RepairableEncoder {
    RepairableEncoder::try_new(
        msg,
        [7; 16],
        RepairableConfig::try_new(profile, t, d, seed, ResourceLimits::default()).unwrap(),
    )
    .unwrap()
}
#[test]
fn message_boundaries_ownership_order_and_padding() {
    for profile in [Profile::Gf65536, Profile::Gf256] {
        for n in [0, 1, 3, 4096] {
            let mut bytes = vec![0; n];
            for (i, b) in bytes.iter_mut().enumerate() {
                *b = (i % 251) as u8;
            }
            let expected = bytes.clone();
            let enc = encoder(profile, &bytes, 64, 16, 5);
            bytes.fill(255); // The encoder has its own source.
            let mut dec =
                RepairableDecoder::try_new(enc.descriptor().clone(), ResourceLimits::default())
                    .unwrap();
            for id in (0..enc.descriptor().source_symbols() as u64).rev() {
                dec.push_unobserved(&enc.symbol_unobserved(id).unwrap())
                    .unwrap();
            }
            assert_eq!(dec.message(), Some(expected.as_slice()));
            assert!(dec.result().unwrap().checks.padding_valid);
            if n == 0 {
                assert!(enc.symbol_unobserved(0).is_err());
                assert_eq!(enc.symbols_from(0).count(), 1);
            }
        }
        let cfg = RepairableConfig::try_new(profile, 64, 16, 0, ResourceLimits::default()).unwrap();
        assert!(RepairableEncoder::try_new(&[0; 4097], [0; 16], cfg).is_err());
    }
}
#[test]
fn duplicates_at_limit_and_conflict_are_transactional() {
    let enc = encoder(Profile::Gf256, &[1, 2, 0], 1, 5, 1);
    let limits = ResourceLimits::try_new(256, 1, 10, 100, 100000).unwrap();
    let mut dec = RepairableDecoder::try_new(enc.descriptor().clone(), limits).unwrap();
    let s = enc.symbol_unobserved(0).unwrap();
    dec.push_unobserved(&s).unwrap();
    let mut events = vec![];
    assert_eq!(
        dec.push(&s, &mut |e: &DecodeEvent| events.push(e.clone()))
            .unwrap()
            .kind,
        SymbolOutcome::Duplicate
    );
    assert!(matches!(events.as_slice(), [DecodeEvent::Duplicate { .. }]));
    let conflicting = EncodedSymbol::try_new(enc.descriptor().clone(), 0, vec![7]).unwrap();
    assert_eq!(
        dec.push_unobserved(&conflicting),
        Err(FountainError::ConflictingSymbol { id: 0 })
    );
    assert!(matches!(
        dec.push_unobserved(&enc.symbol_unobserved(1).unwrap()),
        Err(FountainError::ResourceLimitExceeded {
            resource: "unique_symbols",
            ..
        })
    ));
    assert_eq!(
        (dec.rank(), dec.unique_symbols(), dec.progress()),
        (1, 1, DecodeProgress::NeedMoreSymbols)
    );
}
#[test]
fn zero_equation_and_insufficient_rank_with_many_parts() {
    let enc = encoder(Profile::Gf256, &[8], 1, 1, 0x224);
    let zero = enc.symbol_unobserved(1).unwrap();
    let mut dec =
        RepairableDecoder::try_new(enc.descriptor().clone(), ResourceLimits::default()).unwrap();
    assert_eq!(
        dec.push_unobserved(&zero).unwrap().kind,
        SymbolOutcome::Dependent
    );
    assert_eq!((dec.rank(), dec.unique_symbols()), (0, 1));
    assert!(dec.message().is_none());
    let invalid = EncodedSymbol::try_new(enc.descriptor().clone(), 2, vec![255]).unwrap();
    // Different coefficients are consistent with some source until rank reaches K;
    // the zero equation itself cannot supply a source symbol.
    dec.push_unobserved(&invalid).unwrap();
    let enc = encoder(Profile::Gf256, &[4, 5, 6], 1, 1, 17);
    let mut dec =
        RepairableDecoder::try_new(enc.descriptor().clone(), ResourceLimits::default()).unwrap();
    dec.push_unobserved(&enc.symbol_unobserved(0).unwrap())
        .unwrap();
    for id in 3..100 {
        if canonical_support(enc.descriptor(), id)
            .unwrap()
            .iter()
            .all(|&(i, _)| i == 0)
        {
            dec.push_unobserved(&enc.symbol_unobserved(id).unwrap())
                .unwrap();
        }
    }
    assert!(dec.unique_symbols() > 3);
    assert_eq!(dec.rank(), 1);
    assert!(dec.message().is_none());
}
#[test]
fn padding_errors_before_insertion_and_after_solution() {
    for profile in [Profile::Gf65536, Profile::Gf256] {
        let enc = encoder(profile, &[17], 2, 1, 0);
        let d = enc.descriptor().clone();
        let mut dec = RepairableDecoder::try_new(d.clone(), ResourceLimits::default()).unwrap();
        let bad_source = EncodedSymbol::try_new(d.clone(), 0, vec![17, 1]).unwrap();
        let mut events = vec![];
        assert!(matches!(
            dec.push(&bad_source, &mut |e: &DecodeEvent| events.push(e.clone())),
            Err(FountainError::NonzeroPadding { .. })
        ));
        assert_eq!((dec.rank(), dec.unique_symbols()), (0, 0));
        assert!(events.is_empty());
        // Encode an input with the same layout but nonzero padding, then relabel
        // its parity with N=1. No direct systematic padding check can catch it.
        let other = encoder(profile, &[17, 1], 2, 1, 0);
        let parity = other.symbol_unobserved(1).unwrap();
        let bad = EncodedSymbol::try_new(d, 1, parity.payload().to_vec()).unwrap();
        assert!(matches!(
            dec.push(&bad, &mut |e: &DecodeEvent| events.push(e.clone())),
            Err(FountainError::NonzeroPadding { .. })
        ));
        assert_eq!(
            (dec.rank(), dec.unique_symbols(), dec.progress()),
            (1, 1, DecodeProgress::Invalid)
        );
        assert!(matches!(
            events.as_slice(),
            [
                DecodeEvent::Accepted { .. },
                DecodeEvent::RankIncreased { .. },
                DecodeEvent::Invalidated {
                    reason: InvalidReason::Padding { .. },
                    ..
                }
            ]
        ));
        assert!(dec.result().is_none());
    }
}
#[test]
fn complete_still_validates_and_batch_always_starts_fresh() {
    let enc = encoder(Profile::Gf256, &[17, 0, 3], 1, 5, 7);
    let d = enc.descriptor().clone();
    let mut dec = RepairableDecoder::try_new(d.clone(), ResourceLimits::default()).unwrap();
    let mut parts = vec![];
    for id in 0..3 {
        let s = enc.symbol_unobserved(id).unwrap();
        dec.push_unobserved(&s).unwrap();
        parts.push(s);
    }
    let mut events = vec![];
    assert_eq!(
        dec.push(
            &enc.symbol_unobserved(3).unwrap(),
            &mut |e: &DecodeEvent| events.push(e.clone())
        )
        .unwrap()
        .kind,
        SymbolOutcome::Verified
    );
    assert!(matches!(
        events.as_slice(),
        [DecodeEvent::Accepted { .. }, DecodeEvent::Verified { .. }]
    ));
    let mut batch = RepairableBatchDecoder::try_new(d.clone(), ResourceLimits::default()).unwrap();
    assert!(matches!(
        batch.decode(&parts, &mut NoopObserver).unwrap(),
        DecodeOutcome::Candidate(_)
    ));
    assert!(matches!(
        batch.decode(&[], &mut NoopObserver).unwrap(),
        DecodeOutcome::NeedMore(_)
    ));
    let mut bytes = enc.symbol_unobserved(4).unwrap().payload().to_vec();
    bytes[0] ^= 1;
    let bad = EncodedSymbol::try_new(d, 4, bytes).unwrap();
    parts.push(bad.clone());
    assert!(matches!(
        batch.decode(&parts, &mut NoopObserver),
        Err(FountainError::InconsistentEquation { .. })
    ));
    assert!(matches!(
        dec.push_unobserved(&bad),
        Err(FountainError::InconsistentEquation { .. })
    ));
    assert_eq!(dec.rank(), 3);
    assert_eq!(dec.unique_symbols(), 4);
    assert!(dec.message().is_none());
    assert_eq!(
        dec.push_unobserved(&parts[0]),
        Err(FountainError::InvalidDecoder)
    );
}
#[test]
fn budget_and_context_rejections_do_not_accept() {
    let enc = encoder(Profile::Gf256, &[1, 2], 1, 1, 0);
    let base = ResourceCharge::calculate(enc.descriptor(), 1, 0, 0)
        .unwrap()
        .decoder_bytes;
    let limits = ResourceLimits::try_new(256, 10, 10, 100, base).unwrap();
    let mut dec = RepairableDecoder::try_new(enc.descriptor().clone(), limits).unwrap();
    dec.push_unobserved(&enc.symbol_unobserved(0).unwrap())
        .unwrap();
    let mut events = vec![];
    assert!(matches!(
        dec.push(
            &enc.symbol_unobserved(1).unwrap(),
            &mut |e: &DecodeEvent| events.push(e.clone())
        ),
        Err(FountainError::ResourceLimitExceeded { .. })
    ));
    let other = encoder(Profile::Gf256, &[1, 2], 1, 1, 1);
    assert_eq!(
        dec.push_unobserved(&other.symbol_unobserved(0).unwrap()),
        Err(FountainError::ContextMismatch)
    );
    assert_eq!((dec.rank(), dec.unique_symbols()), (1, 1));
    assert!(events.is_empty());
    let max = BlockDescriptor::try_new(Profile::Gf256, [0; 16], 4096, 1, 256, 0).unwrap();
    // Dimension feasibility is cheap; no maximum-size elimination is run here.
    RepairableDecoder::try_new(max, ResourceLimits::default()).unwrap();
}
