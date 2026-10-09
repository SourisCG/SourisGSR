//! Validated configuration. Mirrors `src/defs.c` + `include/defs.h`
//! (frozen reference v5.10.2, including its `TODO: Vulkan` gaps).

use gsr_i18n::Lang;

/// Video codec. Mirrors `gsr_video_codec` (internal `_VULKAN` variants exist
/// even though the v5.10.2 CLI does not expose them; see `cli_names`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    Auto,
    H264,
    Hevc,
    HevcHdr,
    Hevc10Bit,
    Av1,
    Av1Hdr,
    Av110Bit,
    Vp8,
    Vp9,
    H264Vulkan,
    HevcVulkan,
}

impl VideoCodec {
    /// Parse a `-k` value (`h265` is an alias of `hevc`, as in C).
    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name {
            "auto" => Some(Self::Auto),
            "h264" => Some(Self::H264),
            "h265" | "hevc" => Some(Self::Hevc),
            "hevc_hdr" => Some(Self::HevcHdr),
            "hevc_10bit" => Some(Self::Hevc10Bit),
            "av1" => Some(Self::Av1),
            "av1_hdr" => Some(Self::Av1Hdr),
            "av1_10bit" => Some(Self::Av110Bit),
            "vp8" => Some(Self::Vp8),
            "vp9" => Some(Self::Vp9),
            _ => None,
        }
    }

    /// Accepted `-k` names, in C table order.
    pub fn cli_names() -> &'static [&'static str] {
        &[
            "auto",
            "h264",
            "h265",
            "hevc",
            "hevc_hdr",
            "hevc_10bit",
            "av1",
            "av1_hdr",
            "av1_10bit",
            "vp8",
            "vp9",
        ]
    }

    /// Mirrors `video_codec_is_hdr` (TODO: Vulkan, as in C).
    pub fn is_hdr(self) -> bool {
        // TODO: Vulkan
        matches!(self, Self::HevcHdr | Self::Av1Hdr)
    }

    /// Mirrors `hdr_video_codec_to_sdr_video_codec` (TODO: Vulkan, as in C).
    pub fn to_sdr(self) -> Self {
        // TODO: Vulkan
        match self {
            Self::HevcHdr => Self::Hevc,
            Self::Av1Hdr => Self::Av1,
            other => other,
        }
    }

    /// Mirrors `video_codec_to_bit_depth` (TODO: 10-bit Vulkan, as in C).
    pub fn bit_depth(self) -> u8 {
        // TODO: 10-bit Vulkan
        match self {
            Self::HevcHdr | Self::Hevc10Bit | Self::Av1Hdr | Self::Av110Bit => 10,
            _ => 8,
        }
    }

    /// Mirrors `video_codec_to_string` (`""` for `Auto`, as in C).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "",
            Self::H264 => "h264",
            Self::Hevc => "hevc",
            Self::HevcHdr => "hevc_hdr",
            Self::Hevc10Bit => "hevc_10bit",
            Self::Av1 => "av1",
            Self::Av1Hdr => "av1_hdr",
            Self::Av110Bit => "av1_10bit",
            Self::Vp8 => "vp8",
            Self::Vp9 => "vp9",
            Self::H264Vulkan => "h264_vulkan",
            Self::HevcVulkan => "hevc_vulkan",
        }
    }

    /// Mirrors `video_codec_is_av1` (TODO: Vulkan, as in C).
    pub fn is_av1(self) -> bool {
        // TODO: Vulkan
        matches!(self, Self::Av1 | Self::Av1Hdr | Self::Av110Bit)
    }

    /// Mirrors `video_codec_is_vulkan`.
    pub fn is_vulkan(self) -> bool {
        matches!(self, Self::H264Vulkan | Self::HevcVulkan)
    }
}

/// Audio codec. Mirrors `gsr_audio_codec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodec {
    Aac,
    Opus,
    Flac,
}

impl AudioCodec {
    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name {
            "aac" => Some(Self::Aac),
            "opus" => Some(Self::Opus),
            "flac" => Some(Self::Flac),
            _ => None,
        }
    }

    pub fn cli_names() -> &'static [&'static str] {
        &["opus", "aac", "flac"]
    }

    /// Mirrors `audio_codec_get_name`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aac => "aac",
            Self::Opus => "opus",
            Self::Flac => "flac",
        }
    }
}

/// Pixel format. Mirrors `gsr_pixel_format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Yuv420,
    Yuv444,
}

impl PixelFormat {
    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name {
            "yuv420" => Some(Self::Yuv420),
            "yuv444" => Some(Self::Yuv444),
            _ => None,
        }
    }

    pub fn cli_names() -> &'static [&'static str] {
        &["yuv420", "yuv444"]
    }
}

/// Frame rate mode. Mirrors `gsr_framerate_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramerateMode {
    Constant,
    Variable,
    Content,
}

impl FramerateMode {
    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name {
            "cfr" => Some(Self::Constant),
            "vfr" => Some(Self::Variable),
            "content" => Some(Self::Content),
            _ => None,
        }
    }

    pub fn cli_names() -> &'static [&'static str] {
        &["vfr", "cfr", "content"]
    }
}

/// Bitrate mode. `Auto` resolves to QP outside the CLI (T40, needs GPU info
/// for the Steam Deck exception in `args_parser_validate_with_gl_info`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitrateMode {
    Auto,
    Qp,
    Cbr,
    Vbr,
}

