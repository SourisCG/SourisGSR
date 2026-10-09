//! Structure-mapping test: verifies the mandatory C-to-Rust folder mapping.
//! Fails if any mapped module from `reference/MAPA.md` is missing.
//! Also checks locales key parity and spec citations.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/core two levels below root")
        .to_path_buf()
}

fn must_exist(root: &Path, rel: &str) {
    assert!(
        root.join(rel).exists(),
        "missing mapped path: {rel} (see reference/MAPA.md)"
    );
}

#[test]
fn mapped_folders_exist() {
    let root = workspace_root();
    // Core mirrors src/main.cpp, args_parser.c, defs.c, replay_buffer/*,
    // encoder.c (mux), image_writer.c, library_loader.c, utils.c.
    for f in [
        "crates/core/src/main.rs",
        "crates/core/src/cli.rs",
        "crates/core/src/config.rs",
        "crates/core/src/replay.rs",
        "crates/core/src/replay_ram.rs",
        "crates/core/src/replay_disk.rs",
        "crates/core/src/muxer.rs",
        "crates/core/src/plugins.rs",
        "crates/core/src/screenshot.rs",
        "crates/core/src/library.rs",
        "crates/core/src/util.rs",
    ] {
        must_exist(&root, f);
    }
    // Capture mirrors src/capture/*, window/wayland.c, egl, compose, color,
    // cursor, damage, dbus, kms client, protocol.
    for f in [
        "crates/capture/src/traits.rs",
        "crates/capture/src/kms.rs",
        "crates/capture/src/kms_client.rs",
        "crates/capture/src/protocol.rs",
        "crates/capture/src/portal.rs",
        "crates/capture/src/dbus.rs",
        "crates/capture/src/wayland.rs",
        "crates/capture/src/egl.rs",
        "crates/capture/src/compose.rs",
        "crates/capture/src/color.rs",
        "crates/capture/src/cursor.rs",
        "crates/capture/src/damage.rs",
    ] {
        must_exist(&root, f);
    }
    // Encode mirrors src/encoder/video/*, src/codec_query/*, src/cuda.c.
    for f in [
        "crates/encode/src/traits.rs",
        "crates/encode/src/video.rs",
        "crates/encode/src/nvenc.rs",
        "crates/encode/src/vaapi.rs",
        "crates/encode/src/vulkan.rs",
        "crates/encode/src/software.rs",
        "crates/encode/src/query_nvenc.rs",
        "crates/encode/src/query_vaapi.rs",
        "crates/encode/src/query_vulkan.rs",
        "crates/encode/src/cuda.rs",
    ] {
        must_exist(&root, f);
    }
    // Audio mirrors src/sound.cpp + src/pipewire_audio.c.
    for f in [
        "crates/audio/src/traits.rs",
        "crates/audio/src/pulse.rs",
        "crates/audio/src/pipewire_app.rs",
    ] {
        must_exist(&root, f);
    }
    // Helper mirrors kms/server/kms_server.c; plugin mirrors plugin/plugin.h.
    for f in [
        "crates/kms-server/src/main.rs",
        "crates/kms-server/src/lib.rs",
        "crates/kms-server/src/protocol.rs",
        "crates/kms-server/src/transport.rs",
        "crates/kms-server/src/grab.rs",
        "crates/plugin/src/lib.rs",
        "crates/plugin/src/abi.rs",
        "crates/test-plugin/src/lib.rs",
        "crates/test-plugin-nosymbols/src/lib.rs",
        "crates/i18n/src/lib.rs",
        "crates/i18n/locales/en.toml",
        "crates/i18n/locales/es.toml",
        "reference/MAPA.md",
        ".specify/constitution.md",
        ".specify/spec-01-core.md",
        ".specify/plan.md",
        ".specify/tasks.md",
    ] {
        must_exist(&root, f);
    }
    // No generic utils/helpers folders allowed.
    for d in ["crates/core/src/utils", "crates/core/src/helpers"] {
        assert!(!root.join(d).exists(), "forbidden generic folder: {d}");
    }
}

fn simple_keys(text: &str) -> HashSet<String> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.is_empty() || l.starts_with('#') {
                return None;
            }
            l.split_once('=').map(|(k, _)| k.trim().to_string())
        })
        .collect()
}

#[test]
fn locales_have_identical_keys() {
    let root = workspace_root();
    let en = fs::read_to_string(root.join("crates/i18n/locales/en.toml")).expect("read en.toml");
    let es = fs::read_to_string(root.join("crates/i18n/locales/es.toml")).expect("read es.toml");
    assert_eq!(
        simple_keys(&en),
        simple_keys(&es),
        "en.toml/es.toml keys must be identical"
    );
}

#[test]
fn specs_cite_c_files() {
    let root = workspace_root();
    for spec in [
        ".specify/spec-01-core.md",
        ".specify/spec-02-linux-kms.md",
        ".specify/spec-03-linux-portal.md",
        ".specify/spec-04-windows-dxgi.md",
        ".specify/spec-05-windows-wgc.md",
        ".specify/spec-06-i18n.md",
    ] {
        let text = fs::read_to_string(root.join(spec)).expect("read spec");
        let cites_c = text.contains("reference/gpu-screen-recorder")
            || text.contains("`src/")
            || text.contains("`kms/")
            || text.contains("`plugin/")
            || text.contains("`protocol/")
            || text.contains("`extra/")
            || text.contains("meson.build");
        assert!(cites_c, "{spec} must cite C files or build units");
    }
}
