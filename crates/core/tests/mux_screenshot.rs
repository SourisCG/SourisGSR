//! Muxer timing + screenshot tests (T12, no encoder yet).
//! Timing mirrors `src/main.cpp` GOP/pacing/save-seconds math and the
//! `av_packet_rescale_ts` call in `src/encoder/encoder.c`.
//! Screenshots mirror `src/image_writer.c` (JPEG/PNG from RGBA8).

use gsr_core::muxer::{adjust_save_seconds, gop_size, pace_frame, rescale_ts, TimestampTrack};
use gsr_core::screenshot::{clamp_quality, format_for_path, write_rgba, ImageFormat};
use std::path::{Path, PathBuf};

// --- GOP / save seconds -----------------------------------------------------

#[test]
fn gop_size_truncates_like_c() {
    assert_eq!(gop_size(60, 2.0), 120);
    assert_eq!(gop_size(144, 0.5), 72);
    assert_eq!(gop_size(60, 0.33), 19);
    assert_eq!(gop_size(0, 2.0), 0);
}

#[test]
fn save_seconds_adjustment() {
    assert_eq!(adjust_save_seconds(Some(30), 2.0), Some(32));
    // C truncates the float addition: 30 + 0.5 -> 30.
    assert_eq!(adjust_save_seconds(Some(30), 0.5), Some(30));
    assert_eq!(adjust_save_seconds(None, 2.0), None);
    assert_eq!(adjust_save_seconds(Some(0), 2.0), Some(0));
}

// --- Timestamp rescaling -----------------------------------------------------

#[test]
fn rescale_ts_matches_av_rescale_q() {
    assert_eq!(rescale_ts(7, 30, 30), 7);
    // One 30fps frame in the 90kHz clock.
    assert_eq!(rescale_ts(1, 30, 90000), 3000);
    assert_eq!(rescale_ts(3000, 90000, 30), 1);
    // 1001 ticks @30kHz -> ms: 33.366 -> 33.
    assert_eq!(rescale_ts(1001, 30000, 1000), 33);
    // Ties round away from zero.
    assert_eq!(rescale_ts(1, 2, 1), 1);
    assert_eq!(rescale_ts(-3, 2, 1), -2);
    assert_eq!(rescale_ts(-1001, 30000, 1000), -33);
    // Degenerate zero rates pass through instead of dividing by zero.
    assert_eq!(rescale_ts(5, 0, 30), 5);
}

// --- CFR pacing ---------------------------------------------------------------

#[test]
fn pace_frame_to_next_boundary() {
    let interval = 1.0 / 60.0;
    let approx = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(approx(pace_frame(0.0, interval), interval));
    assert!(approx(
        pace_frame(interval * 0.25, interval),
        interval * 0.75
    ));
    assert!(approx(pace_frame(interval, interval), interval));
    assert!(pace_frame(3.7, interval) <= interval);
}

// --- Monotonicity guard ----------------------------------------------------------

#[test]
fn timestamp_track_rejects_regressions() {
    let mut track = TimestampTrack::default();
    assert!(track.push(100).is_ok());
    assert!(track.push(100).is_ok());
    assert!(track.push(250).is_ok());
    let err = track.push(200).unwrap_err();
    assert_eq!((err.previous, err.next), (250, 200));
}

// --- Screenshots -------------------------------------------------------------------

fn gradient_rgba(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            out.extend_from_slice(&[(x * 32) as u8, (y * 32) as u8, 128, 255]);
        }
    }
    out
}

fn tmp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("gsr-t12-{}-{name}", std::process::id()));
    p
}

#[test]
fn format_from_extension() {
    assert_eq!(format_for_path(Path::new("a.jpg")), Some(ImageFormat::Jpeg));
    assert_eq!(
        format_for_path(Path::new("a.JPEG")),
        Some(ImageFormat::Jpeg)
    );
    assert_eq!(format_for_path(Path::new("a.png")), Some(ImageFormat::Png));
    assert_eq!(format_for_path(Path::new("a.PNG")), Some(ImageFormat::Png));
    assert_eq!(format_for_path(Path::new("a.bmp")), None);
    assert_eq!(format_for_path(Path::new("a.mp4")), None);
    assert_eq!(format_for_path(Path::new("noext")), None);
}

#[test]
fn quality_clamped_like_c() {
    assert_eq!(clamp_quality(0), 1);
    assert_eq!(clamp_quality(1), 1);
    assert_eq!(clamp_quality(100), 100);
    assert_eq!(clamp_quality(200), 100);
}

#[test]
fn png_roundtrip_is_lossless() {
    let path = tmp_path("shot.png");
    let rgba = gradient_rgba(8, 6);
    write_rgba(&path, 8, 6, &rgba, 90).unwrap();
    let img = image::open(&path).unwrap().to_rgba8();
    assert_eq!((img.width(), img.height()), (8, 6));
    assert_eq!(img.into_raw(), rgba);
    std::fs::remove_file(&path).ok();
}

#[test]
fn jpeg_roundtrip_keeps_dimensions() {
    let path = tmp_path("shot.jpg");
    let rgba = gradient_rgba(16, 16);
    write_rgba(&path, 16, 16, &rgba, 90).unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() > 100);
    let img = image::open(&path).unwrap().to_rgba8();
    assert_eq!((img.width(), img.height()), (16, 16));
    std::fs::remove_file(&path).ok();
}

#[test]
fn screenshot_rejects_bad_input() {
    let rgba = gradient_rgba(4, 4);
    assert!(write_rgba(&tmp_path("x.bmp"), 4, 4, &rgba, 90).is_err());
    assert!(write_rgba(&tmp_path("x.png"), 4, 4, &rgba[..10], 90).is_err());
    assert!(write_rgba(&tmp_path("x.png"), 0, 4, &[], 90).is_err());
}

// --- ffprobe harness (proven now, used for parity in T40) --------------------------
#[test]
#[ignore = "needs T40 encoder output; proves the ffprobe harness meanwhile"]
fn ffprobe_harness_proven_for_t40() {
    let dir = std::env::temp_dir().join(format!("gsr-t12-ffprobe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let clip = dir.join("fixture.mp4");
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x240:rate=30:duration=1",
            "-pix_fmt",
            "yuv420p",
            "-c:v",
            "mpeg4",
        ])
        .arg(&clip)
        .output()
        .expect("ffmpeg binary required for the parity harness");
    assert!(status.status.success(), "ffmpeg fixture failed: {status:?}");
    let probe = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_name,width,height",
            "-of",
            "csv",
        ])
        .arg(&clip)
        .output()
        .expect("ffprobe binary required for the parity harness");
    let text = String::from_utf8_lossy(&probe.stdout);
    assert!(
        text.contains("mpeg4") && text.contains("320") && text.contains("240"),
        "{text}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
