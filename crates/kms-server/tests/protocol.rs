//! Wire protocol tests: layout, roundtrips and fail-closed decoding.
//! Mirrors `kms/kms_shared.h` field order (concept v5).

use gsr_kms_server::protocol::*;

fn sample_item() -> Item {
    Item {
        dma: [
            DmaBuf {
                pitch: 1920,
                offset: 0,
            },
            DmaBuf {
                pitch: 960,
                offset: 460800,
            },
            DmaBuf::default(),
            DmaBuf::default(),
        ],
        num_dma_bufs: 2,
        width: 1920,
        height: 1080,
        pixel_format: 0x34325258,
        modifier: 0x0100000000000001,
        connector_id: 67,
        is_cursor: false,
        has_hdr: true,
        rotation: Rotation::Rot90,
        x: 1920,
        y: 0,
        src_w: 1920,
        src_h: 1080,
        hdr: {
            let mut hdr = [0u8; HDR_LEN];
            hdr[0..4].copy_from_slice(&1u32.to_le_bytes());
            hdr[4] = 2;
            hdr
        },
    }
}

#[test]
fn layout_constants() {
    assert_eq!(PROTOCOL_VERSION, 5);
    assert_eq!(MAX_ITEMS, 8);
    assert_eq!(MAX_DMA_BUFS, 4);
    assert_eq!(MAX_FDS, 32);
    assert_eq!(ITEM_LEN, 112);
    assert_eq!(REQUEST_LEN, 8);
    assert_eq!(RESPONSE_LEN, 4 + 4 + ERR_MSG_LEN + 4 + MAX_ITEMS * ITEM_LEN);
    assert_eq!(RESPONSE_LEN, 1036);
}

#[test]
fn request_roundtrip() {
    for kind in [RequestType::ReplaceConnection, RequestType::GetKms] {
        let bytes = encode_request(kind);
        assert_eq!(bytes.len(), REQUEST_LEN);
        assert_eq!(decode_request(&bytes), Ok(kind));
    }
}

#[test]
fn request_decode_rejects() {
    assert_eq!(
        decode_request(&[0u8; 3]),
        Err(DecodeError::Truncated {
            expected: REQUEST_LEN,
            got: 3
        })
    );
    assert_eq!(
        decode_request(&[0u8; REQUEST_LEN + 1]),
        Err(DecodeError::Trailing { trailing: 1 })
    );
    let mut bad_version = encode_request(RequestType::GetKms);
    bad_version[0..4].copy_from_slice(&4u32.to_le_bytes());
    assert_eq!(
        decode_request(&bad_version),
        Err(DecodeError::BadVersion(4))
    );
    let mut bad_type = encode_request(RequestType::GetKms);
    bad_type[4..8].copy_from_slice(&9u32.to_le_bytes());
    assert_eq!(
        decode_request(&bad_type),
        Err(DecodeError::BadRequestType(9))
    );
}

#[test]
fn response_roundtrip_preserves_everything() {
    let item = sample_item();
    let response = Response {
        result: KmsResult::Ok,
        err_msg: "all good".to_string(),
        items: vec![
            item.clone(),
            Item {
                is_cursor: true,
                ..Item::default()
            },
        ],
    };
    let bytes = encode_response(&response).expect("fits");
    assert_eq!(bytes.len(), RESPONSE_LEN);
    let back = decode_response(&bytes).expect("decodes");
    assert_eq!(back, response);
    assert_eq!(back.items[0].hdr_metadata_type(), 1);
    assert_eq!(back.items[0].hdr_eotf(), 2);
}

#[test]
fn response_message_truncates_on_char_boundary() {
    let long = "é".repeat(200);
    let response = Response {
        result: KmsResult::FailedToSend,
        err_msg: long,
        items: vec![],
    };
    let bytes = encode_response(&response).expect("fits");
    let back = decode_response(&bytes).expect("decodes");
    assert!(back.err_msg.len() <= ERR_MSG_LEN);
    assert!(back.err_msg.chars().all(|c| c == 'é'));
    assert_eq!(back.err_msg.chars().count(), ERR_MSG_LEN / 2);
    // NUL termination decodes to the prefix before the first NUL.
    let nulled = Response {
        result: KmsResult::Ok,
        err_msg: "ab\0cd".to_string(),
        items: vec![],
    };
    let bytes = encode_response(&nulled).expect("fits");
    assert_eq!(decode_response(&bytes).expect("decodes").err_msg, "ab");
}

#[test]
fn encode_refuses_overfull_responses() {
    let response = Response {
        result: KmsResult::Ok,
        err_msg: String::new(),
        items: vec![Item::default(); MAX_ITEMS + 1],
    };
    assert!(encode_response(&response).is_none());
}

fn patch_u32(base: &[u8], offset: usize, value: u32) -> Vec<u8> {
    let mut bytes = base.to_vec();
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    bytes
}

#[test]
fn response_decode_rejects() {
    let valid = encode_response(&Response {
        result: KmsResult::Ok,
        err_msg: String::new(),
        items: vec![sample_item()],
    })
    .expect("fits");

    assert_eq!(
        decode_response(&valid[..100]),
        Err(DecodeError::Truncated {
            expected: RESPONSE_LEN,
            got: 100
        })
    );
    let mut trailing = valid.clone();
    trailing.push(0);
    assert_eq!(
        decode_response(&trailing),
        Err(DecodeError::Trailing { trailing: 1 })
    );

    let bad_version = patch_u32(&valid, 0, 4);
    assert_eq!(
        decode_response(&bad_version),
        Err(DecodeError::BadVersion(4))
    );

    let bad_result = patch_u32(&valid, 4, 99);
    assert_eq!(
        decode_response(&bad_result),
        Err(DecodeError::BadResult(99))
    );

    // num_items lives at 4 + 4 + 128.
    let too_many = patch_u32(&valid, 136, 9);
    assert_eq!(
        decode_response(&too_many),
        Err(DecodeError::TooManyItems(9))
    );

    // First item starts at 140; num_dma_bufs follows 4 DMA entries (32 bytes).
    let too_many_planes = patch_u32(&valid, 172, 5);
    assert_eq!(
        decode_response(&too_many_planes),
        Err(DecodeError::TooManyDmaBufs(5))
    );

    // Rotation byte: 140 + 32 + 4 + 12 + 8 + 4 + 2 = 202.
    let mut bad_rotation = valid.clone();
    bad_rotation[202] = 9;
    assert_eq!(
        decode_response(&bad_rotation),
        Err(DecodeError::BadRotation(9))
    );
}