impl BitrateMode {
    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name {
            "auto" => Some(Self::Auto),
            "qp" => Some(Self::Qp),
            "cbr" => Some(Self::Cbr),
            "vbr" => Some(Self::Vbr),
            _ => None,
        }
    }

    pub fn cli_names() -> &'static [&'static str] {
        &["auto", "qp", "cbr", "vbr"]
    }
}

/// Color range. Mirrors `gsr_color_range`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorRange {
    Limited,
    Full,
}

impl ColorRange {
    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name {
            "limited" => Some(Self::Limited),
            "full" => Some(Self::Full),
            _ => None,
        }
    }

    pub fn cli_names() -> &'static [&'static str] {
        &["limited", "full"]
    }
}

/// Encoding tune. Mirrors `gsr_tune` (NVIDIA only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tune {
    Performance,
    Quality,
}

impl Tune {
    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name {
            "performance" => Some(Self::Performance),
            "quality" => Some(Self::Quality),
            _ => None,
        }
    }

    pub fn cli_names() -> &'static [&'static str] {
        &["performance", "quality"]
    }
}

/// Video quality preset. Mirrors `gsr_video_quality`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoQuality {
    Medium,
    High,
    VeryHigh,
    Ultra,
}

impl VideoQuality {
    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name {
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "very_high" => Some(Self::VeryHigh),
            "ultra" => Some(Self::Ultra),
            _ => None,
        }
    }
}

/// `-q` value: preset for QP/VBR, bitrate (bps) for CBR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Preset(VideoQuality),
    Bitrate(i64),
}

/// Encoder hardware. Mirrors `gsr_video_encoder_hardware` (CPU is H264 only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderHw {
    Gpu,
    Cpu,
}

impl EncoderHw {
    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name {
            "gpu" => Some(Self::Gpu),
            "cpu" => Some(Self::Cpu),
            _ => None,
        }
    }

    pub fn cli_names() -> &'static [&'static str] {
        &["gpu", "cpu"]
    }
}

/// Replay storage. Mirrors `gsr_replay_storage`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayStorage {
    Ram,
    Disk,
}

impl ReplayStorage {
    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name {
            "ram" => Some(Self::Ram),
            "disk" => Some(Self::Disk),
            _ => None,
        }
    }

    pub fn cli_names() -> &'static [&'static str] {
        &["ram", "disk"]
    }
}

/// Capture region (`-region WxH+X+Y`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub width: i64,
    pub height: i64,
    pub x: i64,
    pub y: i64,
}

/// Validated recorder configuration. Mirrors the `args_parser` struct
/// (`include/args_parser.h`) after `args_parser_set_values`.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub window: String,
    /// Raw `-c` value (`mkv` normalized to `matroska`); `None` derives from `-o`.
    pub container: Option<String>,
    /// Effective container after `-o` extension fallback.
    pub container_effective: String,
    pub fps: i64,
    /// `(0, 0)` keeps the original resolution.
    pub output_resolution: (i64, i64),
    pub region: Option<Region>,
    pub audio_inputs: Vec<String>,
    pub quality: Quality,
    /// `/dev/stdout` when `-o` is absent without replay.
    pub output: String,
    pub replay_dir: Option<String>,
    /// `None` without `-r`; includes the C `keyint + 0.5` padding.
    pub replay_secs: Option<i64>,
    pub restart_replay_on_save: bool,
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    /// Millibit units (`-ab` x1000); `0` means automatic.
    pub audio_bitrate: i64,
    /// Parsed `-oc`; always neutralized on our platforms with a warning.
    pub overclock: bool,
    pub framerate_mode: FramerateMode,
    pub bitrate_mode: BitrateMode,
    pub pixfmt: PixelFormat,
    pub verbose: bool,
    pub gl_debug: bool,
    pub date_folders: bool,
    pub restore_portal_session: bool,
    pub fallback_cpu_encoding: bool,
    pub record_cursor: bool,
    pub keyint: f64,
    pub portal_token_file: Option<String>,
    /// Nulled when livestreaming (C behavior + warning).
    pub saved_script: Option<String>,
    pub replay_storage: ReplayStorage,
    pub plugins: Vec<String>,
    pub lang: Lang,
    pub color_range: ColorRange,
    pub tune: Tune,
    pub encoder_hw: EncoderHw,
    pub is_livestream: bool,
    pub is_output_piped: bool,
    pub low_latency: bool,
}

/// Default `-o` target when absent without replay (mirrors C).
pub const DEFAULT_STDOUT_OUTPUT: &str = "/dev/stdout";

/// Maximum `-w` length (`char window[64]` in C).
/// DEVIATION: C silently truncates via `snprintf`; we reject overlong
/// values with `err_unsupported_future`-free explicit handling in `cli.rs`
/// (safer, tested).
pub const MAX_WINDOW_LEN: usize = 63;

/// URL schemes that mark `-o` as a live stream.
/// Mirrors `is_livestream_path` in `src/args_parser.c`.
pub fn is_livestream_path(path: &str) -> bool {
    [
        "http://", "https://", "rtmp://", "rtmps://", "rtsp://", "srt://", "tcp://", "udp://",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

/// Mirrors `contains_non_hex_number` in `src/main.cpp`: optional `0x`
/// prefix, then hex digits; all-digit strings (with or without `0x`) are
/// X11 window ids, anything else is a monitor name or keyword.
pub fn contains_non_hex_number(s: &str) -> bool {
    let (hex_start, t) = match s.strip_prefix("0x") {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let mut is_hex = false;
    for c in t.chars() {
        if !c.is_ascii_hexdigit() {
            return true;
        }
        if c.is_ascii_alphabetic() {
            is_hex = true;
        }
    }
    is_hex && !hex_start
}
