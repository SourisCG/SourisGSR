//! KMS wire protocol (concept v5 from `kms/kms_shared.h`).
//!
//! DEVIATION-WIRE: instead of shipping raw C structs (native layout,
//! `int`/`bool` ABI coupling), both ends speak a packed little-endian
//! encoding with identical field order and limits. The helper and the
//! client are always ours (same machine), so no C interop is needed and
//! decoding is bounds-checked by construction. Protocol concept version
//! stays 5; either side rejects anything else and drops received fds.
//!
//! Layout (all integers little-endian). Request: `version u32` + `type
//! u32` (8 bytes; the fd travels ancillary). Response: `version u32` +
//! `result u32` + `err_msg [u8; 128]` (UTF-8, NUL-padded) + `num_items
//! u32` + 8 items of 112 bytes: 4 `(pitch u32, offset u32)` +
//! `num_dma_bufs u32` + `width, height, pixel_format u32` + `modifier u64` +
//! `connector_id u32` + `is_cursor, has_hdr, rotation, _pad u8` + `x, y,
//! src_w, src_h i32` + `hdr [u8; 32]` (raw `struct hdr_output_metadata`
//! from `drm_mode.h`: `metadata_type u32` at [0..4], `eotf u8` at [4];
//! total 32 bytes).
//!
//! Total response: 1036 bytes.
//!
//! DMA-BUF fds travel via `SCM_RIGHTS` in item/packet order (see
//! `transport`), never in-band.

/// Protocol concept version (mirrors `GSR_KMS_PROTOCOL_VERSION`).
pub const PROTOCOL_VERSION: u32 = 5;
/// Maximum items per response (`GSR_KMS_MAX_ITEMS`).
pub const MAX_ITEMS: usize = 8;
/// Maximum DMA-BUFs per item (`GSR_KMS_MAX_DMA_BUFS`).
pub const MAX_DMA_BUFS: usize = 4;
/// Maximum ancillary fds per response (`8 * 4`).
pub const MAX_FDS: usize = MAX_ITEMS * MAX_DMA_BUFS;
/// Error message buffer (`char err_msg[128]`).
pub const ERR_MSG_LEN: usize = 128;
/// Raw `struct hdr_output_metadata` size (`drm_mode.h`).
pub const HDR_LEN: usize = 32;
/// Encoded request length in bytes.
pub const REQUEST_LEN: usize = 8;
/// Encoded response length in bytes.
pub const RESPONSE_LEN: usize = 4 + 4 + ERR_MSG_LEN + 4 + MAX_ITEMS * ITEM_LEN;
/// Encoded item length in bytes.
pub const ITEM_LEN: usize = 4 + MAX_DMA_BUFS * 8 + 12 + 8 + 4 + 4 + 16 + HDR_LEN;

/// Request type (`gsr_kms_request_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RequestType {
    ReplaceConnection = 0,
    GetKms = 1,
}

/// Result code (`gsr_kms_result`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum KmsResult {
    Ok = 0,
    InvalidRequest = 1,
    FailedToGetPlane = 2,
    FailedToGetPlanes = 3,
    FailedToSend = 4,
}

/// Plane rotation (`gsr_kms_rotation`: 0/90/180/270).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Rotation {
    Rot0 = 0,
    Rot90 = 1,
    Rot180 = 2,
    Rot270 = 3,
}

/// One DMA-BUF plane descriptor. The fd itself travels via `SCM_RIGHTS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DmaBuf {
    pub pitch: u32,
    pub offset: u32,
}

/// One captured plane (monitor or cursor). Mirrors `gsr_kms_response_item`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub dma: [DmaBuf; MAX_DMA_BUFS],
    pub num_dma_bufs: u32,
    pub width: u32,
    pub height: u32,
    pub pixel_format: u32,
    pub modifier: u64,
    pub connector_id: u32,
    pub is_cursor: bool,
    pub has_hdr: bool,
    pub rotation: Rotation,
    pub x: i32,
    pub y: i32,
    pub src_w: i32,
    pub src_h: i32,
    /// Raw `struct hdr_output_metadata` (`drm_mode.h`); valid iff `has_hdr`.
    pub hdr: [u8; HDR_LEN],
}

impl Default for Item {
    fn default() -> Self {
        Item {
            dma: [DmaBuf::default(); MAX_DMA_BUFS],
            num_dma_bufs: 0,
            width: 0,
            height: 0,
            pixel_format: 0,
            modifier: 0,
            connector_id: 0,
            is_cursor: false,
            has_hdr: false,
            rotation: Rotation::Rot0,
            x: 0,
            y: 0,
            src_w: 0,
            src_h: 0,
            hdr: [0; HDR_LEN],
        }
    }
}

impl Item {
    /// `metadata_type` at blob bytes [0..4] (`drm_mode.h`).
    pub fn hdr_metadata_type(&self) -> u32 {
        u32::from_le_bytes([self.hdr[0], self.hdr[1], self.hdr[2], self.hdr[3]])
    }

