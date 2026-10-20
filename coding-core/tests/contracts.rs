//! Проверки общих интерфейсов на небольших тестовых кодеках.
#[path = "support/block.rs"]
mod block;
#[path = "support/repair.rs"]
mod repair;
#[path = "support/stream.rs"]
mod stream;

use coding_core::{
    Candidate, Codeword, DecodeOutcome, Decoder, Encoder, IncrementalDecoder, LlrConvention,
    Message, MessageBytes, NoopObserver, Observer, RepairExecutor, RepairHelper, RepairPlan,
    RepairPlanner, Size, SizeError, SizeUnit, SymbolEncoder,
};

fn encode<E: Encoder<I>, I>(codec: &mut E, input: I) -> E::Output
where
    E::Error: core::fmt::Debug,
{
    codec.encode(input, &mut NoopObserver).unwrap()
}
fn candidate<D: Decoder<I>, I>(codec: &mut D, input: I) -> Candidate<D::Value, D::Checks>
where
    D::Error: core::fmt::Debug,
{
    match codec.decode(input, &mut NoopObserver).unwrap() {
        DecodeOutcome::Candidate(result) => result,
        _ => panic!("expected candidate"),
    }
}

#[test]
fn same_encoder_two_decoders_explicit_llr_sign_and_message_order() {
    let layout = block::Layout::try_new([2, 0]).unwrap();
    for message in [[0, 0], [0, 1], [1, 0], [1, 1]] {
        let word = encode(&mut block::Block(layout), message.as_slice());
        let hard = candidate(&mut block::Hard, word.0.as_slice());
        assert_eq!(hard.checks, block::Checks::ParityPassed);
        assert_eq!(layout.extract(hard.value), Message(message));
        for convention in [
            LlrConvention::PositiveForZero,
            LlrConvention::PositiveForOne,
        ] {
            let model = block::Model {
                layout,
                convention,
                gain: 1.0,
            };
            let mut decoder = block::Soft::try_new(layout, convention, model, 4).unwrap();
            let mut values = word.0.map(|b| if b == 0 { 10.0 } else { -10.0 });
            values[0] *= -0.01; // Один бит имеет слабую ошибочную оценку, остальные надёжны.
            let values = values.map(|v| LlrConvention::PositiveForZero.convert(v, convention));
            let soft = candidate(
                &mut decoder,
                block::SoftInput {
                    layout,
                    convention,
                    values: &values,
                    erased: &[false; 3],
                },
            );
            assert_eq!(soft.checks, block::Checks::ParityPassed);
            assert_eq!(layout.extract(soft.value.word), Message(message));
            assert_eq!(
                soft.value.scores[0],
                convention.convert(values[0], LlrConvention::PositiveForZero)
            );
        }
    }
    assert_eq!(
        encode(&mut block::Block(layout), [1, 0].as_slice()),
        Codeword([0, 1, 1])
    );
}

#[test]
fn candidate_parity_is_not_proof_of_original() {
    let layout = block::Layout::try_new([2, 0]).unwrap();
    let original = [1, 0];
    let mut word = encode(&mut block::Block(layout), original.as_slice());
    word.0[0] ^= 1;
    word.0[1] ^= 1; // Две инверсии превращают слово в другое допустимое слово.
    let decoded = candidate(&mut block::Hard, word.0.as_slice());
    assert_eq!(decoded.checks, block::Checks::ParityPassed);
    assert_ne!(layout.extract(decoded.value), Message(original));
}

