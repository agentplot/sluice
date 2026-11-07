# Design

## Context

### What the statement asks

sluice 124, as amended on agentplot/blueprints main at 51b6316, the
user's approval of Flume's settles design on 2026-11-06:

> A submission that names as its head a branch sluice lands on, run
> anywhere in the repository, from a worktree or from none, is of that
> branch the same way, and records no worktree and no session as its
> submitter. Its outcome is pending in no inbox: it is read from the
> branch's records, with the words an inbox would give, the checks that
> failed, the files contested and the revision it was tested on (51). So
> a branch with no worktree on it submits, as flume's base place does
> (flume 202, 203).

"The same way" is the sentence before it: a submission from a worktree
on a branch sluice lands on "is of that branch as the remote holds it:
the submit pushes nothing to that branch, opens or updates the request
from it, and submits the remote's revision".

Flume reads it through its profile (flume C.6): `sluice submit <base>
--head <branch> --no-wait --json`, run in the repository, answered as
any submit; no `sluice inbox` for it; its outcome from `sluice status
<base> --all --json`, the entry whose `from.head` is the branch, failed
and conflict with the record's words, files and `tested.on`. Flume's
design (decision 8, follow-up S1) gives its side: `SubmitHead::Branch {
branch, revision }`, and `collect` never called for a base place.

### What sluice does today

- **`--head` only forecasts.** `Command::Submit` in
  `crates/sluice/src/lib.rs` declares `head` with `requires = "check"`,
  and `forecast` reads it through `Sluice::forecast_branch`
  (`crates/sluice-core/src/ops.rs`): the branch's revision as the branch
  seam reads it, forecast as the work `own = <branch>`, so a forecast
  leaves out the submissions made from that branch (67).
- **A submit reads its worktree.** `Sluice::submit_group` asks the
  driver for each worktree's `Session`
  (`crates/sluice/src/profiles/worktrunk/mod.rs`, `session`): `HEAD`,
  its branch as `from.worktree`, the top as `from.path`, `git status`
  for what it holds beyond its commits, `SLUICE_AGENT` or the branch as
  `from.agent`, the herdr pane as `from.session`, the host as
  `from.host`. `submitted_head` then reads, for a worktree on a branch
  sluice lands on, the remote's revision of that branch (124). In a bare
  repository's git directory the driver reads `HEAD`'s branch and no
  path (record 0085).
