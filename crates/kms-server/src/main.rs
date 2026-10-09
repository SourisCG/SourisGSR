// SKELETON (T03): crate-level allow, remove as T10+ implementation lands.
#![allow(dead_code)]
//! Privileged KMS helper. Mirrors `kms/server/kms_server.c` (v5.10.2).
//! Minimal binary: only DRM + UNIX socket + SCM_RIGHTS. Runs with
//! `cap_sys_admin+ep` (see `extra/udev/`). Protocol v5 (`protocol.rs`).

mod protocol;

fn main() {
    // Placeholder: real DRM + socket loop lands in T20.
    eprintln!("gsr-kms-server: skeleton helper, see .specify/spec-02-linux-kms.md");
}
