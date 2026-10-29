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
//! Every call it makes to the remote runs under the retry policy, which
//! tries again what the remote never judged, a refused credential bought
//! afresh, and never a push the remote answered (134; rule core.29).

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::rc::Rc;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sluice_seams::{Branch, Formed, HostCopy, LandingCommit, Moved, ReadRefused, Rev};

use super::local_refs::LocalRefs;
use crate::identity::{Credentials, Identity, PASSWORD_VAR, USERNAME_VAR};
use crate::retry::{Failed, Judged, Kind, Policy};

pub struct PushedRefs {
    local: LocalRefs,
    remote: String,
    speaker: Option<Speaker>,
    policy: Rc<Policy>,
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
    /// reached with the host's own credentials under a command's retry
    /// policy. Refused when the repository has no remote of that name.
    pub fn open(path: &Path, remote: &str) -> Result<Self> {
        PushedRefs::open_as(path, remote, None, Rc::new(Policy::command()))
    }

    /// As [`PushedRefs::open`], reaching the remote as `identity` where it
    /// has one on the remote's forge, each call under `policy`; `program`
    /// is the `sluice` executable git runs as its credential helper.
    pub fn open_as(
        path: &Path,
        remote: &str,
        identity: Option<(Rc<dyn Identity>, PathBuf)>,
        policy: Rc<Policy>,
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
            policy,
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

    /// git for the remote, a read: a fetch or a listing. With an identity,
    /// the user's credential helpers are cleared for this call alone and
    /// sluice's is named in their place, given the credentials in this
    /// call's environment; nothing is written to a git config, so no other
    /// git finds them.
    ///
    /// The read runs under the retry policy, tried again whatever failed
    /// but the remote's answer, on credentials made afresh after the
    /// remote refused them, since a forge may refuse ones it has just given
    /// (134, rule core.29). A refusal that stands has what each refused
    /// credential was added to git's words, for whoever reports the
    /// failure.
    pub(crate) fn remote_git(&self, args: &[&str]) -> Result<Output> {
        let called = self.call(Kind::Read, args)?;
        Ok(match called.note {
            Some(note) => told(called.out, &note),
            None => called.out,
        })
    }

    /// git for the remote, a call of `kind`, under the retry policy: its
    /// last try's output, what each credential the remote refused was,
    /// apart from git's words (sluice's, never the remote's, 133), and how
    /// the last try stands when it failed. A try after a credential the
    /// remote refused carries one made afresh; the same arguments go every
    /// time, so a push keeps its refspec and its lease (rule core.5).
    fn call(&self, kind: Kind, args: &[&str]) -> Result<Called> {
        // Why sluice could not make a try: git not run, or no credential.
        let mut fault = None;
        let mut unrenewed = None;
        let mut refused: Vec<String> = Vec::new();
        let mut last = None;
        let ran = self.policy.run(kind, |t| {
            let given = match &self.speaker {
                None => None,
                Some(s) if t.renew() => match s.identity.renew(&s.url) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        unrenewed = Some(e);
                        return Err(Failed::new(Judged::Answered, ()));
                    }
                },
                Some(s) => match s.identity.credentials(&s.url) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        let e = e.context(format!("{} on {}", s.identity.name(), self.remote));
                        fault = Some(e);
                        return Err(Failed::new(Judged::Answered, ()));
                    }
                },
            };
            let out = match self.git_as(given.as_ref(), args) {
                Ok(out) if out.status.success() => return Ok(out),
                Ok(out) => out,
                Err(e) => {
                    fault = Some(e);
                    return Err(Failed::new(Judged::Answered, ()));
                }
            };
            let judged = judged(&out, kind, self.speaker.is_some());
            if let (Judged::Credential, Some(c)) = (judged, given) {
                refused.push(c.about);
            }
            last = Some(out);
            Err(Failed::new(judged, ()))
        });
        let stopped = match ran {
            Ok(out) => {
                return Ok(Called {
                    out,
                    note: None,
                    judged: None,
                })
            }
            Err(stopped) => stopped,
        };
        if let Some(e) = fault {
            return Err(e);
        }
        let out = last.expect("a try git failed");
        let note =
            match unrenewed {
                Some(e) => Some(format!(
                    "{} refused {}, and none could be made afresh: {e:#}",
                    self.remote,
                    refused.join(", then ")
                )),
                None if stopped.judged == Judged::Credential && !refused.is_empty() => Some(
                    format!("{} refused {}", self.remote, refused.join(", then ")),
                ),
                None => None,
            };
        Ok(Called {
            out,
            note,
            judged: Some(stopped.judged),
        })
    }

    /// Fetch `refspec`, whose source is `asked`, with `flags` before the
    /// remote. `Ok(false)` when the remote has no such ref; a fetch that
    /// fails otherwise is a [`ReadRefused`] (133).
    pub(crate) fn fetch_spec(&self, asked: &str, refspec: &str, flags: &[&str]) -> Result<bool> {
        self.fetch_specs(asked, &[refspec], flags)
    }

    /// [`PushedRefs::fetch_spec`] of several refspecs in one fetch.
    fn fetch_specs(&self, asked: &str, refspecs: &[&str], flags: &[&str]) -> Result<bool> {
        let mut args = vec!["fetch", "--quiet", "--no-tags", "--no-write-fetch-head"];
        args.extend(flags);
        args.push(self.target());
        args.extend(refspecs);
        let Called { out, note, .. } = self.call(Kind::Read, &args)?;
        if out.status.success() {
            return Ok(true);
        }
        let said = String::from_utf8_lossy(&out.stderr);
        if said.contains(NO_SUCH_REF) {
            return Ok(false);
        }
        Err(ReadRefused {
            remote: self.remote.clone(),
            asked: asked.into(),
            said: said.trim_end().into(),
            note,
        }
        .into())
    }

    /// git for the remote: with the host's own credentials when `given`
    /// is `None`, and otherwise with sluice's helper alone, given them.
    fn git_as(&self, given: Option<&Credentials>, args: &[&str]) -> Result<Output> {
        let (Some(s), Some(c)) = (&self.speaker, given) else {
            return git(self.local.git_dir(), args);
        };
        let helper = format!("credential.helper={}", s.helper);
        let mut all = vec!["-c", "credential.helper=", "-c", &helper];
        all.extend(args);
        command(self.local.git_dir(), &all)
            .env(USERNAME_VAR, &c.username)
            .env(PASSWORD_VAR, &c.password)
            .output()
            .with_context(|| format!("running git {}", args.join(" ")))
    }

    /// `git push` with `args` under the retry policy, setting the remote's
    /// ref `name` to `to`, or removing it when `to` is `None`. A push the
    /// remote never judged is tried again with the same `args`, so the
    /// same refspec and lease (134; rules core.5, core.29). A push whose
    /// answer git did not read is never made again: `name` is read once,
    /// and the push is done when the remote holds `to`, or no `name` for a
    /// removal (122; ADR 0017).
    pub(crate) fn push(&self, args: &[&str], name: &str, to: Option<&str>) -> Result<Pushed> {
        let called = self.call(Kind::Push, args)?;
        let out = match called.note {
            Some(note) => told(called.out, &note),
            None => called.out,
        };
        match called.judged {
            None => Ok(Pushed::Done),
            Some(Judged::Unknown) => {
                let shown = name.strip_prefix("refs/heads/").unwrap_or(name);
                let holds = match self.holds(name) {
                    Ok(at) if at.as_deref() == to => return Ok(Pushed::Done),
                    Ok(Some(at)) => format!("{} holds {shown} at {at}", self.remote),
                    Ok(None) => format!("{} has no {shown}", self.remote),
                    Err(e) => format!("{e:#}"),
                };
                Ok(Pushed::Unanswered(out, holds))
            }
            Some(_) => Ok(Pushed::Refused(out)),
        }
    }

    /// The revision the remote's ref `name` holds, listed under the retry
    /// policy as a read; `None` when it has no such ref.
    fn holds(&self, name: &str) -> Result<Option<String>> {
        let out = self.remote_git(&["ls-remote", "--refs", self.target(), name])?;
        if !out.status.success() {
            bail!(
                "listing {name} on {} failed: {}",
                self.remote,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_once('\t'))
            .find(|(_, r)| *r == name)
            .map(|(at, _)| at.to_string()))
    }

    fn tracking(&self, branch: &str) -> String {
        format!("refs/remotes/{}/{branch}", self.remote)
    }

    /// Fetch `branch` into its remote-tracking ref, and in the same fetch
    /// its intake mark, which may be absent, into [`PushedRefs::intake_mark`]'s
    /// ref, a mark gone from the remote gone here too (137); `None` when
    /// the remote has no such branch, a [`ReadRefused`] when it cannot be
    /// read.
    fn fetch(&self, branch: &str) -> Result<Option<Rev>> {
        let src = format!("refs/heads/{branch}");
        let spec = format!("+{src}:{}", self.tracking(branch));
        // A pattern, so that a remote with no mark answers the fetch all
        // the same; it brings any mark whose branch's name begins with
        // this one's, which nothing reads.
        let mark = format!("+{}*:{}*", intake_ref(branch), self.fetched_mark(branch));
        if !self.fetch_specs(&src, &[&spec, &mark], &["--prune"])? {
            return Ok(None);
        }
        self.local.read_ref(&self.tracking(branch))
    }

    /// Where `branch`'s intake mark is fetched to here.
    fn fetched_mark(&self, branch: &str) -> String {
        format!("refs/sluice-fetched/{}/intake/{branch}", self.remote)
    }

    /// 137: `branch`'s intake mark as the branch's last fetch brought it,
    /// with no fetch of its own: its commit and what it says; `None` when
    /// the remote held none. A mark whose message is not one sluice wrote
    /// reads as a driver's of a host named by its commit, which no wait's
    /// rule takes at once.
    pub fn intake_mark(&self, branch: &str) -> Result<Option<(Rev, Mark)>> {
        let Some(rev) = self.local.read_ref(&self.fetched_mark(branch))? else {
            return Ok(None);
        };
        let out = git(
            self.local.git_dir(),
            &["log", "-1", "--format=%B", rev.as_str()],
        )?;
        let said = String::from_utf8_lossy(&out.stdout);
        let mark = serde_json::from_str::<Mark>(said.trim()).unwrap_or_else(|_| Mark {
            host: format!("the writer of {}", rev.short()),
            holder: Holder::Driver,
            sluice: String::new(),
        });
        Ok(Some((rev, mark)))
    }

    /// 137: push `mark` as `branch`'s intake mark, leased on the remote's
    /// holding `from`, or none when `from` is `None` (rule core.5), under
    /// the retry policy (rule core.29): a new root commit with an empty
    /// tree, whose message is `mark` as one JSON line. The mark pushed,
    /// read here as the next [`PushedRefs::intake_mark`]; `None` when the
    /// remote's mark was not `from`, another host's take, which is never
    /// pushed again blind.
    pub fn put_intake_mark(
        &self,
        branch: &str,
        from: Option<&Rev>,
        mark: &Mark,
    ) -> Result<Option<Rev>> {
        let tree = git_ok(self.local.git_dir(), &["mktree"])?;
        let message = serde_json::to_string(mark)?;
        let out = command(
            self.local.git_dir(),
            &["commit-tree", tree.trim(), "-m", &message],
        )
        .env("GIT_AUTHOR_NAME", "sluice")
        .env("GIT_AUTHOR_EMAIL", "sluice@localhost")
        .env("GIT_COMMITTER_NAME", "sluice")
        .env("GIT_COMMITTER_EMAIL", "sluice@localhost")
        .output()
        .context("running git commit-tree")?;
        if !out.status.success() {
            bail!(
                "git commit-tree: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        let commit = Rev(String::from_utf8_lossy(&out.stdout).trim().to_string());
        match self.leased_ref(&intake_ref(branch), from, Some(&commit))? {
            false => Ok(None),
            true => {
                self.local.write_ref(&self.fetched_mark(branch), &commit)?;
                Ok(Some(commit))
            }
        }
    }

    /// 137: remove `branch`'s intake mark, leased on the remote's holding
    /// `from`. `false` when it held another, which is left.
    pub fn drop_intake_mark(&self, branch: &str, from: &Rev) -> Result<bool> {
        let dropped = self.leased_ref(&intake_ref(branch), Some(from), None)?;
        if dropped {
            self.local.delete_ref(&self.fetched_mark(branch))?;
        }
        Ok(dropped)
    }

    /// Set the remote's ref `name` to `to`, or remove it when `to` is
    /// `None`, leased on its holding `from`, or nothing when `from` is
    /// `None`, through [`PushedRefs::push`]. `false` when the remote
    /// refused it at its lease.
    fn leased_ref(&self, name: &str, from: Option<&Rev>, to: Option<&Rev>) -> Result<bool> {
        let lease = format!("--force-with-lease={name}:{}", from.map_or("", Rev::as_str));
        let refspec = format!("{}:{name}", to.map_or("", Rev::as_str));
        let pushed = self.push(
            &[
                "push",
                "--porcelain",
                "--no-verify",
                &lease,
                self.target(),
                &refspec,
            ],
            name,
            to.map(Rev::as_str),
        )?;
        if let Pushed::Refused(out) = &pushed {
            let report = String::from_utf8_lossy(&out.stdout);
            if report
                .lines()
                .any(|l| l.starts_with('!') && l.contains("[rejected]"))
            {
                return Ok(false);
            }
        }
        match pushed.why() {
            None => Ok(true),
            Some(why) => bail!("pushing {name} to {} failed: {why}", self.remote),
        }
    }

    /// Fetch the remote's ref `src` into the local ref `dst`, which it
    /// replaces; `None` when the remote has no such ref, a [`ReadRefused`]
    /// when it cannot be read. Writes no branch.
    pub fn fetch_ref(&self, src: &str, dst: &str) -> Result<Option<Rev>> {
        if !self.fetch_spec(src, &format!("+{src}:{dst}"), &[])? {
            return Ok(None);
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
        let name = format!("refs/heads/{branch}");
        let refspec = format!("+{rev}:{name}");
        let pushed = self.push(
            &[
                "push",
                "--porcelain",
                "--no-verify",
                self.target(),
                &refspec,
            ],
            &name,
            Some(rev.as_str()),
        )?;
        if let Some(why) = pushed.why() {
            bail!("pushing {branch} to {} failed: {why}", self.remote);
        }
        Ok(())
    }
}

/// 137: the remote's ref of `branch`'s intake mark.
pub fn intake_ref(branch: &str) -> String {
    format!("refs/sluice/intake/{branch}")
}

/// 137: what a branch's intake mark says: the host that holds it, as its
/// peers name it, whether its holder is the host's `sluice run` or a wait,
/// and the sluice that wrote it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    pub host: String,
    pub holder: Holder,
    pub sluice: String,
}

/// Which process of its host holds an intake mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Holder {
    /// The host's `sluice run`, the holder of its driver's lock.
    Driver,
    /// A `sluice wait` or a waiting submit.
    Wait,
}