- **An outcome goes to a worktree by its branch.** `pending_on` takes a
  submission as `me`'s where `from.worktree == me.worktree ||
  from.head == me.worktree`, `me.worktree` being the branch of the
  worktree the inbox or the wait runs in. The home view's `Here::made`
  (`crates/sluice-core/src/overview.rs`) matches `from.path` or
  `from.head` against the worktree's path and branch.
- **The round queues every decided outcome.** Landing, a landing found
  (record 0085), a blame and a conflict each push an `OutboxItem`
  (`crates/sluice-core/src/step.rs`); `perform_outbox` calls the
  driver's `notify` and removes the item, logging `delivered` or
  `pending`.
- **`sluice status --json` gives the record.** `status_json` and the
  lookup by number serialize `Submission`; the inbox adds `notice`
  (`words::notice`) to each record it prints, and the status does not.
- **A failure's way on names an agent.** `report` words a failure or a
  conflict for "the agent working in `<from.head or from.worktree>`"
  (`words::fix_way_on`, `conflict_way_on`, `timed_out_way_on`).

### Looking across

| tool | what it does today | what this change keeps or gives it |
|---|---|---|
| Flume | pinned to sluice 0.67.0: a place's submit `sluice submit <base> --no-wait --json` in its worktree, its outcomes by `sluice inbox --json`, its read-submission the highest `id` whose `from.worktree` is its branch in `sluice status <base> --all --json` | its base place's submit by name and its read by `from.head` (C.6), with `notice` in the status; every read it makes of a place's submission keeps working, since a place's branch is never a branch sluice lands on. Flume's task 5.2 moves its pin to the release that carries this |
| crew | a bolt submitted by `git worktree add --no-checkout <kit>/places/land-<bolt> bolt/<b>`, `wt submit main --no-wait --json --no-remove` there, and a background `sluice wait <n> --timeout 9m` (`plugin/lib/plan.py`, `SUBMIT` 3946 to 3998) | unchanged: a worktree on a branch sluice lands on submits as 124 says, and its entry is that worktree's. Its follow-up C1 retires the path once Flume's release is out; until then a bolt is submitted by one of the two, since each withdraws the other's entry (D4) |
| Boomdrive, the runner | `sluice status <id> --json` read raw for each submission Flume names, its `state` against `landed`, `failed`, `conflict`, `withdrawn` (`crates/boomdrive/src/sluice.rs`) | the same document, gaining `notice`; `from` gains no field |
| the herdr plugin | focus and jump by `from.path`; rows by `from.worktree` and `from.agent` (`plugins/herdr/lib/words.jq`, `bin/jump.sh`) | a submission by name has no path, so no focus link; its row names its branch and agent. No change |
| the sluice board | a session's own by `from.path`, the ids and requests its submit printed (`plugins/sluice-board/hooks/board.ts`, `isMine`); delivery by `sluice inbox` | no change: it delivers through `sluice inbox`, which answers nothing for a submission by name |

One design, one owner: sluice records who submitted and decides the
outcome; Flume names the branch and reads the record; the runner and
crew read what Flume or their own submit names. Flume's
`SubmitHead::Branch` is this change's `--head` without `--check`, and
Flume's `collect` skipped for a base place is this change's outcome
delivered by nothing.

### Rules relied on

core.4 (a submit writes no branch's ref; a waiting submit may run
rounds), core.11 (every record readable with no sluice running),
core.14 (one write per branch, under its lock), core.31 (an outcome
delivered by sluice alone, on its carrier's word; here no carrier),
core.41 (the live records keep their form; a finished submission moves
out once no outbox item names it), core.43 (a landing names the subject,
or the submitted branch), core.48 (work the branch holds is landed; no
session read where no files are checked out). The change draws rule
core.50 and record 0090.

## Goals / Non-Goals

**Goals:**

- `sluice submit <target> --head <branch> --no-wait --json`, without
  `--check`, from anywhere in the repository, a bare repository's git
  directory included, submits a branch sluice lands on as its policy
  holds it.
- Its entry records no worktree and no session as its submitter, and
  the branch as its head, `from.head`.
- `sluice inbox` anywhere answers nothing for it.
- `sluice status <target> --all --json` gives it with an inbox's words.
- `--check --head` and `--head` read as one: the same head, the same
  work.
- Every reader of `from`, sluice's and the family's, keeps working.

**Non-Goals:**

- Submitting by name a branch sluice lands on nowhere. Such a branch is
  a worktree's work and is submitted from its worktree (2, 124).
- Several branches by name as one act (125).
- Flume's, crew's and the runner's own changes.

## Decisions

### D1. `--head` names the head, for a forecast or a submit

`sluice submit <target> --head <branch>`:

- **without `--check`** submits `<branch>` by name (D2 to D9);
- **with `--check`** forecasts `<branch>` as today, and submits nothing.

The flag's help says both: "The branch to submit in place of this
worktree's head, a branch sluice lands on, from anywhere in the
repository; with --check, the branch whose revision to forecast."

The two read together. Both read the head as the branch seam reads
`<branch>`, and both read the work as `<branch>`, the forecast's `own`
and the submission's `from.worktree` (D3). So the forecast's `head` is
the revision the submit would send, and it leaves out the entries the
submit would withdraw (67). A forecast decides nothing (64), so `--check
--head` keeps forecasting any branch the host holds; the submit takes
only a branch sluice lands on (D2).

What it takes with it, as any submit: `--after`, `--priority`,
`--no-wait`, `--json` and one `--subject`, which names what lands
(core.43); with none the landing line and a request's title name the
branch. What it refuses before anything is read, exit 64 with the usage
words:

- `--head` with `--worktree`: "--head names what is submitted, from
  anywhere in the repository; give no --worktree with it".
- `--head` with `--status`: "--status reads a worktree's last
  submission; a branch's are in sluice status <target> --all".

### D2. The head: the branch as its policy holds it

The core reads `<branch>` once, through the branch seam, as
`submitted_head` reads a worktree's branch sluice lands on today: the
remote's revision where the branch's policy names a remote (124), the
host's own ref where its records are the host's (`HostCopy::Itself`).
It reads the host's copy of a remote branch for nothing, so it says
nothing of a copy that differs: no worktree's copy is submitted.

Refused, exit 2, nothing written, each with the core's
`SubmitRefusal` and its words:

| case | refusal | words |
|---|---|---|
| no branch `<branch>` | `NoHead` (as the forecast's) | `no branch <branch> to submit` |
| `<branch>` is `<target>` | `ToItself` (new) | `<branch> is not submitted to itself` |
| `<branch>` has no `[branches]` table at its revision naming it | `NotLandedOn` (new) | `<branch> is no branch sluice lands on: a branch of a worktree's own is submitted from that worktree` |

