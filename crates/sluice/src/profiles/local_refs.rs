//! The branch seam's `local refs` profile (C.4): one host's repository.
//! gix reads refs and objects, writes commits and moves a branch from its
//! expected revision; the git command forms candidates and answers
//! ancestry, so a conflict is exactly what `git merge` would report (ADR
//! 0009). `docs/plans/phase-1-integrations.md` section 1 is the reference.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use gix::refs::file::transaction::prepare::ReferenceOutOfDate;
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use gix::refs::Target;
use gix::ObjectId;
use sluice_seams::{Branch, Formed, HostCopy, LandingCommit, Moved, Rev};

/// Who sluice's commits are by: fixed, never read from git config.
const NAME: &str = "sluice";
const EMAIL: &str = "sluice@localhost";

pub struct LocalRefs {
    repo: gix::Repository,
    git_dir: PathBuf,
}

impl LocalRefs {
    /// The repository holding `path`, a worktree or a git directory.
    pub fn open(path: &Path) -> Result<Self> {
        let repo = gix::open(path)
            .map_err(|e| anyhow!("opening the repository at {}: {e}", path.display()))?;
        let git_dir = repo.git_dir().to_path_buf();
        Ok(LocalRefs { repo, git_dir })
    }

    /// The repository's common git directory, the same from every
    /// worktree, canonicalized (section 1, *Common dir and opening*).
    pub fn common_dir(&self) -> Result<PathBuf> {
        std::fs::canonicalize(self.repo.common_dir())
            .context("canonicalizing the common git directory")
    }

    /// The git directory of the worktree it was opened from.
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// The revision a full ref name points at, or `None` when it does not
    /// exist.
    pub fn read_ref(&self, name: &str) -> Result<Option<Rev>> {
        let found = self
            .repo
            .try_find_reference(name)
            .map_err(|e| anyhow!("reading {name}: {e}"))?;
        let Some(mut r) = found else { return Ok(None) };
        let id = r.peel_to_id().map_err(|e| anyhow!("peeling {name}: {e}"))?;
        Ok(Some(Rev::new(id.to_string())))
    }

    /// Set `branch`, the host's copy of a branch kept on a remote, to `to`
    /// from exactly `expected`, or from its absence when `expected` is
    /// `None` (124); a copy found elsewhere is reported, never forced (rule
    /// core.5).
    pub fn level_copy(&self, branch: &str, expected: Option<&Rev>, to: &Rev) -> Result<Moved> {
        let previous = match expected {
            Some(rev) => PreviousValue::MustExistAndMatch(Target::Object(self.id(rev)?)),
            None => PreviousValue::MustNotExist,
        };
        let message = format!("sluice: level {branch} to its landing");
        match self.edit_as(branch, previous, to, &message) {
            Ok(Moved::Moved) => Ok(Moved::Moved),
            // A copy found elsewhere is reported with what it holds, however
            // the refusal came; one still at `expected` failed for a reason
            // of its own, git's lock among them.
            other => match self.read(branch)? {
                found if found.as_ref() != expected => Ok(Moved::Elsewhere(found)),
                _ => other,
            },
        }
    }

    /// Move `branch` to `to` from `expected`; a branch found elsewhere is
    /// reported, never retried blind (24, rule core.5).
    fn edit(&self, branch: &str, expected: PreviousValue, to: &Rev) -> Result<Moved> {
        self.edit_as(branch, expected, to, &format!("sluice: land on {branch}"))
    }