/// A git call to the remote under the retry policy: its last try's output,
/// what each credential the remote refused was, and how the last try stood
/// when it failed.
struct Called {
    out: Output,
    note: Option<String>,
    judged: Option<Judged>,
}

/// How a push of [`PushedRefs::push`] ended.
pub(crate) enum Pushed {
    /// The remote took it, or holds what it set though its answer was
    /// lost.
    Done,
    /// The remote answered it with a refusal, or it failed before it was
    /// sent: git's output, with what each refused credential was.
    Refused(Output),
    /// Its answer was not read, and the remote does not hold what it set:
    /// git's output, and what the remote holds.
    Unanswered(Output, String),
}

impl Pushed {
    /// Why the push was not done, for a report; `None` when it was.
    pub(crate) fn why(&self) -> Option<String> {
        match self {
            Pushed::Done => None,
            Pushed::Refused(out) => Some(refusal(out)),
            Pushed::Unanswered(out, holds) => Some(format!(
                "{}; the push went unanswered, and {holds}",
                refusal(out)
            )),
        }
    }
}

/// How a failed git call to the remote stands (134): answered, when git
/// printed the remote's answer for a ref or the remote has no such ref; a
/// credential refused before the remote answered for any ref; not known,
/// when the call may have been sent with its answer lost; never judged,
/// when no connection was made or the remote could not answer git's
/// request for its refs, before anything of a push was sent; and not
/// known for any wording sluice does not know, which is never pushed
/// again. On the host's own credentials, which sluice cannot make afresh
/// (`renewable` false), a refused push is answered.
fn judged(out: &Output, kind: Kind, renewable: bool) -> Judged {
    let err = String::from_utf8_lossy(&out.stderr);
    if answered(out) || ANSWERED.iter().any(|w| err.contains(w)) {
        Judged::Answered
    } else if refused(out) {
        match renewable || kind == Kind::Read {
            true => Judged::Credential,
            false => Judged::Answered,
        }
    } else if err.contains(UNANSWERED) {
        Judged::Unknown
    } else if NEVER.iter().any(|w| err.contains(w)) || unable(&err) {
        Judged::Never
    } else {
        Judged::Unknown
    }
}