The target's own refusals, no branch, not one sluice lands on, an
unknown `--after`, are a submit's, as today.

What it reads where it runs: nothing of a worktree. It asks the driver
for no session, so runs no `git status` and reads no pane, wherever it
runs: a bare repository's git directory, the main checkout, a worktree
on any branch (core.48). ADR 0050's question, which checks the runner
keeps out of CI, has no worktree to ask: the submission is taken with
nothing asked and no warning, as a request's entry is.

### D3. The record: the branch as its work and its head

A submission by name is recorded as:

| field | value |
|---|---|
| `from.agent` | the agent the session's harness names (`SLUICE_AGENT`), else the branch |
| `from.worktree` | the branch |
| `from.head` | the branch |
| `from.path` | none |
| `from.session` | none |
| `from.host` | the host it ran on, as its peers name it |

The driver gives it, through a new operation of its seam (rule core.2),
since who submitted and on which host are the driver's to say (rule
core.1):

```rust
/// 124: who submits `branch` by name: the agent the session's harness
/// names, else the branch; the branch as its work and its head; this
/// host; no worktree and no session, wherever the command runs.
fn named(&self, branch: &str) -> Result<Submitter>;
```

The worktrunk driver's reads `SLUICE_AGENT` and its host and nothing
else; the fake driver's is scripted. The core then makes the
submission's `Session` from it, `head` the revision D2 read, nothing
held beyond its commits, not detached, and admits it as any.

`from.head` equal to `from.worktree` tells a submission by name apart,
and nothing else does: a worktree's own submission has no `from.head`
(ADR 0025), and a request's `from.worktree` is `pull/<n>`. A predicate
on the seam's type says it once, `Submitter::by_name()`, for every
reader in sluice; a reader outside reads the same two fields.

`from.worktree` is the branch, not empty, because it names the work
(its doc: "the session's work, as its driver names it"). The finished
index keeps it (130), the overview's `recent_from` and rows show it, 67
and 54 key on it (D4), and `sluice wait <target>` in a worktree on the
branch finds the work's last submission by it.

### D4. One work: by name or from a worktree on the branch

A submission by name and one from a worktree on that branch are the
same work, as `from.worktree` says. `admit` already keys 67 and 54 on
it: the newer withdraws the older's entry on the same target, by name or
from the worktree either way, and the attempt counts the branch's
submissions. A failed or conflicting submission by name stays live
(130) until its caller's next submit withdraws it, as Flume's retry
does after a fix lands on the branch (flume 203), or the operator
withdraws it.

67 says "a submission from a place". Its amendment on
agentplot/blueprints main at f9455e7 adds: "A submission that names a branch
as its head (124) is that branch's work, as one from a worktree on that
branch is. Each withdraws the other's entry on the same branch, and the
count 54 shows is the branch's." 120 gains that an entry is submitted
again by "a new submit … a session's, or one that names as its head the
branch the entry came from (124)".