    /// As [`LocalRefs::edit`], with `message` in the reflog.
    fn edit_as(
        &self,
        branch: &str,
        expected: PreviousValue,
        to: &Rev,
        message: &str,
    ) -> Result<Moved> {
        let new = self.id(to)?;
        let change = Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: false,
                message: message.into(),
            },
            expected,
            new: Target::Object(new),
        };
        self.transact(&full_name(branch), change)
    }

    /// Remove the ref `name`, whatever it holds: a pushed branch's local
    /// copies once the remote's is gone. Removing one that is absent does
    /// nothing.
    pub fn delete_ref(&self, name: &str) -> Result<()> {
        if self.read_ref(name)?.is_none() {
            return Ok(());
        }
        let change = Change::Delete {
            expected: PreviousValue::Any,
            log: RefLog::AndReference,
        };
        match self.transact(name, change)? {
            Moved::Moved => Ok(()),
            Moved::Elsewhere(_) => bail!("{name} could not be removed"),
        }
    }

    /// One ref edit, as sluice; a ref found other than `change` expects is
    /// reported, never retried blind.
    fn transact(&self, name: &str, change: Change) -> Result<Moved> {
        let edit = RefEdit {
            change,
            name: name
                .to_string()
                .try_into()
                .map_err(|e| anyhow!("{name} is not a ref name: {e}"))?,
            deref: false,
        };
        let branch = name;
        let who = Self::signature(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .context("the clock is before 1970")?
                .as_secs() as i64,
        );
        let mut buf = gix::date::parse::TimeBuf::default();
        match self
            .repo
            .edit_references_as([edit], Some(who.to_ref(&mut buf)))
        {
            Ok(_) => Ok(Moved::Moved),
            Err(e) => {
                if let Some(out) = e.downcast_any_ref::<ReferenceOutOfDate>() {
                    let actual = match &out.actual {
                        Target::Object(id) => Some(Rev::new(id.to_string())),
                        Target::Symbolic(_) => None,
                    };
                    Ok(Moved::Elsewhere(actual))
                } else if e.classify().is_not_found() {
                    Ok(Moved::Elsewhere(None))
                } else if e
                    .downcast_any_ref::<std::io::Error>()
                    .is_some_and(|io| io.kind() == std::io::ErrorKind::AlreadyExists)
                {
                    Err(anyhow!("git holds the lock on {name}: {e}"))
                } else {
                    Err(anyhow!("moving {branch}: {e}"))
                }
            }
        }
    }

    fn id(&self, rev: &Rev) -> Result<ObjectId> {
        let id = ObjectId::from_hex(rev.as_str().as_bytes())
            .map_err(|e| anyhow!("{rev} is not an object id: {e}"))?;
        self.repo
            .find_object(id)
            .map_err(|e| anyhow!("no object {rev}: {e}"))?;
        Ok(id)
    }

    fn signature(seconds: i64) -> gix::actor::Signature {
        gix::actor::Signature {
            name: NAME.into(),
            email: EMAIL.into(),
            time: gix::date::Time { seconds, offset: 0 },
        }
    }

    fn write_commit(
        &self,
        tree: ObjectId,
        parents: [ObjectId; 2],
        message: &str,
        seconds: i64,
    ) -> Result<Rev> {
        let who = Self::signature(seconds);
        let commit = gix::objs::Commit {
            tree,
            parents: parents.into_iter().collect(),
            author: who.clone(),
            committer: who,
            encoding: None,
            message: message.into(),
            extra_headers: vec![],
        };
        let id = self
            .repo
            .write_object(&commit)
            .map_err(|e| anyhow!("writing a commit: {e}"))?
            .detach();
        Ok(Rev::new(id.to_string()))
    }

    /// The git command, for what gix has no equivalent of. Its identity and
    /// configuration are pinned, nothing it inherits from a hook or a
    /// session points it elsewhere, and it reads nothing from stdin.
    fn git(&self, args: &[&str]) -> Result<Output> {
        Command::new("git")
            .arg("--git-dir")
            .arg(&self.git_dir)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_OBJECT_DIRECTORY")
            .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", NAME)
            .env("GIT_AUTHOR_EMAIL", EMAIL)
            .env("GIT_COMMITTER_NAME", NAME)
            .env("GIT_COMMITTER_EMAIL", EMAIL)
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("running git {}", args.join(" ")))
    }
}

fn full_name(branch: &str) -> String {
    format!("refs/heads/{branch}")
}

fn is_object_id(field: &[u8]) -> bool {
    matches!(field.len(), 40 | 64) && field.iter().all(u8::is_ascii_hexdigit)
}

impl Branch for LocalRefs {
    fn read(&self, branch: &str) -> Result<Option<Rev>> {
        self.read_ref(&full_name(branch))
    }

    /// The branch is the host's own ref: as read.
    fn fetched(&self, branch: &str) -> Result<Option<Rev>> {
        self.read(branch)
    }

    /// 64: an object lookup; a revision that names no object id is none
    /// the host holds.
    fn holds(&self, rev: &Rev) -> Result<bool> {
        Ok(ObjectId::from_hex(rev.as_str().as_bytes()).is_ok_and(|id| self.repo.has_object(id)))
    }

    /// The host's ref is the branch here: it has no copy apart from it.
    fn copy(&self, _branch: &str) -> Result<HostCopy> {
        Ok(HostCopy::Itself)
    }

    fn level(&self, branch: &str, _expected: Option<&Rev>, _to: &Rev) -> Result<Moved> {
        bail!("{branch} is kept in this host's own refs, which have no copy of it to level")
    }

