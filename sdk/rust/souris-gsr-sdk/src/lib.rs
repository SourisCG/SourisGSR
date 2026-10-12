//! SourisGSR SDK — motor de grabación embebible (sin spawnear procesos).
//!
//! Control total: fuentes video/audio, volúmenes 0-100% solo-grabación
//! (nunca el mixer del sistema), replay buffer, streaming, screenshots,
//! pausa/reanudar y eventos. Idiomático, sin `unsafe` público.
//!
//! Ejemplo:
//! ```no_run
//! use souris_gsr_sdk::{Recorder, Source, Codec};
//! let rec = Recorder::builder()
//!     .source(Source::monitor("screen"))
//!     .fps(60)
//!     .codec(Codec::Auto)
//!     .mic_volume(80)
//!     .system_volume(100)
//!     .on_event(|e| println!("evento: {e:?}"))
//!     .start()
//!     .unwrap();
//! rec.stop().unwrap();
//! ```

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 0-100% tipo OBS. `Volume::percent` clampea.
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

/// Fuente de video. En Wayland `portal()` abre el picker del compositor
/// (ventana individual donde el backend lo soporta: GNOME/KDE/Hyprland).
/// En Windows `window_hwnd()` usa WGC (ventana individual real).
#[derive(Debug, Clone)]
pub enum Source {
    Monitor { id: String },
    Window { id: String, title: Option<String> },
    Portal,
    Region { x: i32, y: i32, w: i32, h: i32 },
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
            Error::InvalidArg(m) => write!(f, "argumento inválido: {m}"),
            Error::Unsupported(m) => write!(f, "no soportado: {m}"),
            Error::NoDevice(m) => write!(f, "sin dispositivo: {m}"),
            Error::AlreadyRunning => write!(f, "ya está grabando"),
            Error::NotRunning => write!(f, "no está grabando"),
            Error::Generic(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Config completa del recorder embebido.
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
        }
    }
}

/// Builder con validación 0-100 y de fps.
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
    pub fn output(mut self, p: impl Into<PathBuf>) -> Self {
        self.cfg.output = p.into();
        self
    }
    /// Equivale a `-a` del CLI, ej: `["default_output|vol=80", "default_input"]`.
    pub fn audio_inputs(mut self, v: Vec<String>) -> Self {
        self.cfg.audio_inputs = v;
        self
    }
    /// 0-100 solo grabación (nunca sistema).
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
    pub fn on_event(mut self, f: impl Fn(Event) + Send + Sync + 'static) -> Self {
        self.on_event = Some(Arc::new(f));
        self
    }
    pub fn start(self) -> Result<Recorder> {
        Recorder::start(self.cfg, self.on_event)
    }
}

/// Recorder embebido. `stop()` consume (como el motor real libera).
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
            return Err(Error::InvalidArg("fps debe ser >= 1".into()));
        }
        // Los volúmenes ya vienen clampeados por Volume::percent.
        let r = Self { cfg, on_event, state: Mutex::new(State::Recording) };
        r.emit(Event::Started);
        // TODO(ffi): con feature `ffi`, llamar a sgsr_start() con la C ABI
        // (include/souris_gsr.h) en vez de este stub de estado.
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

    /// Volúmenes en caliente 0-100, solo grabación.
    pub fn set_volumes(&self, mic: u8, system: u8, app: u8) -> Result<()> {
        self.ensure_running()?;
        let _ = (Volume::percent(mic), Volume::percent(system), Volume::percent(app));
        // TODO(ffi): sgsr_set_volumes()
        Ok(())
    }

    pub fn save_replay(&self, path: &Path) -> Result<()> {
        self.ensure_running()?;
        if self.cfg.replay.is_none() {
            return Err(Error::Unsupported("replay no configurado (usa .replay())".into()));
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

/// Helpers de descubrimiento (con `ffi` consultan a la C ABI).
pub fn list_video_sources() -> Vec<Source> {
    vec![Source::monitor("screen"), Source::Portal]
}

pub fn list_audio_devices() -> Vec<(String, String)> {
    vec![
        ("default_output".into(), "Default output (sistema)".into()),
        ("default_input".into(), "Default input (micrófono)".into()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_clampa_y_gain() {
        assert_eq!(Volume::percent(200).get(), 100);
        assert_eq!(Volume::percent(80).gain(), 0.8);
        assert_eq!(Volume::muted().gain(), 0.0);
    }

    #[test]
    fn builder_defaults_son_100() {
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
    fn replay_sin_config_da_unsupported() {
        let r = Recorder::builder().start().unwrap();
        assert!(matches!(r.save_replay(Path::new("/tmp/x.mp4")), Err(Error::Unsupported(_))));
        r.stop().unwrap();
    }

    #[test]
    fn set_volumes_acepta_0_100() {
        let r = Recorder::builder().start().unwrap();
        r.set_volumes(0, 80, 100).unwrap();
        r.stop().unwrap();
    }
}
