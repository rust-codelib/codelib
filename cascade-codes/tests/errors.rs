//! Проверка входных данных и сообщений об ошибках.

mod common;

use cascade_codes::{
    CascadeCode, CascadeError, CascadeParams, DataDecoder, Encoder, NodeRepairer, RepairPayload,
};
use common::{code, message};
use gfpm::{Gf13, Gf5};

#[test]
fn field_must_have_at_least_n_elements() {
    let params = CascadeParams::try_new(6, 2, 3, 2).unwrap();
    assert_eq!(
        CascadeCode::<Gf5>::try_new(params).unwrap_err(),
        CascadeError::FieldTooSmall { n: 6, order: 5 }
    );
    let params = CascadeParams::try_new(5, 2, 3, 2).unwrap();
    assert!(CascadeCode::<Gf5>::try_new(params).is_ok());
}

#[test]
fn encode_checks_message_length() {
    let code = code::<Gf13>(6, 3, 4, 2);
    let short = vec![Gf13::zero(); code.message_len() - 1];
    assert_eq!(
        code.encode(&short).unwrap_err(),
        CascadeError::MessageLengthMismatch {
            expected: code.message_len(),
            actual: code.message_len() - 1
        }
    );
    let msg = message::<Gf13>(code.message_len(), 1);
    assert_eq!(
        code.encode_node(&msg, 6).unwrap_err(),
        CascadeError::NodeOutOfRange { node: 6, n: 6 }
    );
}

#[test]
fn decode_checks_shares() {
    let code = code::<Gf13>(6, 3, 4, 2);
    let msg = message::<Gf13>(code.message_len(), 2);
    let nodes = code.encode(&msg).unwrap();

    assert_eq!(
        code.decode(&nodes[..2], None).unwrap_err(),
        CascadeError::NotEnoughShares {
            required: 3,
            actual: 2
        }
    );
    let dup = vec![nodes[0].clone(), nodes[1].clone(), nodes[0].clone()];
    assert_eq!(
        code.decode(&dup, None).unwrap_err(),
        CascadeError::DuplicateNode { node: 0 }
    );

    let mut bad = nodes[..3].to_vec();
    bad[1].node = 9;
    assert_eq!(
        code.decode(&bad, None).unwrap_err(),
        CascadeError::NodeOutOfRange { node: 9, n: 6 }
    );

    let mut short = nodes[..3].to_vec();
    short[2].symbols.pop();
    let alpha = code.node_len();
    assert_eq!(
        code.decode(&short, None).unwrap_err(),
        CascadeError::ShareLengthMismatch {
            node: 2,
            expected: alpha,
            actual: alpha - 1
        }
    );
}

#[test]
fn repair_checks_helpers() {
    let code = code::<Gf13>(6, 3, 4, 2);
    let msg = message::<Gf13>(code.message_len(), 3);
    let nodes = code.encode(&msg).unwrap();
    let payloads: Vec<RepairPayload<Gf13>> = (1..=4)
        .map(|h| code.repair_payload(&nodes[h], 0).unwrap())
        .collect();

    assert_eq!(
        code.repair_payload(&nodes[0], 0).unwrap_err(),
        CascadeError::HelperIsFailedNode { node: 0 }
    );
    assert_eq!(
        code.repair_payload(&nodes[1], 6).unwrap_err(),
        CascadeError::NodeOutOfRange { node: 6, n: 6 }
    );
    assert_eq!(
        code.repair(0, &payloads[..3], None).unwrap_err(),
        CascadeError::WrongHelperCount {
            expected: 4,
            actual: 3
        }
    );
    let mut dup = payloads.clone();
    dup[3] = dup[0].clone();
    assert_eq!(
        code.repair(0, &dup, None).unwrap_err(),
        CascadeError::DuplicateNode { node: 1 }
    );

    let mut with_failed = payloads.clone();
    with_failed[0].helper = 0;
    assert_eq!(
        code.repair(0, &with_failed, None).unwrap_err(),
        CascadeError::HelperIsFailedNode { node: 0 }
    );
    let mut short = payloads.clone();
    short[2].symbols.pop();
    let beta = code.repair_payload_len();
    assert_eq!(
        code.repair(0, &short, None).unwrap_err(),
        CascadeError::ShareLengthMismatch {
            node: 3,
            expected: beta,
            actual: beta - 1
        }
    );
}
