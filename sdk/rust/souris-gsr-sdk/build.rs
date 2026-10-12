//! Link helper for the `ffi` feature.
//!
//! Set `SOURIS_GSR_LIB_DIR` to a directory containing `libsouris_gsr.a`
//! (archived from a core meson build, minus the CLI `main` object).
//! Optionally set `SOURIS_GSR_PKG_CONFIG_PATH` to prepend entries to
//! `PKG_CONFIG_PATH` (e.g. a sysroot with libcap-devel-only headers).
//! Without `SOURIS_GSR_LIB_DIR`, `cargo check --features ffi` still
//! typechecks (no linking).

fn main() {
    if std::env::var("CARGO_FEATURE_FFI").is_err() {
        return;
    }
    if std::env::var("SOURIS_GSR_LIB_DIR").is_err() {
        return;
    }
    let dir = std::env::var("SOURIS_GSR_LIB_DIR").unwrap();
    println!("cargo::rustc-link-search=native={dir}");
    println!("cargo::rustc-link-lib=static=souris_gsr");

    let mut pkg_path = std::env::var("PKG_CONFIG_PATH").unwrap_or_default();
    if let Ok(extra) = std::env::var("SOURIS_GSR_PKG_CONFIG_PATH") {
        if !pkg_path.is_empty() {
            pkg_path = format!("{extra}:{pkg_path}");
        } else {
            pkg_path = extra.clone();
        }
        // A ".../lib64/pkgconfig" entry usually pairs with a ".../lib64"
        // sibling holding the actual .so files (sysroot case).
        for entry in extra.split(':') {
            if let Some(libdir) = entry.strip_suffix("/pkgconfig") {
                println!("cargo::rustc-link-search=native={libdir}");
            }
        }
    }
    let libs = [
        "libavcodec",
        "libavformat",
        "libavutil",
        "x11",
        "xcomposite",
        "xrandr",
        "xfixes",
        "xdamage",
        "libpulse",
        "libswresample",
        "libavfilter",
        "libva",
        "libva-drm",
        "libdrm",
        "wayland-egl",
        "wayland-client",
        "dbus-1",
        "libcap",
        "libpipewire-0.3",
        "libspa-0.2",
    ];
    let out = std::process::Command::new("pkg-config")
        .env("PKG_CONFIG_PATH", &pkg_path)
        .arg("--libs")
        .args(libs)
        .output();
    if let Ok(out) = out {
        if out.status.success() {
            for token in String::from_utf8_lossy(&out.stdout).split_whitespace() {
                if let Some(lib) = token.strip_prefix("-l") {
                    println!("cargo::rustc-link-lib={lib}");
                } else if let Some(path) = token.strip_prefix("-L") {
                    println!("cargo::rustc-link-search=native={path}");
                }
            }
        }
    }
    println!("cargo::rustc-link-lib=m");
}
