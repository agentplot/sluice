//! What the pull request driver asks of GitHub (C.5 pull request), behind
//! one interface: [`super::App`] asks GitHub's API as sluice's app, and
//! [`super::fake::FileForge`] answers from a file, for tests.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::app::Repo;

/// The name of the check run sluice posts on a pull request's head.
pub const SLUICE_CHECK: &str = "sluice";

/// An open or closed pull request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pull {
    pub number: u64,
    pub title: String,
    pub body: String,
    /// Who opened it.
    pub author: String,
    pub draft: bool,
    pub open: bool,
    /// The revision of its head.
    pub head: String,
    /// The branch its head is on.
    pub head_ref: String,
    /// The repository its head is in, `owner/name`; another than the
    /// pull request's for a fork, and empty when that fork is gone.
    pub head_repo: String,
    /// The branch it is into.
    pub base: String,
}

impl Pull {
    /// From a branch of `repo` itself, not a fork.
    pub fn from_repo(&self, repo: &Repo) -> bool {
        self.head_repo == repo.to_string()
    }
}

/// One review, as GitHub keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    pub user: String,
    /// `APPROVED`, `CHANGES_REQUESTED`, `COMMENTED` or `DISMISSED`.
    pub state: String,
    /// The revision it was given on.
    pub commit: String,
}

/// How many reviewers approve `head`: each reviewer's last review that
/// decides anything, approving and given on `head` itself. A review on an
/// earlier head does not count (C.7 `reviews`).
pub fn approvals(reviews: &[Review], head: &str) -> u32 {
    let mut last: Vec<(&str, &Review)> = Vec::new();
    for r in reviews.iter().filter(|r| r.state != "COMMENTED") {
        match last.iter_mut().find(|(u, _)| *u == r.user) {
            Some(slot) => slot.1 = r,
            None => last.push((&r.user, r)),
        }
    }
    last.iter()
        .filter(|(_, r)| r.state == "APPROVED" && r.commit == head)
        .count() as u32
}

/// Where a check another system posts stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Standing {
    Pending,
    Passed,
    Failed,
}

/// A check run or a commit status on a revision, by its name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub standing: Standing,
}

/// The state of sluice's check run, as it posts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Waiting, or in the queue.
    Queued,
    /// Under test.
    InProgress,
    /// Failed or conflict.
    Failure,
    /// Landed.
    Success,
}

/// sluice's check run on a head: its state, its title and its summary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Posting {
    pub status: Status,
    pub title: String,
    pub summary: String,
    /// ADR 0044: the check's text, the submission's report; `None` posts
    /// none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// sluice's check run as found on a head.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Posted {
    pub id: u64,
    /// Its name: `sluice`, or `sluice / <check> (<system>)` (ADR 0044).
    #[serde(default = "sluice_check")]
    pub name: String,
    pub posting: Posting,
}

fn sluice_check() -> String {
    SLUICE_CHECK.into()
}

/// A comment on a pull request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub body: String,
    /// Where a person opens it.
    pub url: String,
}

/// ADR 0044: the name of sluice's check of `task` on `system`.
pub fn check_name(task: &str, system: Option<&str>) -> String {
    match system {
        Some(s) => format!("{SLUICE_CHECK} / {task} ({s})"),
        None => format!("{SLUICE_CHECK} / {task}"),
    }
}

pub trait Forge {
    /// The open pull requests into `base`.
    fn pulls(&self, repo: &Repo, base: &str) -> Result<Vec<Pull>>;

    /// Pull request `number`, open or closed; `None` when there is none.
    fn pull(&self, repo: &Repo, number: u64) -> Result<Option<Pull>>;

    /// Every review of pull request `number`, oldest first.
    fn reviews(&self, repo: &Repo, number: u64) -> Result<Vec<Review>>;

    /// The check runs and commit statuses other systems posted on `rev`,
    /// the latest of each name.
    fn checks(&self, repo: &Repo, rev: &str) -> Result<Vec<Check>>;

    /// When pull request `number` was marked ready for review, oldest
    /// first.
    fn readied(&self, repo: &Repo, number: u64) -> Result<Vec<DateTime<Utc>>>;

    /// sluice's own check run named `sluice` on `rev`, if it posted one.
    fn posted(&self, repo: &Repo, rev: &str) -> Result<Option<Posted>> {
        Ok(self
            .posted_all(repo, rev)?
            .into_iter()
            .find(|p| p.name == SLUICE_CHECK))
    }

    /// Every check run sluice's app posted on `rev`, each by its name.
    fn posted_all(&self, repo: &Repo, rev: &str) -> Result<Vec<Posted>>;

    /// Post sluice's check run on `rev`: update `existing`, or add one.
    fn post(&self, repo: &Repo, rev: &str, existing: Option<u64>, posting: &Posting) -> Result<()> {
        self.post_named(repo, rev, SLUICE_CHECK, existing, posting)
    }

    /// Post sluice's check run named `name` on `rev`: update `existing`,
    /// or add one.
    fn post_named(
        &self,
        repo: &Repo,
        rev: &str,
        name: &str,
        existing: Option<u64>,
        posting: &Posting,
    ) -> Result<()>;

    /// The bodies of the comments on pull request `number`.
    fn comments(&self, repo: &Repo, number: u64) -> Result<Vec<String>> {
        Ok(self
            .comment_list(repo, number)?
            .into_iter()
            .map(|c| c.body)
            .collect())
    }

    /// The comments on pull request `number`, oldest first.
    fn comment_list(&self, repo: &Repo, number: u64) -> Result<Vec<Comment>>;

    fn comment(&self, repo: &Repo, number: u64, body: &str) -> Result<()>;

    /// Replace comment `id`'s body.
    fn edit_comment(&self, repo: &Repo, id: u64, body: &str) -> Result<()>;

    /// Open a pull request from the branch `head_ref` of `repo` into
    /// `base`, ready for review.
    fn open(
        &self,
        repo: &Repo,
        base: &str,
        head_ref: &str,
        title: &str,
        body: &str,
    ) -> Result<Pull>;

    /// Replace pull request `number`'s body.
    fn edit(&self, repo: &Repo, number: u64, body: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn review(user: &str, state: &str, commit: &str) -> Review {
        Review {
            user: user.into(),
            state: state.into(),
            commit: commit.into(),
        }
    }

    /// C.7 `reviews`: each reviewer's last say counts, on the head alone.
    #[test]
    fn approvals_count_on_the_head_alone() {
        let rs = [
            review("ann", "APPROVED", "old"),
            review("bob", "APPROVED", "head"),
            review("cat", "APPROVED", "head"),
            review("cat", "COMMENTED", "head"),
            review("dan", "APPROVED", "head"),
            review("dan", "CHANGES_REQUESTED", "head"),
        ];
        assert_eq!(approvals(&rs, "head"), 2);
        assert_eq!(approvals(&rs, "old"), 1);
        assert_eq!(approvals(&[], "head"), 0);
    }
}
