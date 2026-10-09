//! Replay facade: packets, save planning, file names, save scripts.
//! Mirrors `src/replay_buffer/replay_buffer.c/.h` (interface),
//! `save_replay_async` + `create_new_recording_filepath_from_timestamp` +
//! `run_recording_saved_script_async` in `src/main.cpp` (v5.10.2).
//!
//! Deviations from C (all tested):
//! - `DEVIATION-NOW-PARAM`: `now` is an explicit parameter instead of an
//!   internal monotonic-clock read, for deterministic tests. Production
//!   passes monotonic time (T20+).
//! - `DEVIATION-NO-KEYFRAME`: missing video keyframe yields `None`
//!   (`err_replay_no_keyframe`) instead of a `{(size_t)-1, 0}` sentinel.
//! - `DEVIATION-SNAPSHOT`: save snapshots own their packets; C shares them
//!   by refcount. Bounded, simpler, safe; revisit if profiling demands it.
//! - `DEVIATION-UTC-TIME`: file timestamps use UTC civil time (std has no
//!   localtime); C uses localtime. Revisit with a timezone source.
//! - `DEVIATION-SPAWN`: scripts run via detached `Command::spawn` (plus
//!   `flatpak-spawn --host` under Flatpak); C double-forks with setsid.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Encoded packet held by the replay rings (FFmpeg-agnostic view of `AVPacket`).
#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    pub data: Vec<u8>,
    pub stream_index: i32,
    pub pts: i64,
    pub dts: i64,
    pub key: bool,
    pub timestamp: f64,
}

/// Ring position. `file` is always 0 for RAM; disk uses both fields.
/// Mirrors `gsr_replay_buffer_iterator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    pub packet: usize,
    pub file: usize,
}

/// Read side shared by RAM and disk rings (mirrors the vtable in
/// `gsr_replay_buffer`).
pub trait RingView {
    fn packet_count(&self) -> usize;
    /// Binary search from `save_replay_async` callers; empty ring -> `{0,0}`.
    fn find_by_time(&self, seconds: i64, now: f64) -> Cursor;
    /// Linear keyframe search; `None` = C `{(size_t)-1, 0}`.
    fn find_keyframe(&self, start: Cursor, stream: i32, invert: bool) -> Option<Cursor>;
    fn next(&self, cursor: Cursor) -> Option<Cursor>;
}

/// Save plan: video start plus one audio start per track.
/// Missing audio keyframes resolve to the video start with pts offset 0
/// (mirrors `audio_pts_offset = 0` in `save_replay_async`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavePlan {
    pub video: Cursor,
    pub audio: Vec<(i32, Cursor)>,
}

/// Plan a save: full buffer (`None`, SIGUSR1) or last `seconds` (`Some`).
/// Returns `None` when no video keyframe exists (caller reports
/// `err_replay_no_keyframe` and aborts, as in C).
pub fn plan_save<R: RingView>(
    ring: &R,
    seconds: Option<i64>,
    now: f64,
    video_stream: i32,
    audio_streams: &[i32],
) -> Option<SavePlan> {
    let search = match seconds {
        None => Cursor::default(),
        Some(s) => ring.find_by_time(s, now),
    };
    let video = ring.find_keyframe(search, video_stream, false)?;
    let mut audio = Vec::with_capacity(audio_streams.len());
    for stream in audio_streams {
        let start = ring.find_keyframe(video, *stream, false).unwrap_or(video);
        audio.push((*stream, start));
    }
    Some(SavePlan { video, audio })
}

/// Restart rule: the buffer is cleared only on full saves when the flag is
/// set (mirrors `main.cpp`: `restart_replay_on_save && seconds == full`).
pub fn should_restart(full_save: bool, restart_flag: bool) -> bool {
    full_save && restart_flag
}

/// Days since civil 1970-01-01 from a unix timestamp (Howard Hinnant's
/// algorithm, UTC). See `DEVIATION-UTC-TIME`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Split unix seconds into `(date, time)` strings (`%Y-%m-%d`, `%H-%M-%S`).
pub fn split_timestamp(unix_secs: i64) -> (String, String) {
    let days = unix_secs.div_euclid(86400);
    let secs = unix_secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    (
        format!("{y:04}-{m:02}-{d:02}"),
        format!(
            "{:02}-{:02}-{:02}",
            secs / 3600,
            (secs / 60) % 60,
            secs % 60
        ),
    )
}

/// Build `Replay_<ts>.<ext>` (or date-folder variant).
/// Mirrors `create_new_recording_filepath_from_timestamp`.
pub fn recording_filepath(
    dir: &Path,
    prefix: &str,
    extension: &str,
    date_folders: bool,
    unix_secs: i64,
) -> PathBuf {
    let (date, time) = split_timestamp(unix_secs);
    if date_folders {
        dir.join(date).join(format!("{prefix}_{time}.{extension}"))
    } else {
        dir.join(format!("{prefix}_{date}_{time}.{extension}"))
    }
}

/// Run the `-sc` script detached with `(video_file, kind)` args.
/// `kind` is `regular`, `replay` or `screenshot`. Uses
/// `flatpak-spawn --host` under Flatpak. Returns `false` when the script
/// path cannot be resolved (mirrors the C early return).
pub fn run_saved_script(script: &str, video_file: &str, kind: &str) -> bool {
    let resolved = match std::fs::canonicalize(script) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("gsr error: script file not found: {script}");
            return false;
        }
    };
    let under_flatpak = std::env::var_os("FLATPAK_ID").is_some();
    let mut cmd = if under_flatpak {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("--").arg(&resolved);
        c
    } else {
        Command::new(&resolved)
    };
    cmd.arg(video_file).arg(kind);
    match cmd.spawn() {
        Ok(_) => true,
        Err(e) => {
            eprintln!("gsr error: failed to run script {script}: {e}");
            false
        }
    }
}
