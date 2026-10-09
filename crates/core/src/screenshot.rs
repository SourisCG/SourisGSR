//! Screenshots. Mirrors `src/image_writer.c` (JPEG/PNG from RGBA8 pixels).
//!
//! DEVIATION (implementation detail, same formats): pure-Rust `image`
//! writers instead of `stb_image_write.h`. Quality is clamped to 1..=100
//! like C; PNG ignores quality. HDR/10-bit stays a TODO, as in C.

use std::io;
use std::path::Path;

/// Screenshot image format, chosen by output extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
}

/// Pick a format from the output path (`.jpg`/`.jpeg`/`.png`, case-insensitive).
/// `None` means unsupported (T12 only wires JPEG/PNG like C).
pub fn format_for_path(path: &Path) -> Option<ImageFormat> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    {
        Some(ext) if ext == "jpg" || ext == "jpeg" => Some(ImageFormat::Jpeg),
        Some(ext) if ext == "png" => Some(ImageFormat::Png),
        _ => None,
    }
}

/// Clamp JPEG quality to 1..=100 (mirrors C).
pub fn clamp_quality(quality: u8) -> u8 {
    quality.clamp(1, 100)
}

/// Write RGBA8 pixels (`width * height * 4` bytes) to `path`.
/// Format comes from the extension; unknown extensions fail.
/// Errors are technical diagnostics; callers display the localized
/// `err_screenshot_write` message (T20 wiring).
pub fn write_rgba(
    path: &Path,
    width: u32,
    height: u32,
    rgba: &[u8],
    quality: u8,
) -> io::Result<()> {
    let format = format_for_path(path).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported screenshot extension: {}", path.display()),
        )
    })?;
    if width == 0 || height == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "screenshot dimensions must be non-zero",
        ));
    }
    if rgba.len() != width as usize * height as usize * 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "expected {} RGBA bytes, got {}",
                width as usize * height as usize * 4,
                rgba.len()
            ),
        ));
    }
    let result = match format {
        ImageFormat::Jpeg => {
            // The JPEG encoder takes RGB; drop alpha like stb does internally.
            let rgb: Vec<u8> = rgba
                .chunks_exact(4)
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect();
            let file = std::fs::File::create(path)?;
            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(file, clamp_quality(quality));
            encoder
                .encode(&rgb, width, height, image::ExtendedColorType::Rgb8)
                .map_err(io::Error::other)
        }
        ImageFormat::Png => {
            image::save_buffer(path, rgba, width, height, image::ExtendedColorType::Rgba8)
                .map_err(io::Error::other)
        }
    };
    result
}
