//! sluice's own identity on a forge (C.4 pushed refs: the forge's rules,
//! which only sluice's identity bypasses). A profile of the forge gives
//! credentials for a remote; the pushed refs profile hands them to its own
//! git calls through `sluice credential`, a helper named on that call's
//! command line and given the secret in that call's environment alone.
//! Nothing is written to a git config, so an agent's git never finds the
//! helper and never holds the secret.
//!
//! sluice speaks git's credential-helper protocol from the other end too:
//! a host may name in `[forge] credential` a command that gives the app's
//! token, which sluice asks as git asks a helper, [`CredentialCommand`]
//! (134, C.8; record 0108). The client names no forge and no tool, so any
//! host's secrets manager can stand behind it (rules core.1, core.26). The
//! words a command's failure gives, its standard error, go through the
//! scrub first, every shape it knows of a secret and no variable's value,
//! so no reader of the failure finds a token in them (record 0108 as
//! amended 2026-11-10; rule core.28).

use std::io::{BufRead, Read, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};

/// What git is given to authenticate one call.
pub struct Credentials {
    pub username: String,
    pub password: String,
    /// What they are, never the secret: whose, for what, and how old, for
    /// a report of the remote's refusing them.
    pub about: String,
}

/// An identity sluice has on a forge.
pub trait Identity {
    /// What it is called, for `sluice status`.
    fn name(&self) -> String;

    /// The URL sluice's git reaches `remote_url` by as this identity;
    /// `None` when the remote is on no forge it has an identity on.
    fn url(&self, remote_url: &str) -> Option<String>;

    /// Credentials for `url`, one [`Identity::url`] gave.
    fn credentials(&self, url: &str) -> Result<Credentials>;

    /// Credentials for `url` made afresh, never ones given before: for a
    /// call the remote refused the last ones on.
    fn renew(&self, url: &str) -> Result<Credentials>;
}

/// The forge's answer refusing an identity credentials for a remote: a
/// read that needed them is a read of the remote refused, in the forge's
/// words (133; rule core.28).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Withheld {
    /// The forge's words: what was asked of it and its answer, whole.
    pub said: String,
    /// sluice's own words on what to do, where it knows; never the
    /// forge's.
    pub note: Option<String>,
}

impl std::fmt::Display for Withheld {
    /// The forge's words, then the note on the next line.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.said)?;
        if let Some(note) = &self.note {
            write!(f, "\n{note}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Withheld {}

/// The [`Withheld`] `e` is, or that caused it.
pub fn withheld(e: &anyhow::Error) -> Option<&Withheld> {
    e.chain().find_map(|c| c.downcast_ref::<Withheld>())
}

/// The variables a git call of sluice's is given its credentials in, read
/// by the helper that call runs.
pub const USERNAME_VAR: &str = "SLUICE_CREDENTIAL_USERNAME";
pub const PASSWORD_VAR: &str = "SLUICE_CREDENTIAL_PASSWORD";

/// `sluice credential <operation>`, git's credential helper protocol: a
/// `get` answers with the credentials in its environment, when it has
/// any; `store` and `erase` keep and remove nothing. What git sends is
/// read to its end either way.
pub fn helper(operation: &str, input: &mut dyn BufRead, out: &mut dyn Write) -> Result<()> {
    let mut line = String::new();
    while input.read_line(&mut line)? > 0 && line.trim_end() != "" {
        line.clear();
    }
    if operation != "get" {
        return Ok(());
    }
    if let (Ok(username), Ok(password)) = (std::env::var(USERNAME_VAR), std::env::var(PASSWORD_VAR))
    {
        write!(out, "username={username}\npassword={password}\n")?;
    }
    Ok(())
}

/// How long a credential command has to answer before it is stopped: as
/// long as an API call has (record 0108).
pub const COMMAND_LIMIT: Duration = Duration::from_secs(30);

/// The most of a command's standard output, and of its standard error,
/// that is read; what it writes past it is passed over.
const OUTPUT_LIMIT: u64 = 64 * 1024;

/// What a credential command is asked for, in git's words: the protocol,
/// the host and the path of the repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    pub protocol: String,
    pub host: String,
    pub path: String,
}

/// What a credential command gave: the token, and its end where it gave
/// one (`password_expiry_utc`).
pub struct Given {
    pub password: String,
    pub ends: Option<DateTime<Utc>>,
}

/// A command the host names that gives a credential, asked as git asks a
/// credential helper (`gitcredentials(7)`, `git-credential(1)`): run with
/// its arguments and then the operation, with no shell, in a process group
/// of its own, what is asked written on its standard input (record 0108).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialCommand {
    program: PathBuf,
    args: Vec<String>,
    limit: Duration,
}

impl std::fmt::Display for CredentialCommand {
    /// The program and its arguments, space-separated.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.program.display())?;
        for a in &self.args {
            write!(f, " {a}")?;
        }
        Ok(())
    }
}

