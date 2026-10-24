---
title: Rules
description: The rules that bind every change to the repository, cited by clause and decision.
sidebar:
  order: 2
---

A mirror of
[docs/architecture/core.md](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md),
which is the binding copy. The rules are written so a reviewer can point
at a diff and say which rule it breaks. The narrative is the statement
in agentplot/blueprints, `design/sluice/requirements.md`; a rule names
the clause it enforces and never restates it.

## Rules

### The core and its seams

- **[core.1]** Code outside a profile names no task runner, worktree tool, multiplexer, service manager or forge.
- **[core.2]** Every seam of Part B is reached through exactly one interface, and a profile implements that interface and nothing of the core.
- **[core.3]** A seam with a single profile has an ADR saying why one is enough.

### Branches and landings

- **[core.4]** No code path reachable from the submit operation writes a branch's ref; a round of sluice's work may be run by any process, a waiting driver included.
- **[core.5]** A branch is moved only with the expected old revision, and a move whose expectation fails is reported, never retried blind.
- **[core.6]** A candidate is formed from the branch's revision and the entries ahead of it, in order, and from nothing else.
- **[core.17]** A candidate whose entries ahead land exactly as they were tested is the same candidate on the revision they land as: its tree, its runs and its verdict stand, and it is named as that revision extended by the entries still ahead of it.

### Verdicts and attribution

- **[core.7]** Every check run is recorded with its key, outcome and executor before any verdict is read from it, a fail exactly as a pass.
- **[core.8]** A failure is attributed to one entry, and only after every candidate ahead of it has a verdict.
- **[core.9]** A replay is recorded as a replay, never as a run.
- **[core.22]** A result counts for a gate only through a signer the branch's policy names and a claim that signer may make, never through what a record says of its executor alone; on a branch in one host's own files, through the runs that host's sluice starts.

### Places

- **[core.10]** The core starts no service; the checks it runs are one process group, ended with their place or their time limit, and what they hand to another process is removed when their place is released.
- **[core.11]** Every record a reader may need is readable with no sluice process running, from every host that submits to its branch.
- **[core.14]** Nothing the core decides is held between steps outside the records, and no two steps act on one branch at once.
- **[core.15]** A branch's gate is read from the branch's current revision, never from a candidate or a place.
- **[core.16]** The core releases only places it made, and a session's only when its working copy is gone; otherwise a session's place is released only by `sluice release` at its session's or driver's asking, and a preview only by `sluice release` at its caller's asking or when its command failed or its checkout is gone.
- **[core.18]** A session run counts toward the bound from the host's state and is recorded under its keys only when its worktree holds exactly its head.
- **[core.19]** A preview is made only at a revision its branch has landed, and decides nothing: its command is never recorded as a run, read by a gate, or waited on by a submission.
- **[core.20]** The core writes no working copy it neither made nor adopted: a worktree that has checked out a branch sluice lands on is reported, never moved, and never stops a landing.
- **[core.23]** An instance claims no run while a place it made belongs to a candidate with a verdict, an obsolete one, or a claim it no longer holds, until each such place is released or held.

### Design and records

- **[core.12]** A change that bears on a clause cites the clause by number; a clause that no longer holds is changed in the statement first, never worked around here.
- **[core.13]** Nothing commercial, no pricing, tier or customer, appears in this repository.

### The gate

- **[core.21]** Every file the tree tracks is owned by a formatter the gate runs, or is excluded by name with its reason; every language with a linter is linted; every directory under `plugins/`, and `site/`, is a moon project of its own, listed in `.moon/workspace.yml` with its language declared, whose check-fast tasks validate, type-check and test it, one task per check.

## Details

