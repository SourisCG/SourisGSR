//! Raw + safe bindings to the SourisGSR C ABI (`include/souris_gsr.h`).
//!
//! Enable with the `ffi` feature and link `libsouris_gsr` (built from
//! `core/` with meson, then archived, e.g.):
//! ```sh
//! # from the repo root:
//! meson setup /tmp/sgsr-build core && meson compile -C /tmp/sgsr-build
//! SOURIS_GSR_LIB_DIR=/tmp/sgsr-native cargo test --features ffi -p souris-gsr-sdk
//! ```
//! `SOURIS_GSR_LIB_DIR` must contain `libsouris_gsr.a` (or recognize the
//! system library path). Without it, only `cargo check` works (no linking).

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int};
use std::path::Path;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawError {
    Ok = 0,
    Generic = -1,
    InvalidArg = -2,
    Unsupported = -3,
    NoDevice = -4,
    AlreadyRunning = -5,
    NotRunning = -6,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawSourceKind {
    Monitor = 0,
    Window = 1,
    Portal = 2,
    Region = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RawSource {
    pub kind: RawSourceKind,
    pub id: [c_char; 256],
    pub title: [c_char; 256],
    pub width: c_int,
    pub height: c_int,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RawAudioDevice {
    pub name: [c_char; 256],
    pub description: [c_char; 256],
    pub is_input: c_int,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawEventKind {
    Started = 0,
    Stopped = 1,
    Paused = 2,
    Resumed = 3,
    ReplaySaved = 4,
    FrameDropped = 5,
    Error = 6,
    LevelMeter = 7,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RawEvent {
    pub kind: RawEventKind,
    pub code: c_int,
    pub data0: c_double,
    pub message: [c_char; 512],
}

pub type RawEventCb = Option<unsafe extern "C" fn(*const RawEvent, *mut std::ffi::c_void)>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RawConfig {
    pub video_source: RawSource,
    pub fps: c_int,
    pub video_codec: *const c_char,
    pub container: *const c_char,
    pub output_path: *const c_char,
    pub audio_inputs: [*const c_char; 8],
    pub num_audio_inputs: c_int,
    pub mic_volume_percent: c_int,
    pub system_volume_percent: c_int,
    pub app_volume_percent: c_int,
    pub replay_buffer_secs: c_int,
    pub record_cursor: c_int,
    pub on_event: RawEventCb,
    pub event_userdata: *mut std::ffi::c_void,
}

#[repr(C)]
pub struct RawRecorder {
    _opaque: [u8; 0],
}

#[link(name = "souris_gsr")]
unsafe extern "C" {
    pub fn sgsr_version() -> *const c_char;
    pub fn sgsr_api_version() -> c_int;
    pub fn sgsr_init() -> RawError;
    pub fn sgsr_shutdown();
    pub fn sgsr_enumerate_video_sources(out: *mut *mut RawSource, out_n: *mut usize) -> RawError;
    pub fn sgsr_enumerate_audio_devices(out: *mut *mut RawAudioDevice, out_n: *mut usize) -> RawError;
    pub fn sgsr_free_video_sources(p: *mut RawSource);
    pub fn sgsr_free_audio_devices(p: *mut RawAudioDevice);
    pub fn sgsr_start(config: *const RawConfig, out: *mut *mut RawRecorder) -> RawError;
    pub fn sgsr_stop(r: *mut RawRecorder) -> RawError;
    pub fn sgsr_pause(r: *mut RawRecorder) -> RawError;
    pub fn sgsr_resume(r: *mut RawRecorder) -> RawError;
    pub fn sgsr_save_replay(r: *mut RawRecorder, path: *const c_char) -> RawError;
    pub fn sgsr_screenshot(r: *mut RawRecorder, path: *const c_char) -> RawError;
    pub fn sgsr_set_volumes(r: *mut RawRecorder, mic: c_int, system: c_int, app: c_int) -> RawError;
    pub fn sgsr_strerror(e: RawError) -> *const c_char;
}

fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

fn fixed_to_string(buf: &[c_char]) -> String {
    let bytes: Vec<u8> = buf.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// C ABI version string, e.g. "0.2.0-souris".
pub fn version() -> String {
    cstr_to_string(unsafe { sgsr_version() })
}

pub fn api_version() -> i32 {
    unsafe { sgsr_api_version() }
}

/// (name, description, is_input) for every audio device.
pub fn list_audio_devices() -> Result<Vec<(String, String, bool)>, RawError> {
    let mut ptr: *mut RawAudioDevice = std::ptr::null_mut();
    let mut n: usize = 0;
    let rc = unsafe { sgsr_enumerate_audio_devices(&mut ptr, &mut n) };
    if rc != RawError::Ok {
        return Err(rc);
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let d = unsafe { &*ptr.add(i) };
        out.push((fixed_to_string(&d.name), fixed_to_string(&d.description), d.is_input != 0));
    }
    unsafe { sgsr_free_audio_devices(ptr) };
    Ok(out)
}

/// (id, title, kind) for the generic video sources.
pub fn list_video_sources() -> Result<Vec<(String, String, RawSourceKind)>, RawError> {
    let mut ptr: *mut RawSource = std::ptr::null_mut();
    let mut n: usize = 0;
    let rc = unsafe { sgsr_enumerate_video_sources(&mut ptr, &mut n) };
    if rc != RawError::Ok {
        return Err(rc);
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let s = unsafe { &*ptr.add(i) };
        out.push((fixed_to_string(&s.id), fixed_to_string(&s.title), s.kind));
    }
    unsafe { sgsr_free_video_sources(ptr) };
    Ok(out)
}

pub fn path_to_cstring(p: &Path) -> Result<CString, RawError> {
    CString::new(p.to_string_lossy().as_bytes()).map_err(|_| RawError::InvalidArg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_version_matches_header() {
        assert_eq!(api_version(), 1);
        assert!(version().starts_with("0.2"));
    }

    #[test]
    fn abi_enumerate_works() {
        assert_eq!(unsafe { sgsr_init() }, RawError::Ok);
        let audio = list_audio_devices().unwrap();
        assert!(audio.len() >= 2);
        assert_eq!(audio[0].0, "default_output");
        let video = list_video_sources().unwrap();
        assert_eq!(video.len(), 2);
        unsafe { sgsr_shutdown() };
    }

    #[test]
    fn abi_rejects_bad_args() {
        assert_eq!(unsafe { sgsr_init() }, RawError::Ok);
        let mut rec: *mut RawRecorder = std::ptr::null_mut();
        assert_eq!(unsafe { sgsr_start(std::ptr::null(), &mut rec) }, RawError::InvalidArg);
        assert_eq!(unsafe { sgsr_stop(std::ptr::null_mut()) }, RawError::InvalidArg);
        assert_eq!(unsafe { sgsr_set_volumes(std::ptr::null_mut(), 0, 0, 0) }, RawError::InvalidArg);
        unsafe { sgsr_shutdown() };
    }
}
