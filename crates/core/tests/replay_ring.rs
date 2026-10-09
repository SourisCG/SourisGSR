//! Replay ring tests: RAM/disk semantics, save planning, file names, scripts.
//! Mirrors `src/replay_buffer/*.c` + `save_replay_async` + filename/script
//! helpers in `src/main.cpp`. Spec ref: `.specify/spec-01-core.md` 3.

use gsr_core::replay::{
    plan_save, recording_filepath, run_saved_script, should_restart, split_timestamp, Cursor,
    Packet, RingView,
};
use gsr_core::replay_disk::DiskRing;
use gsr_core::replay_ram::RamRing;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn pkt(i: i64, stream: i32, key: bool, ts: f64) -> Packet {
    Packet {
        data: vec![i as u8; 8],
        stream_index: stream,
        pts: i * 100,
        dts: i * 100,
        key,
        timestamp: ts,
    }
}

fn tmpdir(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("gsr-t11-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

// --- RamRing ---------------------------------------------------------------

#[test]
fn ram_append_order_len_and_clear() {
    let mut ring = RamRing::new(8);
    assert!(ring.is_empty());
    for i in 0..5 {
        ring.append(pkt(i, 0, i == 0, i as f64));
    }
    assert_eq!(ring.len(), 5);
    assert_eq!(ring.get(0).unwrap().pts, 0);
    assert_eq!(ring.get(4).unwrap().pts, 400);
    assert!(ring.get(5).is_none());
    ring.clear();
    assert!(ring.is_empty());
    assert_eq!(ring.len(), 0);
}

#[test]
fn ram_overwrites_oldest_at_capacity() {
    let mut ring = RamRing::new(3);
    for i in 0..5 {
        ring.append(pkt(i, 0, false, i as f64));
    }
    assert_eq!(ring.len(), 3);
    let pts: Vec<i64> = (0..3).map(|i| ring.get(i).unwrap().pts).collect();
    assert_eq!(pts, vec![200, 300, 400]);
}

#[test]
#[should_panic(expected = "non-zero")]
fn ram_zero_capacity_panics() {
    let _ = RamRing::new(0);
}

#[test]
fn ram_find_by_time_mirrors_c_binary_search() {
    let mut ring = RamRing::new(16);
    for i in 0..10 {
        ring.append(pkt(i, 0, false, i as f64));
    }
    // now = 10.0: packet ts 7 is the oldest with passed >= 3.
    assert_eq!(ring.find_by_time(3, 10.0), Cursor { packet: 7, file: 0 });
    // seconds = 0 resolves to the newest packet.
    assert_eq!(ring.find_by_time(0, 10.0), Cursor { packet: 9, file: 0 });
    // Older than the buffer resolves to the oldest packet.
    assert_eq!(ring.find_by_time(100, 10.0), Cursor { packet: 0, file: 0 });
    // Empty ring resolves to {0, 0}.
    assert_eq!(RamRing::new(4).find_by_time(5, 10.0), Cursor::default());
}

#[test]
fn ram_keyframe_search_and_next() {
    let mut ring = RamRing::new(16);
    // stream 0 keys at 1, 4; stream 1 key at 2.
    for i in 0..6 {
        ring.append(pkt(i, (i % 2) as i32, i == 1 || i == 2 || i == 4, i as f64));
    }
    // Stream 0 packets are i = 0, 2, 4; keys on stream 0 at 2 and 4.
    assert_eq!(
        ring.find_keyframe(Cursor::default(), 0, false),
        Some(Cursor { packet: 2, file: 0 })
    );
    assert_eq!(
        ring.find_keyframe(Cursor { packet: 3, file: 0 }, 0, false),
        Some(Cursor { packet: 4, file: 0 })
    );
    // Inverted: packet 1 is a key but lives on stream 1, so the first
    // key not on stream 1 is packet 2.
    assert_eq!(
        ring.find_keyframe(Cursor::default(), 1, true),
        Some(Cursor { packet: 2, file: 0 })
    );
    assert_eq!(ring.find_keyframe(Cursor::default(), 7, false), None);
    assert_eq!(
        ring.next(Cursor { packet: 4, file: 0 }),
        Some(Cursor { packet: 5, file: 0 })
    );
    assert_eq!(ring.next(Cursor { packet: 5, file: 0 }), None);
}

#[test]
fn ram_snapshot_isolated_from_later_appends() {
    let mut ring = RamRing::new(3);
    for i in 0..3 {
        ring.append(pkt(i, 0, i == 0, i as f64));
    }
    let snap = ring.snapshot();
    assert_eq!(snap.len(), 3);
    for i in 3..6 {
        ring.append(pkt(i, 0, false, i as f64));
    }
    // Snapshot still sees the original three packets.
    let pts: Vec<i64> = snap.iter().map(|p| p.pts).collect();
    assert_eq!(pts, vec![0, 100, 200]);
    // Range reads from a cursor to the end.
    let range = ring.snapshot_range(Cursor { packet: 1, file: 0 });
    let pts: Vec<i64> = range.iter().map(|p| p.pts).collect();
    assert_eq!(pts, vec![400, 500]);
}

// --- Save planning ----------------------------------------------------------

fn mixed_ring() -> RamRing {
    // Video stream 0 (keys at 2, 5), audio stream 1 (keys at 3, 6).
    let mut ring = RamRing::new(16);
    for i in 0..8 {
        let key = i == 2 || i == 5 || i == 3 || i == 6;
        ring.append(pkt(i, if i % 3 == 0 { 1 } else { 0 }, key, i as f64));
    }
    ring
}

#[test]
fn plan_save_full_and_partial() {
    let ring = mixed_ring();
    let full = plan_save(&ring, None, 100.0, 0, &[1]).expect("plan");
    assert_eq!(full.video.packet, 2);
    // Audio search starts at the video keyframe; first stream-1 key after is 3.
    assert_eq!(full.audio, vec![(1, Cursor { packet: 3, file: 0 })]);

    // Last-3-seconds at now = 7.5: oldest packet with passed >= 3 is ts 4,
    // which is not a key; the video keyframe from there is packet 5 and the
    // next stream-1 key after it is packet 6.
    let partial = plan_save(&ring, Some(3), 7.5, 0, &[1]).expect("plan");
    assert_eq!(partial.video.packet, 5);
    assert_eq!(partial.audio, vec![(1, Cursor { packet: 6, file: 0 })]);
}

#[test]
fn plan_save_missing_video_keyframe_is_none() {
    let mut ring = RamRing::new(8);
    for i in 0..4 {
        ring.append(pkt(i, 0, false, i as f64));
    }
    assert!(plan_save(&ring, None, 100.0, 0, &[]).is_none());
}

#[test]
fn plan_save_missing_audio_keyframe_falls_back_to_video_start() {
    let mut ring = RamRing::new(8);
    for i in 0..4 {
        ring.append(pkt(i, 0, i == 1, i as f64));
    }
    // Stream 9 has no keyframes: audio start = video start (pts offset 0, as in C).
    let plan = plan_save(&ring, None, 100.0, 0, &[9]).expect("plan");
    assert_eq!(plan.video.packet, 1);
    assert_eq!(plan.audio, vec![(9, Cursor { packet: 1, file: 0 })]);
}

#[test]
fn restart_only_on_full_save_with_flag() {
    assert!(should_restart(true, true));
    assert!(!should_restart(true, false));
    assert!(!should_restart(false, true));
    assert!(!should_restart(false, false));
}

// --- DiskRing -----------------------------------------------------------------

#[test]
fn disk_append_read_roundtrip() {
    let base = tmpdir("roundtrip");
    let mut ring = DiskRing::new(&base, 60.0, 0);
    assert!(ring.is_empty());
    for i in 0..4 {
        assert!(ring.append(&pkt(i, 0, i == 0, i as f64), i as f64));
    }
    assert_eq!(ring.len(), 4);
    assert_eq!(ring.segment_count(), 1);
    for i in 0..4usize {
        let p = ring.read_packet(0, i).unwrap();
        assert_eq!(p.pts, (i as i64) * 100);
        assert_eq!(p.data, vec![i as u8; 8]);
    }
    assert!(ring.read_packet(0, 9).is_err());
    assert!(ring.read_packet(3, 0).is_err());
}

#[test]
fn disk_rotates_segments_and_evicts_by_time() {
    let base = tmpdir("rotate");
    // 32-byte segments: each 8-byte packet forces rotation quickly.
    let mut ring = DiskRing::with_segment_bytes(&base, 10.0, 32, "sub.gsr");
    for i in 0..8 {
        assert!(ring.append(&pkt(i, 0, i % 4 == 0, i as f64), i as f64));
    }
    assert!(
        ring.segment_count() >= 2,
        "segments: {}",
        ring.segment_count()
    );
    // Push far-future timestamps: the files[1]-rule evicts one segment per
    // append, exactly like C (no mass eviction).
    for i in 100..104 {
        assert!(ring.append(&pkt(i, 0, false, i as f64), i as f64));
    }
    assert_eq!(ring.segment_count(), 2);
    assert!(!base.join("sub.gsr").join("Replay_0.gsr").exists());
    assert!(base.join("sub.gsr").join("Replay_1.gsr").exists());
    assert!(base.join("sub.gsr").join("Replay_2.gsr").exists());
}

#[test]
fn disk_find_keyframe_next_across_segments() {
    let base = tmpdir("multiseg");
    let mut ring = DiskRing::with_segment_bytes(&base, 3600.0, 32, "sub.gsr");
    for i in 0..8 {
        assert!(ring.append(
            &pkt(i, (i % 2) as i32, i == 3 || i == 6, i as f64),
            i as f64
        ));
    }
    let key = ring
        .find_keyframe(Cursor::default(), 0, false)
        .expect("key");
    let p = ring.read_packet(key.file, key.packet).unwrap();
    assert!(p.key && p.stream_index == 0);
    // next() crosses the segment boundary.
    let mut cursor = Cursor::default();
    let mut count = 0;
    loop {
        count += 1;
        match ring.next(cursor) {
            Some(n) => cursor = n,
            None => break,
        }
    }
    assert_eq!(count, ring.len());
    // Full-range snapshot in save order.
    let all = ring.snapshot_range(Cursor::default()).unwrap();
    assert_eq!(all.len(), 8);
    assert_eq!(all[0].pts, 0);
}

#[test]
fn disk_find_by_time_empty_is_zero() {
    let base = tmpdir("empty");
    let ring = DiskRing::new(&base, 60.0, 0);
    assert_eq!(ring.find_by_time(5, 100.0), Cursor::default());
}

#[test]
fn disk_clear_removes_files_and_drop_removes_subdir() {
    let base = tmpdir("clear");
    let sub = base.join("gsr-replay-1970-01-01_00-00-00.gsr");
    {
        let mut ring = DiskRing::new(&base, 60.0, 0);
        assert_eq!(ring.dir(), sub);
        assert!(ring.append(&pkt(0, 0, true, 0.0), 0.0));
        assert!(sub.join("Replay_0.gsr").exists());
        ring.clear();
        assert!(ring.is_empty());
        assert!(!sub.join("Replay_0.gsr").exists());
        assert!(ring.append(&pkt(1, 0, true, 1.0), 1.0));
    }
    // Drop clears and removes the owned subdirectory.
    assert!(!sub.exists());
    fs::remove_dir_all(&base).ok();
}

// --- File names, timestamps, scripts -------------------------------------------

#[test]
fn timestamps_and_filepaths_match_c_formats() {
    assert_eq!(
        split_timestamp(0),
        ("1970-01-01".to_string(), "00-00-00".to_string())
    );
    assert_eq!(
        split_timestamp(90061),
        ("1970-01-02".to_string(), "01-01-01".to_string())
    );
    assert_eq!(
        split_timestamp(-1),
        ("1969-12-31".to_string(), "23-59-59".to_string())
    );
    let dir = PathBuf::from("/tmp/Videos");
    assert_eq!(
        recording_filepath(&dir, "Replay", "mp4", false, 0),
        PathBuf::from("/tmp/Videos/Replay_1970-01-01_00-00-00.mp4")
    );
    assert_eq!(
        recording_filepath(&dir, "Video", "mkv", true, 90061),
        PathBuf::from("/tmp/Videos/1970-01-02/Video_01-01-01.mkv")
    );
}

#[test]
fn missing_script_returns_false() {
    let catalog = gsr_i18n::Catalog::new(gsr_i18n::Lang::En);
    assert!(!run_saved_script(
        &catalog,
        "/no/such/script.sh",
        "/tmp/x.mp4",
        "replay"
    ));
}

#[cfg(unix)]
#[test]
fn script_runs_detached_with_file_and_kind_args() {
    let base = tmpdir("script");
    let marker = base.join("marker.txt");
    let script = base.join("hook.sh");
    fs::write(
        &script,
        format!("#!/bin/sh\necho \"$1 $2\" > {}\n", marker.display()),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    assert!(run_saved_script(
        &gsr_i18n::Catalog::new(gsr_i18n::Lang::En),
        script.to_str().unwrap(),
        "/tmp/v.mp4",
        "replay"
    ));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if marker.exists() {
            break;
        }
        assert!(Instant::now() < deadline, "script never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(fs::read_to_string(&marker).unwrap(), "/tmp/v.mp4 replay\n");
    fs::remove_dir_all(&base).ok();
}