- **core.1** Rules out: the strings `moon`, `wt`, `worktrunk`, `devenv`, `herdr`, `gh`, `crew`, `flywheel` in any module outside a profile directory; an import of a profile from the core. Not ruled out: `git`, which section 9 of the statement fixes; `plugins/`, which no crate imports. Source: statement preamble and clause 52; ADRs 0003, 0023.
- **core.2** Rules out: a profile calling another profile; a core function with a profile-specific branch. Source: clauses 52, 53; ADR 0003.
- **core.3** Rules out: a seam whose only profile is undocumented as such. Source: ADR 0003.
- **core.4** Rules out: `update-ref`, `push`, a merge or a fast-forward in the submit operation or anything it calls; a round that behaves differently because a driver ran it. Not ruled out: a waiting `sluice submit`, `wt submit` or `sluice wait` running rounds between reads of its submission's state. Source: clause 1; ADRs 0002, 0005, 0011, 0041.
- **core.5** Rules out: `update-ref` without an old value; a landing loop that re-reads and retries on refusal without re-forming the chain. Source: clauses 24, 45, 46.
- **core.6** Rules out: a candidate formed from a cached revision of the branch; a candidate including an entry removed as culprit or conflict, or an entry behind it; a candidate formed on a revision that lacks the branch's last landing (121). Source: clauses 8, 10, 12, 21, 121; ADR 0051.
- **core.17** Rules out: testing a candidate again because the entries ahead of it landed as tested; telling a culprit, or recording, a combination by a revision the branch has left when the branch has since landed the entries ahead of it. Holds because a landing commit carries the tested candidate's tree (23): the branch after the landing and the candidate before it hold the same tree. Source: clauses 10, 20, 23; answers to the reconstruction, 12; ADR 0006.
- **core.7** Rules out: reading a verdict from the task runner's cache alone; discarding a failed run's record; writing a record after the verdict; removing a result's line because it was set aside. Not ruled out: a run's tasks kept while it goes, and when its output grew (ADRs 0028, 0042), which are no record of a run and which no verdict reads; a check a run did not judge, which has no outcome and is recorded under no key (60; could-not-run-1 114); a result the operator set aside, whose line stays and which no verdict reads (could-not-run-1 116). Source: clauses 16, 18, 60; could-not-run-1; ADRs 0004, 0028, 0031, 0042.
- **core.8** Rules out: attributing from a batch with no intermediate verdicts; attributing while a candidate ahead is still running; telling two sessions of one failure. Source: clauses 17, 18, 20.
- **core.9** Rules out: counting a cache hit as an executor's observation. Source: clause 16, 38; ADR 0004.
- **core.10** Rules out: starting an emulator, a server or a process manager from the core; a check's process group left running after its run or its place; releasing a place without running its release command. Not ruled out: the place seam giving a place its ports; a session's run, in its own worktree, under the executor (session-runs-1 clause 89); a preview's command handing off a stack that stays until the preview is released (previews-1 clause 98); a place whose working copy is gone, its stored release run once in its scratch directory and the place freed whatever it did; and a place freed at the operator's accepting that its release failed, with nothing run in its name (123). Source: clauses 31 to 34, 66, 123; previews-1 clause 98; ADRs 0010, 0016, 0018, 0045, 0050.
- **core.11** Rules out: state held only in a running process's memory; a database only a server reads; records reached other than through the records seam; on a branch whose records are on its remote, a state that only one host's files hold. Not ruled out: a run's tasks, kept through the records seam beside the branch's records on the host that runs it and read there alone, since they decide nothing (ADR 0028); a session run's tasks and command, kept by its own `sluice exec` in the host's state beside its count and read there through the check seam, since a session run is no branch's (session-runs-1 C.3; ADR 0035). Source: clauses 26, 51, 61 to 63, 80; C.6; ADRs 0008, 0028, 0035, 0037.
- **core.14** Rules out: a decision kept in a loop's memory and not in the records; a step that proceeds without the branch's lock; behavior that differs between `sluice run` and `sluice step`. Source: clauses 26, 51, S10; ADR 0005.
- **core.15** Rules out: reading policy from a candidate's tree or a session's place; a submission whose change to the policy relaxes its own gate. Not ruled out: the overview's order of branches, read from the default branch's policy, which no round, gate, landing or submission reads (ADR 0036). Source: clauses 27, 56; ADRs 0007, 0036.
- **core.16** Rules out: removing or cleaning a session's worktree on landing; a round or a run releasing a session's place whose worktree exists, running its release command or letting its ports go, however long since its last run; a candidate place left behind after its verdict is read. Not ruled out: `sluice release`, `sluice exec --release` and a driver's removal releasing a session's place; `sluice release --accept-failure` freeing a session's place at its session's or driver's asking, the worktree left (123); the next round or `sluice exec` on the host freeing a session's place whose worktree no longer exists, which is no session's, once its stored release has run once from its scratch directory, no policy read and no toolchain entered (123); a round, `sluice exec`, `sluice preview` or `sluice previews` releasing a preview whose command failed, or freeing one whose checkout is gone once its stored release has run the same way (123). Also rules out: a round, a step or a landing releasing a preview that is up, starting or waiting with its checkout in place. Source: clauses 25, 34, 123; session-runs-1 clauses 95, 97; previews-1 clause 101; ADRs 0018, 0020, 0045, 0050.
- **core.18** Rules out: a session run started when the count, read under the host's lock, leaves no room outside the reserve; a session run's count held in a process's memory or in a branch's records; recording a run from a worktree that holds anything beyond its head, a command's run, or a replay as a run. Not ruled out: a session run's tasks and command kept on its record in the host's state while it runs, and its result's tasks and whether its gate takes it, none of which a round, gate or replay reads (ADRs 0035, 0027). Source: clauses 16, 71; session-runs-1 clauses 89, 91, 92; ADRs 0012, 0018, 0035.
- **core.19** Rules out: a preview of a revision the branch has not landed, a combination's or a session's head; a preview's run written to `results.jsonl`; a gate, a round or a submission reading a preview's state; the gate's executor or limits read from the previewed revision rather than the branch's current one. Not ruled out: the preview's command, ports and release command read from the previewed revision, whose stack they describe. Source: previews-1 clauses 98, 103; clauses 27, 56; ADR 0045.
- **core.20** Rules out: a `read-tree`, `reset`, `checkout`, `switch` or rebase of a worktree that is no place of sluice's, after a landing or at any other time; a landing, a round or a submission refused or held because a worktree has its branch checked out. Not ruled out: moving the branch itself under such a worktree, which a landing does (45); naming the worktree in `sluice status`, `sluice overview` and `sluice protect`. Source: clauses 1, 25; ADRs 0018, 0024.
- **core.22** Rules out: counting a run whose record no signer the policy names has signed, on a branch whose records are on its remote; reading a run as confined, or as on a system, when its signer may not claim it; counting a replay with no trusted signer's pass for its key on its system; landing on a verdict the state holds without reading the signed runs again. Not ruled out: a one-host branch's unsigned runs, which its own sluice started. Source: clauses 27 to 29, 69, 78, 79, 108; ADRs 0032, 0033.
- **core.23** Rules out: a claim taken by a host with a candidate place left from a verdict, an obsolete candidate or a lapsed claim, before that place's release has been tried; a result reported under a claim its reporter no longer holds. Not ruled out: a place on a host that never returns, which is the operator's to remove; a claim taken while a place the host made is held, named with what failed (81, 123). Source: clauses 25, 32, 34, 77, 81, 123; ADRs 0037, 0050.
- **core.12** Rules out: a comment or commit that reinterprets a clause; a behavior the statement forbids kept behind a flag. Source: ADR 0001.
- **core.13** Rules out: a pricing table, a tier name, a customer name, a hosted-plan reference in any file. Source: ADR 0001.
- **core.21** Rules out: a file type treefmt neither formats nor excludes; an exclude with no comment saying why; a plugin directory or the site with no project of its own in `.moon/workspace.yml`, or one with no language declared; a part's check run as a task of another project, the root's included; one task bundling several checks; a plugin directory whose manifest, types or tests no task of its project runs; a task in a change whose only verification is a command run by hand, unless the task says why the gate cannot run it. Not ruled out: a check that spans the parts, the marketplace's, as a task of the root; the formatters and linters treefmt runs over the whole tree in `root:format`; the site's build, which is the Pages workflow's; the site's prose check, `site:prose`, as its one check, since the site has no tests and its type check waits until a place holds the site's dependencies; `*.md` outside the site, which no gate reads. Source: ADRs 0029, 0030.

The rules use the statement's design words where the rest of this site
uses plain ones: a *place* is a checkout, an *entry* is a submission, a
*candidate* is a combination, a *verdict* is a result, a *culprit* is
the submission blamed, and the *records* are the state.
