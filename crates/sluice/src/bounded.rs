//! A process sluice waits on for a limited time, in a process group of its
//! own: its output read as it goes, and at its limit the group ended,
//! asked first and then killed, so nothing it started outlives the wait
//! and nothing sluice holds across the wait outlives the limit (134; rules
//! core.10, core.29). A git call to a remote is run so, every try of it
//! (`profiles::pushed_refs`; record 0109), and a credential command the
//! host names (record 0108).

use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How long a group asked to end at its limit has before it is killed:
/// git, asked, removes the lock files it holds in the repository.
pub const GRACE: Duration = Duration::from_secs(2);

/// How long the output of a group ended at its limit is waited for once
/// the group is gone: a process outside it may hold a pipe open.
const DRAINED: Duration = Duration::from_millis(200);

/// How a process ended, and what it printed until then.
pub struct Ended {
    /// Its exit, by itself or by the signal that ended its group.
    pub status: ExitStatus,
    /// Whether its limit came first, and its group was ended.
    pub stopped: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Whether what was kept of each pipe is less than all it gave: bytes
    /// past the first `kept` passed over, the pipe failing past them, or
    /// the pipe not read to its end when its group was ended. A reader
    /// that must never take part of a line as whole, a credential
    /// command's words among them, cuts it at its last whole line (record
    /// 0108 as amended 2026-11-10).
    pub stdout_cut: bool,
    pub stderr_cut: bool,
}

/// `command` run in a process group of its own, its standard output and
/// error read, to `limit`. Its standard input is the caller's to set.
pub fn output(command: &mut Command, limit: Duration) -> io::Result<Ended> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    finish(&mut child, limit, u64::MAX)
}

/// Wait for `child`, which leads a process group of its own, to `limit`,
/// reading what it prints on each pipe it was given, the first `kept`
/// bytes of each kept. At the limit its group is asked to end, given
/// [`GRACE`], then killed, so a process it started that holds its pipes
/// open ends with it.
pub fn finish(child: &mut Child, limit: Duration, kept: u64) -> io::Result<Ended> {
    let out = drain(child.stdout.take(), kept);
    let err = drain(child.stderr.take(), kept);
    let until = Instant::now() + limit;
    let mut status = None;
    let done = loop {
        if status.is_none() {
            status = child.try_wait()?;
        }
        if status.is_some() && out.done() && err.done() {
            break true;
        }
        if Instant::now() >= until {
            break false;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if !done {
        crate::runs::end_group(child.id() as i32, GRACE, &mut || {
            if status.is_none() {
                status = child.try_wait().ok().flatten();
            }
            status.is_some()
        });
        let until = Instant::now() + DRAINED;
        while !(out.done() && err.done()) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let status = match status {
        Some(s) => s,
        None => child.wait()?,
    };
    let (stdout, stdout_cut) = out.taken();
    let (stderr, stderr_cut) = err.taken();
    Ok(Ended {
        status,
        stopped: !done,
        stdout,
        stderr,
        stdout_cut,
        stderr_cut,
    })
}

/// A limit in the words a person reads: `30s`, or `300ms` under a second.
pub fn span(d: Duration) -> String {
    if d >= Duration::from_secs(1) && d.subsec_millis() == 0 {
        format!("{}s", d.as_secs())
    } else {
        format!("{}ms", d.as_millis())
    }
}

/// A pipe read to its end by a thread of its own, into a buffer the
/// caller may take before the end comes.
struct Drain {
    read: Arc<Mutex<Kept>>,
    thread: Option<JoinHandle<()>>,
}

/// What a [`Drain`] has read of its pipe so far.
#[derive(Default)]
struct Kept {
    bytes: Vec<u8>,
    /// Whether it passed over anything past what it keeps, or the pipe
    /// failed once its kept part was full.
    passed_over: bool,
}

impl Drain {
    fn done(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// What was kept so far, and whether it is less than all the pipe
    /// gave: anything passed over, or the pipe not yet read to its end.
    fn taken(self) -> (Vec<u8>, bool) {
        let unfinished = !self.done();
        let mut read = self.read.lock().unwrap_or_else(|p| p.into_inner());
        (
            std::mem::take(&mut read.bytes),
            read.passed_over || unfinished,
        )
    }
}

/// A thread reading `pipe` to its end, keeping its first `kept` bytes.
fn drain(pipe: Option<impl Read + Send + 'static>, kept: u64) -> Drain {
    let read = Arc::new(Mutex::new(Kept::default()));
    let thread = pipe.map(|pipe| {
        let read = read.clone();
        std::thread::spawn(move || {
            let mut pipe = pipe;
            let mut left = kept;
            let mut chunk = [0u8; 8192];
            loop {
                match pipe.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        let keep = n.min(usize::try_from(left).unwrap_or(usize::MAX));
                        left -= keep as u64;
                        let mut r = read.lock().unwrap_or_else(|p| p.into_inner());
                        r.bytes.extend_from_slice(&chunk[..keep]);
                        r.passed_over |= keep < n;
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => {
                        if left == 0 {
                            read.lock().unwrap_or_else(|p| p.into_inner()).passed_over = true;
                        }
                        break;
                    }
                }
            }
        })
    });
    Drain { read, thread }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rules core.10, core.29: a process that never ends is ended at its
    /// limit with what it started, which holds its pipes, and what it
    /// printed until then is kept.
    #[test]
    fn a_process_that_never_ends_is_ended_with_its_group() {
        let started = Instant::now();
        let ended = output(
            Command::new("sh")
                .args(["-c", "echo begun; echo said >&2; sleep 60 & sleep 60"])
                .stdin(Stdio::null()),
            Duration::from_millis(300),
        )
        .unwrap();
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(ended.stopped);
        assert_eq!(ended.stdout, b"begun\n");
        assert_eq!(ended.stderr, b"said\n");
    }

    /// A process that ends within its limit gives its exit and all it
    /// printed.
    #[test]
    fn a_process_that_ends_gives_its_exit() {
        let ended = output(
            Command::new("sh")
                .args(["-c", "echo out; echo err >&2; exit 3"])
                .stdin(Stdio::null()),
            Duration::from_secs(10),
        )
        .unwrap();
        assert!(!ended.stopped);
        assert_eq!(ended.status.code(), Some(3));
        assert_eq!(ended.stdout, b"out\n");
        assert_eq!(ended.stderr, b"err\n");
    }

    /// Record 0108 as amended: what a pipe gives past its kept part is
    /// passed over and the part said cut; a pipe that gives no more is
    /// whole.
    #[test]
    fn what_passes_the_kept_part_is_said_cut() {
        let mut child = Command::new("sh")
            .args(["-c", "printf 'whole\\npart' >&2; printf 'out'"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .unwrap();
        let ended = finish(&mut child, Duration::from_secs(10), 8).unwrap();
        assert!(!ended.stopped);
        assert_eq!(ended.stderr, b"whole\npa");
        assert!(ended.stderr_cut);
        assert_eq!(ended.stdout, b"out");
        assert!(!ended.stdout_cut);
    }

    #[test]
    fn a_limit_is_said_in_seconds_or_milliseconds() {
        assert_eq!(span(Duration::from_secs(30)), "30s");
        assert_eq!(span(Duration::from_millis(300)), "300ms");
    }
}
