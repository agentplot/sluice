# Design

## Context

A round's decide step (`crates/sluice-core/src/step.rs`, `decide`) reads
the branch through the branch seam and compares it with `base`, the
revision the branch's records last formed on. When they differ it logs
`moved`, sets `base` to the revision read and clears `verified`, and the
chain is formed again on it (10, 24, S8). Nothing asks whether the
revision read holds what sluice landed. In the incident (proposal, Why)
the round read the remote's `10e4791` after landing `c43609b` in local
refs, logged `moved`, and formed #138 on it.

What already exists to build on:

- The branch seam's `contains(tip, rev)`, used today by landing recovery
  and `sluice branch delete`.
- `BranchDoc` in `crates/sluice-core/src/records.rs`, the branch's one
  document, written once per round under the branch's lock (core.14),
  on whichever records profile the branch uses (C.6).
- The `landed` log line, written when a landing is confirmed, and
  `Line::read_log`.
- `sluice branch create` and `delete` (`crates/sluice-core/src/branches.rs`,
  `crates/sluice/src/branch.rs`) with exits 0, 1, 2 (C.5 branch).
- Status and the overview read `base` and never fetch (ADR 0046).

## Goals / Non-Goals

**Goals:**

- 121, its C.4 pushed-refs row and S57, for both branch profiles: the
  check sits in the core, above the seam, so local refs and pushed refs
  hold a branch the same way (core.2).
- The hold is in the records, so every reader on every host sees it with
  no sluice running (26; core.11), and nothing about it lives in a
  round's memory (core.14).

**Non-Goals:**

- The host's copy of a remote branch, which 124 says sluice reads
  nothing from, and the hook guarding it (S58): another change. This
  one reads the branch where the round reads it today.
- Telling a submission's session of the hold. The session's submission
  stays queued; the hold is the branch's, said to whoever runs or reads
  it.

## Decisions

### D1. The branch's records keep one revision every read must contain

`BranchDoc` gains `landed: Option<Rev>`: the revision every read of the
branch must contain. A confirmed landing sets it to the landing's `to`,
in the same write that logs `landed`; `sluice branch accept` sets it to
the accepted revision; `sluice branch create` sets it to the revision it
made the branch at (121). When a document has no `landed`, the round
takes it once and writes it: the last `landed` line's `to` when `base`
is absent or contains it, and otherwise `base`.

The second case is a branch removed and made again before this change.
Neither `make_branch` nor `remove_branch` logs anything and the records
stay on removal, so the log's last landing may be the removed branch's,
which 121 says binds no reading of the new one. `base` is the revision
the round last formed on; on a branch made again it is the new branch's
revision or later, and lacks the old landing unless the new branch was
made from where the old one went. A branch whose `base` already lacks
its last landing when this change arrives was formed on that way by an
earlier sluice; it stands on what it was formed on, and 121 binds every
read after it.

*Alternatives.* Scanning the log every round: a busy branch's log is
hundreds of kilobytes, read on every round for one revision. Reading the
submissions' `landed_as`: a landing of several submissions is one move,
and finished submissions move out to `.finished/`, so the branch's own
document is the one place that says it in one field.

### D2. A hold is a field of the branch's document and one log line

`BranchDoc` gains `held: Option<Held>`, `Held { read, lacks, since }`.
The log gains `held { read, lacks }` and `accepted { revision, lacked }`
events. In `decide`, after the read and before `base` is touched:

1. If `landed` is set and `contains(read, landed)` is false: when `held`
   is absent or names another `read`, set it and log `held`; write the
   document and return `Step::Held`. `base`, `verified`, the candidates
   and the window are left as they are: nothing is formed, polled,
   started or landed.
2. Otherwise clear `held` and go on as today, the `moved` line and the
   re-forming included.

`base` is kept at the last revision formed on, so a hold that ends with
the landing back, the read equal to `base` again, keeps every candidate
and its runs (17, 10): S8's re-forming happens only when the branch
really moved on.

The round's other work before and after decide is unchanged: places of
abandoned sessions and unusable previews are released, an earlier
round's outbox is told, and a landing intent left unfinished is settled
by its own leased read, which already refuses a branch found elsewhere
(24).

