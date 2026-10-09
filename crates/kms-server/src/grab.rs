//! DRM frame grab for the privileged helper.
//! Mirrors `kms_get_fb()` in `kms/server/kms_server.c`, including its
//! quirks: per-plane skips, `num_items > 0` forcing `OK`, fd cleanup on
//! failure, cursor-vs-primary coordinates, and the rotation bitmask math.
//! Hardening vs C: item count capped at 8 (C writes past `items[8]` with
//! many planes), GEM handles closed via RAII-style cleanup.

use std::fs::File;
use std::io;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};

use drm::control::{self, Device as ControlDevice};
use drm::Device as DrmDevice;
use drm_fourcc::DrmModifier;

use super::protocol::{Item, KmsResult, Response, Rotation, HDR_LEN, MAX_DMA_BUFS, MAX_ITEMS};

/// Open DRM card handle. Mirrors the `gsr_drm` struct setup.
pub struct Card {
    file: File,
}

impl Card {
    pub fn open(path: &str) -> io::Result<Self> {
        Ok(Card {
            file: File::open(path)?,
        })
    }

    /// Enable `UNIVERSAL_PLANES` (fatal like C); `ATOMIC` is best-effort
    /// with a warning (C: "wrong monitor may be captured").
    pub fn set_client_caps(&self) -> (bool, bool) {
        use drm::ClientCapability;
        let planes = self
            .set_client_capability(ClientCapability::UniversalPlanes, true)
            .is_ok();
        let atomic = self
            .set_client_capability(ClientCapability::Atomic, true)
            .is_ok();
        (planes, atomic)
    }
}

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl DrmDevice for Card {}
impl ControlDevice for Card {}

/// Grabbed frame: response plus owned PRIME fds in item/packet order.
/// Dropping closes every fd (C closes them after a failed send instead).
pub struct Grabbed {
    pub response: Response,
    pub fds: Vec<OwnedFd>,
}

fn rotation_from_bitmask(mask: u64) -> Rotation {
    // Exact mirror of the C accumulation: +90/+180/+270 mod 4.
    let mut rot = 0u32;
    if mask & 2 != 0 {
        rot = (rot + 1) % 4;
    }
    if mask & 4 != 0 {
        rot = (rot + 2) % 4;
    }
    if mask & 8 != 0 {
        rot = (rot + 3) % 4;
    }
    match rot {
        0 => Rotation::Rot0,
        1 => Rotation::Rot90,
        2 => Rotation::Rot180,
        _ => Rotation::Rot270,
    }
}

struct PlaneProps {
    x: i32,
    y: i32,
    src_x: i32,
    src_y: i32,
    src_w: i32,
    src_h: i32,
    is_cursor: bool,
    is_primary: bool,
    rotation: Rotation,
}

/// Read plane properties, mirroring `plane_get_properties` (name + typed
/// value matching instead of raw flag checks; same outcomes).
fn plane_props(card: &Card, plane: control::plane::Handle) -> PlaneProps {
    let mut props = PlaneProps {
        x: 0,
        y: 0,
        src_x: 0,
        src_y: 0,
        src_w: 0,
        src_h: 0,
        is_cursor: false,
        is_primary: false,
        rotation: Rotation::Rot0,
    };
    let values = match card.get_properties(plane) {
        Ok(values) => values,
        Err(_) => return props,
    };
    for (handle, raw) in values.iter() {
        let info = match card.get_property(*handle) {
            Ok(info) => info,
            Err(_) => continue,
        };
        let name = info.name().to_bytes();
        let value = info.value_type().convert_value(*raw);
        // SRC_* are 16.16 fixed point, like C.
        if name == b"CRTC_X" {
            if let Some(v) = value.as_signed_range() {
                props.x = v as i32;
            }
        } else if name == b"CRTC_Y" {
            if let Some(v) = value.as_signed_range() {
                props.y = v as i32;
            }
        } else if name == b"SRC_X" {
            if let Some(v) = value.as_unsigned_range() {
                props.src_x = (v >> 16) as i32;
            }
        } else if name == b"SRC_Y" {
            if let Some(v) = value.as_unsigned_range() {
                props.src_y = (v >> 16) as i32;
            }
        } else if name == b"SRC_W" {
            if let Some(v) = value.as_unsigned_range() {
                props.src_w = (v >> 16) as i32;
            }
        } else if name == b"SRC_H" {
            if let Some(v) = value.as_unsigned_range() {
                props.src_h = (v >> 16) as i32;
            }
        } else if name == b"type" {
            // `convert_value` already resolved the current enum entry.
            if let Some(entry) = value.as_enum() {
                if entry.name().to_bytes() == b"Primary" {
                    props.is_primary = true;
                } else if entry.name().to_bytes() == b"Cursor" {
                    props.is_cursor = true;
                }
            }
        } else if name == b"rotation" {
            if let Some(mask) = value.as_bitmask() {
                props.rotation = rotation_from_bitmask(mask);
            }
        }
    }
    props
}

struct ConnectorInfo {
    connector_id: u32,
    crtc_id: u32,
    hdr_blob: u64,
}

