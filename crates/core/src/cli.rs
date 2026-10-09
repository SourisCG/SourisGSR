//! CLI parser. Mirrors `src/args_parser.c` (frozen reference v5.10.2).
//!
//! Deviations from C (all covered by tests in `tests/cli_parity.rs`):
//! - `DEVIATION-HELP-EXIT`: `-h`/`--help` is `Action::Help` (exit 0 in
//!   `main.rs`); C prints help and exits 1.
//! - `DEVIATION-STRICT-INT`: integers, doubles, `-s` and `-region` require a
//!   full-string parse; C `sscanf` accepts trailing garbage (`60x` -> 60).
//! - `DEVIATION-TYPO`: `is not a floating-point number` (C has
//!   `is not an floating-point number`).
//! - `DEVIATION-WINDOW-LEN`: `-w` over 63 chars is rejected; C truncates via
//!   `snprintf` into `char window[64]`.
//! - `DEVIATION-ADD`: `--lang en|es|auto` (see spec-06); reserved future
//!   spellings (`-ipc`, `-auth`, `-ffmpeg-*`, V4L2 paths) fail with
//!   `err_unsupported_future` instead of `err_unknown_flag`.
//! - `DEVIATION-X11-NEVER`: `screen-direct`, `screen-direct-force`,
//!   `focused` and raw window ids fail with `err_unsupported_x11`.
//! - GPU-dependent checks from `args_parser_validate_with_gl_info`
//!   (Steam Deck, overclock effect, HDR rules) are deferred to T20/T40;
//!   `-oc` always warns and neutralizes on our platforms (it can never
//!   apply without X11), as documented in spec-01.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use gsr_i18n::{fill, resolve_lang, Catalog, Lang};

use crate::config::{
    is_livestream_path, AudioCodec, BitrateMode, ColorRange, Config, EncoderHw, FramerateMode,
    PixelFormat, Quality, Region, ReplayStorage, Tune, VideoCodec, VideoQuality,
    DEFAULT_STDOUT_OUTPUT, MAX_WINDOW_LEN,
};

/// Process exit code for usage/parse errors (mirrors `_exit(1)` in `main.cpp`).
pub const EXIT_USAGE_ERROR: i32 = 1;

/// Parsed top-level action. Mirrors the `args_handlers` callbacks in C.
/// `Config` is boxed: it is the only large variant (clippy `large_enum_variant`).
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Run(Box<Config>),
    Help,
    Version,
    Info,
    ListCapture { card: Option<String> },
    ListAudio,
    ListAppAudio,
}

/// Successful parse: action plus non-fatal warnings for stderr.
pub struct ParseOk {
    pub action: Action,
    pub warnings: Vec<String>,
}

/// Failed parse: localized message plus which usage text `main` must print.
pub struct ParseErr {
    pub message: String,
    /// `true` = full `--help`, `false` = short usage line (mirrors C
    /// `usage_full()` vs `usage()` placement).
    pub full_help: bool,
}

enum FlagType {
    Str,
    Bool,
    Enum(&'static [&'static str]),
    I64(i64, i64),
    F64(f64, f64),
}

struct FlagDef {
    key: &'static str,
    list: bool,
    optional: bool,
    ty: FlagType,
}

