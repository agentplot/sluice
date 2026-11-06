---
number: 90
title: A submission by name is its branch's work, records no worktree and no session, and is delivered by nothing
status: accepted
date: 2026-11-06
decision-makers:
- the construct stage of change/a-submit-names-its-head-from-no-worktree
---

# A submission by name is its branch's work, records no worktree and no session, and is delivered by nothing

Status: accepted. The submission by name is the statement's, 124 as amended on agentplot/blueprints main at 51b6316 on the user's approval of Flume's settles design, 2026-11-06 ("No worktree: a branch and a record"). How it is recorded, delivered and matched is a decision of the construct stage of `change/a-submit-names-its-head-from-no-worktree`, cheap to change later. 67 and 120 are amended to say a submission by name is its branch's work, on the blueprints branch `design/sluice-a-submit-names-its-head-from-no-worktree` at 97c76e8, cut from main and unpushed.

Deciders: the construct stage of `change/a-submit-names-its-head-from-no-worktree`.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 1, 2, 51, 54, 67, 120, 124, 130 and C.5's pull request row at 51b6316; `design/flume/requirements.md` 133, 202, 203 and C.6's submit row for a base place there; Flume's design `every-place-waiting-on-the-merge-queue-is-settled-at-every-depth`, decision 8 and follow-up S1; rules core.4, core.31, core.41, core.48; ADRs 0025, 0050, 0085; `crates/sluice/src/lib.rs` (`Command::Submit`, `status_json`, `inbox`), `crates/sluice-core/src/ops.rs` (`submit_group`, `admit`, `pending_on`), `crates/sluice-core/src/step.rs` (the outbox), `crates/sluice-core/src/overview.rs` (`Here::made`) and `crates/sluice/src/profiles/worktrunk/mod.rs` (`session`) as main held them at b2263f63; afterthought/crew's `plugin/lib/plan.py` `SUBMIT`; agentplot/boomdrive's `crates/boomdrive/src/sluice.rs`.

## Context and problem statement

Flume's base place is a branch and a record, with no worktree (flume 202). Flume submits it by naming its branch from the repository, `sluice submit <base> --head <branch> --no-wait --json`, and reads its outcome from `sluice status <base> --all --json`, the submission whose `from.head` is the branch (flume 203, C.6). 124 as amended takes that submission: it is of the branch as its policy holds it, records no worktree and no session as its submitter, and its outcome is pending in no inbox, read from the branch's records with an inbox's words.

sluice today takes `--head` only with `--check`, records the worktree a submit runs in, and gives an outcome to the worktree whose branch is the submission's `from.worktree` or `from.head`. How is a submission by name recorded so that it is its branch's work, no worktree's, and delivered by nothing, while every reader of `from` keeps working?

## Considered options

**How the record says a submission is by name.**

1. **`from.worktree` and `from.head` both the branch, no `path`, no `session`** (chosen). The work is the branch, as for a worktree on it, so 67 and 54 key on it; the head is the branch, which Flume reads. The two equal tell it apart from a worktree's own submission, which names no `from.head` (ADR 0025), and from a request's, whose `from.worktree` is `pull/<n>`. No field is added.
2. A field of its own, `from.named`. Every reader learns a field two fields already say.
3. `from.worktree` empty. Nothing for 67 and 54 to key on: two branches submitted by name to one target would withdraw each other.
4. `from.worktree` the branch and no `from.head`. The record of a worktree's own submission: an inbox in a worktree on the branch would take it.

**How its outcome is delivered.**

- a. **By nothing** (chosen). The round queues no notice for it, so no driver is asked, no `pending` line is logged, and `delivered` is never set; the inbox and a wait's delivery take it as no worktree's. It has no carrier (51), and 124 says it is pending in no inbox.
- b. A notice the driver answers undelivered, as for a submission with no session today. Its `pending` line says the outcome waits for an inbox no one runs.

**Which submissions a worktree takes as its own.**

- i. **What it submitted, and a request from its branch where that branch is one sluice lands on nowhere** (chosen). On a target that takes pull requests a submission by name is the request's, `from.head` the branch; ADR 0025 matches a request to the worktree that pushed its branch, and no worktree pushes a branch sluice lands on (1, 124). Whether the branch is one is read only when such a request is found, as the protection reads the branches it guards, with no fetch.
- ii. ADR 0025's match as it stands. A worktree checked out on the branch collects a request opened by name whenever its post on the request has not delivered it, against 124.
- iii. A mark in the request's body for a submission by name, read by intake into the record. A record field again, in a body any person may edit.

**What may be named.**

- **A branch sluice lands on, other than the target** (chosen). A branch of a worktree's own is submitted from that worktree (2); a submit pushes only a worktree's work (1). `--check --head` keeps forecasting any branch the host holds, since a forecast decides nothing (64).
- Any branch, pushing one sluice lands on nowhere. A branch no worktree holds is no session's work.

## Decision outcome

Option 1, a, i, and a branch sluice lands on other than the target. Rule core.50 holds them. A status of a branch or of one submission gives each decided submission's `notice`, the inbox's words, so a submission by name is read there as an inbox would give it (124).

### Consequences

- A submission by name is a record of today's fields; one written before reads as it was. An older sluice sharing a branch's records reads it as a worktree's in a worktree on the branch, and its rounds queue a notice no driver delivers; hosts sharing a branch's records take this sluice together.
- A worktree checked out on a branch sluice lands on, submitting to a target that takes pull requests, no longer collects that request's outcome by its inbox. The request carries it, and its waiting submit or `sluice wait` reports it.
- A submission by name and one from a worktree on the branch withdraw each other's entry (67 as amended). Until crew retires its land worktree (crew's C1), a bolt is submitted by one of the two.
- `sluice status --json` gains `notice` on every landed, failed or conflicting submission.