/// Grab one frame from every capturing plane (mirrors `kms_get_fb`).
pub fn grab_frame(card: &Card) -> Grabbed {
    let mut response = Response {
        result: KmsResult::FailedToGetPlanes,
        err_msg: "no capturing planes found".to_string(),
        items: Vec::new(),
    };
    let mut fds: Vec<OwnedFd> = Vec::new();

    let resources = match card.resource_handles() {
        Ok(resources) => resources,
        Err(e) => {
            response.err_msg = format!("failed to get DRM resources: {e}");
            return Grabbed { response, fds };
        }
    };
    // Connector -> CRTC map plus HDR blob ids (no modeset probe, like C).
    let mut connectors: Vec<ConnectorInfo> = Vec::new();
    for handle in &resources.connectors {
        // Existence check without modeset probe (C `GetConnectorCurrent`).
        if card.get_connector(*handle, false).is_err() {
            continue;
        }
        let mut crtc_id = 0u32;
        let mut hdr_blob = 0u64;
        if let Ok(values) = card.get_properties(*handle) {
            for (prop, raw) in values.iter() {
                let prop_info = match card.get_property(*prop) {
                    Ok(prop_info) => prop_info,
                    Err(_) => continue,
                };
                match prop_info.name().to_bytes() {
                    b"CRTC_ID" => crtc_id = *raw as u32,
                    b"HDR_OUTPUT_METADATA" => hdr_blob = *raw,
                    _ => {}
                }
            }
        }
        connectors.push(ConnectorInfo {
            connector_id: (*handle).into(),
            crtc_id,
            hdr_blob,
        });
    }

    let plane_handles = match card.plane_handles() {
        Ok(planes) => planes,
        Err(e) => {
            response.err_msg = format!("failed to list planes: {e}");
            return Grabbed { response, fds };
        }
    };
    for plane_handle in plane_handles {
        if response.items.len() >= MAX_ITEMS {
            break;
        }
        let plane = match card.get_plane(plane_handle) {
            Ok(plane) => plane,
            Err(_) => continue,
        };
        let fb_handle = match plane.framebuffer() {
            Some(fb) => fb,
            None => continue,
        };
        let fb = match card.get_planar_framebuffer(fb_handle) {
            Ok(fb) => fb,
            Err(_) => continue,
        };
        // C: `!handles[0]` fails the whole plane.
        if fb.buffers()[0].is_none() {
            continue;
        }
        let props = plane_props(card, plane_handle);
        if !props.is_primary && !props.is_cursor {
            continue;
        }
        // Export GEM handles to PRIME fds, stopping at the first gap like C.
        let gem_handles: Vec<_> = fb
            .buffers()
            .iter()
            .flatten()
            .take(MAX_DMA_BUFS)
            .copied()
            .collect();
        let mut plane_fds: Vec<OwnedFd> = Vec::new();
        for gem in &gem_handles {
            match card.buffer_to_prime_fd(*gem, 0) {
                Ok(fd) => plane_fds.push(fd),
                Err(_) => break,
            }
        }
        // Close every GEM handle exactly once (C dedups then closes).
        let mut seen: Vec<drm::buffer::Handle> = Vec::new();
        for gem in &gem_handles {
            if !seen.contains(gem) {
                seen.push(*gem);
                let _ = card.close_buffer(*gem);
            }
        }
        if plane_fds.is_empty() {
            continue;
        }

        let plane_crtc: u32 = plane.crtc().map(|crtc| crtc.into()).unwrap_or(0);
        let pair = connectors
            .iter()
            .find(|c| c.crtc_id == plane_crtc && plane_crtc != 0);
        let mut item = Item::default();
        for (slot, _fd) in plane_fds.iter().enumerate() {
            item.dma[slot].pitch = fb.pitches()[slot];
            item.dma[slot].offset = fb.offsets()[slot];
        }
        item.num_dma_bufs = plane_fds.len() as u32;
        let (width, height) = fb.size();
        item.width = width;
        item.height = height;
        item.pixel_format = fb.pixel_format() as u32;
        item.modifier = fb
            .modifier()
            .map(u64::from)
            .unwrap_or_else(|| u64::from(DrmModifier::Invalid));
        item.connector_id = pair.map(|p| p.connector_id).unwrap_or(0);
        item.rotation = props.rotation;
        item.is_cursor = props.is_cursor;
        if props.is_cursor {
            item.x = props.x;
            item.y = props.y;
            item.src_w = 0;
            item.src_h = 0;
        } else {
            item.x = props.src_x;
            item.y = props.src_y;
            item.src_w = props.src_w;
            item.src_h = props.src_h;
        }
        if let Some(pair) = pair {
            if pair.hdr_blob != 0 {
                if let Ok(blob) = card.get_property_blob(pair.hdr_blob) {
                    if blob.len() >= HDR_LEN {
                        item.hdr.copy_from_slice(&blob[..HDR_LEN]);
                        item.has_hdr = true;
                    }
                }
            }
        }
        fds.extend(plane_fds);
        response.items.push(item);
    }

    // C masks per-plane errors once anything was captured.
    if !response.items.is_empty() {
        response.result = KmsResult::Ok;
        response.err_msg.clear();
    }
    Grabbed { response, fds }
}
