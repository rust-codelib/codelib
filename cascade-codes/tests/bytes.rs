//! Байтовый слой: сообщения до 4096 байт, блоки, восстановление узла.

mod common;

use cascade_codes::{
    CascadeConfigurator, CascadeError, DecodeStatus, EncodedFile, MAX_MESSAGE_BYTES,
};
use common::pick;
use gf2m::Gf256;
use gfpm::Gf13;

fn text(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 31 + 7) as u8).collect()
}

#[test]
fn round_trip_various_lengths_gf256() {
    let code = CascadeConfigurator::new(8, 4, 6)
        .mode(3)
        .build::<Gf256>()
        .unwrap();
    for len in [
        0,
        1,
        2,
        47,
        code.params().file_size(),
        1000,
        MAX_MESSAGE_BYTES,
    ] {
        let data = text(len);
        let file = code.encode_bytes(&data).unwrap();
        assert_eq!(file.stripes, code.stripes_for(len));
        let shares = pick(&file.shares, &[7, 2, 5, 0]);
        let result = code.decode_bytes(len, &shares, None).unwrap();
        assert_eq!(result.bytes(), &data[..], "длина {len}");
        assert_eq!(result.status(), DecodeStatus::Consistent);
    }
}

#[test]
fn round_trip_odd_field() {
    let code = CascadeConfigurator::new(7, 3, 5).build::<Gf13>().unwrap();
    let data = b"cascade codes over GF(13)".to_vec();
    let file = code.encode_bytes(&data).unwrap();
    let result = code
        .decode_bytes(data.len(), &file.shares[4..], None)
        .unwrap();
    assert_eq!(result.bytes(), &data[..]);
}

#[test]
fn too_long_message_is_rejected() {
    let code = CascadeConfigurator::new(5, 2, 3).build::<Gf256>().unwrap();
    assert_eq!(
        code.encode_bytes(&vec![0; MAX_MESSAGE_BYTES + 1])
            .unwrap_err(),
        CascadeError::MessageTooLong {
            max: MAX_MESSAGE_BYTES,
            actual: MAX_MESSAGE_BYTES + 1
        }
    );
}

#[test]
fn file_node_repair_restores_all_stripes() {
    let code = CascadeConfigurator::new(8, 4, 6)
        .mode(2)
        .build::<Gf256>()
        .unwrap();
    let data = text(500);
    let EncodedFile {
        stripes, shares, ..
    } = code.encode_bytes(&data).unwrap();
    assert!(stripes > 1);
    let payloads: Vec<_> = [0, 1, 2, 4, 5, 6]
        .iter()
        .map(|&h| code.file_repair_payload(&shares[h], 3).unwrap())
        .collect();
    assert!(payloads
        .iter()
        .all(|p| p.symbols.len() == stripes * code.params().beta()));
    let repaired = code.file_repair(3, stripes, &payloads, None).unwrap();
    assert_eq!(repaired, shares[3]);
}

#[test]
fn corrupted_node_is_located_in_file() {
    let code = CascadeConfigurator::new(8, 4, 6)
        .mode(3)
        .build::<Gf256>()
        .unwrap();
    let data = text(300);
    let mut file = code.encode_bytes(&data).unwrap();
    file.shares[5].symbols[10] += Gf256::new(1);
    let result = code.decode_bytes(data.len(), &file.shares, None).unwrap();
    assert_eq!(result.bytes(), &data[..]);
    assert_eq!(result.status(), DecodeStatus::Corrected);
    assert_eq!(result.suspect_nodes(), vec![5]);
    assert_eq!(result.mismatched_symbols(), 1);
}

#[test]
fn decode_bytes_checks_share_length() {
    let code = CascadeConfigurator::new(5, 2, 3).build::<Gf256>().unwrap();
    let file = code.encode_bytes(b"abc").unwrap();
    assert!(matches!(
        code.decode_bytes(400, &file.shares, None),
        Err(CascadeError::ShareLengthMismatch { .. })
    ));
}

#[test]
fn file_repair_validates_helpers_even_without_stripes() {
    let code = CascadeConfigurator::new(5, 2, 3).build::<Gf256>().unwrap();
    let file = code.encode_bytes(b"").unwrap();
    assert_eq!(file.stripes, 0);
    let payloads: Vec<_> = [1, 2]
        .iter()
        .map(|&h| code.file_repair_payload(&file.shares[h], 0).unwrap())
        .collect();
    assert_eq!(
        code.file_repair(0, 0, &payloads, None).unwrap_err(),
        CascadeError::WrongHelperCount {
            expected: 3,
            actual: 2
        }
    );
    let payloads: Vec<_> = [1, 2, 3]
        .iter()
        .map(|&h| code.file_repair_payload(&file.shares[h], 0).unwrap())
        .collect();
    assert_eq!(
        code.file_repair(0, 0, &payloads, None).unwrap(),
        file.shares[0]
    );
}