    fn read_file(&self, rev: &Rev, path: &str) -> Result<Option<Vec<u8>>> {
        let id = self.id(rev)?;
        let tree = self
            .repo
            .find_object(id)
            .map_err(|e| anyhow!("{e}"))?
            .peel_to_tree()
            .map_err(|e| anyhow!("{rev} has no tree: {e}"))?;
        let Some(entry) = tree
            .lookup_entry_by_path(path)
            .map_err(|e| anyhow!("looking up {path} in {rev}: {e}"))?
        else {
            return Ok(None);
        };
        let object = entry
            .object()
            .map_err(|e| anyhow!("reading {path} in {rev}: {e}"))?;
        Ok(Some(object.data.clone()))
    }

    fn form(&self, onto: &Rev, head: &Rev) -> Result<Formed> {
        let (base, top) = (self.id(onto)?, self.id(head)?);
        let out = self.git(&[
            "merge-tree",
            "--write-tree",
            "-z",
            "--name-only",
            "--no-messages",
            &base.to_string(),
            &top.to_string(),
        ])?;
        let mut fields = out.stdout.split(|b| *b == 0);
        let first = fields.next().unwrap_or_default();
        let fail = || {
            anyhow!(
                "git merge-tree {onto} {head} exited {:?}: {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr).trim()
            )
        };
        if !is_object_id(first) {
            return Err(fail());
        }
        match out.status.code() {
            Some(0) => {
                let tree = ObjectId::from_hex(first).map_err(|e| anyhow!("{e}"))?;
                // A candidate's time is fixed, so one combination forms one
                // commit whenever it is formed.
                let rev = self.write_commit(tree, [base, top], "sluice candidate\n", 0)?;
                Ok(Formed::Revision(rev))
            }
            Some(1) => {
                let mut paths: Vec<String> = Vec::new();
                for f in fields.take_while(|f| !f.is_empty()) {
                    let p = String::from_utf8_lossy(f).into_owned();
                    if !paths.contains(&p) {
                        paths.push(p);
                    }
                }
                Ok(Formed::Conflict { paths })
            }
            _ => Err(fail()),
        }
    }

    fn commit(&self, commit: &LandingCommit) -> Result<Rev> {
        let tested = self.id(&commit.tree_of)?;
        let tree = self
            .repo
            .find_commit(tested)
            .map_err(|e| anyhow!("{} is not a commit: {e}", commit.tree_of))?
            .tree_id()
            .map_err(|e| anyhow!("{e}"))?
            .detach();
        let parents = [self.id(&commit.parent)?, self.id(&commit.head)?];
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("the clock is before 1970")?
            .as_secs() as i64;
        self.write_commit(tree, parents, &commit.message, now)
    }

    fn contains(&self, tip: &Rev, rev: &Rev) -> Result<bool> {
        let tip = self.id(tip)?;
        // 121: a commit this host does not have, as another host's landing
        // the remote has since lost, is in no revision read here.
        let rev = ObjectId::from_hex(rev.as_str().as_bytes())
            .map_err(|e| anyhow!("{rev} is not an object id: {e}"))?;
        let found = self
            .repo
            .try_find_object(rev)
            .map_err(|e| anyhow!("reading {rev}: {e}"))?;
        if !found.is_some_and(|o| o.kind == gix::object::Kind::Commit) {
            return Ok(false);
        }
        let out = self.git(&[
            "merge-base",
            "--is-ancestor",
            &rev.to_string(),
            &tip.to_string(),
        ])?;
        match out.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            code => bail!(
                "git merge-base --is-ancestor exited {code:?}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        }
    }

    fn move_to(&self, branch: &str, expected: &Rev, to: &Rev) -> Result<Moved> {
        let from = self.id(expected)?;
        self.edit(
            branch,
            PreviousValue::MustExistAndMatch(Target::Object(from)),
            to,
        )
    }

    fn make(&self, branch: &str, at: &Rev) -> Result<Moved> {
        match self.edit(branch, PreviousValue::MustNotExist, at) {
            Ok(Moved::Moved) => Ok(Moved::Moved),
            // A branch present is reported with what it holds, however the
            // refusal came.
            other => match self.read(branch)? {
                Some(found) => Ok(Moved::Elsewhere(Some(found))),
                None => other,
            },
        }
    }

    fn remove(&self, branch: &str, expected: &Rev) -> Result<Moved> {
        let from = self.id(expected)?;
        let change = Change::Delete {
            expected: PreviousValue::MustExistAndMatch(Target::Object(from)),
            log: RefLog::AndReference,
        };
        match self.transact(&full_name(branch), change) {
            Ok(Moved::Moved) => Ok(Moved::Moved),
            other => match self.read(branch)? {
                found if found.as_ref() != Some(expected) => Ok(Moved::Elsewhere(found)),
                _ => other,
            },
        }
    }
}
