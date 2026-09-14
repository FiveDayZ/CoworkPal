//! Bounded subprocess execution.
//!
//! All external helpers we shell out to (nvidia-smi, powershell) can hang if
//! the underlying driver / WMI provider wedges. Previously they were called via
//! `Command::output()`, which blocks forever in that case — and since the
//! monitoring pump holds the hardware-adapter `Mutex` across the call, a single
//! hung subprocess froze the entire sampling pipeline indefinitely.
//!
//! `run_command_with_timeout` spawns the process, polls bounded by `timeout`,
//! and kills the process if it overruns so the caller can degrade gracefully
//! instead of stalling.

use std::{
    io::Read,
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

/// Maximum time we will let a single helper subprocess run before killing it.
/// External monitors (nvidia-smi, powershell Get-CimInstance) normally return
/// within tens to a few hundred ms; 8s is a generous backstop against a wedged
/// driver/WMI provider while still keeping the sampling pump responsive.
pub const DEFAULT_SUBPROCESS_TIMEOUT: Duration = Duration::from_secs(8);

/// Run `command` to completion, but never longer than `timeout`.
///
/// Returns the captured `Output` (stdout/stderr/status) on success. Returns
/// `None` if the process failed to spawn, exceeded `timeout` (it is then
/// killed), or exited in a way that prevented output collection. Callers treat
/// `None` as "no data this tick" and continue, preserving availability.
///
/// Takes an owned, fully-configured `Command` (including any `creation_flags`
/// already set) so call sites keep their existing setup and only swap
/// `command.output()` → `run_command_with_timeout(command, timeout)`.
///
/// This runs entirely on the calling thread (the monitoring pump calls it from
/// `spawn_blocking`), so it never blocks the async runtime. It owns the
/// `Child`, polls with `try_wait`, and kills + reaps it on timeout — no Arc,
/// no extra threads, no Child ownership ambiguity.
pub fn run_command_with_timeout(mut command: Command, timeout: Duration) -> Option<Output> {
    // CRITICAL: `Command::output()` pipes stdout/stderr automatically, but
    // `Command::spawn()` inherits the parent's stdio by default — which leaves
    // `child.stdout` as None and silently loses ALL captured output. This was
    // the regression that broke GPU temperature display: nvidia-smi still ran,
    // but its stdout was discarded so no temperature was parsed. Pipe both
    // streams explicitly so we match `output()` semantics.
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn().ok()?;
    let deadline = Instant::now() + timeout;

    loop {
        match child.try_wait() {
            // Process exited — collect its captured stdout/stderr.
            Ok(Some(status)) => {
                let stdout = drain(child.stdout.take());
                let stderr = drain(child.stderr.take());
                return Some(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            // Still running — check the clock, then poll again shortly.
            Ok(None) => {
                if Instant::now() >= deadline {
                    tracing::warn!("subprocess exceeded {:?} timeout, killing", timeout);
                    let _ = child.kill();
                    let _ = child.wait(); // reap to avoid a zombie
                    return None;
                }
                thread::sleep(Duration::from_millis(20));
            }
            // Waiting itself failed — give up cleanly.
            Err(error) => {
                tracing::warn!("subprocess wait failed: {error}");
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// Read a child pipe to EOF into a Vec, returning an empty buffer on error.
fn drain(mut reader: Option<impl Read>) -> Vec<u8> {
    let mut buf = Vec::new();
    if let Some(r) = reader.as_mut() {
        let _ = r.read_to_end(&mut buf);
    }
    buf
}
