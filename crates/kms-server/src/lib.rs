//! Privileged KMS helper library: wire protocol + fd transport.
//! The binary (`main.rs`) stays a thin CLI wrapper around this library.
//! `grab` and `transport` are Linux-only (DRM + UNIX sockets); `protocol`
//! is pure and portable.

#[cfg(unix)]
pub mod grab;
pub mod protocol;
#[cfg(unix)]
pub mod transport;
