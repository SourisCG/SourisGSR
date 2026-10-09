//! Security tests: hostile peer input must fail closed without leaking fds.
//! Linux-only (`/proc/self/fd` accounting).

#![cfg(unix)]

use gsr_kms_server::protocol::*;
use gsr_kms_server::transport::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

fn fd_count() -> usize {
    std::fs::read_dir("/proc/self/fd")
        .map(|d| d.count())
        .unwrap_or(0)
}

fn make_pipe() -> (OwnedFd, OwnedFd) {
    let mut pair = [0; 2];
    assert_eq!(unsafe { libc::pipe(pair.as_mut_ptr()) }, 0);
    unsafe { (OwnedFd::from_raw_fd(pair[0]), OwnedFd::from_raw_fd(pair[1])) }
}

#[test]
fn rejected_version_drops_ancillary_fds() {
    let baseline = fd_count();
    let (a, b) = socketpair().expect("socketpair");
    // Well-formed body with a hostile version, plus a real fd attached.
    let mut hostile = encode_request(RequestType::GetKms);
    hostile[0..4].copy_from_slice(&4u32.to_le_bytes());
    {
        let (read_end, _write_end) = make_pipe();
        use std::os::fd::AsFd;
        send_msg(a.as_raw_fd(), &hostile, &[read_end.as_fd()]).expect("send");
    }
    let got = recv_exact(b.as_raw_fd(), REQUEST_LEN, MAX_FDS).expect("recv");
    assert_eq!(decode_request(&got.body), Err(DecodeError::BadVersion(4)));
    drop(got);
    drop(a);
    drop(b);
    assert_eq!(fd_count(), baseline, "ancillary fd leaked on rejection");
}

#[test]
fn overlong_counts_rejected_before_allocation() {
    // num_items = u32::MAX must fail, not allocate.
    let mut hostile = encode_response(&Response {
        result: KmsResult::Ok,
        err_msg: String::new(),
        items: vec![],
    })
    .expect("fits");
    hostile[136..140].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        decode_response(&hostile),
        Err(DecodeError::TooManyItems(u32::MAX))
    );
}
