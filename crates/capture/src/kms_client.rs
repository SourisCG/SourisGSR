//! Privileged-helper client. Mirrors `kms/client/kms_client.c`.
//!
//! The helper performs the DRM ioctls; this side owns the rendezvous
//! socket, spawns the helper (direct as root / file capabilities,
//! `flatpak-spawn` proxy, or `pkexec` — same order as C), performs the
//! `REPLACE_CONNECTION` dance and issues `GET_KMS`.
//!
//! Hardening vs C (`DEVIATION-SEC`, all tested):
//! - Socket lives under `$XDG_RUNTIME_DIR` (0700, created with a restrictive
//!   umask) instead of `$HOME`/`tmp` with `umask(0000)`.
//! - The accept wait is bounded (C waits forever); reaping uses `waitpid`
//!   via `Child` (C `SIGKILL`s without reaping, leaking zombies).
//! - Received fds are `OwnedFd` (C leaks them on several error paths).
//! - `GET_KMS` reads carry a timeout (C spins unboundedly).
//! - Announced fd counts are verified against received fds.
//!
//! Non-localized daemon diagnostics stay English inside the helper
//! (`DEVIATION-HELPER-LOGS`); every error returned here is localized.

use std::ffi::CString;
use std::io;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

use gsr_i18n::{fill, Catalog};
use gsr_kms_server::protocol::{
    decode_response, encode_request, KmsResult, RequestType, Response, MAX_FDS, RESPONSE_LEN,
};
use gsr_kms_server::transport::{recv_exact, send_msg, set_recv_timeout, socketpair};

/// Bound for the helper accept wait (C waits indefinitely).
const ACCEPT_TIMEOUT: Duration = Duration::from_secs(10);
/// Per-request receive timeout (C spins without one).
const REQUEST_TIMEOUT_SECS: u64 = 10;
/// Accept poll step.
const ACCEPT_POLL_STEP: Duration = Duration::from_millis(10);

/// How to launch the helper (same precedence as C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    Direct,
    FlatpakProxy,
    Pkexec,
}

/// Pick a launch mode: root runs direct, file capabilities run direct,
/// Flatpak goes through the host proxy, otherwise `pkexec` (kept by design).
pub fn select_mode(euid: u32, has_file_caps: bool, flatpak: bool) -> LaunchMode {
    if euid == 0 || has_file_caps {
        LaunchMode::Direct
    } else if flatpak {
        LaunchMode::FlatpakProxy
    } else {
        LaunchMode::Pkexec
    }
}

/// Resolve the helper binary: next to our own executable, else `PATH`
/// lookup at spawn time (mirrors the `/proc/self/exe` + `PATH` search).
pub fn resolve_helper(exe_path: &Path) -> (PathBuf, bool) {
    if let Some(dir) = exe_path.parent() {
        let next_to_exe = dir.join("gsr-kms-server");
        if next_to_exe.exists() {
            return (next_to_exe, true);
        }
    }
    (PathBuf::from("gsr-kms-server"), false)
}

/// Build the spawn argv for a mode (pure for tests).
/// Flatpak mirrors C: `flatpak-spawn --host <proxy> <sock> <card> <home>`;
/// the host-side `kms-server-proxy` ships with packaging (see T51).
pub fn build_spawn_argv(
    mode: LaunchMode,
    helper: &Path,
    socket_path: &Path,
    card_path: &str,
    home: Option<&str>,
) -> Vec<String> {
    let to_string = |p: &Path| p.to_string_lossy().into_owned();
    match mode {
        LaunchMode::Direct => vec![
            to_string(helper),
            to_string(socket_path),
            card_path.to_string(),
        ],
        LaunchMode::FlatpakProxy => {
            let proxy = helper
                .parent()
                .map(|d| d.join("kms-server-proxy"))
                .unwrap_or_else(|| PathBuf::from("kms-server-proxy"));
            let mut argv = vec![
                "flatpak-spawn".to_string(),
                "--host".to_string(),
                to_string(&proxy),
                to_string(socket_path),
                card_path.to_string(),
            ];
            if let Some(home) = home {
                argv.push(home.to_string());
            }
            argv
        }
        LaunchMode::Pkexec => vec![
            "pkexec".to_string(),
            to_string(helper),
            to_string(socket_path),
            card_path.to_string(),
        ],
    }
}