### D5. Its outcome is delivered by nothing

A submission by name has no carrier: no session to push into, no
worktree whose harness collects (51, 124).

- **No outbox item.** Every place the round decides an outcome pushes it
  through one method of the line, which pushes nothing for a submission
  by name. So `perform_outbox` asks no driver, logs no `pending` line,
  and `delivered` is never set. The decision is logged as any: `landed`,
  `found_landed`, `blamed`, `conflict`. No item names it, so a landed one
  leaves the live records at the next write, as any finished submission
  does (130; core.41).
- **No inbox takes it** (D6).
- **A wait marks nothing.** A wait, a waiting submit or `sluice wait
  <n>` reports it as any, and its delivery read finds it no one's. The
  line record 0085 says for a repository with no files checked out,
  "#<n> stays pending for its session's sluice inbox", is not said for
  it: it is pending nowhere.

### D6. Whose a submission is

One reading in the core decides whether a submission is the worktree
`me`'s, for the inbox and a wait's delivery (`pending_on`) and for the
home view (`Here::made`):

- **its own**: `from.head` absent and `from.worktree` the worktree's
  branch, or `from.path` the worktree's path where the home view reads
  by path;
- **a request's from its branch**: `from.worktree` a request's,
  `from.head` the worktree's branch, and that branch one sluice lands on
  nowhere;
- **never one by name** (`Submitter::by_name()`).

The request's condition reads whether the worktree's branch is one
sluice lands on only when a request with that head is found, once per
read, as the protection reads the branches it guards: every revision the
host holds, with no fetch (124, core.27). A branch whose policy cannot
be read is read as one sluice lands on nowhere, as before.

Why the request's condition: on a target that takes pull requests a
submission by name is the request's entry, `from.head` the branch (D7).
A worktree checked out on that branch, the main checkout or crew's land
worktree, would otherwise collect it by `from.head`, against 124's
"pending in no inbox". ADR 0025 matches a request to "the worktree that
pushed that branch", and no worktree pushes a branch sluice lands on (1,
124). So the reading only names what was always so.

`last_submitted`, for `sluice wait <target>` and `submit --status`,
keeps reading by `from.worktree`: the work's last submission, by name or
from the worktree (D4). A wait on it reports it and, by this reading,
marks only what is the worktree's own.

### D7. A target that takes pull requests

There the submit by name does what a submit from a worktree on the
branch does (124, C.5): it opens or updates the request from the branch
into the target, pushes nothing, and marks it ready, titled by the
subject or the branch. `PullRequests::open` and its `opening` take the
head as given, a worktree or a branch named, and read a branch named's
session from the core (D3) in place of the driver's `session`. The
request's entry is intake's, as every request's: `from.worktree`
`pull/<n>`, `from.head` the branch, no path, no session. Its carrier is
the request, where its outcome is posted and marked delivered (C.5), and
no inbox takes it (D6). Flume reads it by `from.head` as it reads the
other.

### D8. A status gives an inbox's words

