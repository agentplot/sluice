//! The branch seam's `pushed refs` profile (C.4, C.7 `remote`): a branch
//! whose shared copy lives on a git remote. Every read fetches it; a
//! landing is a push leased on the revision the combination was formed on
//! (24, 45, 46). Forming and committing stay local refs: every object is
//! on this host.
//!
//! Reading writes only the remote-tracking ref, never the local branch,
//! so the submit operation, which reads the branch, moves no branch
//! (rule core.4). The local branch is sluice's copy of the remote's: a
//! landing's push leaves it, and only a round levels it (124). The git
//! command fetches and pushes, carrying the
//! host's credential helpers and SSH setup, or, where sluice has an
//! [`Identity`] on the remote's forge, that identity's credentials alone.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::rc::Rc;

use anyhow::{bail, Context, Result};
use sluice_seams::{Branch, Formed, HostCopy, LandingCommit, Moved, Rev};

use super::local_refs::LocalRefs;
use crate::identity::{Credentials, Identity, PASSWORD_VAR, USERNAME_VAR};

pub struct PushedRefs {
    local: LocalRefs,
    remote: String,
    speaker: Option<Speaker>,
}

/// sluice's identity on the remote: the URL its git calls reach the
/// remote by, and the helper command that gives git its credentials.
struct Speaker {
    identity: Rc<dyn Identity>,
    url: String,
    helper: String,
}

impl PushedRefs {
    /// The repository holding `path`, whose branches live on `remote`,
    /// reached with the host's own credentials. Refused when the
    /// repository has no remote of that name.
    pub fn open(path: &Path, remote: &str) -> Result<Self> {
        PushedRefs::open_as(path, remote, None)
    }

