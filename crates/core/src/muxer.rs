//! FFmpeg muxer timing helpers. Mirrors timestamp/GOP/pacing math in
//! `src/main.cpp` + `src/encoder/encoder.c` (frozen reference v5.10.2).
//!
//! T12 scope: pure timing engine with mocks. Actual `libavformat` writing
//! lands in T40 (FFmpeg 8.x present in this environment); the `ffprobe`
//! parity harness is proven by an ignored test until then.

/// GOP length in frames. Mirrors `codec_context->gop_size = fps * keyint`
/// (`src/main.cpp`); the float-to-int truncation is intentional.
pub fn gop_size(fps: i64, keyint: f64) -> i64 {
    (fps as f64 * keyint) as i64
}

/// Save-seconds adjustment. Mirrors `main.cpp`: positive requests grow by
/// `keyint` (truncated); the full-buffer sentinel passes through, and only
/// it triggers `should_restart` afterwards.
pub fn adjust_save_seconds(seconds: Option<i64>, keyint: f64) -> Option<i64> {
    match seconds {
        Some(s) if s > 0 => Some(s + keyint as i64),
        other => other,
    }
}

/// Rescale a timestamp between clock rates with FFmpeg `av_rescale_q`
/// rounding (`AV_ROUND_NEAR_INF`). Mirrors the `av_packet_rescale_ts` call
/// in `src/encoder/encoder.c`. Uses `i128` internally to avoid overflow;
/// zero rates pass the stamp through (degenerate guard, C would crash).
pub fn rescale_ts(ts: i64, src_rate: i64, dst_rate: i64) -> i64 {
    if src_rate == dst_rate || src_rate == 0 || dst_rate == 0 {
        return ts;
    }
    let negative = (ts < 0) ^ (dst_rate < 0) ^ (src_rate < 0);
    let (a, b, c) = (
        (ts as i128).abs(),
        (dst_rate as i128).abs(),
        (src_rate as i128).abs(),
    );
    let product = a * b;
    let mut result = product / c;
    if product % c * 2 >= c {
        result += 1;
    }
    let result = result as i64;
    if negative {
        -result
    } else {
        result
    }
}

/// CFR frame pacing. Mirrors the end-of-loop math in `src/main.cpp`:
/// next frame boundary minus elapsed, capped at one interval.
/// (`target_fps` there is a frame *interval* despite its name.)
pub fn pace_frame(elapsed_total: f64, frame_interval: f64) -> f64 {
    let frames_elapsed = (elapsed_total / frame_interval).floor();
    let at_next = (frames_elapsed + 1.0) * frame_interval;
    let mut wait = at_next - elapsed_total;
    if wait > frame_interval {
        wait = frame_interval;
    }
    wait
}

/// Monotonicity guard for muxed packet timestamps (new invariant for the
/// T40 write path; VFR repeats are allowed, regressions are not).
#[derive(Debug, Default)]
pub struct TimestampTrack {
    last: Option<i64>,
}

/// Regression error carrying both stamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonMonotonic {
    pub previous: i64,
    pub next: i64,
}

impl TimestampTrack {
    pub fn push(&mut self, ts: i64) -> Result<(), NonMonotonic> {
        if let Some(previous) = self.last {
            if ts < previous {
                return Err(NonMonotonic { previous, next: ts });
            }
        }
        self.last = Some(ts);
        Ok(())
    }
}