impl CredentialCommand {
    /// The command `program` with `args`, given [`COMMAND_LIMIT`] to answer.
    pub fn new(program: impl Into<PathBuf>, args: Vec<String>) -> CredentialCommand {
        CredentialCommand {
            program: program.into(),
            args,
            limit: COMMAND_LIMIT,
        }
    }

    /// `self`, given `limit` to answer instead: a test's.
    pub fn limited(mut self, limit: Duration) -> CredentialCommand {
        self.limit = limit;
        self
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    /// The credential for what is `asked`: the command run with `get`, its
    /// `password` and `password_expiry_utc` read from its standard output,
    /// every other key passed over as git passes it over. A command that
    /// exits non-zero, prints no `password` or gives nothing within its
    /// limit fails, its failure naming the command and giving its standard
    /// error, never its standard output, which may hold a token; at the
    /// limit its process group is killed, so a child holding its pipes
    /// open ends with it. The standard error is given as the scrub leaves
    /// it, every string shaped as a secret it knows replaced by
    /// `[redacted]` before any failure is made, so the failure, and every
    /// report, log line, kept state and status made from it, holds no token
    /// (record 0108 as amended 2026-11-10).
    pub fn get(&self, asked: &Asked) -> Result<Given> {
        let input = format!(
            "protocol={}\nhost={}\npath={}\n\n",
            asked.protocol, asked.host, asked.path
        );
        let mut child = self.spawn("get", true)?;
        let group = child.id() as i32;
        if let Some(mut stdin) = child.stdin.take() {
            // A command that reads nothing may have ended already.
            let _ = stdin.write_all(input.as_bytes());
        }
        let out = drain(child.stdout.take());
        let err = drain(child.stderr.take());
        let status = self.wait(&mut child, group, &[&out, &err])?;
        let out = out.join().unwrap_or_default();
        let err = err.join().unwrap_or_default();
        let words = crate::scrub::Scrub::shapes().scrub(String::from_utf8_lossy(&err).trim());
        let failed = |what: String| {
            let mut e = format!("[forge] credential {self} {what}");
            if !words.is_empty() {
                e.push_str(&format!(": {words}"));
            }
            anyhow!(e)
        };
        let status = match status {
            None => {
                return Err(failed(format!(
                    "gave nothing in {} and was stopped",
                    span(self.limit)
                )))
            }
            Some(s) => s,
        };
        if !status.success() {
            use std::os::unix::process::ExitStatusExt;
            let how = match (status.code(), status.signal()) {
                (Some(code), _) => format!("exited {code}"),
                (None, Some(signal)) => format!("was ended by signal {signal}"),
                (None, None) => format!("ended {status}"),
            };
            return Err(failed(how));
        }
        let (mut password, mut ends) = (None, None);
        for line in String::from_utf8_lossy(&out).lines() {
            if line.is_empty() {
                break;
            }
            match line.split_once('=') {
                Some(("password", v)) => password = Some(v.to_string()),
                Some(("password_expiry_utc", v)) => ends = Some(v.to_string()),
                _ => {}
            }
        }
        let Some(password) = password.filter(|p| !p.is_empty()) else {
            return Err(failed("gave no password".into()));
        };
        let ends = ends.and_then(|v| {
            let end = v
                .trim()
                .parse::<i64>()
                .ok()
                .and_then(|s| DateTime::from_timestamp(s, 0));
            if end.is_none() {
                static TOLD: std::sync::Once = std::sync::Once::new();
                TOLD.call_once(|| {
                    tracing::warn!(
                        "[forge] credential {self} gave a password_expiry_utc that is no time in \
                         seconds since the epoch, {v:?}: its token is held as one given with no end"
                    )
                });
            }
            end
        });
        Ok(Given { password, ends })
    }

    /// Tell the command that `password`, given for what is `asked`, was
    /// refused: run with `erase`, as git runs a helper, its exit and output
    /// not read, as git reads none. A command that cannot be started is
    /// logged, and nothing more.
    pub fn erase(&self, asked: &Asked, username: &str, password: &str) {
        let input = format!(
            "protocol={}\nhost={}\npath={}\nusername={username}\npassword={password}\n\n",
            asked.protocol, asked.host, asked.path
        );
        let mut child = match self.spawn("erase", false) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("{e:#}");
                return;
            }
        };
        let group = child.id() as i32;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
        let _ = self.wait(&mut child, group, &[]);
    }

    /// The command started for `operation`, its standard input piped, and
    /// its standard output and error piped where they are `read`. Started
    /// again for a moment while the system calls the program busy, as it
    /// does while another process still holds it open for writing.
    fn spawn(&self, operation: &str, read: bool) -> Result<Child> {
        let out = || if read { Stdio::piped() } else { Stdio::null() };
        let mut tries = 0;
        loop {
            let started = Command::new(&self.program)
                .args(&self.args)
                .arg(operation)
                .stdin(Stdio::piped())
                .stdout(out())
                .stderr(out())
                .process_group(0)
                .spawn();
            match started {
                Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy && tries < 20 => {
                    tries += 1;
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    return Err(anyhow!(
                        "[forge] credential {self} could not be started: {e}"
                    ))
                }
                Ok(c) => return Ok(c),
            }
        }
    }

    /// The command's exit once it and its readers are done, or `None` when
    /// its limit came first: its group is then killed, so a child holding
    /// its pipes open ends with it.
    fn wait(
        &self,
        child: &mut Child,
        group: i32,
        readers: &[&JoinHandle<Vec<u8>>],
    ) -> Result<Option<std::process::ExitStatus>> {
        use nix::sys::signal::{killpg, Signal};
        use nix::unistd::Pid;
        let until = Instant::now() + self.limit;
        let mut status = None;
        let done = loop {
            if status.is_none() {
                status = child.try_wait()?;
            }
            if status.is_some() && readers.iter().all(|r| r.is_finished()) {
                break true;
            }
            if Instant::now() >= until {
                break false;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if !done {
            let _ = killpg(Pid::from_raw(group), Signal::SIGKILL);
        }
        let status = match status {
            Some(s) => s,
            None => child.wait()?,
        };
        Ok(done.then_some(status))
    }
}

/// A thread reading `pipe` to its end, keeping the first [`OUTPUT_LIMIT`]
/// bytes of it.
fn drain(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut kept = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = (&mut pipe).take(OUTPUT_LIMIT).read_to_end(&mut kept);
            let _ = std::io::copy(&mut pipe, &mut std::io::sink());
        }
        kept
    })
}

