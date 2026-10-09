//! KMS protocol v5. Mirrors `kms/kms_shared.h` (v5.10.2).

/// Protocol version pinned to the C header.
pub const PROTOCOL_VERSION: u32 = 5;
/// Maximum items per response (`GSR_KMS_MAX_ITEMS`).
pub const MAX_ITEMS: usize = 8;
/// Maximum DMA-BUFs per item (`GSR_KMS_MAX_DMA_BUFS`).
pub const MAX_DMA_BUFS: usize = 4;
