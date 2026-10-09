//! Disk replay ring. Mirrors `src/replay_buffer/replay_buffer_disk.c`.
//!
//! Segment files `Replay_{id}.gsr` (mode `0o700`) rotate at `segment_bytes`
//! (default 256MB, `REPLAY_BUFFER_FILE_SIZE_BYTES`); at most 1024 segments
//! (`GSR_REPLAY_BUFFER_CAPACITY_NUM_FILES`); time eviction drops the oldest
//! segment once `now - files[1].start >= time_window` (note `files[1]`, as in
//! C). Packet bytes stream contiguously per file with recorded offsets and
//! are read back lazily. `segment_bytes` is a constructor parameter (test
//! seam; behavior identical).
//!
//! `DEVIATION-SNAPSHOT` (see `replay.rs`): save snapshots own their packets.

use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use super::replay::{split_timestamp, Cursor, Packet, RingView};

/// Default segment size (C `REPLAY_BUFFER_FILE_SIZE_BYTES`).
pub const SEGMENT_BYTES: u64 = 256 * 1024 * 1024;
/// Maximum segment files (C `GSR_REPLAY_BUFFER_CAPACITY_NUM_FILES`).
pub const MAX_FILES: usize = 1024;
const FILE_PREFIX: &str = "Replay";
const FILE_EXT: &str = "gsr";

/// Packet metadata; bytes live in the segment file at `offset`.
struct SegPacket {
    stream_index: i32,
    pts: i64,
    dts: i64,
    key: bool,
    timestamp: f64,
    offset: u64,
    len: u64,
}

struct Segment {
    id: u64,
    start_ts: f64,
    end_ts: f64,
    packets: Vec<SegPacket>,
    bytes_written: u64,
    /// Open handle for the current (writable) segment only; closed segments
    /// reopen on read, mirroring C's lazy `fd`.
    handle: Option<File>,
}

impl Segment {
    fn path(&self, dir: &Path) -> PathBuf {
        dir.join(format!("{FILE_PREFIX}_{}.{FILE_EXT}", self.id))
    }
}

/// Disk-backed replay ring. Owns a unique `gsr-replay-<ts>.gsr`
/// subdirectory of `dir` (mirrors `gsr_replay_buffer_disk_create`).
pub struct DiskRing {
    dir: PathBuf,
    time_window: f64,
    segment_bytes: u64,
    files: VecDeque<Segment>,
    storage_counter: u64,
}

impl DiskRing {
    /// `unix_secs` names the owned subdirectory (injectable for tests).
    pub fn new(dir: &Path, time_window: f64, unix_secs: i64) -> Self {
        let (date, time) = split_timestamp(unix_secs);
        Self::with_segment_bytes(
            dir,
            time_window,
            SEGMENT_BYTES,
            &format!("gsr-replay-{date}_{time}.gsr"),
        )
    }

    pub fn with_segment_bytes(
        dir: &Path,
        time_window: f64,
        segment_bytes: u64,
        subdir: &str,
    ) -> Self {
        DiskRing {
            dir: dir.join(subdir),
            time_window,
            segment_bytes,
            files: VecDeque::new(),
            storage_counter: 0,
        }
    }

    /// Owned segment directory (created on first append).
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn segment_count(&self) -> usize {
        self.files.len()
    }