/// A limit in the words a person reads: `30s`, or `300ms` under a second.
fn span(d: Duration) -> String {
    if d >= Duration::from_secs(1) && d.subsec_millis() == 0 {
        format!("{}s", d.as_secs())
    } else {
        format!("{}ms", d.as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory of the test's own, made afresh.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sluice-credential-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A credential command's script in `dir`, mode `0755`, written and
    /// closed before anything runs it.
    fn script(dir: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::OpenOptionsExt;
        let path = dir.join("helper");
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o755)
            .open(&path)
            .unwrap();
        f.write_all(format!("#!/bin/sh\n{body}").as_bytes())
            .unwrap();
        f.sync_all().unwrap();
        drop(f);
        path
    }

    fn asked() -> Asked {
        Asked {
            protocol: "https".into(),
            host: "github.com".into(),
            path: "o/r".into(),
        }
    }

    /// The forge-identity spec's "The command asked as a credential helper"
    /// (record 0108).
    #[test]
    fn a_credential_command_is_asked_as_git_asks_a_helper() {
        let dir = scratch("asked");
        let program = script(
            &dir,
            &format!(
                "echo \"$@\" > {d}/args\ncat > {d}/input\n\
                 printf 'username=x-access-token\\npassword=tok-1\\npassword_expiry_utc=1790000000\\nquit=0\\n'\n",
                d = dir.display()
            ),
        );
        let command = CredentialCommand::new(&program, vec!["--app".into(), "sluice".into()]);
        assert_eq!(
            command.to_string(),
            format!("{} --app sluice", program.display())
        );
        let given = command.get(&asked()).unwrap();
        assert_eq!(given.password, "tok-1");
        assert_eq!(given.ends, DateTime::from_timestamp(1_790_000_000, 0));
        let args = std::fs::read_to_string(dir.join("args")).unwrap();
        let input = std::fs::read_to_string(dir.join("input")).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(args.trim_end(), "--app sluice get");
        assert_eq!(input, "protocol=https\nhost=github.com\npath=o/r\n\n");
    }

    /// The forge-identity spec's "The command refuses", "The command prints
    /// no password" and "The command hangs" (record 0108).
    #[test]
    fn a_credential_command_that_gives_no_token_says_why() {
        let refuses = scratch("refuses");
        let program = script(
            &refuses,
            "cat > /dev/null\necho password=tok-1\necho 'no drive on this host serves o/r' >&2\nexit 1\n",
        );
        let e = format!(
            "{:#}",
            CredentialCommand::new(&program, vec![])
                .get(&asked())
                .err()
                .unwrap()
        );
        assert!(e.starts_with("[forge] credential "), "{e}");
        assert!(e.contains(&program.display().to_string()), "{e}");
        assert!(e.contains("exited 1"), "{e}");
        assert!(e.contains("no drive on this host serves o/r"), "{e}");
        assert!(!e.contains("tok-1"), "{e}");

        let no_password = scratch("no-password");
        let program = script(
            &no_password,
            "cat > /dev/null\necho username=x-access-token\n",
        );
        let e = format!(
            "{:#}",
            CredentialCommand::new(&program, vec![])
                .get(&asked())
                .err()
                .unwrap()
        );
        assert!(e.starts_with("[forge] credential "), "{e}");
        assert!(e.contains("gave no password"), "{e}");

        let hangs = scratch("hangs");
        let program = script(&hangs, "sleep 30 &\nwait\n");
        let started = Instant::now();
        let e = format!(
            "{:#}",
            CredentialCommand::new(&program, vec![])
                .limited(Duration::from_millis(300))
                .get(&asked())
                .err()
                .unwrap()
        );
        assert!(started.elapsed() < Duration::from_secs(5), "{e}");
        assert!(e.starts_with("[forge] credential "), "{e}");
        assert!(e.contains("gave nothing in 300ms and was stopped"), "{e}");
        for dir in [refuses, no_password, hangs] {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    /// The forge-identity spec's "The command's words hold a token" and "A
    /// command that prints no password names a token" (record 0108 as
    /// amended 2026-11-10).
    #[test]
    fn a_credential_commands_words_carry_no_token() {
        let refuses = scratch("words-refuses");
        let program = script(
            &refuses,
            "cat > /dev/null
\
             echo 'the cached ghs_0123456789abcdefghijABCDEFGHIJ012345 was refused' >&2
exit 1
",
        );
        let e = format!(
            "{:#}",
            CredentialCommand::new(&program, vec![])
                .get(&asked())
                .err()
                .unwrap()
        );
        assert!(e.starts_with("[forge] credential "), "{e}");
        assert!(e.contains(&program.display().to_string()), "{e}");
        assert!(e.contains("exited 1"), "{e}");
        assert!(e.contains("the cached [redacted] was refused"), "{e}");
        assert!(!e.contains("0123456789abcdefghij"), "{e}");

        let no_password = scratch("words-no-password");
        let program = script(
            &no_password,
            "cat > /dev/null
echo username=x-access-token
\
             echo github_pat_0123456789abcdefghij_ABCDEFGHIJ >&2
",
        );
        let e = format!(
            "{:#}",
            CredentialCommand::new(&program, vec![])
                .get(&asked())
                .err()
                .unwrap()
        );
        assert!(e.starts_with("[forge] credential "), "{e}");
        assert!(e.contains("gave no password"), "{e}");
        assert!(e.contains("[redacted]"), "{e}");
        assert!(!e.contains("0123456789abcdefghij"), "{e}");
        for dir in [refuses, no_password] {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    /// `erase` tells the command the refused token as git tells a helper,
    /// its exit not read (record 0108).
    #[test]
    fn erase_tells_the_command_the_refused_token() {
        let dir = scratch("erase");
        let program = script(
            &dir,
            &format!(
                "for a in \"$@\"; do last=$a; done\necho \"$last\" > {d}/operation\n\
                 cat > {d}/input\nexit 1\n",
                d = dir.display()
            ),
        );
        CredentialCommand::new(&program, vec!["--app".into(), "sluice".into()]).erase(
            &asked(),
            "x-access-token",
            "tok-1",
        );
        let operation = std::fs::read_to_string(dir.join("operation")).unwrap();
        let input = std::fs::read_to_string(dir.join("input")).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(operation.trim_end(), "erase");
        assert_eq!(
            input,
            "protocol=https\nhost=github.com\npath=o/r\nusername=x-access-token\npassword=tok-1\n\n"
        );
    }
}