    /// `eotf` at blob byte [4] (`drm_mode.h` hdmi static metadata).
    pub fn hdr_eotf(&self) -> u8 {
        self.hdr[4]
    }
}

/// Full response. Mirrors `gsr_kms_response`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub result: KmsResult,
    pub err_msg: String,
    pub items: Vec<Item>,
}

/// Wire decode failure (fail-closed; the C code trusts peer counts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Truncated { expected: usize, got: usize },
    Trailing { trailing: usize },
    BadVersion(u32),
    BadRequestType(u32),
    BadResult(u32),
    BadRotation(u8),
    TooManyItems(u32),
    TooManyDmaBufs(u32),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { expected, got } => {
                write!(f, "truncated message: expected {expected} bytes, got {got}")
            }
            Self::Trailing { trailing } => write!(f, "{trailing} unexpected trailing bytes"),
            Self::BadVersion(v) => write!(f, "unsupported protocol version {v}"),
            Self::BadRequestType(t) => write!(f, "unknown request type {t}"),
            Self::BadResult(r) => write!(f, "unknown result code {r}"),
            Self::BadRotation(r) => write!(f, "unknown rotation {r}"),
            Self::TooManyItems(n) => write!(f, "too many items: {n}"),
            Self::TooManyDmaBufs(n) => write!(f, "too many DMA-BUFs: {n}"),
        }
    }
}

impl std::error::Error for DecodeError {}

fn get_u32(input: &[u8]) -> Result<(u32, &[u8]), DecodeError> {
    if input.len() < 4 {
        return Err(DecodeError::Truncated {
            expected: 4,
            got: input.len(),
        });
    }
    Ok((
        u32::from_le_bytes([input[0], input[1], input[2], input[3]]),
        &input[4..],
    ))
}

fn get_u64(input: &[u8]) -> Result<(u64, &[u8]), DecodeError> {
    if input.len() < 8 {
        return Err(DecodeError::Truncated {
            expected: 8,
            got: input.len(),
        });
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&input[..8]);
    Ok((u64::from_le_bytes(bytes), &input[8..]))
}

