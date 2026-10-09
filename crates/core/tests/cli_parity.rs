//! CLI parity tests: every validation branch of `src/args_parser.c`.
//! Spec refs: `.specify/spec-01-core.md` sections 1-2, 7-8.

use gsr_core::cli::{parse, render_help, Action, EXIT_USAGE_ERROR};
use gsr_core::config::{
    contains_non_hex_number, is_livestream_path, AudioCodec, BitrateMode, FramerateMode, Quality,
    VideoCodec, VideoQuality,
};
use gsr_i18n::{Catalog, Lang};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| w.to_string()).collect()
}

fn ok(words: &[&str]) -> Action {
    match parse(&argv(words), Some("C")) {
        Ok(ok) => {
            assert!(
                ok.warnings.is_empty(),
                "unexpected warnings: {:?}",
                ok.warnings
            );
            ok.action
        }
        Err(e) => panic!("expected Ok, got error: {}", e.message),
    }
}

fn err(words: &[&str]) -> String {
    match parse(&argv(words), Some("C")) {
        Ok(_) => panic!("expected Err for {words:?}"),
        Err(e) => {
            assert!(!e.full_help, "errors must show short usage");
            e.message
        }
    }
}

fn run_config(words: &[&str]) -> gsr_core::config::Config {
    match ok(words) {
        Action::Run(config) => *config,
        other => panic!("expected Run, got {other:?}"),
    }
}

fn unique_tmp(name: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("gsr-t10-{}-{}", std::process::id(), name));
    p
}

fn write_executable(path: &std::path::Path) {
    fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}

// --- Valid parses and defaults -------------------------------------------

#[test]
fn minimal_record_has_c_defaults() {
    let c = run_config(&["gsr", "-w", "screen", "-o", "v.mp4"]);
    assert_eq!(c.window, "screen");
    assert_eq!(c.output, "v.mp4");
    assert_eq!(c.container_effective, "mp4");
    assert_eq!(c.fps, 60);
    assert_eq!(c.video_codec, VideoCodec::Auto);
    assert_eq!(c.audio_codec, AudioCodec::Opus);
    assert_eq!(c.bitrate_mode, BitrateMode::Auto);
    assert_eq!(c.framerate_mode, FramerateMode::Variable);
    assert_eq!(c.quality, Quality::Preset(VideoQuality::VeryHigh));
    assert_eq!(c.audio_bitrate, 0);
    assert!((c.keyint - 2.0).abs() < f64::EPSILON);
    assert!(c.record_cursor && c.verbose);
    assert!(!c.gl_debug && !c.date_folders && !c.overclock);
    assert_eq!(c.replay_secs, None);
    assert!(!c.is_livestream && !c.is_output_piped && !c.low_latency);
    assert_eq!(c.lang, Lang::En);
}

#[test]
fn explicit_options_map() {
    let script = unique_tmp("hook.sh");
    write_executable(&script);
    let out = unique_tmp("out.mp4");
    let c = run_config(&[
        "gsr",
        "-w",
        "DP-1",
        "-c",
        "mkv",
        "-f",
        "144",
        "-s",
        "1920x1080",
        "-a",
        "default_output",
        "-a",
        "default_input",
        "-q",
        "ultra",
        "-o",
        out.to_str().unwrap(),
        "-r",
        "30",
        "-k",
        "h265",
        "-ac",
        "aac",
        "-ab",
        "160",
        "-fm",
        "cfr",
        "-bm",
        "vbr",
        "-pixfmt",
        "yuv444",
        "-v",
        "no",
        "-cursor",
        "no",
        "-keyint",
        "0.5",
        "-cr",
        "full",
        "-tune",
        "quality",
        "-encoder",
        "cpu",
        "-fallback-cpu-encoding",
        "yes",
        "-replay-storage",
        "disk",
        "-restart-replay-on-save",
        "yes",
        "-df",
        "yes",
        "-gl-debug",
        "yes",
        "-restore-portal-session",
        "yes",
        "-sc",
        script.to_str().unwrap(),
        "-p",
        "a.so",
        "-p",
        "b.so",
        "--lang",
        "es",
    ]);
    assert_eq!(c.container, Some("matroska".to_string()));
    assert_eq!(c.container_effective, "matroska");
    assert_eq!(c.fps, 144);
    assert_eq!(c.output_resolution, (1920, 1080));
    assert_eq!(
        c.audio_inputs,
        vec!["default_output".to_string(), "default_input".to_string()]
    );
    assert_eq!(c.quality, Quality::Preset(VideoQuality::Ultra));
    // -r 30 + keyint 0.5 -> 30 + (0.5 + 0.5) as i64 = 31 (C padding rule).
    assert_eq!(c.replay_secs, Some(31));
    assert_eq!(c.video_codec, VideoCodec::Hevc);
    assert_eq!(c.audio_codec, AudioCodec::Aac);
    assert_eq!(c.audio_bitrate, 160000);
    assert_eq!(c.plugins, vec!["a.so".to_string(), "b.so".to_string()]);
    assert_eq!(c.lang, Lang::Es);
    assert!(!c.verbose && !c.record_cursor && c.gl_debug && c.date_folders);
    fs::remove_file(&script).ok();
}

