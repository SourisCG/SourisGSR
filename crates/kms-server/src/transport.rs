//! UNIX-socket transport with `SCM_RIGHTS` fd passing.
//! Mirrors the `sendmsg`/`recvmsg` flow of `kms_server.c`/`kms_client.c`.
//!
//! Hardening vs C (all tested, `DEVIATION-SEC`):
//! - Received fds arrive with `MSG_CMSG_CLOEXEC` (C leaks them into children).
//! - Every `SCM_RIGHTS` control message is consumed and validated, not just
//!   the first; unknown/unbounded counts fail closed with fds owned (and
//!   therefore closed) by `Received`.
//! - `recv_exact` reads fixed-size bodies; timeouts are caller-configured
//!   (`set_recv_timeout`) instead of C's unbounded client wait.

use std::io;
use std::mem;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};

/// A fully received message: body plus owned ancillary fds.
/// Dropping closes every fd (fixes the C fd leaks on error paths).
#[derive(Debug)]
pub struct Received {
    pub body: Vec<u8>,
    pub fds: Vec<OwnedFd>,
}

/// Maximum ancillary fds accepted per message (protocol needs at most 32).
pub const MAX_ANCILLARY_FDS: usize = 32;

/// Send `body` with `fds` attached via a single `SCM_RIGHTS` message.
pub fn send_msg(sock: RawFd, body: &[u8], fds: &[BorrowedFd<'_>]) -> io::Result<()> {
    let iov = libc::iovec {
        iov_base: body.as_ptr() as *mut libc::c_void,
        iov_len: body.len(),
    };
    let mut header: libc::msghdr = unsafe { mem::zeroed() };
    header.msg_iov = &iov as *const libc::iovec as *mut libc::iovec;
    header.msg_iovlen = 1;

    // Control buffer sized for the fd array; kept alive for the call.
    let control_len = if fds.is_empty() {
        0
    } else {
        unsafe { libc::CMSG_SPACE((fds.len() * mem::size_of::<RawFd>()) as libc::c_uint) as usize }
    };
    let mut control = vec![0u8; control_len];
    if !fds.is_empty() {
        header.msg_control = control.as_mut_ptr() as *mut libc::c_void;
        header.msg_controllen = control.len();
        unsafe {
            let cmsg = libc::CMSG_FIRSTHDR(&header);
            if cmsg.is_null() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "short control buffer",
                ));
            }
            (*cmsg).cmsg_level = libc::SOL_SOCKET;
            (*cmsg).cmsg_type = libc::SCM_RIGHTS;
            (*cmsg).cmsg_len =
                libc::CMSG_LEN((fds.len() * mem::size_of::<RawFd>()) as libc::c_uint) as usize;
            let payload = libc::CMSG_DATA(cmsg) as *mut RawFd;
            for (i, fd) in fds.iter().enumerate() {
                *payload.add(i) = fd.as_raw_fd();
            }
        }
    }

    let sent = unsafe { libc::sendmsg(sock, &header, libc::MSG_NOSIGNAL) };
    if sent < 0 {
        return Err(io::Error::last_os_error());
    }
    if sent as usize != body.len() {
        return Err(io::Error::new(
            io::ErrorKind::WriteZero,
            "short sendmsg write",
        ));
    }
    Ok(())
}

/// Set a receive timeout in whole seconds (0 disables). Used by the client
/// so a dead helper cannot hang it forever (C waits unboundedly).
#[cfg(target_os = "linux")]
pub fn set_recv_timeout(sock: RawFd, secs: u64) -> io::Result<()> {
    let timeout = libc::timeval {
        tv_sec: secs as libc::time_t,
        tv_usec: 0,
    };
    let ret = unsafe {
        libc::setsockopt(
            sock,
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            &timeout as *const libc::timeval as *const libc::c_void,
            mem::size_of::<libc::timeval>() as libc::socklen_t,
        )
    };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Receive exactly `body_len` bytes plus up to `max_fds` ancillary fds.
/// Fails closed: any protocol violation drops (closes) received fds.
pub fn recv_exact(sock: RawFd, body_len: usize, max_fds: usize) -> io::Result<Received> {
    let mut body = vec![0u8; body_len];
    let mut filled = 0;
    let mut fds: Vec<OwnedFd> = Vec::new();

    while filled < body_len {
        let mut iov = libc::iovec {
            iov_base: body[filled..].as_mut_ptr() as *mut libc::c_void,
            iov_len: body_len - filled,
        };
        let control_len = unsafe {
            libc::CMSG_SPACE((MAX_ANCILLARY_FDS * mem::size_of::<RawFd>()) as libc::c_uint) as usize
        };
        let mut control = vec![0u8; control_len];
        let mut header: libc::msghdr = unsafe { mem::zeroed() };
        header.msg_iov = &mut iov;
        header.msg_iovlen = 1;
        header.msg_control = control.as_mut_ptr() as *mut libc::c_void;
        header.msg_controllen = control.len();

        // WAITALL for the body chunk; CLOEXEC for received fds.
        let got = unsafe {
            libc::recvmsg(
                sock,
                &mut header,
                libc::MSG_WAITALL | libc::MSG_CMSG_CLOEXEC,
            )
        };
        if got < 0 {
            return Err(io::Error::last_os_error());
        }
        if got == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "peer closed the socket",
            ));
        }
        filled += got as usize;

        // Consume and validate every control message (C reads only the first).
        let mut cmsg = unsafe { libc::CMSG_FIRSTHDR(&header) };
        while !cmsg.is_null() {
            unsafe {
                if (*cmsg).cmsg_level == libc::SOL_SOCKET && (*cmsg).cmsg_type == libc::SCM_RIGHTS {
                    let data_len = (*cmsg).cmsg_len as usize;
                    let header_len = libc::CMSG_LEN(0) as usize;
                    if data_len < header_len
                        || !(data_len - header_len).is_multiple_of(mem::size_of::<RawFd>())
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "malformed SCM_RIGHTS message",
                        ));
                    }
                    let count = (data_len - header_len) / mem::size_of::<RawFd>();
                    if fds.len() + count > max_fds {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "too many ancillary fds",
                        ));
                    }
                    let payload = libc::CMSG_DATA(cmsg) as *const RawFd;
                    for i in 0..count {
                        fds.push(OwnedFd::from_raw_fd(*payload.add(i)));
                    }
                }
                cmsg = libc::CMSG_NXTHDR(&header, cmsg);
            }
        }
    }
    body.truncate(filled);
    Ok(Received { body, fds })
}

/// Create a `SOCK_STREAM | SOCK_CLOEXEC` UNIX socketpair.
pub fn socketpair() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut pair = [0 as RawFd; 2];
    let ret = unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_CLOEXEC,
            0,
            pair.as_mut_ptr(),
        )
    };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    unsafe { Ok((OwnedFd::from_raw_fd(pair[0]), OwnedFd::from_raw_fd(pair[1]))) }
}

/// Peer UID via `SO_PEERCRED` (Linux). The client refuses helpers running
/// as an unexpected user (C never checks).
#[cfg(target_os = "linux")]
pub fn peer_uid(sock: RawFd) -> io::Result<u32> {
    let mut cred: libc::ucred = unsafe { mem::zeroed() };
    let mut len = mem::size_of::<libc::ucred>() as libc::socklen_t;
    let ret = unsafe {
        libc::getsockopt(
            sock,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut cred as *mut libc::ucred as *mut libc::c_void,
            &mut len,
        )
    };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(cred.uid)
}