fn get_i32(input: &[u8]) -> Result<(i32, &[u8]), DecodeError> {
    let (value, rest) = get_u32(input)?;
    Ok((value as i32, rest))
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_i32(out: &mut Vec<u8>, value: i32) {
    put_u32(out, value as u32);
}

fn decode_request_type(value: u32) -> Result<RequestType, DecodeError> {
    match value {
        0 => Ok(RequestType::ReplaceConnection),
        1 => Ok(RequestType::GetKms),
        other => Err(DecodeError::BadRequestType(other)),
    }
}

fn decode_result(value: u32) -> Result<KmsResult, DecodeError> {
    match value {
        0 => Ok(KmsResult::Ok),
        1 => Ok(KmsResult::InvalidRequest),
        2 => Ok(KmsResult::FailedToGetPlane),
        3 => Ok(KmsResult::FailedToGetPlanes),
        4 => Ok(KmsResult::FailedToSend),
        other => Err(DecodeError::BadResult(other)),
    }
}

fn decode_rotation(value: u8) -> Result<Rotation, DecodeError> {
    match value {
        0 => Ok(Rotation::Rot0),
        1 => Ok(Rotation::Rot90),
        2 => Ok(Rotation::Rot180),
        3 => Ok(Rotation::Rot270),
        other => Err(DecodeError::BadRotation(other)),
    }
}

/// Truncate UTF-8 to fit `ERR_MSG_LEN` bytes on a char boundary.
fn truncate_msg(message: &str) -> [u8; ERR_MSG_LEN] {
    let mut out = [0u8; ERR_MSG_LEN];
    let mut len = message.len().min(ERR_MSG_LEN);
    while len > 0 && !message.is_char_boundary(len) {
        len -= 1;
    }
    out[..len].copy_from_slice(&message.as_bytes()[..len]);
    out
}

fn decode_msg(raw: &[u8; ERR_MSG_LEN]) -> String {
    let end = raw.iter().position(|b| *b == 0).unwrap_or(ERR_MSG_LEN);
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

/// Encode a request (8 bytes).
pub fn encode_request(kind: RequestType) -> Vec<u8> {
    let mut out = Vec::with_capacity(REQUEST_LEN);
    put_u32(&mut out, PROTOCOL_VERSION);
    put_u32(&mut out, kind as u32);
    out
}

/// Decode a request (strict: exactly 8 bytes).
pub fn decode_request(input: &[u8]) -> Result<RequestType, DecodeError> {
    if input.len() < REQUEST_LEN {
        return Err(DecodeError::Truncated {
            expected: REQUEST_LEN,
            got: input.len(),
        });
    }
    if input.len() > REQUEST_LEN {
        return Err(DecodeError::Trailing {
            trailing: input.len() - REQUEST_LEN,
        });
    }
    let (version, rest) = get_u32(input)?;
    if version != PROTOCOL_VERSION {
        return Err(DecodeError::BadVersion(version));
    }
    let (kind, _) = get_u32(rest)?;
    decode_request_type(kind)
}

fn encode_item(item: &Item, out: &mut Vec<u8>) {
    for plane in &item.dma {
        put_u32(out, plane.pitch);
        put_u32(out, plane.offset);
    }
    put_u32(out, item.num_dma_bufs);
    put_u32(out, item.width);
    put_u32(out, item.height);
    put_u32(out, item.pixel_format);
    put_u64(out, item.modifier);
    put_u32(out, item.connector_id);
    out.push(item.is_cursor as u8);
    out.push(item.has_hdr as u8);
    out.push(item.rotation as u8);
    out.push(0);
    put_i32(out, item.x);
    put_i32(out, item.y);
    put_i32(out, item.src_w);
    put_i32(out, item.src_h);
    out.extend_from_slice(&item.hdr);
}

fn decode_item(input: &[u8]) -> Result<(Item, &[u8]), DecodeError> {
    let mut rest = input;
    let mut dma = [DmaBuf::default(); MAX_DMA_BUFS];
    for plane in dma.iter_mut() {
        let (pitch, next) = get_u32(rest)?;
        let (offset, next) = get_u32(next)?;
        plane.pitch = pitch;
        plane.offset = offset;
        rest = next;
    }
    let (num_dma_bufs, next) = get_u32(rest)?;
    rest = next;
    if num_dma_bufs as usize > MAX_DMA_BUFS {
        return Err(DecodeError::TooManyDmaBufs(num_dma_bufs));
    }
    let (width, next) = get_u32(rest)?;
    let (height, next) = get_u32(next)?;
    let (pixel_format, next) = get_u32(next)?;
    let (modifier, next) = get_u64(next)?;
    let (connector_id, next) = get_u32(next)?;
    rest = next;
    if rest.len() < 4 + 16 + HDR_LEN {
        return Err(DecodeError::Truncated {
            expected: 4 + 16 + HDR_LEN,
            got: rest.len(),
        });
    }
    let is_cursor = rest[0] != 0;
    let has_hdr = rest[1] != 0;
    let rotation = decode_rotation(rest[2])?;
    let (x, next) = get_i32(&rest[4..])?;
    let (y, next) = get_i32(next)?;
    let (src_w, next) = get_i32(next)?;
    let (src_h, next) = get_i32(next)?;
    let mut hdr = [0u8; HDR_LEN];
    hdr.copy_from_slice(&next[..HDR_LEN]);
    Ok((
        Item {
            dma,
            num_dma_bufs,
            width,
            height,
            pixel_format,
            modifier,
            connector_id,
            is_cursor,
            has_hdr,
            rotation,
            x,
            y,
            src_w,
            src_h,
            hdr,
        },
        &next[HDR_LEN..],
    ))
}

/// Encode a response (exactly 1036 bytes; at most 8 items).
/// Extra items are a caller bug: returns `None` instead of truncating.
pub fn encode_response(response: &Response) -> Option<Vec<u8>> {
    if response.items.len() > MAX_ITEMS {
        return None;
    }
    let mut out = Vec::with_capacity(RESPONSE_LEN);
    put_u32(&mut out, PROTOCOL_VERSION);
    put_u32(&mut out, response.result as u32);
    out.extend_from_slice(&truncate_msg(&response.err_msg));
    put_u32(&mut out, response.items.len() as u32);
    for item in &response.items {
        encode_item(item, &mut out);
    }
    for _ in response.items.len()..MAX_ITEMS {
        encode_item(&Item::default(), &mut out);
    }
    debug_assert_eq!(out.len(), RESPONSE_LEN);
    Some(out)
}

/// Decode a response (strict: exactly 1036 bytes).
pub fn decode_response(input: &[u8]) -> Result<Response, DecodeError> {
    if input.len() < RESPONSE_LEN {
        return Err(DecodeError::Truncated {
            expected: RESPONSE_LEN,
            got: input.len(),
        });
    }
    if input.len() > RESPONSE_LEN {
        return Err(DecodeError::Trailing {
            trailing: input.len() - RESPONSE_LEN,
        });
    }
    let (version, rest) = get_u32(input)?;
    if version != PROTOCOL_VERSION {
        return Err(DecodeError::BadVersion(version));
    }
    let (result, rest) = get_u32(rest)?;
    let result = decode_result(result)?;
    if rest.len() < ERR_MSG_LEN + 4 {
        return Err(DecodeError::Truncated {
            expected: ERR_MSG_LEN + 4,
            got: rest.len(),
        });
    }
    let mut raw_msg = [0u8; ERR_MSG_LEN];
    raw_msg.copy_from_slice(&rest[..ERR_MSG_LEN]);
    let err_msg = decode_msg(&raw_msg);
    let (num_items, mut rest) = get_u32(&rest[ERR_MSG_LEN..])?;
    if num_items as usize > MAX_ITEMS {
        return Err(DecodeError::TooManyItems(num_items));
    }
    let mut items = Vec::with_capacity(num_items as usize);
    for _ in 0..num_items {
        let (item, next) = decode_item(rest)?;
        items.push(item);
        rest = next;
    }
    Ok(Response {
        result,
        err_msg,
        items,
    })
}
