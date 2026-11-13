---
number: 91
title: A removed branch answers a status from its records, and the overview names it gone
status: accepted
date: 2026-11-06
decision-makers:
- the code stage of fix/a-branch-the-overview-names-has-a-status
---

# A removed branch answers a status from its records, and the overview names it gone

Status: accepted on the user's yes of 2026-11-06 to clause 105's amendment, on `boomdrivedev/blueprints` main at e8225aa; proposed by the code stage of `fix/a-branch-the-overview-names-has-a-status`, under clauses 26, 61, 105 and 131 and rules core.11 and core.41, which it keeps.

Deciders: the code stage of `fix/a-branch-the-overview-names-has-a-status`.
Sources: `boomdrivedev/blueprints:design/sluice/requirements.md` clauses 26, 61, 129 and 131 and C.6 at c94c496, and 105 as amended at e8225aa; rules core.11, core.25, core.41; record 0046; `openspec/specs/branch-lifecycle/spec.md`, "sluice deletes a branch once its work has landed"; swancloud's person's copy on deneb, `~/Code/github_afterthought/swancloud/main`, sluice 0.68.1, on 2026-11-06; Flume's `crates/flume/src/profiles/merge_queue/sluice/mod.rs`, `branches` and `read_submissions`, at its main of 2026-11-06.

## Context and problem statement

In swancloud's person's copy on deneb, `sluice overview --json` names `bolt/a-rift-step-finds-its-pane` and `bolt/the-herdr-sign-in-step-says-which-command-and-which-machine`, each with `"at": null` and nothing queued or recent. `sluice status bolt/a-rift-step-finds-its-pane` answers `no branch bolt/a-rift-step-finds-its-pane; branches with records: ..., bolt/a-rift-step-finds-its-pane, ...`: the branch is called absent and listed as having records in one line.

Each bolt was made in that copy by `sluice branch create`, which records the revision it made the branch at (121), and was removed once it landed. Every submission to it was made in another copy, so this copy's records hold nothing else. The overview lists every branch the state holds (`State::branches`). A status called a branch no branch when it had no revision, no live submission and nothing finished (`Status::is_unknown`), so a branch whose records exist, and which is gone, was refused.

Flume reads `sluice status <branch> --all --json` for every branch the overview names, to find where each Flume worktree's work went, so every Flume worktree in swancloud and in sluice printed five to ten `cannot read: the merge queue's bolt/...` lines.

The statement says the removed branch's records stay (105), every submission's state is readable (26), its state is read given a branch (61), and a status of a branch reads the live state and the index of finished submissions (131). It does not say how a status names a branch that is gone, nor whether the overview shows such a branch with the others.

## Considered options

**When is a branch no branch to a status?**

1. **When it has neither records nor a revision** (chosen). A branch has records where its read found any, or, where it found none of a branch the host holds no revision of, where the state lists it as the overview does, so a status never calls absent a branch the overview names. A status of such a branch answers from its records: its queue, its history when asked, and that the branch is gone.
2. Keep reading no revision, no live submission and nothing finished as no branch, and leave such branches out of the overview. The state 105 keeps would be read by no status, and a branch's whole history, when its entries were made elsewhere, would be unreachable by name.
3. Remove a branch's records with it. Against 105.

**How is "gone" read?**

1. **By the read the status already makes** (chosen): where its state has seen no revision of the branch, its own read, a fetch for a branch on a remote; otherwise the revision the host holds of it, with no fetch, as the overview reads `revision` (record 0046). A status asks the remote nothing it did not ask before. A branch on a remote removed by another host, whose remote-tracking ref this host has not pruned since, reads as there until a fetch of it.
2. Read the branch itself on every status. Exact, but a fetch for each status of each branch on a remote, and Flume reads a status of every branch the overview names on each of its own status reads.

The overview says a branch is gone where the host holds no revision of it and the read of its records was not refused: the same holding its `revision` gives.

**Should the overview still list a removed branch with nothing queued and nothing recent, or show it apart?**

The statement makes the overview a view of the repository's branches that reads every branch it can and hides none (26), written by one reader for everyone watching the repository's queues and followed by every watch and wait on the host (129; core.25). A removed branch's records stay (105).

1. **The document names it, marked gone; the text shows it apart** (chosen). `branches[]` keeps every branch with records, and a removed one has `"gone": true`. The text form, which a person reads to watch the queues, names a gone branch with nothing to watch (nothing queued, under test or recent, no hold, no held checkout, no refused write, no read refused) on one line after the branches it lands on: `gone, nothing queued or recent: <branch>, ...`. A gone branch with something to watch, its last landings among its recent lines, is shown as any other, its first line saying it is gone and the revision it was last at.
2. Leave it out of the overview. Every reader on the host follows the overview's document, and a reader that finds a branch's history by the branches the document names, as Flume's read of where a Flume worktree's work went does, would lose the state 105 keeps. 26 has the view read every branch it can.
3. A list of its own in the document, `gone[]`. Additive, but every reader that reads a branch's history from `branches[]` would have to read a second list to keep reading a removed branch's, Flume among them.
4. List it as before, unmarked. A person watching reads an idle queue for a branch that no longer exists.

## Decision outcome

The first option of each.

### Consequences

- `sluice status <branch>`, plain and with `--all`, answers a removed branch whose records exist, exiting 0: its first line `<branch> is gone`, with `, last at <revision>` where its records saw one, then its queue and, asked, its history. `--json` gives `gone`, `true` or `false`, beside `at`.
- `sluice status <branch>` of a branch with neither records nor a revision still exits 1, naming the branches with records.
- `sluice overview --json` gives a removed branch `"gone": true`, left out otherwise; the text names a gone branch with nothing to watch apart, on one line.
- Flume needs no change: its read of where a Flume worktree's work went asks a status of every branch the overview names, and that status now answers. Its `cannot read` lines stop once the host's sluice holds this.
- A removed branch whose records are on its remote, under a policy naming signers, is routed by the revisions of it the host holds (124; core.27), and with none held its records are read from the host's own files. A status and the overview agree on it, reading it there, but neither reaches its records on the remote. Routing a branch to a remote no revision names is what core.27 rules out, so that case waits on the statement's word and is outside this record.
