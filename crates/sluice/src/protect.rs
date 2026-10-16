//! Branch protection on one host: a `reference-transaction` hook refuses
//! any update to a branch the policy names that sluice did not make, and
//! reports it in the branch's log (1; C.4 local refs; rule core.4;
//! `docs/plans/phase-1-integrations.md` section 2).
//!
//! sluice moves branches through gix, which runs no hooks, so its own
//! moves never reach the hook. Everything that runs git does. This guards
//! against sessions running git; it is no security boundary.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context as _, Result};
use sluice_core::Sluice;
use sluice_seams::Rev;

/// The line that marks a hook as sluice's.
const MARKER: &str =
    "# sluice: refuses updates to branches sluice protects (docs: branch protection). Installed by sluice.";
const HOOK: &str = "reference-transaction";
const CHAINED: &str = "reference-transaction.pre-sluice";

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .stdin(Stdio::null())
        .output()
        .context("running git")?;
    if !out.status.success() {
        bail!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

/// The repository's hook directory, as git finds it from `dir`: it
/// follows `core.hooksPath`. A hook directory inside a worktree is a
/// tracked one, which sluice will not write.
fn hookdir(dir: &Path) -> Result<PathBuf> {
    let hooks = PathBuf::from(git(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-path", "hooks"],
    )?);
    let common = PathBuf::from(git(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    if hooks.starts_with(&common) {
        return Ok(hooks);
    }
    let list = git(dir, &["worktree", "list", "--porcelain"])?;
    let mut path = None;
    for line in list.lines().chain(std::iter::once("")) {
        if let Some(p) = line.strip_prefix("worktree ") {
            path = Some(PathBuf::from(p));
        } else if line == "bare" {
            path = None;
        } else if line.is_empty() {
            if let Some(p) = path.take() {
                if hooks.starts_with(&p) {
                    bail!(
                        "core.hooksPath is {}, inside the worktree {}: sluice will not write a tracked hook; \
                         add `{} hook {HOOK} \"$1\"` to that hook yourself",
                        hooks.display(),
                        p.display(),
                        std::env::current_exe()?.display()
                    );
                }
            }
        }
    }
    Ok(hooks)
}

fn shim(program: &Path) -> String {
    let p = program.display().to_string().replace('\'', "'\\''");
    format!(
        "#!/bin/sh\n{MARKER}\ninput=$(cat)\nd=$(dirname \"$0\")\n\
         if [ -x '{p}' ]; then\n\
         \x20 printf '%s\\n' \"$input\" | '{p}' hook {HOOK} \"$1\" || exit $?\n\
         else\n\
         \x20 echo \"warning: sluice's branch protection is installed but {p} is missing\" >&2\n\
         fi\n\
         [ -x \"$d/{CHAINED}\" ] || exit 0\n\
         printf '%s\\n' \"$input\" | \"$d/{CHAINED}\" \"$@\"\n"
    )
}

/// Install sluice's hook in the repository holding `dir`, for `program`,
/// keeping any hook already there by running it after sluice's. Writes
/// nothing when sluice's hook for `program` is already in place.
pub fn install(dir: &Path, program: &Path) -> Result<()> {
    let hooks = hookdir(dir)?;
    std::fs::create_dir_all(&hooks)?;
    let hook = hooks.join(HOOK);
    let want = shim(program);
    match std::fs::read_to_string(&hook) {
        Ok(have) if have == want => return Ok(()),
        Ok(have) if have.lines().nth(1) == Some(MARKER) => {}
        Ok(_) => {
            let chained = hooks.join(CHAINED);
            if chained.exists() {
                bail!(
                    "{} and {} both exist: sluice will not replace either",
                    hook.display(),
                    chained.display()
                );
            }
            std::fs::rename(&hook, &chained)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    let tmp = hooks.join(format!(".{HOOK}.sluice.{}", std::process::id()));
    std::fs::write(&tmp, want)?;
    std::fs::set_permissions(&tmp, std::os::unix::fs::PermissionsExt::from_mode(0o755))?;
    std::fs::rename(&tmp, &hook)?;
    Ok(())
}

/// Take sluice's hook out, putting back the hook it chained.
pub fn remove(dir: &Path) -> Result<()> {
    let hooks = hookdir(dir)?;
    let hook = hooks.join(HOOK);
    match std::fs::read_to_string(&hook) {
        Ok(have) if have.lines().nth(1) == Some(MARKER) => {
            std::fs::remove_file(&hook)?;
            let chained = hooks.join(CHAINED);
            if chained.exists() {
                std::fs::rename(&chained, &hook)?;
            }
            Ok(())
        }
        Ok(_) => bail!("{} is not sluice's hook", hook.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// What a refused update is told, on stderr.
fn refusal(branch: &str, why: &str) -> String {
    format!(
        "error: {branch} is protected by sluice; submit the change to sluice instead{why}\n\
         the worktree may hold the attempted change: git merge --abort or git reset --hard"
    )
}

/// The command that ran git: the hook is sluice's parent's child, and git
/// the shim's parent. Read from the kernel, never by running `ps`, which
/// a sandbox refuses to start (it is setuid on macOS).
fn caller() -> String {
    let shim = nix::unistd::getppid().as_raw();
    parent_of(shim)
        .and_then(command_of)
        .unwrap_or_else(|| "unknown".into())
}

#[cfg(target_os = "macos")]
fn parent_of(pid: i32) -> Option<i32> {
    use nix::libc;
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
    // SAFETY: `info` is a `proc_bsdinfo` of `size` bytes, as the call asks.
    let n = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    (n == size).then_some(info.pbi_ppid as i32)
}

/// The process's arguments, from `KERN_PROCARGS2`: its argument count,
/// the executable's path, padding, then each argument.
#[cfg(target_os = "macos")]
fn command_of(pid: i32) -> Option<String> {
    use nix::libc;
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
    let mut size: usize = 0;
    // SAFETY: a null buffer asks only for the size.
    let rc = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 || size < 4 {
        return None;
    }
    let mut buf = vec![0u8; size];
    // SAFETY: `buf` holds `size` bytes, as the call asks.
    let rc = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            buf.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 || size < 4 {
        return None;
    }
    buf.truncate(size);
    let argc = i32::from_ne_bytes(buf[..4].try_into().ok()?) as usize;
    let rest = &buf[4..];
    let path_end = rest.iter().position(|b| *b == 0)?;
    let args: Vec<String> = rest[path_end..]
        .split(|b| *b == 0)
        .filter(|a| !a.is_empty())
        .take(argc)
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect();
    (!args.is_empty()).then(|| args.join(" "))
}

#[cfg(not(target_os = "macos"))]
fn parent_of(pid: i32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command is in parentheses and may hold spaces: the parent's id is
    // the second field after the last `)`.
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

#[cfg(not(target_os = "macos"))]
fn command_of(pid: i32) -> Option<String> {
    let args = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let args: Vec<String> = args
        .split(|b| *b == 0)
        .filter(|a| !a.is_empty())
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect();
    (!args.is_empty()).then(|| args.join(" "))
}

/// `sluice hook reference-transaction <state>`: refuse, in `prepared`,
/// any update to a protected branch that is not a creation or a no-op.
/// Returns the exit: 0 allows, 1 refuses.
pub fn hook(s: &Sluice<'_>, state: &str, input: &mut dyn Read) -> Result<i32> {
    // Symbolic refs are resolved only in `prepared`: in `preparing` a
    // commit on a checked-out branch arrives as HEAD.
    if state != "prepared" {
        return Ok(0);
    }
    let mut text = String::new();
    input.read_to_string(&mut text)?;
    let seams = s.seams();
    let mut refused = Vec::new();
    for line in text.lines() {
        let mut f = line.split(' ');
        let (Some(_old), Some(new), Some(name)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        let Some(branch) = name.strip_prefix("refs/heads/") else {
            continue;
        };
        // The caller's expected value is often all zeros; the ref as it
        // stands is still the old one while the transaction is prepared.
        let Some(cur) = seams.branch.read(branch)? else {
            continue; // a creation
        };
        let deleting = new.bytes().all(|b| b == b'0');
        let new = (!deleting).then(|| Rev::new(new));
        if new.as_ref() == Some(&cur) || !s.protects(branch, &cur)? {
            continue;
        }
        let why = if deleting {
            let needing = s.needing(branch)?;
            if needing.is_empty() {
                continue;
            }
            let ids: Vec<String> = needing.iter().map(|i| format!("#{i}")).collect();
            format!(
                "\n{branch} still holds {}: withdraw or land them before deleting it",
                ids.join(", ")
            )
        } else {
            String::new()
        };
        crate::telemetry::warn(&refusal(branch, &why));
        refused.push((branch.to_string(), cur, new));
    }
    if refused.is_empty() {
        return Ok(0);
    }
    let by = caller();
    for (branch, from, to) in &refused {
        // Reported when the branch's lock comes within a second; the
        // refusal stands either way.
        let until = Instant::now() + Duration::from_secs(1);
        let guard = loop {
            if let Some(g) = seams.state.lock(branch)? {
                break Some(g);
            }
            if Instant::now() >= until {
                break None;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        if guard.is_some() {
            s.record_protected(branch, from, to.as_ref(), &by)
                .map_err(|e| anyhow!("reporting the refused update to {branch}: {e:#}"))?;
        }
    }
    Ok(1)
}