/// Flag table in C `args_parser.c` order, plus `--lang` (`DEVIATION-ADD`).
fn flag_table() -> Vec<FlagDef> {
    vec![
        FlagDef {
            key: "-w",
            list: false,
            optional: false,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "-c",
            list: false,
            optional: true,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "-f",
            list: false,
            optional: true,
            ty: FlagType::I64(1, 1000),
        },
        FlagDef {
            key: "-s",
            list: false,
            optional: true,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "-region",
            list: false,
            optional: true,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "-a",
            list: true,
            optional: true,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "-q",
            list: false,
            optional: true,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "-o",
            list: false,
            optional: true,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "-ro",
            list: false,
            optional: true,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "-r",
            list: false,
            optional: true,
            ty: FlagType::I64(2, 86400),
        },
        FlagDef {
            key: "-restart-replay-on-save",
            list: false,
            optional: true,
            ty: FlagType::Bool,
        },
        FlagDef {
            key: "-k",
            list: false,
            optional: true,
            ty: FlagType::Enum(VideoCodec::cli_names()),
        },
        FlagDef {
            key: "-ac",
            list: false,
            optional: true,
            ty: FlagType::Enum(AudioCodec::cli_names()),
        },
        FlagDef {
            key: "-ab",
            list: false,
            optional: true,
            ty: FlagType::I64(0, 50000),
        },
        FlagDef {
            key: "-oc",
            list: false,
            optional: true,
            ty: FlagType::Bool,
        },
        FlagDef {
            key: "-fm",
            list: false,
            optional: true,
            ty: FlagType::Enum(FramerateMode::cli_names()),
        },
        FlagDef {
            key: "-bm",
            list: false,
            optional: true,
            ty: FlagType::Enum(BitrateMode::cli_names()),
        },
        FlagDef {
            key: "-pixfmt",
            list: false,
            optional: true,
            ty: FlagType::Enum(PixelFormat::cli_names()),
        },
        FlagDef {
            key: "-v",
            list: false,
            optional: true,
            ty: FlagType::Bool,
        },
        FlagDef {
            key: "-gl-debug",
            list: false,
            optional: true,
            ty: FlagType::Bool,
        },
        FlagDef {
            key: "-df",
            list: false,
            optional: true,
            ty: FlagType::Bool,
        },
        FlagDef {
            key: "-sc",
            list: false,
            optional: true,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "-cr",
            list: false,
            optional: true,
            ty: FlagType::Enum(ColorRange::cli_names()),
        },
        FlagDef {
            key: "-tune",
            list: false,
            optional: true,
            ty: FlagType::Enum(Tune::cli_names()),
        },
        FlagDef {
            key: "-cursor",
            list: false,
            optional: true,
            ty: FlagType::Bool,
        },
        FlagDef {
            key: "-keyint",
            list: false,
            optional: true,
            ty: FlagType::F64(0.0, 500.0),
        },
        FlagDef {
            key: "-restore-portal-session",
            list: false,
            optional: true,
            ty: FlagType::Bool,
        },
        FlagDef {
            key: "-portal-session-token-filepath",
            list: false,
            optional: true,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "-encoder",
            list: false,
            optional: true,
            ty: FlagType::Enum(EncoderHw::cli_names()),
        },
        FlagDef {
            key: "-fallback-cpu-encoding",
            list: false,
            optional: true,
            ty: FlagType::Bool,
        },
        FlagDef {
            key: "-replay-storage",
            list: false,
            optional: true,
            ty: FlagType::Enum(ReplayStorage::cli_names()),
        },
        FlagDef {
            key: "-p",
            list: true,
            optional: true,
            ty: FlagType::Str,
        },
        FlagDef {
            key: "--lang",
            list: false,
            optional: true,
            ty: FlagType::Enum(&["en", "es", "auto"]),
        },
    ]
}

/// Spellings reserved for v2 (`DEVIATION-FUTURE`, absent in v5.10.2).
fn is_future_flag(flag: &str) -> bool {
    matches!(
        flag,
        "-ipc" | "-auth" | "-ffmpeg-opts" | "-ffmpeg-video-opts" | "-ffmpeg-audio-opts"
    )
}

fn usage_err(catalog: &Catalog, key: &str, pairs: &[(&str, &str)]) -> ParseErr {
    ParseErr {
        message: format!("gsr error: {}", fill(&catalog.get(key), pairs)),
        full_help: false,
    }
}

