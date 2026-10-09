//! The helper programs some metrics come from (nvidia-smi, dumpsys, pmset, getprop), run with
//! a time limit. They run on the sampler thread: one that hangs (nvidia-smi after the GPU fell
//! off the bus, dumpsys while `system_server` is stuck) would freeze every metric while the
//! client still looks online.

use std::{
    io::{self, ErrorKind, Read},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

const TIMEOUT: Duration = Duration::from_secs(3);

pub struct Output {
    pub success: bool,
    pub stdout: String,
}

/// Runs `command` and collects its stdout. Errors: it could not start (`NotFound` when the
/// program is missing), or it did not finish within 3 seconds (`TimedOut`; it is killed).
pub fn run(command: &mut Command) -> io::Result<Output> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let (tx, rx) = mpsc::channel();
    // Read on another thread so a full pipe never blocks the child, and the wait can time out
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    if let Ok(buf) = rx.recv_timeout(TIMEOUT) {
        return Ok(Output {
            success: child.wait()?.success(),
            stdout: String::from_utf8_lossy(&buf).into_owned(),
        });
    }
    let _ = child.kill();
    // A process stuck in the kernel may not die at once; reap it without waiting here
    thread::spawn(move || child.wait());
    Err(io::Error::new(ErrorKind::TimedOut, "timed out"))
}