#[test]
fn model_context_numeric_policy_erasures_and_stopping() {
    use block::{Error, Layout, Model, Soft, SoftInput, Stop};
    let layout = Layout::try_new([2, 0]).unwrap();
    let other = Layout::try_new([0, 2]).unwrap();
    let convention = LlrConvention::PositiveForZero;
    let model = Model {
        layout,
        convention,
        gain: 1.0,
    };
    assert!(matches!(Layout::try_new([0, 0]), Err(Error::Context)));
    assert!(matches!(Layout::try_new([0, 3]), Err(Error::Context)));
    assert!(matches!(
        Soft::try_new(other, convention, model, 4),
        Err(Error::Context)
    ));
    assert!(matches!(
        Soft::try_new(layout, LlrConvention::PositiveForOne, model, 4),
        Err(Error::Context)
    ));
    assert!(matches!(
        Soft::try_new(
            layout,
            convention,
            Model {
                gain: f64::NAN,
                ..model
            },
            4
        ),
        Err(Error::Model)
    ));
    let mut decoder = Soft::try_new(layout, convention, model, 4).unwrap();
    fn input(values: &[f64]) -> SoftInput<'_> {
        SoftInput {
            layout: Layout::try_new([2, 0]).unwrap(),
            convention: LlrConvention::PositiveForZero,
            values,
            erased: &[false; 3],
        }
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            decoder.decode(input(&[bad, 1.0, 1.0]), &mut NoopObserver),
            Err(Error::Value)
        );
    }
    assert_eq!(
        decoder.decode(input(&[1.0]), &mut NoopObserver),
        Err(Error::Shape)
    );
    assert_eq!(
        decoder.decode(
            SoftInput {
                layout: other,
                ..input(&[1.0; 3])
            },
            &mut NoopObserver
        ),
        Err(Error::Context)
    );
    assert_eq!(
        decoder.decode(
            SoftInput {
                convention: LlrConvention::PositiveForOne,
                ..input(&[1.0; 3])
            },
            &mut NoopObserver
        ),
        Err(Error::Context)
    );
    assert_eq!(
        decoder.decode(input(&[-0.0, 0.0, -0.0]), &mut NoopObserver),
        Ok(DecodeOutcome::Stopped(Stop::Tie))
    );
    assert_eq!(
        decoder.decode(
            SoftInput {
                erased: &[true; 3],
                ..input(&[2.0; 3])
            },
            &mut NoopObserver
        ),
        Ok(DecodeOutcome::Stopped(Stop::Tie))
    );
    let mut limited = Soft::try_new(layout, convention, model, 0).unwrap();
    assert_eq!(
        limited.decode(input(&[1.0; 3]), &mut NoopObserver),
        Ok(DecodeOutcome::Stopped(Stop::Budget))
    );
    assert_eq!(
        block::Hard.decode(&[0, 0][..], &mut NoopObserver),
        Err(Error::Shape)
    );
    assert_eq!(
        block::Hard.decode(&[2, 0, 0][..], &mut NoopObserver),
        Err(Error::Value)
    );
    assert_eq!(
        block::Hard.decode(&[1, 0, 0][..], &mut NoopObserver),
        Ok(DecodeOutcome::Stopped(Stop::ParityFailed))
    );
    assert_eq!(
        block::Block(layout).encode(&[2, 0][..], &mut NoopObserver),
        Err(Error::Value)
    );
}

#[test]
fn transform_input_and_nonlinear_codebook_need_no_field_or_matrix() {
    let transform = candidate(&mut block::Transform, [1, 0, 0]);
    assert_eq!(transform.checks, block::Checks::FrozenPassed);
    assert_eq!(
        Message([transform.value.0[2], transform.value.0[0]]),
        Message([0, 1])
    );
    assert!(!block::TABLE.contains(&[0, 0, 0]));
    for message in 0..4 {
        let word = encode(&mut block::Nonlinear, message);
        let result = candidate(&mut block::Nonlinear, word);
        assert_eq!(result.value, Message(message));
        assert_eq!(result.checks, block::Checks::TableMember);
    }
    assert_eq!(
        block::Nonlinear.encode(4, &mut NoopObserver),
        Err(block::Error::Value)
    );
}

fn session() -> (stream::Context, stream::Session) {
    let context = stream::Context::try_new(7, 2, 1).unwrap();
    (
        context,
        encode(&mut stream::Prepare(context), [42, 0].as_slice()),
    )
}