/// Parse full argv (including program name) with `LANG` env value.
/// Errors carry localized messages; `main` prints short usage to stdout.
pub fn parse(argv: &[String], lang_env: Option<&str>) -> Result<ParseOk, ParseErr> {
    let lang = resolve_lang(argv, lang_env);
    let catalog = Catalog::new(lang);
    let no = |key: &str, pairs: &[(&str, &str)]| usage_err(&catalog, key, pairs);

    if argv.len() <= 1 {
        return Err(ParseErr {
            message: String::new(),
            full_help: true,
        });
    }
    let args = &argv[1..];
    if args.len() == 1 {
        match args[0].as_str() {
            "-h" | "--help" => {
                return Ok(ParseOk {
                    action: Action::Help,
                    warnings: vec![],
                })
            }
            "--info" => {
                return Ok(ParseOk {
                    action: Action::Info,
                    warnings: vec![],
                })
            }
            "--list-audio-devices" => {
                return Ok(ParseOk {
                    action: Action::ListAudio,
                    warnings: vec![],
                })
            }
            "--list-application-audio" => {
                return Ok(ParseOk {
                    action: Action::ListAppAudio,
                    warnings: vec![],
                });
            }
            "--version" => {
                return Ok(ParseOk {
                    action: Action::Version,
                    warnings: vec![],
                })
            }
            _ => {}
        }
    }
    if args[0] == "--list-capture-options" {
        match args.len() {
            1 => {
                return Ok(ParseOk {
                    action: Action::ListCapture { card: None },
                    warnings: vec![],
                })
            }
            2 | 3 => {
                return Ok(ParseOk {
                    action: Action::ListCapture {
                        card: Some(args[1].clone()),
                    },
                    warnings: vec![],
                });
            }
            _ => return Err(no("err_list_capture_args", &[])),
        }
    }

    let table = flag_table();
    let mut values: HashMap<&str, Vec<String>> = HashMap::new();
    let mut i = 0;
    while i < args.len() {
        let name = args[i].as_str();
        let def = table.iter().find(|d| d.key == name);
        let Some(def) = def else {
            if is_future_flag(name) {
                return Err(no("err_unsupported_future", &[]));
            }
            return Err(no("err_unknown_flag", &[("flag", name)]));
        };
        let seen = values.get(def.key).map(|v| !v.is_empty()).unwrap_or(false);
        if seen && !def.list {
            return Err(no("err_repeated_flag", &[("flag", def.key)]));
        }
        let Some(value) = args.get(i + 1) else {
            return Err(no("err_missing_value", &[("flag", def.key)]));
        };
        match &def.ty {
            FlagType::Str => {}
            FlagType::Bool => {
                if value != "yes" && value != "no" {
                    return Err(no(
                        "err_bool_expected",
                        &[("flag", def.key), ("got", value)],
                    ));
                }
            }
            FlagType::Enum(names) => {
                if !names.contains(&value.as_str()) {
                    let expected = names
                        .iter()
                        .map(|n| format!("'{n}'"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(no(
                        "err_enum_expected",
                        &[("flag", def.key), ("expected", &expected), ("got", value)],
                    ));
                }
            }
            FlagType::I64(min, max) => match value.parse::<i64>() {
                Ok(v) if v < *min => {
                    return Err(no(
                        "err_int_too_small",
                        &[("flag", def.key), ("min", &min.to_string()), ("got", value)],
                    ));
                }
                Ok(v) if v > *max => {
                    return Err(no(
                        "err_int_too_large",
                        &[("flag", def.key), ("max", &max.to_string()), ("got", value)],
                    ));
                }
                Ok(_) => {}
                Err(_) => return Err(no("err_int_invalid", &[("flag", def.key), ("got", value)])),
            },
            FlagType::F64(min, max) => match value.parse::<f64>() {
                Ok(v) if v < *min => {
                    return Err(no(
                        "err_int_too_small",
                        &[("flag", def.key), ("min", &min.to_string()), ("got", value)],
                    ));
                }
                Ok(v) if v > *max => {
                    return Err(no(
                        "err_int_too_large",
                        &[("flag", def.key), ("max", &max.to_string()), ("got", value)],
                    ));
                }
                Ok(_) => {}
                Err(_) => {
                    return Err(no(
                        "err_double_invalid",
                        &[("flag", def.key), ("got", value)],
                    ))
                }
            },
        }
        values.entry(def.key).or_default().push(value.clone());
        i += 2;
    }

    for def in &table {
        if !def.optional {
            let empty = values.get(def.key).map(|v| v.is_empty()).unwrap_or(true);
            if empty {
                return Err(no("err_missing_argument", &[("flag", def.key)]));
            }
        }
    }

    build_config(&catalog, lang, &values).map(|(config, warnings)| ParseOk {
        action: Action::Run(Box::new(config)),
        warnings,
    })
}

fn one<'a>(values: &'a HashMap<&str, Vec<String>>, key: &str) -> Option<&'a str> {
    values.get(key).and_then(|v| v.first()).map(String::as_str)
}

fn boolean(values: &HashMap<&str, Vec<String>>, key: &str, default: bool) -> bool {
    one(values, key).map(|v| v == "yes").unwrap_or(default)
}

