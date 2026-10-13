//! SourisGSR SDK — embeddable recording engine (no child processes).
//!
//! Full control: video/audio sources, 0-100% recording-only volumes
//! (never the system mixer), replay buffer, streaming, screenshots,
//! pause/resume and events. Idiomatic, no public `unsafe`.
//!
//! Headless by construction: the SDK never creates windows, tray icons or
//! notifications (the system tray lives in `gsr-ui` only).
//!
//! Example:
//! ```no_run
//! use souris_gsr_sdk::{Recorder, Source, Codec};
//! let rec = Recorder::builder()
//!     .source(Source::monitor("screen"))
//!     .fps(60)
//!     .codec(Codec::Auto)
//!     .mic_volume(80)
//!     .system_volume(100)
//!     .on_event(|e| println!("event: {e:?}"))
//!     .start()
//!     .unwrap();
//! rec.stop().unwrap();
//! ```

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Raw + safe C ABI bindings (`ffi` feature, needs `libsouris_gsr` linked).
#[cfg(feature = "ffi")]
pub mod ffi;

/// OBS-style 0-100%. `Volume::percent` clamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume(u8);

impl Volume {
    pub fn percent(p: u8) -> Self {
        Self(p.min(100))
    }
    pub fn muted() -> Self {
        Self(0)
    }
    pub fn full() -> Self {
        Self(100)
    }
    pub fn get(self) -> u8 {
        self.0
    }
    pub fn gain(self) -> f32 {
        self.0 as f32 / 100.0
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self(100)
    }
}

/// Video source. On Wayland `portal()` opens the compositor picker
/// (individual window where the backend supports it: GNOME/KDE/Hyprland).
/// On Windows `window_hwnd()` uses WGC (a real individual window).
#[derive(Debug, Clone)]
pub enum Source {
    Monitor { id: String },
    Window { id: String, title: Option<String> },
    Portal,
    Region { x: i32, y: i32, w: i32, h: i32 },
    Focused,
}