/// Parse a `security.capability` xattr value (v3 layout): effective flag set
/// and `CAP_SYS_ADMIN` (21) in the permitted set. Pure for tests.
pub fn parse_cap_v3(value: &[u8]) -> bool {
    // struct vfs_cap_data: magic_etc u32 LE, then permitted/inheritable pairs.
    if value.len() < 20 {
        return false;
    }
    let magic = u32::from_le_bytes([value[0], value[1], value[2], value[3]]);
    const V3: u32 = 0x03;
    const EFFECTIVE: u32 = 0x000001;
    const SYS_ADMIN: u32 = 21;
    if magic >> 24 != V3 {
        return false;
    }
    if magic & EFFECTIVE == 0 {
        return false;
    }
    let permitted = u32::from_le_bytes([value[4], value[5], value[6], value[7]]);
    permitted & (1 << SYS_ADMIN) != 0
}

/// Read `security.capability` from a file (works unprivileged like `getcap`).
pub fn has_file_cap_sys_admin(path: &Path) -> bool {
    let cpath = match CString::new(path.as_os_str().as_encoded_bytes()) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let mut buf = [0u8; 64];
    // SAFETY: `cpath` is NUL-terminated; `buf` is a valid out param.
    let len = unsafe {
        libc::getxattr(
            cpath.as_ptr(),
            c"security.capability".as_ptr(),
            buf.as_mut_ptr() as *mut libc::c_void,
            buf.len(),
        )
    };
    if len < 0 {
        return false;
    }
    parse_cap_v3(&buf[..len as usize])
}

/// Ten alphanumeric characters from `/dev/urandom` (C uses 10 rand chars).
/// Falls back to a pid/time mix if urandom is unreadable.
pub fn random_suffix() -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut bytes = [0u8; 10];
    let read = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| {
            use std::io::Read;
            f.read_exact(&mut bytes).map(|_| ())
        })
        .is_ok();
    if !read {
        let fallback = std::process::id() as u64
            ^ std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
        for (i, slot) in bytes.iter_mut().enumerate() {
            *slot = ((fallback >> (i * 6)) & 0x3f) as u8;
        }
    }
    bytes
        .iter()
        .map(|b| ALPHABET[(b % 62) as usize] as char)
        .collect()
}

/// Private rendezvous directory: `$XDG_RUNTIME_DIR`, else `$HOME/.cache`,
/// else `/tmp` (C used `$HOME` or `/tmp` directly with world permissions).
pub fn rendezvous_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        let dir = PathBuf::from(dir);
        if dir.is_absolute() {
            return dir;
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".cache");
    }
    PathBuf::from("/tmp")
}

/// Bind a `SOCK_STREAM` listener at `path` with `0700` permissions.
/// A restrictive umask applies only around `bind` (C used `umask(0000)`).
pub fn bind_listener(path: &Path) -> io::Result<OwnedFd> {
    // SAFETY: umask is process-global; this runs before capture threads exist
    // in the startup path, and the previous mask is always restored.
    let previous = unsafe { libc::umask(0o077) };
    let result = (|| {
        let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: owned immediately; closed on every path below.
        let owned = unsafe { OwnedFd::from_raw_fd(fd) };
        let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
        let bytes = path.as_os_str().as_encoded_bytes();
        if bytes.len() >= addr.sun_path.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "socket path too long",
            ));
        }
        for (slot, byte) in bytes.iter().enumerate() {
            addr.sun_path[slot] = *byte as libc::c_char;
        }
        let addr_len =
            (std::mem::size_of::<libc::sa_family_t>() + bytes.len() + 1) as libc::socklen_t;
        let bound = unsafe {
            libc::bind(
                owned.as_raw_fd(),
                &addr as *const libc::sockaddr_un as *const libc::sockaddr,
                addr_len,
            )
        };
        if bound < 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::listen(owned.as_raw_fd(), 1) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // Non-blocking: the accept loop polls child liveness and the
        // deadline (a blocking accept would hang forever on early exit).
        let flags = unsafe { libc::fcntl(owned.as_raw_fd(), libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(owned.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) }
                < 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(owned)
    })();
    unsafe { libc::umask(previous) };
    result
}

/// Accept one peer, refusing unexpected users where the platform allows the
/// check (Linux `SO_PEERCRED`; the 0700 directory is the real control).
#[cfg(target_os = "linux")]
fn peer_accepted(fd: BorrowedFd<'_>, euid: u32) -> bool {
    match gsr_kms_server::transport::peer_uid(fd.as_raw_fd()) {
        Ok(uid) => uid == euid || uid == 0,
        Err(_) => false,
    }
}

/// No peer credentials off Linux: the 0700 directory remains the control.
#[cfg(not(target_os = "linux"))]
fn peer_accepted(_fd: BorrowedFd<'_>, _euid: u32) -> bool {
    true
}