#[test]
fn region_parses() {
    let c = run_config(&[
        "gsr",
        "-w",
        "region",
        "-region",
        "800x600+10+20",
        "-o",
        "v.mp4",
    ]);
    let r = c.region.expect("region");
    assert_eq!((r.width, r.height, r.x, r.y), (800, 600, 10, 20));
}

#[test]
fn cbr_quality_is_bitrate_bps() {
    let c = run_config(&[
        "gsr", "-w", "screen", "-bm", "cbr", "-q", "20000", "-o", "v.mp4",
    ]);
    assert_eq!(c.quality, Quality::Bitrate(20000 * 1000));
}

#[test]
fn stdout_default_and_livestream_derive() {
    // C requires -c whenever -o is absent, even for /dev/stdout.
    assert!(err(&["gsr", "-w", "screen"]).contains("-c is required"));
    let c = run_config(&["gsr", "-w", "screen", "-c", "mp4"]);
    assert_eq!(c.output, "/dev/stdout");
    assert!(c.is_output_piped && c.low_latency && !c.is_livestream);

    let c = run_config(&["gsr", "-w", "screen", "-o", "rtmp://live/x/key"]);
    assert!(c.is_livestream && c.low_latency && !c.is_output_piped);
    for scheme in [
        "http://h/x",
        "https://h/x",
        "rtmps://h/x",
        "rtsp://h/x",
        "srt://h/x",
        "tcp://h/x",
        "udp://h/x",
    ] {
        assert!(is_livestream_path(scheme), "{scheme}");
    }
    assert!(!is_livestream_path("video.mp4"));
    assert!(!is_livestream_path("/dev/stdout"));
}

#[test]
fn livestream_nulls_script_with_warning() {
    let script = unique_tmp("hook2.sh");
    write_executable(&script);
    match parse(
        &argv(&[
            "gsr",
            "-w",
            "screen",
            "-o",
            "rtmp://live/x/key",
            "-sc",
            script.to_str().unwrap(),
        ]),
        Some("C"),
    ) {
        Ok(ok) => match ok.action {
            Action::Run(c) => {
                assert_eq!(c.saved_script, None);
                assert_eq!(ok.warnings.len(), 1);
                assert!(ok.warnings[0].contains("-sc"));
            }
            other => panic!("{other:?}"),
        },
        Err(e) => panic!("{}", e.message),
    }
    fs::remove_file(&script).ok();
}

#[test]
fn flac_falls_back_to_opus_with_warning() {
    match parse(
        &argv(&["gsr", "-w", "screen", "-ac", "flac", "-o", "v.mp4"]),
        Some("C"),
    ) {
        Ok(ok) => match ok.action {
            Action::Run(c) => {
                assert_eq!(c.audio_codec, AudioCodec::Opus);
                assert_eq!(ok.warnings.len(), 1);
            }
            other => panic!("{other:?}"),
        },
        Err(e) => panic!("{}", e.message),
    }
}