/// What git prints when the remote has no ref a fetch asks for.
const NO_SUCH_REF: &str = "couldn't find remote ref";

/// What git prints for a call the remote answered with no line for a ref:
/// a fetch's ref the remote lacks, and a removal of one it lacks, which
/// git finds in the remote's listing.
const ANSWERED: &[&str] = &[NO_SUCH_REF, "remote ref does not exist"];

/// What git prints when a request it sent the remote over HTTP failed,
/// git's `remote-curl.c`: after a push was sent, so its answer was lost.
const UNANSWERED: &str = "RPC failed";

/// What git and curl print when no connection to the remote was made.
const NEVER: &[&str] = &[
    "Could not resolve host",
    "Could not resolve hostname",
    "Failed to connect",
    "Connection refused",
    "Connection timed out",
    "Operation timed out",
];

/// Whether git says the remote answered its request for the refs with a
/// 5xx or a 429: unable to answer then, and nothing of a push sent.
fn unable(err: &str) -> bool {
    const SAID: &str = "The requested URL returned error: ";
    err.match_indices(SAID).any(|(at, _)| {
        let code = &err[at + SAID.len()..];
        code.starts_with('5') || code.starts_with("429")
    })
}

/// What git prints when the remote refuses a call's credentials over
/// HTTP: 404, which GitHub answers a token that may not read the
/// repository, 401 and 403.
const REFUSED: &[&str] = &[
    // repository '…' not found; GitHub's own: Repository not found.
    "not found",
    // 401
    "Authentication failed",
    // 401, on the host's own credentials, when it has none to give
    "could not read Username",
    "could not read Password",
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

/// [`git`] that succeeded: what it printed.
fn git_ok(git_dir: &Path, args: &[&str]) -> Result<String> {
    let out = git(git_dir, args)?;
    if !out.status.success() {
        bail!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
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
    /// `expected`. A push the remote never judged, its credential refused
    /// among them, is tried again under the retry policy with the same
    /// lease (134); a refusal at the ref is reported with what the remote
    /// holds, never retried blind (rules core.5, core.29); any other
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
    /// `expected` is empty. A push the remote never judged, its credential
    /// refused among them, is tried again under the retry policy with the
    /// same lease (134); a refusal at the ref is reported with what the
    /// remote holds, never retried blind (rules core.5, core.29). A push
    /// whose answer git never read is never made again: the branch is
    /// read, and the move is made when the remote holds `to` (122);
    /// otherwise, and on any other failure, it is an error, which leaves a
    /// landing for the next step to settle.
    fn leased(&self, branch: &str, expected: &str, to: Option<&Rev>) -> Result<Moved> {
        let name = format!("refs/heads/{branch}");
        let lease = format!("--force-with-lease={name}:{expected}");
        let refspec = format!("{}:{name}", to.map_or("", Rev::as_str));
        let pushed = self.push(
            &[
                "push",
                "--porcelain",
                "--no-verify",
                &lease,
                self.target(),
                &refspec,
            ],
            &name,
            to.map(Rev::as_str),
        )?;
        if let Pushed::Refused(out) = &pushed {
            let report = String::from_utf8_lossy(&out.stdout);
            let refused_at_the_ref = report
                .lines()
                .any(|l| l.starts_with('!') && l.contains("[rejected]"));
            if refused_at_the_ref {
                return Ok(Moved::Elsewhere(self.fetch(branch)?));
            }
        }
        match pushed.why() {
            None => Ok(Moved::Moved),
            Some(why) => bail!("pushing {branch} to {} failed: {why}", self.remote),
        }
    }
}
