//! Branch protection on one host: a `reference-transaction` hook refuses
//! any update to a branch the policy names that sluice did not make, and
//! reports it in the branch's log (1, 124; C.4 local refs; rule core.4;
//! `docs/plans/phase-1-integrations.md` section 2).
//!
//! It judges a write by the value the branch's ref holds on this host,
//! never by a fetch of its remote: for a branch kept on a remote that ref
//! is sluice's copy, which only sluice's rounds level (124). It judges the
//! repository git names, wherever the command ran, and lets through, said
//! once, a transaction it cannot read for a reason of its own.
//!
//! sluice moves branches through gix, which runs no hooks, so its own
//! moves never reach the hook. Everything that runs git does. This guards
//! against sessions running git; it is no security boundary.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context as _, Result};
use sluice_core::Sluice;
use sluice_seams::{HostCopy, Rev};

use crate::exec::Host;

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
    match located(dir)? {
        (hooks, None) => Ok(hooks),
        (hooks, Some(worktree)) => bail!(
            "core.hooksPath is {}, inside the worktree {}: sluice will not write a tracked hook; \
             add `{} hook {HOOK} \"$1\"` to that hook yourself",
            hooks.display(),
            worktree.display(),
            std::env::current_exe()?.display()
        ),
    }
}

/// The repository's hook directory, as git finds it from `dir`, and the
/// worktree it is inside, where it is a tracked one.
fn located(dir: &Path) -> Result<(PathBuf, Option<PathBuf>)> {
    let hooks = PathBuf::from(git(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-path", "hooks"],
    )?);
    let common = PathBuf::from(git(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    if hooks.starts_with(&common) {
        return Ok((hooks, None));
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
                    return Ok((hooks, Some(p)));
                }
            }
        }
    }
    Ok((hooks, None))
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
/// nothing when sluice's hook for `program` is already in place, and says
/// so with `false`.
pub fn install(dir: &Path, program: &Path) -> Result<bool> {
    let hooks = hookdir(dir)?;
    std::fs::create_dir_all(&hooks)?;
    let hook = hooks.join(HOOK);
    let want = shim(program);
    match std::fs::read_to_string(&hook) {
        Ok(have) if have == want => return Ok(false),
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
    write(&hooks, &want)?;
    Ok(true)
}

/// 124: put sluice's hook for `program` where the hook in place is
/// sluice's for another, keeping the hook it chains, so a host whose
/// sluice was upgraded is guarded by the newer one from its next command
/// in the repository. Writes nothing where there is no hook, where it is
/// someone else's or already `program`'s, or where the hooks directory is
/// tracked: a hook is installed only by `sluice protect` and the commands
/// that run sluice's work (ADR 0059). `true` when it wrote the hook.
pub fn reinstall(dir: &Path, program: &Path) -> Result<bool> {
    // No repository to find a hook in is the command's to report, and a
    // tracked hooks directory holds no hook sluice wrote.
    let Ok((hooks, None)) = located(dir) else {
        return Ok(false);
    };
    let hook = hooks.join(HOOK);
    let want = shim(program);
    match std::fs::read_to_string(&hook) {
        Ok(have) if have != want && have.lines().nth(1) == Some(MARKER) => {
            write(&hooks, &want)?;
            Ok(true)
        }
        Ok(_) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).with_context(|| format!("reading {}", hook.display())),
    }
}

/// Write `shim` as the hook in `hooks`, in one rename.
fn write(hooks: &Path, shim: &str) -> Result<()> {
    let tmp = hooks.join(format!(".{HOOK}.sluice.{}", std::process::id()));
    std::fs::write(&tmp, shim)?;
    std::fs::set_permissions(&tmp, std::os::unix::fs::PermissionsExt::from_mode(0o755))?;
    std::fs::rename(&tmp, hooks.join(HOOK))?;
    Ok(())
}