    pub fn len(&self) -> usize {
        self.files.iter().map(|f| f.packets.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn segment_file(&self, id: u64) -> PathBuf {
        self.dir.join(format!("{FILE_PREFIX}_{id}.{FILE_EXT}"))
    }

    /// Create the next segment file (mirrors `..._create_next_file`).
    /// Fails past `MAX_FILES`, like C.
    fn create_next_file(&mut self, timestamp: f64) -> bool {
        if self.files.len() + 1 >= MAX_FILES {
            eprintln!(
                "gsr error: too many replay buffer files created! (> {MAX_FILES}), \
                 either reduce the replay buffer time or report this as a bug"
            );
            return false;
        }
        if fs::create_dir_all(&self.dir).is_err() {
            return false;
        }
        let id = self.storage_counter;
        let path = self.segment_file(id);
        let mut options = OpenOptions::new();
        options.write(true).create(true);
        #[cfg(unix)]
        options.mode(0o700);
        let file = match options.open(&path) {
            Ok(f) => f,
            Err(_) => return false,
        };
        self.files.push_back(Segment {
            id,
            start_ts: timestamp,
            end_ts: timestamp,
            packets: Vec::new(),
            bytes_written: 0,
            handle: Some(file),
        });
        self.storage_counter += 1;
        true
    }

    /// Append a packet, rotating/evicting exactly like C.
    /// Returns `false` on I/O failure or past `MAX_FILES`.
    pub fn append(&mut self, packet: &Packet, timestamp: f64) -> bool {
        let current_open = self
            .files
            .back()
            .map(|f| f.handle.is_some())
            .unwrap_or(false);
        if !current_open && !self.create_next_file(timestamp) {
            return false;
        }
        {
            let segment = self.files.back_mut().expect("segment exists");
            segment.end_ts = timestamp;
            let offset = segment.bytes_written;
            let handle = segment.handle.as_mut().expect("current segment open");
            if handle.write_all(&packet.data).is_err() {
                return false;
            }
            segment.packets.push(SegPacket {
                stream_index: packet.stream_index,
                pts: packet.pts,
                dts: packet.dts,
                key: packet.key,
                timestamp,
                offset,
                len: packet.data.len() as u64,
            });
            segment.bytes_written += packet.data.len() as u64;
            if segment.bytes_written >= self.segment_bytes {
                segment.handle = None;
            }
        }

        // Time eviction compares against files[1], exactly like C.
        if self.files.len() > 1 && timestamp - self.files[1].start_ts >= self.time_window {
            if let Some(old) = self.files.pop_front() {
                let _ = fs::remove_file(self.segment_file(old.id));
            }
        }
        true
    }

    /// Read one packet back (mirrors `..._iterator_get_packet_data`).
    pub fn read_packet(&self, file: usize, packet: usize) -> std::io::Result<Packet> {
        let segment = self.files.get(file).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "file index out of range")
        })?;
        let meta = segment.packets.get(packet).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "packet index out of range",
            )
        })?;
        let mut handle = File::open(self.segment_file(segment.id))?;
        handle.seek(SeekFrom::Start(meta.offset))?;
        let mut data = vec![0u8; meta.len as usize];
        handle.read_exact(&mut data)?;
        Ok(Packet {
            data,
            stream_index: meta.stream_index,
            pts: meta.pts,
            dts: meta.dts,
            key: meta.key,
            timestamp: meta.timestamp,
        })
    }

    /// Owned packets from `from` to the end, in save order.
    pub fn snapshot_range(&self, from: Cursor) -> std::io::Result<Vec<Packet>> {
        let mut out = Vec::new();
        for (fi, segment) in self.files.iter().enumerate() {
            let start = if fi == from.file { from.packet } else { 0 };
            for pi in start..segment.packets.len() {
                out.push(self.read_packet(fi, pi)?);
            }
        }
        Ok(out)
    }

    /// Drop all segments and their files (mirrors `..._disk_clear`).
    pub fn clear(&mut self) {
        let paths: Vec<PathBuf> = self.files.iter().map(|s| self.segment_file(s.id)).collect();
        self.files.clear();
        for path in paths {
            let _ = fs::remove_file(path);
        }
    }
}

impl Drop for DiskRing {
    fn drop(&mut self) {
        self.clear();
        // Mirrors C `remove(replay_directory)` (fails silently if non-empty).
        let _ = fs::remove_dir(&self.dir);
    }
}

impl RingView for DiskRing {
    fn packet_count(&self) -> usize {
        self.len()
    }

    /// Two-level binary search mirroring C (files by start/end window,
    /// then packets within the file); empty ring -> `{0,0}`.
    fn find_by_time(&self, seconds: i64, now: f64) -> Cursor {
        if self.files.is_empty() {
            return Cursor::default();
        }
        let seconds = seconds as f64;
        let mut lower: usize = 0;
        let mut upper: usize = self.files.len();
        let file_index = loop {
            let file_index = lower + (upper - lower) / 2;
            let file = &self.files[file_index];
            let since_start = now - file.start_ts;
            let since_end = now - file.end_ts;
            if since_start >= seconds && since_end <= seconds {
                break file_index;
            } else if since_start >= seconds {
                if lower == file_index {
                    break file_index;
                }
                lower = file_index;
            } else {
                if upper == file_index {
                    break file_index;
                }
                upper = file_index;
            }
        };
        let file = &self.files[file_index];
        if file.packets.is_empty() {
            return Cursor {
                packet: 0,
                file: file_index,
            };
        }
        let mut lower: usize = 0;
        let mut upper: usize = file.packets.len();
        let index = loop {
            let index = lower + (upper - lower) / 2;
            let passed = now - file.packets[index].timestamp;
            if passed >= seconds {
                if lower == index {
                    break index;
                }
                lower = index;
            } else {
                if upper == index {
                    break index;
                }
                upper = index;
            }
        };
        Cursor {
            packet: index,
            file: file_index,
        }
    }

    fn find_keyframe(&self, start: Cursor, stream: i32, invert: bool) -> Option<Cursor> {
        for (fi, segment) in self.files.iter().enumerate().skip(start.file) {
            let pi0 = if fi == start.file { start.packet } else { 0 };
            for (pi, packet) in segment.packets.iter().enumerate().skip(pi0) {
                let stream_match = if invert {
                    packet.stream_index != stream
                } else {
                    packet.stream_index == stream
                };
                if packet.key && stream_match {
                    return Some(Cursor {
                        packet: pi,
                        file: fi,
                    });
                }
            }
        }
        None
    }

    fn next(&self, cursor: Cursor) -> Option<Cursor> {
        let segment = self.files.get(cursor.file)?;
        if cursor.packet + 1 < segment.packets.len() {
            return Some(Cursor {
                packet: cursor.packet + 1,
                file: cursor.file,
            });
        }
        if cursor.file + 1 < self.files.len() {
            return Some(Cursor {
                packet: 0,
                file: cursor.file + 1,
            });
        }
        None
    }
}