`sluice status <branch> --json`, with `--all` or without, and `sluice
status <id> --json` give each landed, failed or conflicting
submission's `notice`, the inbox's words for it (`words::notice`, from
the record's own outcome). Beside it they give, as now, the record's
`failed` with each check's task, `conflict.paths` and `conflict.with`,
and `tested` with `on`; a conflict found while its combination was
formed has no `tested`, as in the inbox. One function makes a decided
record's document for the inbox and the status. Every submission's,
not a named one's alone: the words are the record's, and a field added
breaks no reader (the runner's raw read, Flume's, the plugins'). A
withdrawn or undecided submission has no `notice`.

### D9. A submit by name that waits

Without `--no-wait` it waits as any submit (135): it moves the queue
while it waits as any waiting submit does (core.4, core.25, core.31),
follows a pull request where the target takes them, reports the outcome
and exits by it. It delivers nothing (D5). Its text line is `submission
<n> to <target> from <branch> · <ahead>`.

### D10. The way on of a submission by name

A text report of a submission by name that failed, timed out or
conflicts names its way on as its caller's: a fix landed on `<branch>`,
then `sluice submit <target> --head <branch>` again; for a conflict, the
fix merges `<target>` at the revision it was tested on, or the target's
revision where it was not tested, first (flume 112, 203). Never "the
agent working in `<branch>`", since no agent works in a branch sluice
lands on.

### D11. How it is held

- **The statement.** 124 as amended at agentplot/blueprints 51b6316;
  67 and 120 as amended at agentplot/blueprints f9455e7, on the user's
  approval of this design, 2026-11-06.
- **Record 0090**: "A submission by name is its branch's work, records no
  worktree and no session, and is delivered by nothing", with the
  options weighed for D3, D5 and D6.
- **Rule core.50**, under "Branches and landings", cites 1, 51, 54, 67,
  120, 124, C.5, flume 202 and 203, and ADRs 0025, 0050 and 0090.
- **The site**: the commands reference gives `--head` without `--check`
  and the status documents' `notice`; the state reference gives the
  record of a submission by name. The `using-sluice` skill gives the
  submit by name beside the forecast.

## Alternatives Considered

- **An explicit field for a submission by name**, `from.named: true` or
  `from.worktree` empty. The first is a field every reader must learn,
  where two fields already say it; the second leaves 67 and 54 nothing
  to key on, so two branches submitted by name to one target would
  withdraw each other. Rejected for D3.
- **`from.worktree` as the branch and `from.head` absent.** The record of
  a worktree's own submission: every inbox in a worktree on the branch
  would take it. Rejected for D3.
- **An outbox item that the driver answers undelivered.** What the code
  does for a submission with no session today: a `pending` line saying
  the outcome waits for an inbox no one will run. Rejected for D5.
- **Leave ADR 0025's request match as it is.** A worktree checked out on
  a branch sluice lands on would collect a request opened by name until
  the request is posted, and whenever the post fails. Rejected for D6,
  since 124 says no inbox.
- **A mark in the request's body for a submission by name**, read by
  intake into the record. A record field again (above), and a request
  any person may edit. Rejected for D7.
- **Take any branch by name**, pushing one sluice lands on nowhere. A
  branch with no worktree is no session's work (2), and a submit pushes
  only from a worktree's work (1). Rejected for D2.
- **A command of its own, `sluice submit-branch`.** A second spelling of
  what `--head` already names for the forecast. Rejected for D1.

## Risks / Trade-offs

- **Two hands on one bolt.** Until crew's C1, a bolt submitted by crew's
  land worktree and by name at once would have each withdraw the other's
  entry (D4). Flume submits a base place only on its caller's retry, and
  crew's bolts are no base places until C1, so neither submits the
  other's.
- **A worktree on a branch sluice lands on, submitting to a target that
  takes pull requests,** no longer collects that request's outcome by
  its inbox (D6). Its request carries it, as before, and its waiting
  submit or `sluice wait` reports it.
- **An older sluice sharing a branch's records** reads a submission by
  name as a worktree's, by `from.worktree`, in a worktree on the branch,
  and its rounds queue a notice no driver delivers. Hosts sharing a
  branch's records take this sluice together, as for each record form
  before.
- **A failed submission by name stays live** until resubmitted or
  withdrawn (130), as any blamed one does; Flume's retry after a fix
  withdraws it.

## Migration Plan

Nothing held changes form. A submission by name is a record of today's
fields; one written before reads as it was. `sluice status --json` only
gains `notice`. crew's land worktree, Flume's places and the runner's
reads keep working through the release that carries this.

## Release

CI tags a release from each landing whose commits git-cliff finds a
version in (`.github/workflows/ci.yml`, `cliff.toml`). This change lands
as a `feat`, a minor release: v0.71.0 where it lands next after v0.70.0,
otherwise the next minor after the latest tag then. The build's delivery
names the tag, for Flume's task 5.2 to pin.

## Open Questions

None.