#[test]
fn stream_independence_duplicates_late_verification_and_ordered_events() {
    use stream::{Event, Kind, Progress};
    let (context, encoder) = session();
    let mut decoder = stream::Stream::try_new(context, 8).unwrap();
    let mut events = Vec::new();
    let first = encoder.symbol(0, &mut NoopObserver).unwrap();
    assert_eq!(
        decoder
            .push(&first, &mut |e: &Event| events.push(*e))
            .unwrap()
            .kind,
        Kind::Independent
    );
    assert_eq!(
        decoder
            .push(&first, &mut |e: &Event| events.push(*e))
            .unwrap()
            .kind,
        Kind::Duplicate
    );
    let dependent = encoder.symbol(4, &mut NoopObserver).unwrap();
    assert_eq!(
        decoder
            .push(&dependent, &mut |e: &Event| events.push(*e))
            .unwrap()
            .kind,
        Kind::Dependent
    );
    assert_eq!(
        decoder.progress(),
        Progress::NeedMore { rank: 1, unique: 2 }
    );
    assert!(decoder.result().is_none());
    let second = encoder.symbol(2, &mut NoopObserver).unwrap();
    let update = decoder
        .push(&second, &mut |e: &Event| events.push(*e))
        .unwrap();
    assert_eq!(update.rank, 2);
    assert_eq!(update.unique, 3);
    assert_eq!(decoder.result().unwrap().value.0.as_slice(), &[42, 0]);
    let late = encoder.symbol(u64::MAX, &mut NoopObserver).unwrap();
    assert_eq!(late, encoder.symbol(u64::MAX, &mut NoopObserver).unwrap());
    assert_eq!(
        decoder
            .push(&late, &mut |e: &Event| events.push(*e))
            .unwrap()
            .kind,
        Kind::Verified
    );
    assert_eq!(
        events,
        [
            Event::Accepted(0),
            Event::RankIncreased(1),
            Event::Duplicate(0),
            Event::Accepted(4),
            Event::Dependent(4),
            Event::Accepted(2),
            Event::RankIncreased(2),
            Event::Completed,
            Event::Accepted(u64::MAX),
            Event::Verified(u64::MAX)
        ]
    );
    let mut silent = stream::Stream::try_new(context, 8).unwrap();
    for symbol in [&first, &first, &dependent, &second, &late] {
        silent.push(symbol, &mut NoopObserver).unwrap();
    }
    assert_eq!(decoder, silent);
}

#[test]
fn stream_misuse_preserves_state_but_new_contradictions_invalidate() {
    use stream::{Error, Event, Progress, Symbol};
    let (context, encoder) = session();
    let mut decoder = stream::Stream::try_new(context, 8).unwrap();
    let first = encoder.symbol(0, &mut NoopObserver).unwrap();
    decoder.push(&first, &mut NoopObserver).unwrap();
    let before = decoder.clone();
    let mut events = Vec::new();
    let wrong = Symbol {
        context: stream::Context::try_new(7, 2, 2).unwrap(),
        ..first
    }; // ID блока совпадает, но полный контекст отличается.
    assert_eq!(
        decoder.push(&wrong, &mut |e: &Event| events.push(*e)),
        Err(Error::Context)
    );
    assert_eq!(
        decoder.push(&Symbol { value: 99, ..first }, &mut |e: &Event| events
            .push(*e)),
        Err(Error::Conflict)
    );
    assert_eq!(decoder, before);
    assert!(events.is_empty());
    let mut limited = stream::Stream::try_new(context, 1).unwrap();
    limited.push(&first, &mut NoopObserver).unwrap();
    let before_limit = limited.clone();
    assert_eq!(
        limited.push(
            &encoder.symbol(1, &mut NoopObserver).unwrap(),
            &mut NoopObserver
        ),
        Err(Error::Limit)
    );
    assert_eq!(limited, before_limit);
    // Противоречивое уравнение с новым ID делает накопленные данные непригодными.
    assert_eq!(
        decoder.push(
            &Symbol {
                id: 4,
                value: 99,
                ..first
            },
            &mut |e: &Event| events.push(*e)
        ),
        Err(Error::Inconsistent)
    );
    assert_eq!(events, [Event::Invalidated]);
    assert_eq!(decoder.progress(), Progress::Invalid);
    assert!(decoder.result().is_none());
    assert_eq!(decoder.push(&first, &mut NoopObserver), Err(Error::Invalid));
    // Позднее противоречие отменяет ранее полученный результат.
    let mut complete = stream::Stream::try_new(context, 8).unwrap();
    for id in [1, 0] {
        complete
            .push(
                &encoder.symbol(id, &mut NoopObserver).unwrap(),
                &mut NoopObserver,
            )
            .unwrap();
    }
    let retained_copy = *complete.result().unwrap();
    assert_eq!(
        complete.push(
            &Symbol {
                id: 4,
                value: 99,
                ..first
            },
            &mut NoopObserver
        ),
        Err(Error::Inconsistent)
    );
    assert!(complete.result().is_none());
    assert_eq!(retained_copy.value.0.as_slice(), &[42, 0]);
}