    /// As [`PushedRefs::open`], reaching the remote as `identity` where it
    /// has one on the remote's forge; `program` is the `sluice` executable
    /// git runs as its credential helper.
    pub fn open_as(
        path: &Path,
        remote: &str,
        identity: Option<(Rc<dyn Identity>, PathBuf)>,
    ) -> Result<Self> {
        let local = LocalRefs::open(path)?;
        let out = git(local.git_dir(), &["remote"])?;
        if !out.status.success() {
            bail!(
                "git remote: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        let names = String::from_utf8_lossy(&out.stdout);
        if !names.lines().any(|n| n == remote) {
            bail!("the policy names the remote {remote:?}, which this repository does not have");
        }
        let speaker = match identity {
            None => None,
            Some((identity, program)) => {
                let url = configured_url(local.git_dir(), remote)?;
                identity.url(&url).map(|url| Speaker {
                    identity,
                    url,
                    helper: format!("!{} credential", quoted(&program)),
                })
            }
        };
        Ok(PushedRefs {
            local,
            remote: remote.into(),
            speaker,
        })
    }

    /// The repository's git directory.
    pub fn git_dir(&self) -> &Path {
        self.local.git_dir()
    }

    /// The remote's name.
    pub fn remote(&self) -> &str {
        &self.remote
    }

    /// The identity sluice reaches the remote as; `None` for the host's
    /// own credentials.
    pub fn identity(&self) -> Option<String> {
        self.speaker.as_ref().map(|s| s.identity.name())
    }

    /// Where git reaches the remote: its name, or the identity's URL.
    pub(crate) fn target(&self) -> &str {
        self.speaker.as_ref().map_or(&self.remote, |s| &s.url)
    }

    /// git for the remote. With an identity, the user's credential helpers
    /// are cleared for this call alone and sluice's is named in their
    /// place, given the credentials in this call's environment; nothing is
    /// written to a git config, so no other git finds them.
    ///
    /// A read or a push the remote refuses the credentials on before it
    /// answers for any ref is run once more on credentials made afresh,
    /// since a forge may refuse ones it has just given, with the same
    /// arguments: a push with its refspec and its lease (134, rules core.5,
    /// core.29). A push the remote answered at the ref is never run again.
    /// A refusal that stands has what each refused credential was added to
    /// git's words, for whoever reports the failure.
    pub(crate) fn remote_git(&self, args: &[&str]) -> Result<Output> {
        let Some(s) = &self.speaker else {
            return git(self.local.git_dir(), args);
        };
        let given = s
            .identity
            .credentials(&s.url)
            .with_context(|| format!("{} on {}", s.identity.name(), self.remote))?;
        let out = self.git_as(s, &given, args)?;
        if !stopped(&out) {
            return Ok(out);
        }
        let fresh = match s.identity.renew(&s.url) {
            Ok(c) => c,
            Err(e) => {
                return Ok(told(
                    out,
                    &format!(
                        "{} refused {}, and none could be made afresh: {e:#}",
                        self.remote, given.about
                    ),
                ))
            }
        };
        let again = self.git_as(s, &fresh, args)?;
        if !stopped(&again) {
            return Ok(again);
        }
        Ok(told(
            again,
            &format!(
                "{} refused {}, then {}",
                self.remote, given.about, fresh.about
            ),
        ))
    }

    /// git with sluice's helper alone, given `c`.
    fn git_as(&self, s: &Speaker, c: &Credentials, args: &[&str]) -> Result<Output> {
        let helper = format!("credential.helper={}", s.helper);
        let mut all = vec!["-c", "credential.helper=", "-c", &helper];
        all.extend(args);
        command(self.local.git_dir(), &all)
            .env(USERNAME_VAR, &c.username)
            .env(PASSWORD_VAR, &c.password)
            .output()
            .with_context(|| format!("running git {}", args.join(" ")))
    }

    fn tracking(&self, branch: &str) -> String {
        format!("refs/remotes/{}/{branch}", self.remote)
    }

    /// Fetch `branch` into its remote-tracking ref; `None` when the remote
    /// has no such branch.
    fn fetch(&self, branch: &str) -> Result<Option<Rev>> {
        let spec = format!("+refs/heads/{branch}:{}", self.tracking(branch));
        let out = self.remote_git(&[
            "fetch",
            "--quiet",
            "--no-tags",
            "--no-write-fetch-head",
            self.target(),
            &spec,
        ])?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            if err.contains("couldn't find remote ref") {
                return Ok(None);
            }
            bail!(
                "fetching {branch} from {} failed: {}",
                self.remote,
                err.trim()
            );
        }
        self.local.read_ref(&self.tracking(branch))
    }

    /// Fetch the remote's ref `src` into the local ref `dst`, which it
    /// replaces; `None` when the remote has no such ref. Writes no branch.
    pub fn fetch_ref(&self, src: &str, dst: &str) -> Result<Option<Rev>> {
        let spec = format!("+{src}:{dst}");
        let out = self.remote_git(&[
            "fetch",
            "--quiet",
            "--no-tags",
            "--no-write-fetch-head",
            self.target(),
            &spec,
        ])?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            if err.contains("couldn't find remote ref") {
                return Ok(None);
            }
            bail!("fetching {src} from {} failed: {}", self.remote, err.trim());
        }
        self.local.read_ref(dst)
    }

    /// A local ref's revision, `None` when there is none.
    pub fn read_ref(&self, name: &str) -> Result<Option<Rev>> {
        self.local.read_ref(name)
    }

    /// Set the remote's branch `branch` to `rev`, whatever it held: a
    /// session's own branch, which no policy protects, pushed for its
    /// pull request.
    pub fn push_branch(&self, rev: &Rev, branch: &str) -> Result<()> {
        let refspec = format!("+{rev}:refs/heads/{branch}");
        let out = self.remote_git(&[
            "push",
            "--porcelain",
            "--no-verify",
            self.target(),
            &refspec,
        ])?;
        if !out.status.success() {
            bail!(
                "pushing {branch} to {} failed: {}",
                self.remote,
                refusal(&out)
            );
        }
        Ok(())
    }
}

