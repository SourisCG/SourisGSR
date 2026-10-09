//! Overlay plugin manager. Mirrors `src/plugins.c` + `include/plugins.h`
//! load/unload/draw logic (frozen reference v5.10.2).
//!
//! Load protocol (same order, same failures as C): capacity check (128),
//! `dlopen`, `gsr_plugin_init` + `gsr_plugin_deinit` symbols, init call,
//! non-null name, non-zero version. Unload runs in reverse order with one
//! info line per plugin. GL framebuffer setup around `draw` is capture-phase
//! work (T21+); here dispatch only. Messages are fully localized.

use std::ffi::CStr;

use gsr_i18n::{fill, Catalog};
use gsr_plugin::{DeinitFn, DrawFn, DrawParams, InitParams, InitReturn, MAX_PLUGINS};

/// One loaded overlay plugin. `Drop` closes the library handle; `deinit`
/// runs only via [`PluginManager::unload_all`] (which also logs), mirroring
/// the C deinit loop.
pub struct LoadedPlugin {
    name: String,
    version: u32,
    draw: DrawFn,
    userdata: *mut std::ffi::c_void,
    deinit: DeinitFn,
    _lib: libloading::Library,
}

impl LoadedPlugin {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    /// Call the optional draw callback (no-op when the plugin sets none).
    pub fn draw(&self, params: &DrawParams) {
        if let Some(draw) = self.draw {
            // SAFETY: `draw`/`userdata` come from the validated init return
            // of a live library handle owned by `self`.
            unsafe { draw(params, self.userdata) };
        }
    }

    fn deinit(&mut self) {
        // SAFETY: paired with the init call that produced `userdata`.
        unsafe { (self.deinit)(self.userdata) };
    }
}

/// Validate an init return: non-null name, non-zero version (mirrors C).
/// Pure for testability; `load` feeds it the plugin's answer.
pub fn check_init_return(
    catalog: &Catalog,
    path: &str,
    ret: &InitReturn,
) -> Result<(String, u32), String> {
    if ret.name.is_null() {
        return Err(fill(&catalog.get("err_plugin_noname"), &[("path", path)]));
    }
    // SAFETY: null checked above; C guarantees a NUL-terminated name.
    let name = unsafe { CStr::from_ptr(ret.name) }
        .to_string_lossy()
        .into_owned();
    if ret.version == 0 {
        return Err(fill(
            &catalog.get("err_plugin_noversion"),
            &[("path", path)],
        ));
    }
    Ok((name, ret.version))
}

/// Ordered plugin set (`-p` repeatable, max 128 like C).
#[derive(Default)]
pub struct PluginManager {
    plugins: Vec<LoadedPlugin>,
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            plugins: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn names(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name()).collect()
    }

    /// Load one plugin file. `Ok` carries the info line (caller prints it
    /// to stderr, as C does); `Err` carries the localized failure.
    pub fn load(
        &mut self,
        catalog: &Catalog,
        path: &str,
        params: &InitParams,
    ) -> Result<String, String> {
        if self.plugins.len() >= MAX_PLUGINS {
            return Err(catalog.get("err_plugin_too_many"));
        }
        // SAFETY: `Library::new` only maps the file; no plugin code runs yet.
        let lib = unsafe { libloading::Library::new(path) }.map_err(|e| {
            fill(
                &catalog.get("err_plugin_load"),
                &[("path", path), ("reason", &e.to_string())],
            )
        })?;
        // SAFETY: symbols are looked up by exact C export name; signatures
        // are checked by the typed alias before any call.
        unsafe {
            let init: libloading::Symbol<gsr_plugin::InitFn> =
                lib.get(b"gsr_plugin_init").map_err(|_| {
                    fill(
                        &catalog.get("err_plugin_symbol"),
                        &[("symbol", "gsr_plugin_init"), ("path", path)],
                    )
                })?;
            let deinit: libloading::Symbol<gsr_plugin::DeinitFn> =
                lib.get(b"gsr_plugin_deinit").map_err(|_| {
                    fill(
                        &catalog.get("err_plugin_symbol"),
                        &[("symbol", "gsr_plugin_deinit"), ("path", path)],
                    )
                })?;
            let mut ret = std::mem::zeroed::<InitReturn>();
            if !init(params, &mut ret) {
                return Err(fill(
                    &catalog.get("err_plugin_init_failed"),
                    &[("path", path)],
                ));
            }
            let (name, version) = check_init_return(catalog, path, &ret)?;
            let info = fill(
                &catalog.get("info_plugin_loaded"),
                &[
                    ("path", path),
                    ("name", &name),
                    ("version", &version.to_string()),
                ],
            );
            self.plugins.push(LoadedPlugin {
                name,
                version,
                draw: ret.draw,
                userdata: ret.userdata,
                deinit: *deinit,
                _lib: lib,
            });
            Ok(info)
        }
    }

    /// Dispatch draw to every plugin in load order (skips plugins without
    /// a draw callback, as in C).
    pub fn draw_all(&self, width: u32, height: u32) {
        let params = DrawParams { width, height };
        for plugin in &self.plugins {
            plugin.draw(&params);
        }
    }

    /// Unload in reverse order, returning one info line per plugin (C order).
    pub fn unload_all(&mut self, catalog: &Catalog) -> Vec<String> {
        let mut infos = Vec::with_capacity(self.plugins.len());
        while let Some(mut plugin) = self.plugins.pop() {
            infos.push(fill(
                &catalog.get("info_plugin_unloaded"),
                &[("name", plugin.name())],
            ));
            plugin.deinit();
        }
        infos
    }
}
