# Design

## Context

Today `Sluice::submit` (`crates/sluice-core/src/ops.rs`) admits one
head: it reads the worktree's session, refuses a dirty or detached one,
an unmanaged branch or an unknown `--after`, publishes the head (82),
allocates an id, and inserts the submission in one `update` of the
branch's records, a compare-and-set write a refused write retries (62).
Rounds read the records under the branch's lock (`step.rs`) and form
combinations from what the records hold (rule core.6).

On a branch that takes pull requests, `sluice submit` never writes a
submission: `submit_pull` (`crates/sluice/src/lib.rs`) pushes the
branch and opens or updates the request through `PullRequests::open`;
intake (`Pulls::take_in`, `profiles/github/pulls.rs`) later submits
each ready request with its own `s.submit`, one records write per
request, under intake's own file lock, not the branch's.

## Goals / Non-Goals

**Goals:**
- One act, one records write: the group's submissions appear to every
  reader at once (125; rule core.14).
- The pull-request form gives the same guarantee through intake.

**Non-Goals:**
- An atomic landing of the set (proposal's option C, declined: 17, 18).
- Any change to how a round forms, tests, blames or lands; S59 holds
  through the existing chain once the submissions arrive together.

## Decisions

### 1. The group is written in one records write

`Sluice::submit_group(&GroupRequest)` takes the branch, `after`,
`priority` and the worktrees in order. It reads every worktree's session
first and collects every refusal (dirty, detached, a worktree named
twice or two resolving to one `from.worktree`, no branch, unmanaged,
unknown `after`); any refusal returns all of them and writes nothing.
Then it publishes every head (82), allocates one id per worktree in
order, takes the branch's `wait_lock`, and in one `update` closure
withdraws each worktree's earlier submission (67) and inserts every
submission. Because the write is one compare-and-set of the branch's
records, a reader sees none of them or all of them (125, 62), on this
host or, for records on a remote, on any (C.6). The wake to a pool
(111) is sent once, after the write. `submit` becomes `submit_group`
with one worktree, so the two never drift.

Alternative: a "group pending" flag on each submission that rounds skip
until cleared. Rejected: it is a second write and a state rounds must
read, where one write gives the guarantee with nothing to read.

### 2. "After the one before" is order of submission, not an after-entry

Clause 125 says each is its own entry "after the one before (6)". Two
readings: each names the previous as an after-entry, or each simply
follows the previous in order of submission. The first contradicts the
same clause and S59: under 7, an entry whose after-entry is removed as
conflict or culprit, neither landed nor ahead, waits forever, so the
third bolt would never land after the second conflicts. The second
reading gives what 6 promises: ids allocated in the given order, equal
priority, so `order::order` keeps them in that order; `--after` and
`--priority` go on every submission of the group alike, so no member
is let in ahead of an earlier one because only the earlier waits on an
`--after`. A later submission with higher priority may still come
between them (6), which tests and lands them in the same order.

ADR 0048 records this reading and decision 4's.

### 3. The act's answer and exits

Without `--json`, a no-wait act prints one line per submission (`submission <n> to <branch> from <worktree> · <ahead>`). A waiting act waits on
the submissions one after another in the given order through the
existing `wait_on` loop, which runs rounds between reads (rule core.4):
waiting on the first runs the rounds that decide the others too, and a
submission already decided is read at once with no round run. Each
outcome is then reported through the existing `report`, so a conflict
names its files and the submissions it is contested with (65), and a
failure its blamed tasks. The exit is 0 when all landed, otherwise the
exit (`exit_for`) of the first in the given order that did not land;
Ctrl-C stops the wait with 130 and prints the document as it stands.
Under `--json` one document `{"branch", "submissions": [...]}`.

Alternative: exit with the worst outcome. Rejected: no order between
failed and conflict is given; the first in the submitter's order is
the one the submitter reads first.

### 4. Under pull requests: a group mark, and intake takes the group whole

The act checks every worktree as decision 1 does and pushes nothing if
any is refused. It then makes a group token (the act's time and the
first head, short), and for each worktree in order, with `k` its place
and `n` the group's size, writes into the request's body a mark beside
the agent mark, `<!-- sluice: group=<token> <k>/<n> head=<oid> -->`,
before the head reaches the request: an open request's body is edited
first, then the branch pushed; for a new one the branch is pushed, then
the request opened with the mark in its body. So no
request of the group is ever open at the group's head without its mark,
and intake cannot take one alone in between.

Intake reads the marks of the branch's open requests. A marked request
is bound to its group only while its head is the marked `head` and it
has had no submission at that head. Intake submits a bound group only
when all `n` places are present, each open, out of draft, at its marked
head, and meeting its conditions; then it submits them through
`submit_group`, one write, in `k` order. Otherwise each bound request's
state names the group's requests it waits for, and its forecast is
posted as before (64). A plain `sluice submit` of one worktree rewrites
the body without a group mark.

128 says what sluice reads of one request decides that request alone;
a group's request that intake cannot read holds the rest of its group,
because the group was submitted to be taken all or none (125). That is
the act's own condition, set by its submitter, and holds no request
outside the group. ADR 0048 says so.

Alternatives: open every request as a draft and mark them ready
together afterwards. Rejected: marking ready is a GraphQL mutation the
forge seam does not have (`markPullRequestReadyForReview`; the REST API
has none), and a request marked ready one at a time is still taken one
at a time. Or take each request as it becomes ready. Rejected: it
forms a combination on the first alone, which 125 forbids.

### 5. The forge seam does not change

The mark rides on `Forge::open` and `Forge::edit`, both already in the
seam and the fake; no new vendor call is needed.

## Risks / Trade-offs

- [A push fails midway through a pull-request group] → the requests
  already opened carry a group of `n` with fewer present, so intake
  takes none; the answer names what failed, and the same act run again
  replaces every mark with a new token.
- [A group's request closed before the group is taken] → the rest wait
  and say so, by design (all or none); a new submit, as a group or one
  by one, frees them.
- [ADR number] → another unit in flight may also take 0047; whichever
  lands second renumbers.

## Migration Plan

None: a submit with one worktree, and a request with no group mark,
behave as today.