#[test]
fn overclock_warns_and_neutralizes() {
    match parse(
        &argv(&["gsr", "-w", "screen", "-oc", "yes", "-o", "v.mp4"]),
        Some("C"),
    ) {
        Ok(ok) => match ok.action {
            Action::Run(c) => {
                assert!(!c.overclock);
                assert_eq!(ok.warnings.len(), 1);
                assert!(ok.warnings[0].contains("-oc"));
            }
            other => panic!("{other:?}"),
        },
        Err(e) => panic!("{}", e.message),
    }
}

#[test]
fn portal_without_restore_warns() {
    match parse(&argv(&["gsr", "-w", "portal", "-o", "v.mp4"]), Some("C")) {
        Ok(ok) => {
            assert_eq!(ok.warnings.len(), 1);
            assert!(ok.warnings[0].contains("portal"));
        }
        Err(e) => panic!("{}", e.message),
    }
}

#[test]
fn non_replay_output_creates_parent_dirs() {
    let base = unique_tmp("mkdir");
    let out = base.join("sub").join("v.mp4");
    let c = run_config(&["gsr", "-w", "screen", "-o", out.to_str().unwrap()]);
    assert_eq!(c.output, out.to_str().unwrap());
    assert!(base.join("sub").is_dir());
    fs::remove_dir_all(&base).ok();
}

// --- Special commands -----------------------------------------------------

#[test]
fn special_commands() {
    assert_eq!(ok(&["gsr", "-h"]), Action::Help);
    assert_eq!(ok(&["gsr", "--help"]), Action::Help);
    assert_eq!(ok(&["gsr", "--version"]), Action::Version);
    assert_eq!(ok(&["gsr", "--info"]), Action::Info);
    assert_eq!(ok(&["gsr", "--list-audio-devices"]), Action::ListAudio);
    assert_eq!(
        ok(&["gsr", "--list-application-audio"]),
        Action::ListAppAudio
    );
    assert_eq!(
        ok(&["gsr", "--list-capture-options"]),
        Action::ListCapture { card: None }
    );
    assert_eq!(
        ok(&["gsr", "--list-capture-options", "/dev/dri/card0"]),
        Action::ListCapture {
            card: Some("/dev/dri/card0".to_string())
        }
    );
    // C accepts (and ignores) one extra trailing argument.
    assert!(matches!(
        ok(&["gsr", "--list-capture-options", "a", "b"]),
        Action::ListCapture { .. }
    ));
    assert!(
        err(&["gsr", "--list-capture-options", "a", "b", "c"]).contains("--list-capture-options")
    );
    match parse(&argv(&["gsr"]), Some("C")) {
        Err(e) => assert!(e.full_help && e.message.is_empty()),
        Ok(_) => panic!("expected full-help error"),
    }
}

// --- Token-level errors ---------------------------------------------------

#[test]
fn token_errors() {
    assert!(err(&["gsr", "-Z", "x", "-w", "screen"]).contains("'-Z'"));
    assert!(err(&["gsr", "-ipc", "x", "-w", "screen"]).contains("v2"));
    assert!(err(&["gsr", "-w", "screen", "-f", "30", "-f", "60"]).contains("'-f'"));
    assert!(err(&["gsr", "-w", "screen", "-f"]).contains("'-f'"));
    assert!(err(&["gsr", "-o", "v.mp4"]).contains("'-w'"));
    assert!(err(&["gsr", "-w", "screen", "-cursor", "maybe"]).contains("'maybe'"));
    assert!(err(&["gsr", "-w", "screen", "-k", "vp7"]).contains("vp8"));
    assert!(err(&["gsr", "-w", "screen", "-f", "abc"]).contains("not an integer"));
    assert!(err(&["gsr", "-w", "screen", "-f", "0"]).contains("larger than 1"));
    assert!(err(&["gsr", "-w", "screen", "-f", "5000"]).contains("less than 1000"));
    assert!(err(&["gsr", "-w", "screen", "-r", "1", "-c", "mp4"]).contains("larger than 2"));
    assert!(err(&["gsr", "-w", "screen", "-keyint", "xyz"]).contains("floating-point"));
    assert!(err(&["gsr", "-w", "screen", "-keyint", "600"]).contains("less than 500"));
    assert!(err(&["gsr", "-w", "screen", "--lang", "fr"]).contains("--lang"));
    // C consumes the next token unconditionally, even if flag-shaped.
    assert_eq!(run_config(&["gsr", "-w", "-f", "-o", "v.mp4"]).window, "-f");
}