impl Source {
    pub fn monitor(id: impl Into<String>) -> Self {
        Self::Monitor { id: id.into() }
    }
    pub fn window(id: impl Into<String>) -> Self {
        Self::Window { id: id.into(), title: None }
    }
    pub fn window_hwnd(hwnd: u64) -> Self {
        Self::Window { id: hwnd.to_string(), title: None }
    }
    pub fn portal() -> Self {
        Self::Portal
    }
    pub fn region(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self::Region { x, y, w, h }
    }
    pub fn focused() -> Self {
        Self::Focused
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Codec {
    #[default]
    Auto,
    H264,
    Hevc,
    Av1,
}

impl Codec {
    pub fn as_str(self) -> &'static str {
        match self {
            Codec::Auto => "auto",
            Codec::H264 => "h264",
            Codec::Hevc => "hevc",
            Codec::Av1 => "av1",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Started,
    Stopped,
    Paused,
    Resumed,
    ReplaySaved { path: PathBuf },
    FrameDropped { count: u64 },
    LevelMeter { value: f32 },
    Error { message: String },
}

#[derive(Debug)]
pub enum Error {
    InvalidArg(String),
    Unsupported(String),
    NoDevice(String),
    AlreadyRunning,
    NotRunning,
    Generic(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidArg(m) => write!(f, "invalid argument: {m}"),
            Error::Unsupported(m) => write!(f, "unsupported: {m}"),
            Error::NoDevice(m) => write!(f, "no device: {m}"),
            Error::AlreadyRunning => write!(f, "already recording"),
            Error::NotRunning => write!(f, "not recording"),
            Error::Generic(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Full config of the embedded recorder.
#[derive(Debug, Clone)]
pub struct Config {
    pub source: Source,
    pub fps: u32,
    pub codec: Codec,
    pub container: String,
    pub output: PathBuf,
    pub audio_inputs: Vec<String>,
    pub mic_volume: Volume,
    pub system_volume: Volume,
    pub app_volume: Volume,
    pub replay: Option<Duration>,
    pub record_cursor: bool,
    pub portal_token_path: Option<PathBuf>,
    pub restore_portal: bool,
}

impl Config {
    /// True for rtmp/srt/rtsp/http(s) streaming URLs (mirrors sgsr_is_livestream_path).
    pub fn is_stream_url(&self) -> bool {
        let s = self.output.to_string_lossy();
        s.starts_with("rtmp://")
            || s.starts_with("rtmps://")
            || s.starts_with("rtsp://")
            || s.starts_with("srt://")
            || s.starts_with("tcp://")
            || s.starts_with("udp://")
            || s.starts_with("http://")
            || s.starts_with("https://")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source: Source::Monitor { id: "screen".into() },
            fps: 60,
            codec: Codec::Auto,
            container: "mp4".into(),
            output: PathBuf::from("video.mp4"),
            audio_inputs: vec!["default_output".into()],
            mic_volume: Volume::full(),
            system_volume: Volume::full(),
            app_volume: Volume::full(),
            replay: None,
            record_cursor: true,
            portal_token_path: None,
            restore_portal: false,
        }
    }
}

/// Builder with 0-100 and fps validation.
pub struct Builder {
    cfg: Config,
    on_event: Option<Arc<dyn Fn(Event) + Send + Sync>>,
}

impl Builder {
    pub fn source(mut self, s: Source) -> Self {
        self.cfg.source = s;
        self
    }
    pub fn fps(mut self, fps: u32) -> Self {
        self.cfg.fps = fps.clamp(1, 1000);
        self
    }
    pub fn codec(mut self, c: Codec) -> Self {
        self.cfg.codec = c;
        self
    }
    pub fn container(mut self, c: impl Into<String>) -> Self {
        self.cfg.container = c.into();
        self
    }
    pub fn output(mut self, p: impl Into<PathBuf>) -> Self {
        self.cfg.output = p.into();
        self
    }
    /// Streaming URL (rtmp/srt/rtsp/http). Same field as output; the engine
    /// auto-detects livestream mode like the CLI -o URL.
    pub fn output_url(mut self, url: impl Into<String>) -> Self {
        self.cfg.output = PathBuf::from(url.into());
        self
    }
    /// Same as CLI `-a`, e.g. `["default_output|vol=80", "default_input"]`.
    pub fn audio_inputs(mut self, v: Vec<String>) -> Self {
        self.cfg.audio_inputs = v;
        self
    }
    /// 0-100 recording only (never the system).
    pub fn mic_volume(mut self, p: u8) -> Self {
        self.cfg.mic_volume = Volume::percent(p);
        self
    }
    pub fn system_volume(mut self, p: u8) -> Self {
        self.cfg.system_volume = Volume::percent(p);
        self
    }
    pub fn app_volume(mut self, p: u8) -> Self {
        self.cfg.app_volume = Volume::percent(p);
        self
    }
    pub fn replay(mut self, d: Duration) -> Self {
        self.cfg.replay = Some(d);
        self
    }
    pub fn record_cursor(mut self, v: bool) -> Self {
        self.cfg.record_cursor = v;
        self
    }
    /// Wayland portal session persistence (mirrors -restore-portal-session +
    /// -portal-session-token-filepath). No-op on X11.
    pub fn portal_token(mut self, p: impl Into<PathBuf>) -> Self {
        self.cfg.portal_token_path = Some(p.into());
        self
    }
    pub fn restore_portal(mut self, v: bool) -> Self {
        self.cfg.restore_portal = v;
        self
    }
    pub fn on_event(mut self, f: impl Fn(Event) + Send + Sync + 'static) -> Self {
        self.on_event = Some(Arc::new(f));
        self
    }
    pub fn start(self) -> Result<Recorder> {
        Recorder::start(self.cfg, self.on_event)
    }
    /// Real engine (requires `ffi` feature + libsouris_gsr linked).
    /// Unlike `start()` (offline mock), this brings up windowing+EGL and runs
    /// the real pipeline on a worker thread. Headless CI without display/GPU
    /// returns `NoDevice`/`Generic` — map it, don't unwrap in unit tests.
    #[cfg(feature = "ffi")]
    pub fn start_engine(self) -> Result<Engine> {
        Engine::start(self.cfg, self.on_event)
    }
}

/// Embedded recorder. `stop()` consumes (like the real engine frees).
pub struct Recorder {
    cfg: Config,
    on_event: Option<Arc<dyn Fn(Event) + Send + Sync>>,
    state: Mutex<State>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Recording,
    Paused,
    Stopped,
}

impl Recorder {
    pub fn builder() -> Builder {
        Builder { cfg: Config::default(), on_event: None }
    }

    fn start(cfg: Config, on_event: Option<Arc<dyn Fn(Event) + Send + Sync>>) -> Result<Self> {
        if cfg.fps == 0 {
            return Err(Error::InvalidArg("fps must be >= 1".into()));
        }
        // Volumes already come clamped via Volume::percent.
        let r = Self { cfg, on_event, state: Mutex::new(State::Recording) };
        r.emit(Event::Started);
        // Note: with the `ffi` feature, `ffi::` exposes the same surface
        // against the real engine (see ffi module docs to link libsouris_gsr).
        Ok(r)
    }

    fn emit(&self, e: Event) {
        if let Some(cb) = &self.on_event {
            cb(e);
        }
    }

    fn ensure_running(&self) -> Result<()> {
        match *self.state.lock().unwrap() {
            State::Recording | State::Paused => Ok(()),
            State::Stopped => Err(Error::NotRunning),
        }
    }

    pub fn pause(&self) -> Result<()> {
        self.ensure_running()?;
        *self.state.lock().unwrap() = State::Paused;
        self.emit(Event::Paused);
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        let mut s = self.state.lock().unwrap();
        if *s != State::Paused {
            return Err(Error::NotRunning);
        }
        *s = State::Recording;
        drop(s);
        self.emit(Event::Resumed);
        Ok(())
    }

    /// Hot 0-100 volumes, recording only.
    pub fn set_volumes(&self, mic: u8, system: u8, app: u8) -> Result<()> {
        self.ensure_running()?;
        let _ = (Volume::percent(mic), Volume::percent(system), Volume::percent(app));
        // TODO(ffi): sgsr_set_volumes()
        Ok(())
    }

    pub fn save_replay(&self, path: &Path) -> Result<()> {
        self.ensure_running()?;
        if self.cfg.replay.is_none() {
            return Err(Error::Unsupported("replay not configured (use .replay())".into()));
        }
        self.emit(Event::ReplaySaved { path: path.to_path_buf() });
        Ok(())
    }

    pub fn screenshot(&self, _path: &Path) -> Result<()> {
        self.ensure_running()?;
        // TODO(ffi): sgsr_screenshot()
        Ok(())
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    pub fn stop(self) -> Result<()> {
        let mut s = self.state.lock().unwrap();
        if *s == State::Stopped {
            return Err(Error::NotRunning);
        }
        *s = State::Stopped;
        drop(s);
        self.emit(Event::Stopped);
        Ok(())
    }
}

/// Discovery helpers (with `ffi` they query the C ABI).
#[cfg(not(feature = "ffi"))]
pub fn list_video_sources() -> Vec<Source> {
    vec![Source::monitor("screen"), Source::Portal]
}

#[cfg(feature = "ffi")]
pub fn list_video_sources() -> Vec<Source> {
    match ffi::list_video_sources() {
        Ok(v) => v
            .into_iter()
            .map(|(id, title, kind)| match kind {
                ffi::RawSourceKind::Monitor => Source::Monitor { id },
                ffi::RawSourceKind::Window => Source::Window { id, title: Some(title) },
                ffi::RawSourceKind::Portal => Source::Portal,
                ffi::RawSourceKind::Region => Source::Region { x: 0, y: 0, w: 0, h: 0 },
                ffi::RawSourceKind::Focused => Source::Focused,
            })
            .collect(),
        Err(_) => vec![Source::monitor("screen"), Source::Portal],
    }
}

#[cfg(not(feature = "ffi"))]
pub fn list_audio_devices() -> Vec<(String, String)> {
    vec![
        ("default_output".into(), "Default output (system)".into()),
        ("default_input".into(), "Default input (microphone)".into()),
    ]
}

#[cfg(feature = "ffi")]
pub fn list_audio_devices() -> Vec<(String, String)> {
    match ffi::list_audio_devices() {
        Ok(v) => v.into_iter().map(|(n, d, _)| (n, d)).collect(),
        Err(_) => vec![
            ("default_output".into(), "Default output (system)".into()),
            ("default_input".into(), "Default input (microphone)".into()),
        ],
    }
}

/// Real engine recorder (ffi only). Wraps `sgsr_*` handle + event trampoline.
/// `stop()` consumes like the C engine frees. All control calls are thread-safe.
#[cfg(feature = "ffi")]
pub struct Engine {
    raw: std::sync::Mutex<*mut ffi::RawRecorder>,
    cfg: Config,
    on_event: Option<Arc<dyn Fn(Event) + Send + Sync>>,
    // Owns the boxed Arc passed as userdata to C (reclaimed on stop).
    holder: std::sync::Mutex<*mut std::ffi::c_void>,
}

#[cfg(feature = "ffi")]
struct EngineHolder {
    cb: Arc<dyn Fn(Event) + Send + Sync>,
}

#[cfg(feature = "ffi")]
unsafe extern "C" fn engine_event_trampoline(
    ev: *const ffi::RawEvent,
    userdata: *mut std::ffi::c_void,
) {
    if ev.is_null() || userdata.is_null() {
        return;
    }
    let holder = &*(userdata as *const EngineHolder);
    let r = &*ev;
    let msg = {
        let bytes: Vec<u8> = r.message.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    };
    let e = match r.kind {
        ffi::RawEventKind::Started => Event::Started,
        ffi::RawEventKind::Stopped => Event::Stopped,
        ffi::RawEventKind::Paused => Event::Paused,
        ffi::RawEventKind::Resumed => Event::Resumed,
        ffi::RawEventKind::ReplaySaved => Event::ReplaySaved { path: PathBuf::from(msg) },
        ffi::RawEventKind::FrameDropped => Event::FrameDropped { count: r.code as u64 },
        ffi::RawEventKind::LevelMeter => Event::LevelMeter { value: r.data0 as f32 },
        ffi::RawEventKind::Error => Event::Error { message: msg },
    };
    (holder.cb)(e);
}

#[cfg(feature = "ffi")]
fn map_raw(e: ffi::RawError, what: &str) -> Error {
    match e {
        ffi::RawError::Ok => Error::Generic(format!("{what}: ok as error?")),
        ffi::RawError::InvalidArg => Error::InvalidArg(what.into()),
        ffi::RawError::Unsupported => Error::Unsupported(what.into()),
        ffi::RawError::NoDevice => Error::NoDevice(what.into()),
        ffi::RawError::AlreadyRunning => Error::AlreadyRunning,
        ffi::RawError::NotRunning => Error::NotRunning,
        ffi::RawError::Generic => Error::Generic(what.into()),
    }
}

#[cfg(feature = "ffi")]
impl Engine {
    fn start(cfg: Config, on_event: Option<Arc<dyn Fn(Event) + Send + Sync>>) -> Result<Self> {
        use std::ffi::CString;
        if cfg.fps == 0 || cfg.fps > 1000 {
            return Err(Error::InvalidArg("fps must be 1-1000".into()));
        }
        if cfg.audio_inputs.len() > 8 {
            return Err(Error::InvalidArg("max 8 audio inputs".into()));
        }
        // SAFETY: sgsr_* is thread-safe; config strings live through sgsr_start (copied).
        unsafe {
            if ffi::sgsr_init() != ffi::RawError::Ok {
                return Err(Error::Generic("sgsr_init failed".into()));
            }
        }
        let codec = CString::new(cfg.codec.as_str()).unwrap();
        let container = CString::new(cfg.container.clone()).unwrap();
        let output = CString::new(cfg.output.to_string_lossy().as_bytes()).unwrap();
        let audio_c: Vec<CString> = cfg
            .audio_inputs
            .iter()
            .map(|s| CString::new(s.as_str()).unwrap_or_default())
            .collect();
        let mut audio_ptrs: [*const std::os::raw::c_char; 8] = [std::ptr::null(); 8];
        for (i, c) in audio_c.iter().enumerate().take(8) {
            audio_ptrs[i] = c.as_ptr();
        }
        let token_c = cfg
            .portal_token_path
            .as_ref()
            .map(|p| CString::new(p.to_string_lossy().as_bytes()).unwrap_or_default());
        // Event holder: Box<EngineHolder> -> raw userdata, reclaimed on stop.
        // Holder is a concrete sized struct (thin pointer), safe for c_void.
        let holder: *mut std::ffi::c_void = if let Some(cb) = &on_event {
            let b = Box::new(EngineHolder { cb: Arc::clone(cb) });
            Box::into_raw(b) as *mut std::ffi::c_void
        } else {
            std::ptr::null_mut()
        };
        let (kind, id, title, w, h, x, y) = match &cfg.source {
            Source::Monitor { id } => (ffi::RawSourceKind::Monitor, id.clone(), String::new(), 0, 0, 0, 0),
            Source::Window { id, title } => (
                ffi::RawSourceKind::Window,
                id.clone(),
                title.clone().unwrap_or_default(),
                0,
                0,
                0,
                0,
            ),
            Source::Portal => (ffi::RawSourceKind::Portal, String::new(), String::new(), 0, 0, 0, 0),
            Source::Region { x, y, w, h } => (
                ffi::RawSourceKind::Region,
                String::new(),
                String::new(),
                *w,
                *h,
                *x,
                *y,
            ),
            Source::Focused => {
                // Focused needs output size; use 1920x1080 fallback if unset.
                (ffi::RawSourceKind::Focused, String::new(), String::new(), 1920, 1080, 0, 0)
            }
        };
        let mut raw_src = ffi::RawSource {
            kind,
            id: [0; 256],
            title: [0; 256],
            width: w,
            height: h,
            x,
            y,
        };
        for (i, b) in id.bytes().take(255).enumerate() {
            raw_src.id[i] = b as std::os::raw::c_char;
        }
        for (i, b) in title.bytes().take(255).enumerate() {
            raw_src.title[i] = b as std::os::raw::c_char;
        }
        // Focused with explicit w/h from Region? No, keep fallback.
        let raw_cfg = ffi::RawConfig {
            video_source: raw_src,
            fps: cfg.fps as std::os::raw::c_int,
            video_codec: codec.as_ptr(),
            container: container.as_ptr(),
            output_path: output.as_ptr(),
            audio_inputs: audio_ptrs,
            num_audio_inputs: cfg.audio_inputs.len() as std::os::raw::c_int,
            mic_volume_percent: cfg.mic_volume.get() as std::os::raw::c_int,
            system_volume_percent: cfg.system_volume.get() as std::os::raw::c_int,
            app_volume_percent: cfg.app_volume.get() as std::os::raw::c_int,
            replay_buffer_secs: cfg.replay.as_ref().map(|d| d.as_secs() as std::os::raw::c_int).unwrap_or(0),
            record_cursor: if cfg.record_cursor { 1 } else { 0 },
            portal_session_token_path: token_c.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null()),
            restore_portal_session: if cfg.restore_portal { 1 } else { 0 },
            on_event: if holder.is_null() {
                None
            } else {
                Some(engine_event_trampoline)
            },
            event_userdata: holder,
        };
        let mut out: *mut ffi::RawRecorder = std::ptr::null_mut();
        let rc = unsafe { ffi::sgsr_start(&raw_cfg, &mut out) };
        if rc != ffi::RawError::Ok || out.is_null() {
            if !holder.is_null() {
                unsafe {
                    let _ = Box::from_raw(holder as *mut EngineHolder);
                }
            }
            unsafe { ffi::sgsr_shutdown() };
            return Err(map_raw(rc, &format!("sgsr_start: {}", cfg.output.display())));
        }
        Ok(Self {
            raw: std::sync::Mutex::new(out),
            cfg,
            on_event,
            holder: std::sync::Mutex::new(holder),
        })
    }

    fn raw_ptr(&self) -> *mut ffi::RawRecorder {
        *self.raw.lock().unwrap()
    }

    fn emit(&self, e: Event) {
        if let Some(cb) = &self.on_event {
            cb(e);
        }
    }

    pub fn pause(&self) -> Result<()> {
        let rc = unsafe { ffi::sgsr_pause(self.raw_ptr()) };
        if rc != ffi::RawError::Ok {
            return Err(map_raw(rc, "pause"));
        }
        self.emit(Event::Paused);
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        let rc = unsafe { ffi::sgsr_resume(self.raw_ptr()) };
        if rc != ffi::RawError::Ok {
            return Err(map_raw(rc, "resume"));
        }
        self.emit(Event::Resumed);
        Ok(())
    }

    pub fn set_volumes(&self, mic: u8, system: u8, app: u8) -> Result<()> {
        let rc = unsafe { ffi::sgsr_set_volumes(self.raw_ptr(), mic as i32, system as i32, app as i32) };
        if rc != ffi::RawError::Ok {
            return Err(map_raw(rc, "set_volumes 0-100"));
        }
        Ok(())
    }

    pub fn save_replay(&self, path: &Path) -> Result<()> {
        if self.cfg.replay.is_none() {
            return Err(Error::Unsupported("replay not configured (use .replay())".into()));
        }
        let c = ffi::path_to_cstring(path).map_err(|_| Error::InvalidArg("bad replay path".into()))?;
        let rc = unsafe { ffi::sgsr_save_replay(self.raw_ptr(), c.as_ptr()) };
        if rc != ffi::RawError::Ok {
            return Err(map_raw(rc, "save_replay"));
        }
        Ok(())
    }

    pub fn screenshot(&self, path: &Path) -> Result<()> {
        let c = ffi::path_to_cstring(path).map_err(|_| Error::InvalidArg("bad screenshot path".into()))?;
        let rc = unsafe { ffi::sgsr_screenshot(self.raw_ptr(), c.as_ptr()) };
        if rc != ffi::RawError::Ok {
            return Err(map_raw(rc, "screenshot"));
        }
        Ok(())
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    pub fn stop(self) -> Result<()> {
        let raw = *self.raw.lock().unwrap();
        let holder = *self.holder.lock().unwrap();
        let rc = unsafe { ffi::sgsr_stop(raw) };
        if !holder.is_null() {
            unsafe {
                let _ = Box::from_raw(holder as *mut EngineHolder);
            }
        }
        unsafe { ffi::sgsr_shutdown() };
        if rc != ffi::RawError::Ok {
            return Err(map_raw(rc, "stop"));
        }
        self.emit(Event::Stopped);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_clamps_and_gain() {
        assert_eq!(Volume::percent(200).get(), 100);
        assert_eq!(Volume::percent(80).gain(), 0.8);
        assert_eq!(Volume::muted().gain(), 0.0);
    }

    #[test]
    fn builder_defaults_are_100() {
        let c = Config::default();
        assert_eq!(c.mic_volume.get(), 100);
        assert_eq!(c.system_volume.get(), 100);
        assert_eq!(c.app_volume.get(), 100);
    }

    #[test]
    fn start_pause_resume_stop() {
        let r = Recorder::builder().source(Source::portal()).fps(60).start().unwrap();
        r.pause().unwrap();
        r.resume().unwrap();
        r.stop().unwrap();
    }

    #[test]
    fn replay_without_config_is_unsupported() {
        let r = Recorder::builder().start().unwrap();
        assert!(matches!(r.save_replay(Path::new("/tmp/x.mp4")), Err(Error::Unsupported(_))));
        r.stop().unwrap();
    }

    #[test]
    fn set_volumes_accepts_0_100() {
        let r = Recorder::builder().start().unwrap();
        r.set_volumes(0, 80, 100).unwrap();
        r.stop().unwrap();
    }

    #[test]
    fn source_helpers_and_stream_detect() {
        let c = Config {
            output: PathBuf::from("rtmp://live/test"),
            ..Config::default()
        };
        assert!(c.is_stream_url());
        let c2 = Config::default();
        assert!(!c2.is_stream_url());
        let _ = Source::region(10, 20, 800, 600);
        let _ = Source::focused();
    }

    #[cfg(feature = "ffi")]
    #[test]
    fn engine_rejects_bad_args_without_display() {
        // Null-equivalent via invalid fps is caught in Rust before C.
        let r = (Builder { cfg: Config { fps: 0, ..Config::default() }, on_event: None }).start_engine();
        assert!(matches!(r, Err(Error::InvalidArg(_))));
        // Too many audio inputs.
        let mut cfg = Config::default();
        cfg.audio_inputs = vec!["x".to_string(); 9];
        let r = (Builder { cfg, on_event: None }).start_engine();
        assert!(matches!(r, Err(Error::InvalidArg(_))));
    }

    #[cfg(feature = "ffi")]
    #[test]
    fn engine_start_fails_cleanly_headless_or_no_gpu() {
        // Portal start headless should fail with NoDevice/Generic, not crash.
        // Accept Ok too (real display present) but then stop it.
        let cfg = Config {
            source: Source::Portal,
            output: PathBuf::from("/tmp/sgsr-engine-test.mp4"),
            ..Config::default()
        };
        match (Builder { cfg, on_event: None }).start_engine() {
            Ok(e) => {
                // Started: pause/resume/volumes/screenshot paths (screenshot may fail w/o frames, ignore).
                let _ = e.pause();
                let _ = e.resume();
                let _ = e.set_volumes(80, 90, 100);
                let _ = e.screenshot(Path::new("/tmp/sgsr-engine-shot.jpg"));
                let _ = e.stop();
            }
            Err(Error::NoDevice(_)) | Err(Error::Generic(_)) | Err(Error::Unsupported(_)) => {}
            Err(e) => panic!("unexpected engine error: {e:?}"),
        }
    }
}
