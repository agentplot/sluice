# Spec Delta

## MODIFIED Requirements

### Requirement: submit opens or updates a pull request
`sluice submit <branch>` and `wt submit <branch>`, to a branch with
`pull_requests`, SHALL open a pull request into the branch from the
agent's branch, ready for review, or update the one already open from
that agent's branch and mark it ready for review at the head
submitted. From a worktree on a branch sluice lands on nowhere, they
SHALL push that branch to the branch's remote first, and the head
submitted is the worktree's head. From a worktree that has checked out
a branch sluice lands on, such as a bolt, they SHALL push nothing:
the pull request is from that branch as the remote holds it, the head
submitted is the remote's revision of it, and where the worktree's
head, the host's copy, differs from that revision, the output SHALL say
so with both revisions. A submit SHALL push only a branch sluice lands
on nowhere, and SHALL write no ref of a branch sluice lands on, on the
host or the remote. They SHALL NOT submit the pull request: it waits
for its conditions like any other (1, 120, 124).

#### Scenario: First submit
- **WHEN** an agent runs `wt submit main` from a worktree on `unit/review` and `main` has `pull_requests`
- **THEN** `unit/review` is on the remote, an open pull request from it into `main` exists, and the output names the pull request and what it waits for

#### Scenario: Submit again
- **WHEN** the agent commits again and runs `wt submit main` from the same worktree
- **THEN** the same pull request now has the new head, and no second pull request is opened

#### Scenario: Submit a draft sluice made
- **WHEN** sluice marked the agent's pull request a draft after it was sent back, and the agent runs `wt submit main`
- **THEN** the pull request is ready for review at the head pushed, and the next round submits it once its conditions hold

#### Scenario: Submitted by the operator's ops
- **WHEN** a pull request an agent opened was sent back, and the operator's ops runs `sluice submit main` from a worktree on the same branch
- **THEN** the pull request is ready for review at the head pushed, names ops as its agent, and the next round submits it once its conditions hold

#### Scenario: A worktree holding more than its commits
- **WHEN** the worktree holds uncommitted changes
- **THEN** the submit is refused as it is today, and nothing is pushed

#### Scenario: A bolt's landing worktree, its copy two landings behind (S58)
- **WHEN** a worktree has checked out `bolt/x`, which sluice lands on and whose policy names `remote = "origin"`, this host's `bolt/x` stands two landings behind origin's `8079db3`, `main` has `pull_requests`, and the worktree runs `wt submit main --no-wait`
- **THEN** origin's `bolt/x` is still `8079db3`, this host's `bolt/x` is where it was, a pull request from `bolt/x` into `main` is open and ready for review at `8079db3`, and the output names the pull request and says this host's `bolt/x` is behind origin's, naming both revisions, without saying it pushed anything

#### Scenario: The next read levels the copy, and a hand cannot
- **WHEN** after that submit a tool in the worktree runs `git reset --hard 8079db3`, and then `sluice step bolt/x` runs
- **THEN** the reset is refused, naming `git reset --hard 8079db3`, the revision this host's `bolt/x` holds and `8079db3`; the step sets this host's `bolt/x` to `8079db3`

#### Scenario: Submit again from the bolt after another landing on it
- **WHEN** the bolt's pull request is open at `8079db3`, another submission lands on `bolt/x` as `5a1e2f0`, and the worktree runs `wt submit main --no-wait` again
- **THEN** no second pull request is opened, nothing is pushed, and the pull request is ready for review at `5a1e2f0`