// --- Cross-field errors (C order) -----------------------------------------

#[test]
fn cross_field_errors() {
    assert!(err(&[
        "gsr",
        "-w",
        "screen",
        "-portal-session-token-filepath",
        "/tmp/x/"
    ])
    .contains('/'));
    assert!(err(&["gsr", "-w", "screen", "-sc", "/no/such/script.sh"]).contains("not a file"));
    assert!(err(&["gsr", "-w", "screen", "-bm", "cbr", "-o", "v.mp4"]).contains("'-q' is required"));
    assert!(
        err(&["gsr", "-w", "screen", "-bm", "cbr", "-q", "fast", "-o", "v.mp4"])
            .contains("not an integer value")
    );
    assert!(
        err(&["gsr", "-w", "screen", "-bm", "cbr", "-q", "-5", "-o", "v.mp4"])
            .contains("0 or larger")
    );
    assert!(err(&["gsr", "-w", "screen", "-q", "low", "-o", "v.mp4"]).contains("very_high"));
    assert!(err(&["gsr", "-w", "focused", "-o", "v.mp4"]).contains("-s is required"));
    assert!(err(&["gsr", "-w", "screen", "-s", "1920", "-o", "v.mp4"]).contains("WxH"));
    assert!(
        err(&["gsr", "-w", "screen", "-s", "-1x1080", "-o", "v.mp4"])
            .contains("greater or equal to 0")
    );
    assert!(
        err(&["gsr", "-w", "screen", "-region", "1x1+0+0", "-o", "v.mp4"]).contains("-w region")
    );
    assert!(err(&["gsr", "-w", "region", "-o", "v.mp4"]).contains("-region is required"));
    assert!(err(&["gsr", "-w", "region", "-region", "800x600", "-o", "v.mp4"]).contains("WxH+X+Y"));
    assert!(err(&[
        "gsr",
        "-w",
        "region",
        "-region",
        "-5x600+0+0",
        "-o",
        "v.mp4"
    ])
    .contains("greater or equal to 0"));
    assert!(err(&[
        "gsr",
        "-w",
        "screen",
        "-o",
        "rtmp://live/x",
        "-r",
        "10",
        "-c",
        "flv"
    ])
    .contains("not applicable"));
    assert!(err(&[
        "gsr",
        "-w",
        "screen",
        "-r",
        "10",
        "-o",
        unique_tmp("replay-no-c").to_str().unwrap()
    ])
    .contains("-c is required"));
    assert!(err(&["gsr", "-w", "screen", "-r", "10"]).contains("-o is required"));
}

#[test]
fn replay_output_must_be_directory() {
    let file = unique_tmp("not-a-dir.mp4");
    fs::write(&file, b"x").unwrap();
    let msg = err(&[
        "gsr",
        "-w",
        "screen",
        "-r",
        "10",
        "-c",
        "mp4",
        "-o",
        file.to_str().unwrap(),
    ]);
    assert!(msg.contains("not a directory"), "{msg}");
    fs::remove_file(&file).ok();
}

#[cfg(unix)]
#[test]
fn script_must_be_executable() {
    let script = unique_tmp("not-exec.sh");
    fs::write(&script, "#!/bin/sh\n").unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&script, perms).unwrap();
    let msg = err(&[
        "gsr",
        "-w",
        "screen",
        "-sc",
        script.to_str().unwrap(),
        "-o",
        "v.mp4",
    ]);
    assert!(msg.contains("not executable"), "{msg}");
    fs::remove_file(&script).ok();
}