#[test]
fn stream_empty_padding_batch_and_new_block_lifecycle() {
    use stream::{Context, Error, Progress, Symbol};
    let context = Context::try_new(8, 0, 1).unwrap();
    let encoder = encode(&mut stream::Prepare(context), &[][..]);
    assert_eq!(encoder.symbol(0, &mut NoopObserver), Err(Error::Empty));
    let mut empty = stream::Stream::try_new(context, 8).unwrap();
    assert_eq!(empty.progress(), Progress::Complete);
    assert_eq!(empty.result().unwrap().value.0.as_slice(), &[]);
    assert_eq!(
        empty.push(
            &Symbol {
                context,
                id: 0,
                value: 0
            },
            &mut NoopObserver
        ),
        Err(Error::Empty)
    );
    let short = Context::try_new(8, 1, 1).unwrap();
    let mut decoder = stream::Stream::try_new(short, 8).unwrap();
    let before = decoder.clone();
    assert_eq!(
        decoder.push(
            &Symbol {
                context: short,
                id: 1,
                value: 7
            },
            &mut NoopObserver
        ),
        Err(Error::Padding)
    );
    assert_eq!(decoder, before);
    decoder
        .push(
            &Symbol {
                context: short,
                id: 0,
                value: 42,
            },
            &mut NoopObserver,
        )
        .unwrap();
    assert_eq!(
        decoder.push(
            &Symbol {
                context: short,
                id: 2,
                value: 41
            },
            &mut NoopObserver
        ),
        Err(Error::Padding)
    );
    assert_eq!(decoder.progress(), Progress::Invalid);
    let (context, encoder) = session();
    let parts = [
        encoder.symbol(0, &mut NoopObserver).unwrap(),
        encoder.symbol(1, &mut NoopObserver).unwrap(),
    ];
    let mut batch = stream::Batch(context);
    assert!(matches!(
        batch.decode(&parts[..1], &mut NoopObserver),
        Ok(DecodeOutcome::NeedMore(Progress::NeedMore { rank: 1, .. }))
    ));
    assert!(matches!(
        batch.decode(&parts[1..], &mut NoopObserver),
        Ok(DecodeOutcome::NeedMore(Progress::NeedMore { rank: 1, .. }))
    ));
    assert_eq!(
        candidate(&mut batch, parts.as_slice()).value.0.as_slice(),
        &[42, 0]
    );
    // Следующий блок получает отдельное состояние и не принимает части предыдущего.
    let next = Context::try_new(9, 2, 1).unwrap();
    let mut next_decoder = stream::Stream::try_new(next, 8).unwrap();
    assert_eq!(
        next_decoder.push(&parts[0], &mut NoopObserver),
        Err(Error::Context)
    );
}

#[test]
fn local_and_target_specific_helper_repair() {
    use repair::*;
    for mode in [Mode::Local, Mode::Projection] {
        let context = Context { block: 7, mode };
        let protocol = Protocol(context);
        let stored = match mode {
            Mode::Local => [[5, 0], [7, 0], [2, 0]],
            Mode::Projection => [[5, 7], [7, 2], [2, 5]],
        };
        for target in 0..3 {
            let mut events = Vec::new();
            let RepairPlan::Ready(plan) = protocol
                .plan(
                    Request {
                        target,
                        available: &[2, 1, 0, 1],
                    },
                    &mut |e: &Event| events.push(*e),
                )
                .unwrap()
            else {
                panic!("plan expected")
            };
            let responses: Vec<_> = plan
                .requests()
                .iter()
                .map(|request| {
                    Helper {
                        context,
                        id: request.helper,
                        stored: stored[usize::from(request.helper)],
                        online: true,
                    }
                    .respond(request, &mut |e: &Event| events.push(*e))
                    .unwrap()
                })
                .collect();
            let output = protocol
                .repair(
                    Execute {
                        plan: &plan,
                        responses: &responses,
                    },
                    &mut |e: &Event| events.push(*e),
                )
                .unwrap();
            assert_eq!(
                output,
                match mode {
                    Mode::Local => Repaired::Symbol(stored[usize::from(target)][0]),
                    Mode::Projection => Repaired::Node(stored[usize::from(target)]),
                }
            );
            assert_eq!(events.first(), Some(&Event::Selected(target)));
            assert_eq!(events.last(), Some(&Event::Completed(target)));
            assert_eq!(events.len(), 4);
            let mut reversed = responses.clone();
            reversed.reverse();
            assert_eq!(
                protocol
                    .repair(
                        Execute {
                            plan: &plan,
                            responses: &reversed
                        },
                        &mut NoopObserver
                    )
                    .unwrap(),
                output
            );
        }
    }
    let context = Context {
        block: 7,
        mode: Mode::Projection,
    };
    let helper = Helper {
        context,
        id: 1,
        stored: [7, 2],
        online: true,
    };
    let request = HelperRequest {
        context,
        helper: 1,
        target: 0,
    };
    assert_eq!(
        helper.respond(&request, &mut NoopObserver).unwrap().value,
        5
    );
    assert_eq!(
        helper
            .respond(
                &HelperRequest {
                    target: 2,
                    ..request
                },
                &mut NoopObserver
            )
            .unwrap()
            .value,
        7
    );
}

