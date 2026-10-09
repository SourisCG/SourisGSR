//! Test-only overlay plugin implementing the C ABI for load tests.
//! Records draw calls so `crates/core/tests/plugin_load.rs` can observe them.

use gsr_plugin::{DrawParams, InitParams, InitReturn};
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

static DRAWS: AtomicU32 = AtomicU32::new(0);
static LAST_W: AtomicU32 = AtomicU32::new(0);
static LAST_H: AtomicU32 = AtomicU32::new(0);
static NAME: &[u8] = b"test-triangle\0";

unsafe extern "C" fn draw(params: *const DrawParams, _userdata: *mut c_void) {
    if params.is_null() {
        return;
    }
    let params = &*params;
    LAST_W.store(params.width, Ordering::SeqCst);
    LAST_H.store(params.height, Ordering::SeqCst);
    DRAWS.fetch_add(1, Ordering::SeqCst);
}

/// Exported `gsr_plugin_init`. Accepts any params (null rejected).
///
/// # Safety
///
/// `params` and `ret` must be valid non-null pointers (null is rejected).
#[no_mangle]
pub unsafe extern "C" fn gsr_plugin_init(params: *const InitParams, ret: *mut InitReturn) -> bool {
    if params.is_null() || ret.is_null() {
        return false;
    }
    // Touch params so a bad pointer would fault here, not later.
    let _ = unsafe { (*params).width };
    unsafe {
        *ret = InitReturn {
            name: NAME.as_ptr().cast(),
            version: 1,
            userdata: std::ptr::null_mut(),
            draw: Some(draw),
        };
    }
    true
}

/// Exported `gsr_plugin_deinit`.
///
/// # Safety
///
/// `userdata` must be the value produced by this module's init (null here).
#[no_mangle]
pub unsafe extern "C" fn gsr_plugin_deinit(_userdata: *mut c_void) {}

/// Test observer: number of draw calls so far.
#[no_mangle]
pub extern "C" fn gsr_testplugin_draws() -> u32 {
    DRAWS.load(Ordering::SeqCst)
}

/// Test observer: last drawn width.
#[no_mangle]
pub extern "C" fn gsr_testplugin_last_width() -> u32 {
    LAST_W.load(Ordering::SeqCst)
}

/// Test observer: last drawn height.
#[no_mangle]
pub extern "C" fn gsr_testplugin_last_height() -> u32 {
    LAST_H.load(Ordering::SeqCst)
}