/// Connected helper session. `Drop` kills and reaps the helper (C leaves a
/// zombie) and removes the socket path.
pub struct KmsClient {
    child: Option<Child>,
    conn: Option<OwnedFd>,
    socket_path: PathBuf,
}

impl KmsClient {
    /// Spawn the helper and perform the connection dance.
    pub fn init(catalog: &Catalog, card_path: &str) -> Result<Self, String> {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("gpu-screen-recorder"));
        let (helper, _) = resolve_helper(&exe);
        Self::init_with_helper(catalog, card_path, &helper)
    }

    /// Same as [`KmsClient::init`] with an explicit helper path
    /// (tests build the binary into an isolated target dir).
    pub fn init_with_helper(
        catalog: &Catalog,
        card_path: &str,
        helper: &Path,
    ) -> Result<Self, String> {
        let euid = unsafe { libc::geteuid() };
        let flatpak = std::env::var_os("FLATPAK_ID").is_some();
        let mode = if euid == 0 {
            LaunchMode::Direct
        } else {
            select_mode(euid, has_file_cap_sys_admin(helper), flatpak)
        };
        Self::init_with_launcher(catalog, card_path, helper, mode)
    }

    /// Full dance with an explicit launch mode (tests drive a fake helper
    /// in [`LaunchMode::Direct`] without privileges or DRM).
    pub fn init_with_launcher(
        catalog: &Catalog,
        card_path: &str,
        helper: &Path,
        mode: LaunchMode,
    ) -> Result<Self, String> {
        let fail = |key: &str, pairs: &[(&str, &str)]| fill(&catalog.get(key), pairs);

        let euid = unsafe { libc::geteuid() };
        let socket_path = rendezvous_dir().join(format!("gsr-kms-{}", random_suffix()));
        let listener = bind_listener(&socket_path)
            .map_err(|e| fail("err_kms_socket", &[("reason", &e.to_string())]))?;

        let home = std::env::var("HOME").ok();
        let argv = build_spawn_argv(mode, helper, &socket_path, card_path, home.as_deref());
        let helper_label = argv.first().cloned().unwrap_or_default();
        let mut child = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                let _ = std::fs::remove_file(&socket_path);
                fail(
                    "err_kms_spawn",
                    &[("helper", &helper_label), ("reason", &e.to_string())],
                )
            })?;

        // Single-exit cleanup: every failure below kills and reaps the
        // helper and removes the socket (no orphans, no litter).
        match Self::handshake(catalog, card_path, &mut child, &listener, euid) {
            Ok(conn) => {
                // The socket file served its purpose; the established
                // channels no longer need it (mirrors C).
                let _ = std::fs::remove_file(&socket_path);
                Ok(KmsClient {
                    child: Some(child),
                    conn: Some(conn),
                    socket_path,
                })
            }
            Err(e) => {
                kill_and_reap(&mut child);
                let _ = std::fs::remove_file(&socket_path);
                Err(e)
            }
        }
    }
    /// Accept loop + REPLACE handshake. Cleanup (kill, reap, socket
    /// removal) happens once at the caller on every error path.
    fn handshake(
        catalog: &Catalog,
        card_path: &str,
        child: &mut Child,
        listener: &OwnedFd,
        euid: u32,
    ) -> Result<OwnedFd, String> {
        let fail = |key: &str, pairs: &[(&str, &str)]| fill(&catalog.get(key), pairs);

        // Bounded accept loop with liveness checks (C waits forever).
        let deadline = Instant::now() + ACCEPT_TIMEOUT;
        let accepted: OwnedFd = loop {
            // SAFETY: blocking accept on our own listener; EINTR retried.
            let fd = unsafe {
                libc::accept(
                    listener.as_raw_fd(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if fd >= 0 {
                // SAFETY: just accepted, owned from here on.
                let owned = unsafe { OwnedFd::from_raw_fd(fd) };
                if peer_accepted(owned.as_fd(), euid) {
                    break owned;
                }
                continue;
            }
            if io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                let err = io::Error::last_os_error();
                // Nobody connected yet: keep polling liveness and deadline.
                // (EAGAIN and EWOULDBLOCK share a value on Linux; compare
                // both spellings without a match to stay portable.)
                let code = err.raw_os_error();
                if code != Some(libc::EAGAIN) && code != Some(libc::EWOULDBLOCK) {
                    return Err(fail("err_kms_socket", &[("reason", &err.to_string())]));
                }
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        return Err(hint_for_exit(catalog, card_path, status));
                    }
                    return Err(fail("err_kms_died", &[]));
                }
                Ok(None) => {}
                Err(e) => {
                    return Err(fail("err_kms_socket", &[("reason", &e.to_string())]));
                }
            }
            if Instant::now() >= deadline {
                return Err(fail("err_kms_wait", &[]));
            }
            std::thread::sleep(ACCEPT_POLL_STEP);
        };

        // Socketpair + REPLACE_CONNECTION handshake (mirrors C): the request
        // goes over the accepted rendezvous connection; the server adopts
        // the passed fd and answers on it, so the reply arrives on `local`.
        let (local, remote) =
            socketpair().map_err(|e| fail("err_kms_socket", &[("reason", &e.to_string())]))?;
        let replace = send_msg(
            accepted.as_raw_fd(),
            &encode_request(RequestType::ReplaceConnection),
            &[remote.as_fd()],
        )
        .map_err(|e| fail("err_kms_socket", &[("reason", &e.to_string())]));
        drop(remote);
        replace?;
        set_recv_timeout(local.as_raw_fd(), REQUEST_TIMEOUT_SECS)
            .map_err(|e| fail("err_kms_socket", &[("reason", &e.to_string())]))?;
        let reply = recv_exact(local.as_raw_fd(), RESPONSE_LEN, 0)
            .map_err(|e| fail("err_kms_socket", &[("reason", &e.to_string())]))?;
        // The rendezvous connection served its purpose; drop it now that
        // the replacement channel is established.
        drop(accepted);
        match decode_response(&reply.body) {
            Ok(response) if response.result == KmsResult::Ok => Ok(local),
            Ok(response) => Err(fail("err_kms_failed", &[("message", &response.err_msg)])),
            Err(e) => Err(fail("err_kms_failed", &[("message", &e.to_string())])),
        }
    }

    /// Issue `GET_KMS`: response plus owned DMA-BUF fds in wire order.
    /// Announced fd counts are verified against received fds.
    pub fn get_kms(&self, catalog: &Catalog) -> Result<(Response, Vec<OwnedFd>), String> {
        let fail = |key: &str, pairs: &[(&str, &str)]| fill(&catalog.get(key), pairs);
        let conn = self.conn.as_ref().expect("connected client");
        send_msg(conn.as_raw_fd(), &encode_request(RequestType::GetKms), &[])
            .map_err(|e| fail("err_kms_socket", &[("reason", &e.to_string())]))?;
        let reply = recv_exact(conn.as_raw_fd(), RESPONSE_LEN, MAX_FDS)
            .map_err(|e| fail("err_kms_socket", &[("reason", &e.to_string())]))?;
        match decode_response(&reply.body) {
            Err(e) => Err(fail("err_kms_failed", &[("message", &e.to_string())])),
            Ok(response) if response.result != KmsResult::Ok => {
                if response.err_msg.is_empty() {
                    Err(fail(
                        "err_kms_failed",
                        &[("message", &format!("{:?}", response.result))],
                    ))
                } else {
                    Err(fail("err_kms_failed", &[("message", &response.err_msg)]))
                }
            }
            Ok(response) => {
                check_fd_count(catalog, &response, reply.fds.len())?;
                Ok((response, reply.fds))
            }
        }
    }

    /// Stop the helper: SIGKILL, reap (no zombie, unlike C), remove socket.
    pub fn deinit(mut self) {
        if let Some(mut child) = self.child.take() {
            kill_and_reap(&mut child);
        }
        self.conn.take();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Verify announced fd counts against received fds (fail-closed mapping).
pub fn check_fd_count(catalog: &Catalog, response: &Response, got: usize) -> Result<(), String> {
    let expected: usize = response
        .items
        .iter()
        .map(|item| item.num_dma_bufs as usize)
        .sum();
    if got != expected {
        return Err(fill(
            &catalog.get("err_kms_fd_mismatch"),
            &[
                ("expected", &expected.to_string()),
                ("got", &got.to_string()),
            ],
        ));
    }
    Ok(())
}
/// Map a failed helper exit to the permission hint when plausible.
fn hint_for_exit(catalog: &Catalog, card_path: &str, status: std::process::ExitStatus) -> String {
    // Helper exit 2 = setup failure (bad args, card open, caps); surface the
    // permission hint since that is the actionable cause for users.
    if status.code() == Some(2) {
        fill(
            &catalog.get("err_kms_no_permission"),
            &[("card", card_path)],
        )
    } else {
        fill(
            &catalog.get("err_kms_failed"),
            &[("message", &format!("exit {status}"))],
        )
    }
}

fn kill_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

impl Drop for KmsClient {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            kill_and_reap(&mut child);
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
