//! Transport tests: body transfer, fd passing, limits and peer identity.
//! No DRM needed: socketpairs plus pipes stand in for DMA-BUFs.
//!
//! Linux-only (UNIX sockets + SCM_RIGHTS).

#![cfg(unix)]

use gsr_kms_server::transport::*;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

fn make_pipe() -> (OwnedFd, OwnedFd) {
    let mut pair = [0; 2];
    assert_eq!(unsafe { libc::pipe(pair.as_mut_ptr()) }, 0);
    unsafe { (OwnedFd::from_raw_fd(pair[0]), OwnedFd::from_raw_fd(pair[1])) }
}

fn write_all(fd: BorrowedFd<'_>, data: &[u8]) {
    let ret = unsafe {
        libc::write(
            fd.as_raw_fd(),
            data.as_ptr() as *const libc::c_void,
            data.len(),
        )
    };
    assert_eq!(ret as usize, data.len());
}

fn read_exact(fd: BorrowedFd<'_>, len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    let ret = unsafe { libc::read(fd.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, len) };
    assert_eq!(ret as usize, len);
    buf
}

#[test]
fn body_only_roundtrip() {
    let (a, b) = socketpair().expect("socketpair");
    send_msg(a.as_raw_fd(), b"hello-kms", &[]).expect("send");
    let got = recv_exact(b.as_raw_fd(), 9, 0).expect("recv");
    assert_eq!(got.body, b"hello-kms");
    assert!(got.fds.is_empty());
}

#[test]
fn fds_arrive_usable_and_cloexec() {
    let (a, b) = socketpair().expect("socketpair");
    let (read_end, write_end) = make_pipe();
    send_msg(a.as_raw_fd(), b"with-fd", &[read_end.as_fd()]).expect("send");
    drop(read_end);
    let got = recv_exact(b.as_raw_fd(), 7, 4).expect("recv");
    assert_eq!(got.body, b"with-fd");
    assert_eq!(got.fds.len(), 1);
    // The received fd is functional (roundtrip through the pipe)...
    write_all(write_end.as_fd(), b"ping");
    assert_eq!(read_exact(got.fds[0].as_fd(), 4), b"ping");
    // ...and close-on-exec (C leaks received fds into children).
    let flags = unsafe { libc::fcntl(got.fds[0].as_raw_fd(), libc::F_GETFD) };
    assert_ne!(flags & libc::FD_CLOEXEC, 0);
}

#[test]
fn thirty_two_fds_roundtrip() {
    let (a, b) = socketpair().expect("socketpair");
    let pipes: Vec<(OwnedFd, OwnedFd)> = (0..32).map(|_| make_pipe()).collect();
    let reads: Vec<BorrowedFd<'_>> = pipes.iter().map(|(r, _)| r.as_fd()).collect();
    send_msg(a.as_raw_fd(), b"max", &reads).expect("send");
    let got = recv_exact(b.as_raw_fd(), 3, 32).expect("recv");
    assert_eq!(got.fds.len(), 32);
}

#[test]
fn over_limit_fails_closed() {
    let (a, b) = socketpair().expect("socketpair");
    let pipes: Vec<(OwnedFd, OwnedFd)> = (0..4).map(|_| make_pipe()).collect();
    let reads: Vec<BorrowedFd<'_>> = pipes.iter().map(|(r, _)| r.as_fd()).collect();
    send_msg(a.as_raw_fd(), b"over", &reads).expect("send");
    let err = recv_exact(b.as_raw_fd(), 4, 2).expect_err("must reject");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn closed_peer_is_an_error_not_a_hang() {
    let (a, b) = socketpair().expect("socketpair");
    drop(a);
    let err = recv_exact(b.as_raw_fd(), 8, 0).expect_err("must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
}

#[test]
fn recv_timeout_unblocks_dead_peers() {
    let (a, b) = socketpair().expect("socketpair");
    set_recv_timeout(b.as_raw_fd(), 1).expect("timeout");
    let start = std::time::Instant::now();
    let err = recv_exact(b.as_raw_fd(), 8, 0).expect_err("must time out");
    assert!(
        start.elapsed() < std::time::Duration::from_secs(5),
        "took too long"
    );
    let _ = a;
    assert!(
        matches!(
            err.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ),
        "unexpected: {err:?}"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn peer_uid_matches_self() {
    let (a, b) = socketpair().expect("socketpair");
    let me = unsafe { libc::geteuid() };
    assert_eq!(peer_uid(a.as_raw_fd()).expect("peercred"), me);
    assert_eq!(peer_uid(b.as_raw_fd()).expect("peercred"), me);
}