/// What git prints when the remote refuses a call's credentials over
/// HTTP: 404, which GitHub answers a token that may not read the
/// repository, 401 and 403.
const REFUSED: &[&str] = &[
    // repository '…' not found; GitHub's own: Repository not found.
    "not found",
    // 401
    "Authentication failed",
    // The requested URL returned error: …
    "error: 401",
    "error: 403",
    "error: 404",
    // RPC failed; HTTP …
    "HTTP 401",
    "HTTP 403",
    "HTTP 404",
];

/// Whether git says the remote refused the call's credentials.
fn refused(out: &Output) -> bool {
    let err = String::from_utf8_lossy(&out.stderr);
    REFUSED.iter().any(|w| err.contains(w))
}

/// Whether git printed the remote's answer for a ref: a line of `git push
/// --porcelain`, `<flag>\t<from>:<to>\t<summary>`. No other output of a
/// failed git call of sluice's holds a tab.
fn answered(out: &Output) -> bool {
    out.stdout.contains(&b'\t')
}

/// Whether the call failed on the remote's refusing its credentials before
/// it answered for any ref, so the remote is as it was.
fn stopped(out: &Output) -> bool {
    !out.status.success() && refused(out) && !answered(out)
}

/// `out` with `said` on the line after what git printed on stderr.
fn told(mut out: Output, said: &str) -> Output {
    out.stderr.truncate(out.stderr.trim_ascii_end().len());
    out.stderr.extend_from_slice(format!("\n{said}").as_bytes());
    out
}

/// Why a `git push --porcelain` failed: the remote's reason for each ref
/// it refused, which porcelain prints on stdout, then what git printed on
/// stderr, which alone says only that some refs failed.
pub(crate) fn refusal(out: &Output) -> String {
    let report = String::from_utf8_lossy(&out.stdout);
    let err = String::from_utf8_lossy(&out.stderr);
    report
        .lines()
        .filter(|l| l.starts_with('!'))
        .filter_map(|l| l.rsplit('\t').next())
        .map(str::trim)
        .chain(std::iter::once(err.trim()))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

/// The git command for the remote: the host's own configuration and
/// credentials, and nothing read from stdin. sluice runs unattended, so a
/// credential git cannot find is an error, never a prompt: no terminal
/// prompt, no askpass program, and a credential manager told not to ask.
pub(crate) fn git(git_dir: &Path, args: &[&str]) -> Result<Output> {
    command(git_dir, args)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))
}

