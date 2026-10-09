//! Plugin load tests: ABI layout, real `.so` load/draw/unload, error paths.
//! Mirrors `src/plugins.c` + `plugin/plugin.h`. The two `gsr-test-plugin*`
//! cdylib fixtures build as dev-dependencies, so no nested cargo runs here
//! (a nested build would deadlock on the target-dir lock).

use gsr_core::plugins::{check_init_return, PluginManager};
use gsr_i18n::{Catalog, Lang};
use gsr_plugin::{
    make_version, ColorDepth, DrawParams, GraphicsApi, InitParams, InitReturn, INTERFACE_VERSION,
    MAX_PLUGINS,
};
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/core two levels below root")
        .to_path_buf()
}

/// Build a fixture cdylib into an isolated target dir and return its path.
/// An isolated `--target-dir` avoids the workspace target lock (a nested
/// plain `cargo build` could deadlock against the running `cargo test`).
/// Dev-dependencies still guarantee the fixtures compile in every gate.
fn ensure_cdylib(pkg: &str, crate_name: &str) -> PathBuf {
    let build_dir = std::env::temp_dir().join(format!("gsr-t14-{}-fixture", std::process::id()));
    let cargo = option_env!("CARGO").unwrap_or("cargo");
    let status = std::process::Command::new(cargo)
        .current_dir(workspace_root())
        .args(["build", "-p", pkg, "--target-dir"])
        .arg(&build_dir)
        .status()
        .expect("cargo build for fixture");
    assert!(status.success(), "fixture build failed for {pkg}");
    let file = format!(
        "{}{crate_name}{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    for profile in ["debug", "release"] {
        let candidate = build_dir.join(profile).join(&file);
        if candidate.exists() {
            return candidate;
        }
    }
    panic!("cdylib {file} missing after building {pkg}");
}

fn test_params() -> InitParams {
    InitParams {
        width: 320,
        height: 240,
        fps: 60,
        color_depth: ColorDepth::Bits8,
        graphics_api: GraphicsApi::EglEs,
    }
}

fn draws(lib_path: &std::path::Path) -> (u32, u32, u32) {
    unsafe {
        let lib = libloading::Library::new(lib_path).expect("reload fixture");
        let draws: libloading::Symbol<unsafe extern "C" fn() -> u32> =
            lib.get(b"gsr_testplugin_draws").expect("draws symbol");
        let w: libloading::Symbol<unsafe extern "C" fn() -> u32> =
            lib.get(b"gsr_testplugin_last_width").expect("width symbol");
        let h: libloading::Symbol<unsafe extern "C" fn() -> u32> = lib
            .get(b"gsr_testplugin_last_height")
            .expect("height symbol");
        (draws(), w(), h())
    }
}

#[test]
fn abi_constants_and_layout_mirror_plugin_h() {
    assert_eq!(make_version(0, 1), 1);
    assert_eq!(INTERFACE_VERSION, 1);
    assert_eq!(GraphicsApi::EglEs as u32, 0);
    assert_eq!(GraphicsApi::Glx as u32, 1);
    assert_eq!(ColorDepth::Bits8 as u32, 0);
    assert_eq!(ColorDepth::Bits10 as u32, 1);
    assert_eq!(MAX_PLUGINS, 128);
    #[cfg(target_pointer_width = "64")]
    {
        assert_eq!(std::mem::size_of::<DrawParams>(), 8);
        assert_eq!(std::mem::size_of::<InitParams>(), 20);
        assert_eq!(std::mem::size_of::<InitReturn>(), 32);
    }
}

#[test]
fn load_draw_unload_roundtrip() {
    let catalog = Catalog::new(Lang::En);
    let path = ensure_cdylib("gsr-test-plugin", "gsr_test_plugin");
    let mut manager = PluginManager::new();
    assert!(manager.is_empty());
    let info = manager
        .load(&catalog, path.to_str().unwrap(), &test_params())
        .expect("load fixture");
    assert!(info.contains("test-triangle"), "{info}");
    assert!(info.contains(path.to_str().unwrap()), "{info}");
    assert_eq!(manager.len(), 1);
    assert_eq!(manager.names(), vec!["test-triangle"]);

    manager.draw_all(320, 240);
    manager.draw_all(320, 240);
    // Draw dispatch reaches the plugin (fresh process per test binary run,
    // so exactly two calls land here).
    let (draws, w, h) = draws(&path);
    assert_eq!((draws, w, h), (2, 320, 240));

    let infos = manager.unload_all(&catalog);
    assert_eq!(infos.len(), 1);
    assert!(infos[0].contains("test-triangle"), "{}", infos[0]);
    assert!(manager.is_empty());
}

#[test]
fn load_missing_file_reports_path() {
    let catalog = Catalog::new(Lang::En);
    let mut manager = PluginManager::new();
    let err = manager
        .load(&catalog, "/no/such/plugin.so", &test_params())
        .expect_err("must fail");
    assert!(err.contains("/no/such/plugin.so"), "{err}");
    assert!(manager.is_empty());
}

#[test]
fn load_without_symbols_reports_symbol() {
    let catalog = Catalog::new(Lang::En);
    let path = ensure_cdylib("gsr-test-plugin-nosymbols", "gsr_test_plugin_nosymbols");
    let mut manager = PluginManager::new();
    let err = manager
        .load(&catalog, path.to_str().unwrap(), &test_params())
        .expect_err("must fail");
    assert!(err.contains("gsr_plugin_init"), "{err}");
    assert!(err.contains(path.to_str().unwrap()), "{err}");
    assert!(manager.is_empty());
}

#[test]
fn init_return_validation() {
    let catalog = Catalog::new(Lang::En);
    let name = c"named".as_ptr();
    let ret = InitReturn {
        name,
        version: 3,
        userdata: std::ptr::null_mut(),
        draw: None,
    };
    assert_eq!(
        check_init_return(&catalog, "p.so", &ret).expect("valid"),
        ("named".to_string(), 3)
    );
    let null_name = InitReturn {
        name: std::ptr::null(),
        version: 3,
        userdata: std::ptr::null_mut(),
        draw: None,
    };
    assert!(check_init_return(&catalog, "p.so", &null_name)
        .expect_err("null name")
        .contains("p.so"));
    let zero_version = InitReturn {
        name,
        version: 0,
        userdata: std::ptr::null_mut(),
        draw: None,
    };
    assert!(check_init_return(&catalog, "p.so", &zero_version)
        .expect_err("zero version")
        .contains("p.so"));
}

#[test]
fn localized_errors_in_spanish() {
    let catalog = Catalog::new(Lang::Es);
    let mut manager = PluginManager::new();
    let err = manager
        .load(&catalog, "/no/existe/plugin.so", &test_params())
        .expect_err("must fail");
    assert!(err.contains("/no/existe/plugin.so"), "{err}");
    assert!(err.contains("no se pudo cargar"), "{err}");
}