/// Mirror `args_parser_set_values` order, then `-w` source gating.
fn build_config(
    catalog: &Catalog,
    lang: Lang,
    values: &HashMap<&str, Vec<String>>,
) -> Result<(Config, Vec<String>), ParseErr> {
    let no = |key: &str, pairs: &[(&str, &str)]| usage_err(catalog, key, pairs);
    let mut warnings = Vec::new();

    let encoder_hw = one(values, "-encoder")
        .map(EncoderHw::from_cli_name)
        .unwrap_or(Some(EncoderHw::Gpu));
    let pixfmt = one(values, "-pixfmt")
        .map(PixelFormat::from_cli_name)
        .unwrap_or(Some(PixelFormat::Yuv420));
    let framerate_mode = one(values, "-fm")
        .map(FramerateMode::from_cli_name)
        .unwrap_or(Some(FramerateMode::Variable));
    let color_range = one(values, "-cr")
        .map(ColorRange::from_cli_name)
        .unwrap_or(Some(ColorRange::Limited));
    let tune = one(values, "-tune")
        .map(Tune::from_cli_name)
        .unwrap_or(Some(Tune::Performance));
    let video_codec = one(values, "-k")
        .map(VideoCodec::from_cli_name)
        .unwrap_or(Some(VideoCodec::Auto));
    let audio_codec = one(values, "-ac")
        .map(AudioCodec::from_cli_name)
        .unwrap_or(Some(AudioCodec::Opus));
    let bitrate_mode = one(values, "-bm")
        .map(BitrateMode::from_cli_name)
        .unwrap_or(Some(BitrateMode::Auto));
    let replay_storage = one(values, "-replay-storage")
        .map(ReplayStorage::from_cli_name)
        .unwrap_or(Some(ReplayStorage::Ram));
    // Enum(token-level) validation guarantees these; expect holds by construction.
    let (
        encoder_hw,
        pixfmt,
        framerate_mode,
        color_range,
        tune,
        video_codec,
        bitrate_mode,
        replay_storage,
    ) = (
        encoder_hw.expect("enc"),
        pixfmt.expect("pix"),
        framerate_mode.expect("fm"),
        color_range.expect("cr"),
        tune.expect("tune"),
        video_codec.expect("k"),
        bitrate_mode.expect("bm"),
        replay_storage.expect("storage"),
    );
    let mut audio_codec = audio_codec.expect("ac");

    let window = one(values, "-w").expect("-w required checked").to_string();
    if window.len() > MAX_WINDOW_LEN {
        return Err(no("err_window_too_long", &[]));
    }
    let verbose = boolean(values, "-v", true);
    let gl_debug = boolean(values, "-gl-debug", false);
    let record_cursor = boolean(values, "-cursor", true);
    let date_folders = boolean(values, "-df", false);
    let restore_portal_session = boolean(values, "-restore-portal-session", false);
    let restart_replay_on_save = boolean(values, "-restart-replay-on-save", false);
    let mut overclock = boolean(values, "-oc", false);
    let fallback_cpu_encoding = boolean(values, "-fallback-cpu-encoding", false);

    let audio_bitrate: i64 = one(values, "-ab")
        .map(|v| v.parse::<i64>().expect("-ab range checked") * 1000)
        .unwrap_or(0);
    let keyint: f64 = one(values, "-keyint")
        .map(|v| v.parse::<f64>().expect("keyint checked"))
        .unwrap_or(2.0);

    if audio_codec == AudioCodec::Flac {
        warnings.push(format!(
            "gsr warning: {}",
            catalog.get("warn_flac_disabled_fallback_opus")
        ));
        audio_codec = AudioCodec::Opus;
    }

    let portal_token_file = one(values, "-portal-session-token-filepath").map(str::to_string);
    if let Some(path) = &portal_token_file {
        if path.ends_with('/') {
            return Err(no("err_token_path_dir", &[("path", path)]));
        }
    }

    let mut saved_script = one(values, "-sc").map(str::to_string);
    if let Some(path) = &saved_script {
        match fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_file() => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if m.permissions().mode() & 0o100 == 0 {
                        return Err(no("err_script_not_executable", &[("path", path)]));
                    }
                }
            }
            _ => return Err(no("err_script_missing", &[("path", path)])),
        }
    }

    let quality = match bitrate_mode {
        BitrateMode::Cbr => {
            let Some(q) = one(values, "-q") else {
                return Err(no("err_quality_required_cbr", &[]));
            };
            match q.parse::<i64>() {
                Ok(bps) if bps < 0 => return Err(no("err_quality_negative", &[("got", q)])),
                Ok(bps) => Quality::Bitrate(bps * 1000),
                Err(_) => return Err(no("err_quality_cbr_integer", &[("got", q)])),
            }
        }
        _ => {
            let name = one(values, "-q").unwrap_or("very_high");
            match VideoQuality::from_cli_name(name) {
                Some(preset) => Quality::Preset(preset),
                None => return Err(no("err_quality_preset", &[("got", name)])),
            }
        }
    };

    let resolution_str = one(values, "-s");
    if resolution_str.is_none() && window == "focused" {
        return Err(no("err_output_s_required", &[]));
    }
    let mut output_resolution = (0i64, 0i64);
    if let Some(text) = resolution_str {
        let (w, h) =
            parse_pair(text, 'x').ok_or_else(|| no("err_resolution_format", &[("got", text)]))?;
        if w < 0 || h < 0 {
            return Err(no("err_resolution_negative", &[("got", text)]));
        }
        output_resolution = (w, h);
    }

    let region_str = one(values, "-region");
    let mut region = None;
    if let Some(text) = region_str {
        if window != "region" {
            return Err(no("err_region_requires_window", &[]));
        }
        let (size, pos) =
            parse_region(text).ok_or_else(|| no("err_region_format", &[("got", text)]))?;
        if size.0 < 0 || size.1 < 0 {
            return Err(no("err_region_negative", &[("got", text)]));
        }
        region = Some(Region {
            width: size.0,
            height: size.1,
            x: pos.0,
            y: pos.1,
        });
    } else if window == "region" {
        return Err(no("err_region_required", &[]));
    }

    let fps: i64 = one(values, "-f")
        .map(|v| v.parse::<i64>().expect("-f checked"))
        .unwrap_or(60);

    let replay_secs =
        one(values, "-r").map(|v| v.parse::<i64>().expect("-r checked") + (keyint + 0.5) as i64);

    let mut container = one(values, "-c").map(str::to_string);
    if container.as_deref() == Some("mkv") {
        container = Some("matroska".to_string());
    }

    let replaying = replay_secs.is_some();
    let output_opt = one(values, "-o").map(str::to_string);
    let had_output = output_opt.is_some();
    let output;
    if let Some(path) = output_opt {
        let livestream = is_livestream_path(&path);
        if livestream && replaying {
            return Err(no("err_replay_no_livestream", &[]));
        }
        if !livestream && !replaying {
            let parent = Path::new(&path).parent();
            if let Some(dir) = parent {
                if !dir.as_os_str().is_empty() && dir != Path::new("/") {
                    if let Err(e) = fs::create_dir_all(dir) {
                        let _ = e;
                        return Err(no("err_mkdir_output", &[("path", &path)]));
                    }
                }
            }
        }
        if !livestream && replaying {
            if container.is_none() {
                return Err(no("err_container_required_replay", &[]));
            }
            if let Ok(meta) = fs::symlink_metadata(&path) {
                if !meta.file_type().is_dir() {
                    return Err(no("err_output_not_dir", &[("path", &path)]));
                }
            }
        }
        output = path;
    } else if !replaying {
        output = DEFAULT_STDOUT_OUTPUT.to_string();
    } else {
        return Err(no("err_output_required_replay", &[]));
    }
    if !had_output && container.is_none() {
        return Err(no("err_container_required", &[]));
    }

    let is_livestream = is_livestream_path(&output);
    let is_output_piped = output == DEFAULT_STDOUT_OUTPUT;
    let low_latency = is_livestream || is_output_piped;

    let container_effective = match container.clone() {
        Some(c) => c,
        None => output
            .rsplit('/')
            .next()
            .and_then(|base| base.rsplit('.').next())
            .filter(|ext| *ext != output.rsplit('/').next().unwrap_or(""))
            .map(str::to_string)
            .unwrap_or_else(|| "mp4".to_string()),
    };

    let replay_dir = one(values, "-ro").map(str::to_string);

    if !restore_portal_session && window == "portal" {
        warnings.push(format!(
            "gsr info: {}",
            catalog.get("info_portal_no_restore")
        ));
    }
    if is_livestream && saved_script.is_some() {
        warnings.push(format!(
            "gsr warning: {}",
            catalog.get("warn_sc_ignored_livestream")
        ));
        saved_script = None;
    }
    if overclock {
        warnings.push(format!(
            "gsr warning: {}",
            catalog.get("warn_oc_requires_x11")
        ));
        overclock = false;
    }

    // `-w` source gating (after C-ordered validation above).
    match window.as_str() {
        "screen-direct" | "screen-direct-force" | "focused" => {
            return Err(no("err_unsupported_x11", &[]));
        }
        _ => {}
    }
    if window.starts_with("/dev/") {
        return Err(no("err_unsupported_future", &[]));
    }
    if !crate::config::contains_non_hex_number(&window)
        && !matches!(window.as_str(), "screen" | "portal" | "region")
    {
        return Err(no("err_unsupported_x11", &[]));
    }

    let audio_inputs: Vec<String> = values.get("-a").cloned().unwrap_or_default();
    let plugins: Vec<String> = values.get("-p").cloned().unwrap_or_default();
    let flag_lang = one(values, "--lang").unwrap_or("auto");
    let lang = match flag_lang {
        "auto" => lang,
        code => Lang::from_code(code).unwrap_or(Lang::En),
    };

    Ok((
        Config {
            window,
            container,
            container_effective,
            fps,
            output_resolution,
            region,
            audio_inputs,
            quality,
            output,
            replay_dir,
            replay_secs,
            restart_replay_on_save,
            video_codec,
            audio_codec,
            audio_bitrate,
            overclock,
            framerate_mode,
            bitrate_mode,
            pixfmt,
            verbose,
            gl_debug,
            date_folders,
            restore_portal_session,
            fallback_cpu_encoding,
            record_cursor,
            keyint,
            portal_token_file,
            saved_script,
            replay_storage,
            plugins,
            lang,
            color_range,
            tune,
            encoder_hw,
            is_livestream,
            is_output_piped,
            low_latency,
        },
        warnings,
    ))
}