/// Take sluice's hook out, putting back the hook it chained; `false` when
/// it was not in.
pub fn remove(dir: &Path) -> Result<bool> {
    let hooks = hookdir(dir)?;
    let hook = hooks.join(HOOK);
    match std::fs::read_to_string(&hook) {
        Ok(have) if have.lines().nth(1) == Some(MARKER) => {
            std::fs::remove_file(&hook)?;
            let chained = hooks.join(CHAINED);
            if chained.exists() {
                std::fs::rename(&chained, &hook)?;
            }
            Ok(true)
        }
        Ok(_) => bail!("{} is not sluice's hook", hook.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// A write the hook refuses: the branch, the value it holds on this host,
/// the one the write would set or none for a deletion, the submissions
/// that still need it, and whether it is sluice's copy of a branch kept
/// on a remote.
struct Refused {
    branch: String,
    from: Rev,
    to: Option<Rev>,
    needing: Vec<u64>,
    copy: bool,
}

/// What a refused update is told, on stderr, before git's own error.
fn refusal(r: &Refused, by: &str) -> String {
    let branch = &r.branch;
    let mut said =
        format!("error: {branch} is protected by sluice; submit the change to sluice instead\n");
    match &r.to {
        Some(to) => said.push_str(&format!(
            "  `{by}` would move it from {} to {}\n",
            r.from.short(),
            to.short()
        )),
        None => {
            let ids: Vec<String> = r.needing.iter().map(|i| format!("#{i}")).collect();
            said.push_str(&format!(
                "  `{by}` would delete it at {}; it still holds {}: withdraw or land them first\n",
                r.from.short(),
                ids.join(", ")
            ));
        }
    }
    if r.copy {
        said.push_str(&format!(
            "  sluice levels this host's {branch} to each landing itself\n"
        ));
    }
    said.push_str(
        "  the worktree may hold the attempted change: git merge --abort or git reset --hard",
    );
    said
}

/// 124: what the hook says when it lets a transaction through unread.
pub fn unread(why: &anyhow::Error) -> String {
    format!("warning: sluice's branch protection let this through unread: {why:#}")
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

/// Whether deleting `branch`, at `cur`, leaves it at `cur`: git's
/// `pack-refs` writes the branch into `packed-refs` at the value it holds,
/// then prunes the loose file in a transaction that leaves `packed-refs`
/// alone. Every other deletion locks `packed-refs` before the hook runs,
/// to take the branch out of it. `common` is the repository's common git
/// directory.
fn pruned(common: &Path, branch: &str, cur: &Rev) -> Result<bool> {
    if common.join("packed-refs.lock").exists() {
        return Ok(false);
    }
    let packed = match std::fs::read_to_string(common.join("packed-refs")) {
        Ok(packed) => packed,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    let name = format!("refs/heads/{branch}");
    Ok(packed
        .lines()
        .filter_map(|l| l.split_once(' '))
        .any(|(id, n)| n == name && id == cur.as_str()))
}

/// `sluice hook reference-transaction <state>`: refuse, in `prepared`,
/// any update to a protected branch that is not a creation or leaves its
/// value on this host as it was (1, 124). Returns the exit: 0 allows, 1
/// refuses. A transaction it cannot read for a reason of its own passes,
/// said once.
pub fn hook(s: &Sluice<'_>, host: &dyn Host, state: &str, input: &mut dyn Read) -> Result<i32> {
    // Symbolic refs are resolved only in `prepared`: in `preparing` a
    // commit on a checked-out branch arrives as HEAD.
    if state != "prepared" {
        return Ok(0);
    }
    let by = caller();
    let mut text = String::new();
    let refused = match input
        .read_to_string(&mut text)
        .map_err(anyhow::Error::from)
        .and_then(|_| judge(s, host, &text))
    {
        Ok(refused) => refused,
        Err(e) => {
            eprintln!("{}", unread(&e));
            return Ok(0);
        }
    };
    if refused.is_empty() {
        return Ok(0);
    }
    for r in &refused {
        crate::telemetry::warn(&refusal(r, &by));
    }
    let seams = s.seams();
    for r in &refused {
        // Reported when the branch's lock comes within a second; the
        // refusal stands either way.
        let until = Instant::now() + Duration::from_secs(1);
        let guard = loop {
            match seams.state.lock(&r.branch) {
                Ok(Some(g)) => break Some(g),
                Ok(None) if Instant::now() < until => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Ok(None) => break None,
                Err(e) => {
                    eprintln!("warning: locking {}'s records: {e:#}", r.branch);
                    break None;
                }
            }
        };
        if guard.is_some() {
            if let Err(e) = s.record_protected(&r.branch, &r.from, r.to.as_ref(), &by) {
                eprintln!(
                    "warning: reporting the refused update to {}: {e:#}",
                    r.branch
                );
            }
        }
    }
    Ok(1)
}

/// The writes of a transaction the hook refuses, each judged by the
/// branch's own value on this host; none fetches.
fn judge(s: &Sluice<'_>, host: &dyn Host, text: &str) -> Result<Vec<Refused>> {
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
        let (cur, copy) = match seams.branch.copy(branch)? {
            HostCopy::At(at) => (at, true),
            HostCopy::Itself => (seams.branch.read(branch)?, false),
        };
        let Some(cur) = cur else {
            continue; // a creation
        };
        let deleting = new.bytes().all(|b| b == b'0');
        let new = (!deleting).then(|| Rev::new(new));
        if new.as_ref() == Some(&cur) || !s.protects(branch, &cur)? {
            continue;
        }
        let needing = if deleting {
            let needing = s.needing(branch)?;
            if needing.is_empty() || pruned(Path::new(&host.repository()?), branch, &cur)? {
                continue;
            }
            needing
        } else {
            Vec::new()
        };
        refused.push(Refused {
            branch: branch.to_string(),
            from: cur,
            to: new,
            needing,
            copy,
        });
    }
    Ok(refused)
}