#[test]
fn repair_unavailable_missing_duplicates_wrong_target_and_context() {
    use repair::*;
    let context = Context {
        block: 7,
        mode: Mode::Local,
    };
    let protocol = Protocol(context);
    assert_eq!(
        protocol.plan(
            Request {
                target: 0,
                available: &[0, 1]
            },
            &mut NoopObserver
        ),
        Ok(RepairPlan::Unavailable(MissingHelpers(4)))
    );
    assert_eq!(
        protocol.plan(
            Request {
                target: 3,
                available: &[]
            },
            &mut NoopObserver
        ),
        Err(Error::Target)
    );
    let RepairPlan::Ready(plan) = protocol
        .plan(
            Request {
                target: 0,
                available: &[1, 2],
            },
            &mut NoopObserver,
        )
        .unwrap()
    else {
        panic!("plan")
    };
    let mut helper = Helper {
        context,
        id: 1,
        stored: [7, 0],
        online: false,
    };
    assert_eq!(
        helper.respond(&plan.requests()[0], &mut NoopObserver),
        Err(Error::Unavailable)
    );
    helper.online = true;
    let a = helper
        .respond(&plan.requests()[0], &mut NoopObserver)
        .unwrap();
    let b = Response {
        request: plan.requests()[1],
        value: 2,
    };
    assert_eq!(
        protocol.repair(
            Execute {
                plan: &plan,
                responses: &[a]
            },
            &mut NoopObserver
        ),
        Err(Error::Missing)
    );
    assert_eq!(
        protocol.repair(
            Execute {
                plan: &plan,
                responses: &[a, a]
            },
            &mut NoopObserver
        ),
        Err(Error::Duplicate)
    );
    for (request, expected) in [
        (
            HelperRequest {
                target: 2,
                ..a.request
            },
            Error::Target,
        ),
        (
            HelperRequest {
                helper: 0,
                ..a.request
            },
            Error::Helper,
        ),
        (
            HelperRequest {
                context: Context {
                    block: 8,
                    ..context
                },
                ..a.request
            },
            Error::Context,
        ),
    ] {
        let wrong = Response { request, ..a };
        assert_eq!(
            protocol.repair(
                Execute {
                    plan: &plan,
                    responses: &[wrong, b]
                },
                &mut NoopObserver
            ),
            Err(expected)
        );
    }
    let foreign = Protocol(Context {
        block: 8,
        ..context
    });
    assert_eq!(
        foreign.repair(
            Execute {
                plan: &plan,
                responses: &[a, b]
            },
            &mut NoopObserver
        ),
        Err(Error::Context)
    );
}

#[test]
fn explicit_size_units_and_message_boundaries() {
    for bytes in [0, 1, 4096] {
        assert_eq!(MessageBytes::try_new(bytes).unwrap().get(), bytes);
    }
    assert_eq!(
        MessageBytes::try_new(4097),
        Err(SizeError::MessageTooLong { actual: 4097 })
    );
    assert_eq!(MessageBytes::from_bits(32768).unwrap().get(), 4096);
    assert!(MessageBytes::from_bits(32769).is_err());
    assert!(MessageBytes::from_bits(usize::MAX).is_err());
    // Предел длины сообщения не ограничивает количество частей.
    assert_eq!(
        Size {
            count: 16384,
            unit: SizeUnit::Parts
        }
        .count,
        16384
    );
    let _: &mut dyn Observer<()> = &mut NoopObserver;
}