/// Strict `AxB` integer pair (see `DEVIATION-STRICT-INT`).
fn parse_pair(text: &str, sep: char) -> Option<(i64, i64)> {
    let (a, b) = text.split_once(sep)?;
    if a.is_empty() || b.is_empty() {
        return None;
    }
    Some((a.parse::<i64>().ok()?, b.parse::<i64>().ok()?))
}

/// Strict `WxH+X+Y` region (see `DEVIATION-STRICT-INT`).
fn parse_region(text: &str) -> Option<((i64, i64), (i64, i64))> {
    let (size, pos) = text.split_once('+')?;
    let (w, h) = parse_pair(size, 'x')?;
    let (x, rest) = pos.split_once('+')?;
    let y: i64 = rest.parse().ok()?;
    let x: i64 = x.parse().ok()?;
    Some(((w, h), (x, y)))
}

/// Flag description keys in help order.
fn help_keys() -> Vec<&'static str> {
    vec![
        "help_flag_w",
        "help_flag_c",
        "help_flag_f",
        "help_flag_s",
        "help_flag_region",
        "help_flag_a",
        "help_flag_q",
        "help_flag_o",
        "help_flag_ro",
        "help_flag_r",
        "help_flag_restart",
        "help_flag_k",
        "help_flag_ac",
        "help_flag_ab",
        "help_flag_oc",
        "help_flag_fm",
        "help_flag_bm",
        "help_flag_pixfmt",
        "help_flag_v",
        "help_flag_gldebug",
        "help_flag_df",
        "help_flag_sc",
        "help_flag_cr",
        "help_flag_tune",
        "help_flag_cursor",
        "help_flag_keyint",
        "help_flag_restore",
        "help_flag_tokenfile",
        "help_flag_encoder",
        "help_flag_fallback",
        "help_flag_storage",
        "help_flag_p",
        "help_flag_lang",
    ]
}

/// Full `--help` text (snapshot-tested per language).
pub fn render_help(catalog: &Catalog) -> String {
    let mut out = format!("{}\n", catalog.get("help_usage"));
    for key in help_keys() {
        out.push_str(&format!("  {}\n", catalog.get(key)));
    }
    out
}

/// Short usage line printed after errors (mirrors C `usage()` placement).
pub fn render_usage(catalog: &Catalog) -> String {
    format!("{}\n", catalog.get("help_usage"))
}