// --- Source gating --------------------------------------------------------

#[test]
fn x11_and_future_sources_rejected() {
    for w in [
        "screen-direct",
        "screen-direct-force",
        "focused",
        "0x1400003",
        "67108873",
    ] {
        let args = if w == "focused" {
            vec!["gsr", "-w", w, "-s", "800x600", "-o", "v.mp4"]
        } else {
            vec!["gsr", "-w", w, "-o", "v.mp4"]
        };
        assert!(err(&args).contains("X11"), "{w}");
    }
    assert!(err(&["gsr", "-w", "/dev/video0", "-o", "v.mp4"]).contains("v2"));
    // Monitor names and keywords pass gating.
    assert_eq!(
        run_config(&["gsr", "-w", "DP-1", "-o", "v.mp4"]).window,
        "DP-1"
    );
    assert!(err(&["gsr", "-w", &"M".repeat(64), "-o", "v.mp4"]).contains("63"));
}

// --- Codec helpers mirror defs.c ------------------------------------------

#[test]
fn codec_helpers() {
    assert_eq!(VideoCodec::from_cli_name("h265"), Some(VideoCodec::Hevc));
    assert_eq!(VideoCodec::from_cli_name("hevc"), Some(VideoCodec::Hevc));
    assert_eq!(VideoCodec::from_cli_name("h264_vulkan"), None);
    assert!(VideoCodec::HevcHdr.is_hdr() && VideoCodec::Av1Hdr.is_hdr());
    assert!(!VideoCodec::Hevc10Bit.is_hdr() && !VideoCodec::Auto.is_hdr());
    assert_eq!(VideoCodec::HevcHdr.to_sdr(), VideoCodec::Hevc);
    assert_eq!(VideoCodec::Av1Hdr.to_sdr(), VideoCodec::Av1);
    assert_eq!(VideoCodec::Av110Bit.bit_depth(), 10);
    assert_eq!(VideoCodec::H264.bit_depth(), 8);
    assert!(VideoCodec::Av1.is_av1() && !VideoCodec::Hevc.is_av1());
    assert!(VideoCodec::HevcVulkan.is_vulkan() && !VideoCodec::Hevc.is_vulkan());
    assert_eq!(VideoCodec::Hevc.as_str(), "hevc");
    assert_eq!(VideoCodec::Auto.as_str(), "");
}

#[test]
fn window_id_rule_mirrors_c() {
    // Mirrors contains_non_hex_number in main.cpp.
    assert!(!contains_non_hex_number("0x1400003"));
    assert!(!contains_non_hex_number("67108873"));
    assert!(!contains_non_hex_number(""));
    assert!(!contains_non_hex_number("0x"));
    assert!(contains_non_hex_number("DP-1"));
    assert!(contains_non_hex_number("screen"));
    assert!(contains_non_hex_number("abc"));
    assert!(contains_non_hex_number("-f"));
}

// --- Help snapshots (EN/ES frozen text) ------------------------------------

fn snapshot_path(lang: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(format!("help_{lang}.txt"))
}

#[test]
fn help_snapshots() {
    for lang in [Lang::En, Lang::Es] {
        let text = render_help(&Catalog::new(lang));
        let path = snapshot_path(lang.as_code());
        if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, &text).unwrap();
        }
        let expected = fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!(
                "missing snapshot {}; run with UPDATE_SNAPSHOTS=1",
                path.display()
            )
        });
        assert_eq!(text, expected, "help text drift for {}", lang.as_code());
    }
    let en = render_help(&Catalog::new(Lang::En));
    let es = render_help(&Catalog::new(Lang::Es));
    assert_ne!(en, es);
    // 1 usage line + 33 flag lines, same shape both languages.
    assert_eq!(en.lines().count(), 34);
    assert_eq!(es.lines().count(), 34);
}

#[test]
fn usage_error_exit_code_is_one() {
    // Mirrors main.cpp _exit(1) on parse failure.
    assert_eq!(EXIT_USAGE_ERROR, 1);
}
