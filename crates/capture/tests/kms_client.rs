//! Helper client tests: pure logic plus live helper integration.
//! The `/dev/null` case runs everywhere (graceful permission-hint path);
//! the real-card case adapts: full dance where DRM allows, permission hint
//! where it does not, skip where no card node exists.
//!
//! Linux-only (UNIX sockets + helper spawn); Windows needs no helper.

#![cfg(unix)]

use gsr_capture::kms_client::*;
use gsr_i18n::{Catalog, Lang};
use std::os::fd::AsRawFd;
use std::path::PathBuf;

fn catalog() -> Catalog {
    Catalog::new(Lang::En)
}

/// Serializes helper-spawning tests: concurrent `pkexec` prompts and
/// accept loops would otherwise interfere (and storm the auth agent).
static SPAWN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn spawn_guard() -> std::sync::MutexGuard<'static, ()> {
    SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Build the real helper binary into an isolated target dir (same pattern
/// as the plugin fixture builds: no lock contention with the outer cargo).
fn ensure_helper() -> PathBuf {
    let build_dir = std::env::temp_dir().join(format!("gsr-t20-{}-helper", std::process::id()));
    let cargo = option_env!("CARGO").unwrap_or("cargo");
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.ancestors().nth(2).expect("two levels below root");
    let status = std::process::Command::new(cargo)
        .current_dir(root)
        .args([
            "build",
            "-p",
            "gsr-kms-server",
            "--bin",
            "gsr-kms-server",
            "--target-dir",
        ])
        .arg(&build_dir)
        .status()
        .expect("cargo build for helper");
    assert!(status.success(), "helper build failed");
    for profile in ["debug", "release"] {
        let candidate = build_dir.join(profile).join("gsr-kms-server");
        if candidate.exists() {
            return candidate;
        }
    }
    panic!("helper binary missing after build");
}

#[test]
fn launch_mode_precedence_mirrors_c() {
    use LaunchMode::*;
    assert_eq!(select_mode(0, false, false), Direct);
    assert_eq!(select_mode(0, true, true), Direct);
    assert_eq!(select_mode(1000, true, false), Direct);
    assert_eq!(select_mode(1000, false, true), FlatpakProxy);
    assert_eq!(select_mode(1000, false, false), Pkexec);
}

#[test]
fn helper_resolution_prefers_next_to_exe() {
    let dir = std::env::temp_dir().join(format!("gsr-t20-resolve-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("gsr-kms-server"), b"x").unwrap();
    let exe = dir.join("gpu-screen-recorder");
    let (path, next_to_exe) = resolve_helper(&exe);
    assert!(next_to_exe);
    assert_eq!(path, dir.join("gsr-kms-server"));
    std::fs::remove_dir_all(&dir).ok();

    let (path, next_to_exe) =
        resolve_helper(PathBuf::from("/no/such/dir/gpu-screen-recorder").as_path());
    assert!(!next_to_exe);
    assert_eq!(path, PathBuf::from("gsr-kms-server"));
}

#[test]
fn spawn_argv_shapes() {
    let helper = PathBuf::from("/usr/bin/gsr-kms-server");
    let sock = PathBuf::from("/run/user/1000/gsr-kms-abcd");
    assert_eq!(
        build_spawn_argv(LaunchMode::Direct, &helper, &sock, "/dev/dri/card0", None),
        vec![
            "/usr/bin/gsr-kms-server",
            "/run/user/1000/gsr-kms-abcd",
            "/dev/dri/card0"
        ]
    );
    assert_eq!(
        build_spawn_argv(LaunchMode::Pkexec, &helper, &sock, "/dev/dri/card0", None)[..2],
        vec!["pkexec".to_string(), "/usr/bin/gsr-kms-server".to_string()]
    );
    let flatpak = build_spawn_argv(
        LaunchMode::FlatpakProxy,
        &helper,
        &sock,
        "/dev/dri/card0",
        Some("/home/u"),
    );
    assert_eq!(
        flatpak,
        vec![
            "flatpak-spawn",
            "--host",
            "/usr/bin/kms-server-proxy",
            "/run/user/1000/gsr-kms-abcd",
            "/dev/dri/card0",
            "/home/u",
        ]
    );
}

fn cap_bytes(magic_etc: u32, permitted: u32) -> Vec<u8> {
    let mut out = magic_etc.to_le_bytes().to_vec();
    out.extend_from_slice(&permitted.to_le_bytes());
    out.extend_from_slice(&[0u8; 12]);
    out
}

#[test]
fn capability_parsing() {
    const V3_EFFECTIVE: u32 = 0x03000001;
    // Revision 3, effective, CAP_SYS_ADMIN set.
    assert!(parse_cap_v3(&cap_bytes(V3_EFFECTIVE, 1 << 21)));
    // Wrong revision.
    assert!(!parse_cap_v3(&cap_bytes(0x02000001, 1 << 21)));
    // Not effective.
    assert!(!parse_cap_v3(&cap_bytes(0x03000000, 1 << 21)));
    // Different capability only.
    assert!(!parse_cap_v3(&cap_bytes(V3_EFFECTIVE, 1 << 20)));
    // Truncated.
    assert!(!parse_cap_v3(&[0u8; 8]));
    assert!(!parse_cap_v3(&[]));
}

#[test]
fn capability_probe_is_safe() {
    assert!(!has_file_cap_sys_admin(
        PathBuf::from("/no/such/file").as_path()
    ));
    // Must not panic on any real file; result depends on the environment.
    let _ = has_file_cap_sys_admin(PathBuf::from("/bin/true").as_path());
}

#[test]
fn random_suffix_shape() {
    let a = random_suffix();
    let b = random_suffix();
    assert_eq!(a.len(), 10);
    assert!(a.chars().all(|c| c.is_ascii_alphanumeric()), "{a}");
    assert_ne!(a, b);
}

#[test]
fn rendezvous_dir_prefers_xdg_runtime() {
    std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
    assert_eq!(rendezvous_dir(), PathBuf::from("/run/user/1000"));
    std::env::remove_var("XDG_RUNTIME_DIR");
    assert!(rendezvous_dir().is_absolute());
}

#[test]
fn listener_is_private_and_connectable() {
    let dir = std::env::temp_dir().join(format!("gsr-t20-sock-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.sock");
    let listener = bind_listener(&path).expect("bind");
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o700);
    }
    let connected = std::os::unix::net::UnixStream::connect(&path).expect("connect");
    drop(connected);
    drop(listener);
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn fd_count_check() {
    use gsr_capture::protocol::{Item, KmsResult, Response};
    let catalog = catalog();
    let response = Response {
        result: KmsResult::Ok,
        err_msg: String::new(),
        items: vec![Item {
            num_dma_bufs: 2,
            ..Item::default()
        }],
    };
    assert!(check_fd_count(&catalog, &response, 2).is_ok());
    let err = check_fd_count(&catalog, &response, 1).expect_err("mismatch");
    assert!(err.contains('2') && err.contains('1'), "{err}");
}

#[test]
fn null_card_yields_permission_hint() {
    // /dev/null opens but fails every DRM ioctl: full spawn + dance up to
    // the helper's setup failure, surfaced as the actionable hint.
    let _guard = spawn_guard();
    let helper = ensure_helper();
    let catalog = catalog();
    match KmsClient::init_with_helper(&catalog, "/dev/null", &helper) {
        Err(e) => assert!(
            e.contains("/dev/null") || e.contains("privileg") || e.contains("pkexec"),
            "{e}"
        ),
        Ok(_) => panic!("/dev/null unexpectedly worked as a DRM card"),
    }
}

#[test]
fn full_dance_against_fake_helper() {
    // A Python fake speaks the exact wire protocol (REPLACE adopt + OK,
    // GET_KMS + one item + one fd): the whole client dance with no DRM,
    // no privileges and no pkexec.
    let dir = std::env::temp_dir().join(format!("gsr-t20-fake-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("fake-kms-server");
    std::fs::write(&script, FAKE_HELPER).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();
    }

    let _guard = spawn_guard();
    let catalog = catalog();
    let client = KmsClient::init_with_launcher(&catalog, "/dev/null", &script, LaunchMode::Direct)
        .expect("dance with fake helper");
    let (response, fds) = client.get_kms(&catalog).expect("get_kms");
    assert_eq!(response.result, gsr_capture::protocol::KmsResult::Ok);
    assert_eq!(response.items.len(), 1);
    let item = &response.items[0];
    assert_eq!((item.width, item.height), (64, 64));
    assert_eq!(item.num_dma_bufs, 1);
    assert_eq!(item.dma[0].pitch, 1920);
    assert_eq!(item.connector_id, 7);
    assert_eq!(fds.len(), 1);
    let link =
        std::fs::read_link(format!("/proc/self/fd/{}", fds[0].as_raw_fd())).expect("fd link");
    assert!(link.to_string_lossy().contains("null"), "{link:?}");
    client.deinit();
    std::fs::remove_dir_all(&dir).ok();
}

/// Fake helper: reverse-connects, adopts the REPLACE fd, answers OK, then
/// serves one canned item plus a `/dev/null` fd per `GET_KMS`.
const FAKE_HELPER: &str = r#"#!/usr/bin/env python3
import array
import os
import socket
import struct
import sys

RESPONSE_LEN = 1036


def send_msg(sock, body, fds):
    anc = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", fds))] if fds else []
    sock.sendmsg([body], anc)


def recv_exact(sock, size):
    chunks = []
    while sum(map(len, chunks)) < size:
        data, anc, _, _ = sock.recvmsg(size, socket.CMSG_SPACE(4 * 32))
        if not data:
            raise EOFError("peer closed")
        chunks.append(data)
        for level, kind, payload in anc:
            if level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS:
                fds = array.array("i")
                fds.frombytes(payload[: len(payload) // 4 * 4])
                return b"".join(chunks), list(fds)
    return b"".join(chunks), []


def response(result, items):
    body = struct.pack("<II", 5, result) + bytes(128)
    body += struct.pack("<I", len(items))
    for pitch, offset in items:
        # C field order: 4 DMA entries first, then the count.
        item = struct.pack("<II", pitch, offset) + bytes(3 * 8)
        item += struct.pack("<I", 1)
        item += struct.pack("<III", 64, 64, 0x34325258)
        item += struct.pack("<Q", 0) + struct.pack("<I", 7)
        item += bytes([0, 0, 0, 0]) + struct.pack("<iiii", 0, 0, 64, 64)
        item += bytes(32)
        body += item
    body += bytes(112 * (8 - len(items)))
    assert len(body) == RESPONSE_LEN, len(body)
    return body


def main():
    sock_path = sys.argv[1]
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect(sock_path)
    req, fds = recv_exact(sock, 8)
    version, kind = struct.unpack("<II", req)
    assert (version, kind) == (5, 0), (version, kind)
    conn = socket.fromfd(fds[0], socket.AF_UNIX, socket.SOCK_STREAM)
    for fd in fds[1:]:
        os.close(fd)
    send_msg(conn, response(0, []), [])
    null_fd = os.open("/dev/null", os.O_RDONLY)
    try:
        while True:
            try:
                req, _ = recv_exact(conn, 8)
            except EOFError:
                return 0
            _, kind = struct.unpack("<II", req)
            if kind == 1:
                send_msg(conn, response(0, [(1920, 0)]), [null_fd])
    finally:
        os.close(null_fd)


sys.exit(main())
"#;

#[test]
fn helper_cli_validation() {
    // Usage errors exit 1; a missing card exits 2. No hangs, no privileges.
    let helper = ensure_helper();
    for args in [&[] as &[&str], &["only-one"], &["a", "b", "c"]] {
        let status = std::process::Command::new(&helper)
            .args(args)
            .output()
            .expect("run helper")
            .status;
        assert_eq!(status.code(), Some(1), "{args:?}");
    }
    let status = std::process::Command::new(&helper)
        .args(["/tmp/gsr-t20-nope.sock", "/no/such/card"])
        .output()
        .expect("run helper")
        .status;
    assert_eq!(status.code(), Some(2));
}

#[test]
fn real_card_full_dance_or_hint() {
    let card = std::env::var("KMS_TEST_CARD").unwrap_or_else(|_| "/dev/dri/card0".to_string());
    if !PathBuf::from(&card).exists() {
        eprintln!("no DRM card node; skipping live dance");
        return;
    }
    let catalog = catalog();
    let helper = ensure_helper();
    let _guard = spawn_guard();
    match KmsClient::init_with_helper(&catalog, &card, &helper) {
        Err(e) => {
            // Without privileges the helper fails fast with the hint; when
            // it launches via pkexec with no polkit agent, pkexec waits for
            // auth, so the client bounds the wait and reaps it instead of
            // hanging; a helper dying mid-dance surfaces as a socket error.
            // All three prove graceful, bounded failure with cleanup.
            assert!(
                e.contains(&card)
                    || e.contains("pkexec")
                    || e.contains("privileg")
                    || e.contains("timed out")
                    || e.contains("KMS socket"),
                "{e}"
            );
        }
        Ok(client) => {
            let (response, fds) = client.get_kms(&catalog).expect("get_kms on real card");
            assert_eq!(response.result, gsr_capture::protocol::KmsResult::Ok);
            let expected: usize = response.items.iter().map(|i| i.num_dma_bufs as usize).sum();
            assert_eq!(fds.len(), expected);
            assert!(!response.items.is_empty());
            client.deinit();
        }
    }
}
