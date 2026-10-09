//! Privileged KMS helper binary. Mirrors `kms/server/kms_server.c`.
//! Usage: `gsr-kms-server <domain_socket_path> <card_path>`.
//!
//! Single-purpose process: open the DRM card, reverse-connect to the
//! client's socket (5 s deadline like C — `pkexec` cannot pass fds, hence
//! the reversed roles), then serve `GET_KMS` and connection-replacement
//! requests. Exit codes mirror C: 1 = usage, 2 = setup failure,
//! 3 = orderly client disconnect.
//!
//! DEVIATION-HELPER-LOGS: daemon diagnostics stay English on stderr (like
//! C); every user-facing message surfaces localized through the client.

#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(unix)]
use std::time::{Duration, Instant};

#[cfg(unix)]
use gsr_kms_server::grab::{grab_frame, Card};
#[cfg(unix)]
use gsr_kms_server::protocol::{
    decode_request, encode_response, RequestType, Response, REQUEST_LEN,
};
#[cfg(unix)]
use gsr_kms_server::transport::{recv_exact, send_msg};

/// Reverse-connect deadline (C loops `connect` for 5 s).
#[cfg(unix)]
const CONNECT_DEADLINE: Duration = Duration::from_secs(5);
/// Delay between connect attempts (C `usleep(30ms)`).
#[cfg(unix)]
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(30);

#[cfg(unix)]
fn usage() -> ! {
    eprintln!("usage: gsr-kms-server <domain_socket_path> <card_path>");
    std::process::exit(1);
}

/// Connect to the client's listening socket, retrying refused/missing
/// sockets until the deadline (mirrors C; other errors are fatal).
#[cfg(unix)]
fn reverse_connect(socket_path: &str) -> io::Result<UnixStream> {
    let deadline = Instant::now() + CONNECT_DEADLINE;
    loop {
        match UnixStream::connect(socket_path) {
            Ok(stream) => return Ok(stream),
            Err(e)
                if matches!(
                    e.raw_os_error(),
                    Some(libc::ECONNREFUSED) | Some(libc::ENOENT)
                ) && Instant::now() < deadline =>
            {
                std::thread::sleep(CONNECT_RETRY_DELAY);
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(unix)]
fn send_response(stream: &UnixStream, response: &Response, fds: &[OwnedFd]) {
    let body = encode_response(response).expect("grabbed response always fits");
    let borrowed: Vec<_> = fds.iter().map(|fd| fd.as_fd()).collect();
    if send_msg(stream.as_raw_fd(), &body, &borrowed).is_err() {
        eprintln!("kms server error: failed to send response");
    }
    // Owned fds drop here: ownership transfers to the client, like C's
    // close-after-send.
}

#[cfg(unix)]
fn serve(stream: UnixStream, card: &Card) -> ! {
    let mut conn = stream;
    loop {
        let received = match recv_exact(conn.as_raw_fd(), REQUEST_LEN, 1) {
            Ok(received) => received,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => std::process::exit(3),
            Err(e) => {
                eprintln!("kms server error: failed to receive request: {e}");
                continue;
            }
        };
        // Received fds drop here on every path (C leaks some of them).
        match decode_request(&received.body) {
            Err(e) => {
                eprintln!("kms server error: bad request: {e}");
            }
            Ok(RequestType::ReplaceConnection) => {
                let mut fds = received.fds;
                if fds.len() != 1 {
                    eprintln!("kms server error: REPLACE_CONNECTION without exactly one fd");
                    continue;
                }
                // Adopt the replacement connection for subsequent requests.
                let fd = fds.pop().expect("checked");
                conn = unsafe { UnixStream::from_raw_fd(fd.into_raw_fd()) };
            }
            Ok(RequestType::GetKms) => {
                let grabbed = grab_frame(card);
                send_response(&conn, &grabbed.response, &grabbed.fds);
            }
        }
    }
}

#[cfg(unix)]
fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() != 3 {
        usage();
    }
    let card = match Card::open(&argv[2]) {
        Ok(card) => card,
        Err(e) => {
            eprintln!("kms server error: failed to open {}: {e}", argv[2]);
            std::process::exit(2);
        }
    };
    let (planes, atomic) = card.set_client_caps();
    if !planes {
        eprintln!("kms server error: failed to enable UNIVERSAL_PLANES");
        std::process::exit(2);
    }
    if !atomic {
        eprintln!("kms server warning: failed to enable ATOMIC, wrong monitor may be captured");
    }
    match reverse_connect(&argv[1]) {
        Ok(stream) => serve(stream, &card),
        Err(e) => {
            eprintln!("kms server error: failed to connect to {}: {e}", argv[1]);
            std::process::exit(2);
        }
    }
}

/// The KMS helper is Linux-only (DRM + UNIX sockets); Windows captures via
/// DXGI/WGC with no privileges needed.
#[cfg(not(unix))]
fn main() {
    eprintln!("gsr-kms-server is only supported on Linux");
    std::process::exit(1);
}