*Alternatives.* Holding by `bail!` in `decide`, as a missing branch
does: the round would end in an error each time, write nothing, and
`sluice run` would log it every pass; the hold would not be in the
records. Moving `base` to the read and forming nothing: an ended hold
would then re-form every candidate even when the landing came back.

### D3. Runs already under test are left alone

A held round neither polls nor releases. A run under test goes on in its
place; its result is read by the first round after the hold ends. If the
hold ends by the landing coming back, the result counts as it would
have. If it ends by acceptance, the round sees `base` differ from the
read, logs `moved`, and the candidates are formed again (S8), the run's
place released as for any re-formed candidate.

*Alternative.* Ending the runs at once: their results would be lost in
the common case, a remote put back within minutes.

### D4. `sluice branch accept <branch> <revision>`

In `crates/sluice-core/src/branches.rs`, beside `make_branch` and
`remove_branch`: take the branch's lock; read the branch through the
seam; refuse with exit 1 when absent; refuse with exit 2 when the
records hold no hold, or when the read or the hold's `read` is not the
revision given (a prefix of at least seven hex digits resolves as git
does). Otherwise set `landed` to the revision, clear `held`, log
`accepted { revision, lacked }`, and write. It moves no ref and touches
no candidate; the next round does the rest through D2's step 2.

This is C.5's branch row: `sluice branch accept <branch> <revision>`
records the revision the branch stands on when its reading and its
records disagree, the operator's act and no session's, exiting 0, 1 and
2 as `create` and `delete` do. 121 makes it the landing the branch then
stands on, which is why it sets `landed`. It is refused when the branch
is not held because then reading and records agree, and refused for any
revision but the one held at because the act names what the operator
saw.

*Alternatives.* A flag on `sluice step`: a round may be run by any
process, a waiting driver included (core.4), and acceptance is the
operator's alone. A setting in the policy: the policy is read from the
branch's revision, the very thing in dispute (core.15).

### D5. Making a branch sets the revision it stands on; removing it clears the hold

121: a branch sluice makes stands on the revision it was made at until
its first landing, and one removed and made again owes nothing to the
removed one's landings. `make_branch` writes `landed` as the made
revision and clears `held`, in one update after the move succeeds. A
make that is refused writes nothing. `remove_branch` clears `held` in
the update after its move succeeds, so no reader sees a hold on a
branch that is gone.

### D6. How the hold is said

- `Step::Held`; `sluice step`'s text says
  `held: <branch> reads <read>, which lacks the landing <lacks>; put it back on the branch, or run sluice branch accept <branch> <read>`;
  under `--json`, `"step": "held"` and `held: { read, lacks }`.
- `sluice run` prints the same line once per hold, not every pass.
- `sluice status <branch>`: the line above the queue; `--json` gains
  `held`. `sluice overview`: the branch's view gains `held`, and its
  text the same line.
- A waiting `sluice submit` or `sluice wait`: prints the line once when
  it first sees the branch held, the way it prints the progress line
  when it changes; its `--json` record is the submission's and does not
  change.

User words throughout: branch, landing, revision; no "anchor", "base"
or "candidate" reaches the output.

## Risks / Trade-offs

- [A force-push that drops a landing on purpose now stops the queue] →
  That is 121's intent; the hold names the command that accepts it.
- [A remote rewritten while a long run is under test leaves its place
  held through the hold] → The place is released when the hold ends,
  as for any re-formed candidate; the operator sees the hold in status.
- [`contains` asked of a landing whose commit this host does not have,
  as when another host landed it and the remote has since lost it] →
  The round reads the branch after fetching it, which brings every
  commit the read contains; a landing missing here is therefore not in
  the read, and the check answers "not contained" rather than failing.
  The local-refs profile's `contains` returns `false` for an unknown
  commit instead of an error.
- [A branch whose records moved to its remote takes another host's
  `landed`] → That is the branch's record of its landings; a read
  lacking it is exactly 121's case.

## Migration Plan

Additive to the records: `landed` and `held` default to absent, and an
older sluice reading them ignores unknown fields (`sluice/branch@1` is
unchanged). D1 fills `landed` from the log on the first round. Rolling
back leaves the two fields in place, unread.

## Open Questions

None that change what is built.
