---
number: 99
title: A delivery read where no files are checked out reads as a worktree on the branch its HEAD names, reading no files and no commit
status: accepted
date: 2026-11-07
decision-makers:
- the construct stage of change/a-delivery-read-needs-no-work-tree
---

# A delivery read where no files are checked out reads as a worktree on the branch its HEAD names, reading no files and no commit

Status: accepted, a decision of the construct stage of `change/a-delivery-read-needs-no-work-tree` under the statement as written (2, 3, 50, 51, 124), which no clause of it changes, and cheap to change later. It amends record 0085's reading of a repository with no files checked out; the rest of record 0085 stands.

Deciders: the construct stage of `change/a-delivery-read-needs-no-work-tree`, on the planner's brief for the unit of that name in the bolt `sluice-tells-each-submitter-its-own-end` (agentplot's proposal 30, approved 2026-11-06): that a wait, a waiting submit or `sluice inbox` run where no files are checked out reads every submission's delivery as it would from a worktree, a worktree's submission among them, and never fails, or leaves an outcome pending, because no files are checked out.
Sources: `boomdrivedev/blueprints:design/sluice/requirements.md` clauses 2, 3, 50, 51, 124, 130, 131 and C.5's worktrunk and pull request rows, at c26c425; rules core.2, core.31, core.41, core.48, core.50; records 0068, 0070, 0085, 0090; the signal `signals/2026-11-07-agentplot-ops-d4947942/02-sluice-inbox-reads-delivery-in-a-bare-repo` on `refs/crew/agentplot/main` of afterthought/crew-state; `crates/sluice/src/lib.rs` (`recipient`, `delivering`, `inbox`, `no_work_tree`) and `crates/sluice/src/profiles/worktrunk/mod.rs` (`session`) as `bolt/sluice-tells-each-submitter-its-own-end` held them at 68d68a9a; crew's `plugin/lib/plan.py` `SUBMIT`, its main at 130ce42b; the worktrees guide, `site/src/content/docs/guides/worktrees.md`.

## Context and problem statement

On deneb at 03:52Z on 2026-11-07, agentplot-ops landed a bolt on boomdrivedev/sluice's `main`, which takes pull requests, through crew. crew cuts a short-lived worktree on the bolt's branch, runs `wt submit main --no-wait --json` there, and runs its `sluice wait <n>` in the kit's main checkout, which on deneb is the bare `sluice.git`. sluice 0.68.1's wait ran `git status` there to learn whom it delivered to, and said: `sluice: could not read whether #2 is delivered, so it stays pending for sluice inbox: git status in .: fatal: this operation must be run in a work tree`.

Record 0085 already has the driver read such a folder with no `git status` (sluice 0.70.0). It also decided that a delivery read there delivers nothing, since no session works there, and says so: "#<n> stays pending for its session's sluice inbox: <dir> is a repository with no files checked out, so no session works in it and no outcome is delivered there". That says less than is so:

- A session does work there. crew's agentplot-ops runs its waits in the bare clone, and the layout the worktrees guide recommends keeps the git directory bare beside `main/`.
- The line is false for most of what it is said of. Submission #2 was a pull request's entry from a branch sluice lands on, which is pending in no inbox: its carrier is the request (124, C.5; record 0090). A worktree's submission read there is another worktree's, which a wait from any other worktree reports and leaves for its own carrier, saying nothing.
- It delivers nothing that is the folder's own. A git directory whose `HEAD` names a branch holds that branch's work as much as a worktree on it does.
- The driver reads `HEAD`'s commit before anything else, so a folder whose `HEAD` names a branch with no commit, as a repository made by `git init --bare` has before its default branch is pushed, fails the read outright.

How does a wait, a waiting submit or `sluice inbox` read delivery in a folder with no files checked out?

## Considered options

1. As record 0085: deliver nothing there, and say the outcome stays pending for its session's inbox.
2. **Read the folder as a worktree on the branch its `HEAD` names, with no path** (chosen): the one reading of whose a submission is (record 0090) applies unchanged, and who is there is read without the folder's files or its `HEAD`'s commit.
3. A wait delivers the submission it reports wherever it runs, whoever submitted it.
4. Read who is there by the agent its harness names, `SLUICE_AGENT`, as well as by the worktree's branch, so a session collects what it submitted from any worktree.

## Decision outcome

Option 2.

- **Whose.** A folder with no files checked out, a bare repository or a git directory, takes as its own exactly what a worktree on the branch its `HEAD` names would: a submission naming no head whose work is that branch; a pull request's from that branch, where sluice lands on it nowhere; never one by name. A folder on no branch takes nothing, as a detached worktree takes nothing (3).
- **Who is there.** The driver gives it through one operation of its seam, `Driver::recipient` (rule core.2). It reads the branch `HEAD` names, the top level only where files are checked out, the agent the harness names and the multiplexer's session, and neither the folder's files nor its `HEAD`'s commit (2). `Session::bare` goes, since nothing reads it.
- **Delivered to.** The multiplexer's session where the read runs in one, else the worktree's path, else, where no files are checked out, the agent the harness names, `SLUICE_AGENT`, or the branch (51).
- **Words.** Nothing is said of an outcome staying pending because no files are checked out. An outcome that is another's is reported and left to its own carrier with nothing said, as from any worktree. A read that cannot read the folder as a repository at all is said, as any failed delivery read is.

**Why not the others.** Option 1 is false for a request's entry and for the folder's own, and fails on a `HEAD` with no commit. Option 3 hands one session's outcome to whichever process waits on it, so its own session, by its pane or its harness's collect, never gets it, against 50 ("the outcome delivered to that session and no other"). Option 4 is nearest 51, which finds a session's outcomes "by the submitter" each recorded, and it would deliver crew's submission from its short-lived worktree to its wait in the bare clone. But the index of the finished names no agent (130), so an inbox would read every finished record to find its landings, against 131 and rule core.41. It would also change what every worktree collects, not only what a folder with no files checked out does. It needs 130 amended first (rule core.12), so it is left to a change of its own.

### Consequences

- Rule core.48's last clause says it, with its details; record 0085 carries an amendment line naming this record.
- A git directory's own submissions, made from it or from a worktree on the branch its `HEAD` names, are collected there by `sluice inbox` and delivered there by a wait.
- crew's submission from a short-lived worktree, to a branch that takes no pull requests, stays pending for its own carrier when its wait runs in the kit's main checkout, bare or not: its pane where the submit recorded one, else nothing, until option 4 is taken. Its wait still reports it, and the wait's exit is crew's outcome. To a branch that takes pull requests, as on deneb, its carrier is the request, and nothing is pending.
- `delivered_to` may name an agent, where an outcome was collected where no files are checked out and in no pane.