pub(crate) fn command(git_dir: &Path, args: &[&str]) -> Command {
    let mut c = Command::new("git");
    c.arg("--git-dir")
        .arg(git_dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove(USERNAME_VAR)
        .env_remove(PASSWORD_VAR)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GIT_ASKPASS", "")
        .stdin(Stdio::null());
    c
}

/// The URL of the remote `remote` of the repository holding `path`.
pub fn remote_url(path: &Path, remote: &str) -> Result<String> {
    configured_url(LocalRefs::open(path)?.git_dir(), remote)
}

/// The names of the remotes of the repository at `git_dir`.
pub fn remotes(git_dir: &Path) -> Result<Vec<String>> {
    let out = git(git_dir, &["remote"])?;
    if !out.status.success() {
        bail!(
            "listing remotes failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

/// The remote's URL as configured, before any `insteadOf`: the forge it
/// names is the one an identity is for.
fn configured_url(git_dir: &Path, remote: &str) -> Result<String> {
    let key = format!("remote.{remote}.url");
    let out = git(git_dir, &["config", "--get", &key])?;
    if !out.status.success() {
        bail!("the remote {remote:?} has no url");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `path` quoted for the shell git runs a `!` helper with.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

impl Branch for PushedRefs {
    /// 42: the remote's revision of the branch, fetched on every read.
    fn read(&self, branch: &str) -> Result<Option<Rev>> {
        self.fetch(branch)
    }

    /// 124: the host's `refs/heads/<branch>`, sluice's copy of the remote's
    /// branch, read with no fetch.
    fn copy(&self, branch: &str) -> Result<HostCopy> {
        Ok(HostCopy::At(self.local.read(branch)?))
    }

    /// 124: set the copy with gix, from exactly the value expected.
    fn level(&self, branch: &str, expected: Option<&Rev>, to: &Rev) -> Result<Moved> {
        self.local.level_copy(branch, expected, to)
    }

    fn read_file(&self, rev: &Rev, path: &str) -> Result<Option<Vec<u8>>> {
        self.local.read_file(rev, path)
    }

    fn form(&self, onto: &Rev, head: &Rev) -> Result<Formed> {
        self.local.form(onto, head)
    }

    fn commit(&self, commit: &LandingCommit) -> Result<Rev> {
        self.local.commit(commit)
    }

    fn contains(&self, tip: &Rev, rev: &Rev) -> Result<bool> {
        self.local.contains(tip, rev)
    }

    /// 45, 46: push `to`, accepted only while the remote's branch is
    /// `expected`. A refusal of the credential is made once more with the
    /// same lease (134); a refusal at the ref is reported with what the
    /// remote holds, never retried blind (rules core.5, core.29); any other
    /// failure is an error, which leaves the landing for the next step to
    /// settle. An accepted push is the move, whatever the host's copy does
    /// next (122).
    fn move_to(&self, branch: &str, expected: &Rev, to: &Rev) -> Result<Moved> {
        self.leased(branch, expected.as_str(), Some(to))
    }

    /// branches-1 104: push `at` as the branch, leased on its being absent
    /// (`--force-with-lease=<ref>:` with nothing after the colon).
    fn make(&self, branch: &str, at: &Rev) -> Result<Moved> {
        self.leased(branch, "", Some(at))
    }

    /// branches-1 105: delete the remote's branch, leased on `expected`;
    /// once it is gone, so are its local branch and remote-tracking ref.
    fn remove(&self, branch: &str, expected: &Rev) -> Result<Moved> {
        let moved = self.leased(branch, expected.as_str(), None)?;
        if moved == Moved::Moved {
            self.local.delete_ref(&format!("refs/heads/{branch}"))?;
            self.local.delete_ref(&self.tracking(branch))?;
        }
        Ok(moved)
    }
}

impl PushedRefs {
    /// Set the branch to `to`, or remove it when `to` is `None`, accepted
    /// only while the remote's branch is `expected`, or absent when
    /// `expected` is empty. A refusal of the credential is made once more
    /// with the same lease (134); a refusal at the ref is reported with
    /// what the remote holds, never retried blind (rules core.5, core.29).
    /// A push whose answer git never read is never made again: the branch
    /// is read, and the move is made when the remote holds `to` (122);
    /// otherwise, and on any other failure, it is an error, which leaves a
    /// landing for the next step to settle.
    fn leased(&self, branch: &str, expected: &str, to: Option<&Rev>) -> Result<Moved> {
        let lease = format!("--force-with-lease=refs/heads/{branch}:{expected}");
        let refspec = format!("{}:refs/heads/{branch}", to.map_or("", Rev::as_str));
        let out = self.remote_git(&[
            "push",
            "--porcelain",
            "--no-verify",
            &lease,
            self.target(),
            &refspec,
        ])?;
        if out.status.success() {
            return Ok(Moved::Moved);
        }
        let report = String::from_utf8_lossy(&out.stdout);
        let refused_at_the_ref = report
            .lines()
            .any(|l| l.starts_with('!') && l.contains("[rejected]"));
        if refused_at_the_ref {
            return Ok(Moved::Elsewhere(self.fetch(branch)?));
        }
        let failed = format!(
            "pushing {branch} to {} failed: {}",
            self.remote,
            refusal(&out)
        );
        if answered(&out) || refused(&out) {
            bail!(failed);
        }
        let holds = match self.fetch(branch) {
            Ok(at) if at.as_ref() == to => return Ok(Moved::Moved),
            Ok(Some(at)) => format!("{} holds {branch} at {at}", self.remote),
            Ok(None) => format!("{} has no {branch}", self.remote),
            Err(e) => format!("{e:#}"),
        };
        bail!("{failed}; the push went unanswered, and {holds}")
    }
}
