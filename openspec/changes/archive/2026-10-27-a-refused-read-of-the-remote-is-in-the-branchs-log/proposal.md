# Proposal

## Why

When a round cannot read a branch's remote, today the round fails and
only whoever ran it hears why: a waiting submit prints the fetch's
error and exits, and the branch's log says nothing. On 2026-10-26 an
operator read a branch's `log.jsonl` for half an hour while every
round's fetch was refused and found no trace of why the queue stood
still (signal `2026-10-26-madswan-planner-c13058ca/01`). Clause 133
makes such a refusal an event in the branch's log, so whoever reads
the queue after can read its stalls there.

## What Changes

- A round that cannot read the branch's revision, or its records, from
  the remote logs `read-refused` on the branch: the host, the remote,
  what was asked for (the ref fetched) and git's own words for the
  refusal, whole and unchanged. It is logged once per distinct cause
  for as long as the refusal stands on that host, never once per
  round (133).
- When a round on that host reads the remote again, `read-again` is
  logged: the host, the remote, when the refusal began, how long it
  stood and how many rounds it refused (133).
- The host keeps the refusal in its own state while it stands, as it
  keeps what it observed of a lead (73): its causes, its first time
  and its count of rounds. A line the round could not write, because
  the records are on the refusing remote, is written in the host's
  first write to the branch's records after, with the time it
  happened (133).
- A round whose read of the records was answered from what was last
  fetched, because the remote refused it, now forms, starts and lands
  nothing, as a round whose read of the branch was refused already
  does (133).
- The round's error to whoever ran it is unchanged: a waiting submit
  or `sluice wait` still prints why the round failed, sluice's note on
  the refused credentials included. The log stands beside it (133).
- `sluice status <branch>` shows this host's standing refusal above
  the queue, and under `refused` in `--json` (record 0061).
- The state reference page documents both events and where the host
  keeps a standing refusal.

## Out of scope

- Making a refused read or push again on a fresh credential: the
  chore `a-fetch-with-sluices-token-is-never-silently-refused` made a
  read's second try, and the unit
  `a-push-the-credential-stopped-is-made-once-more` makes a push's
  (134). What this change logs is the refusal that stands after the
  read's second try.
- Showing a refusal in `sluice overview`, the board or the herdr
  plugins: they read the log and pass over events they do not know,
  as they pass over `copy-behind`; showing it there is a later change.
- A refused push, a landing's or the records': it is the move refused
  or the write failed, reported as today (24, 45, 134).
- Reads made outside a round, by `sluice status`, `sluice overview` or
  a submit: they say what failed to whoever asked, as today (80).

## Capabilities

### New Capabilities

- `remote-read-log`: how a read of a branch's remote that a round
  could not make is logged on the branch, once per cause and host,
  with its end, and kept on the host until the records can be written.

### Modified Capabilities

- `remote-records`: a round whose read of the branch's state on the
  remote was answered from what was last fetched forms, starts and
  lands nothing.

## Impact

- `sluice-seams`: a typed error for a read of the remote that could
  not be made, carrying the remote, the ref asked for and git's words
  apart from sluice's note; the records seam keeps this host's
  standing refusal per branch, and says when the records' read was
  answered from what was last fetched and why.
- `sluice-core`: two log events; the round notes a refused read in
  the host's state, logs it once per cause, closes it on the next
  round that reads; the one write path carries lines still unwritten;
  status reads this host's standing refusal.
- `sluice` profiles: pushed refs and remote refs raise the typed error
  from every fetch a round makes; local files, remote refs and the
  routing binding keep the host's refusal beside what they keep
  already; the fakes can refuse reads.
- `sluice status` text and `--json`; `site/.../reference/state.md`.
- `sluice/log@1` and `sluice/branch@1` are unchanged: the two events
  are new lines an older sluice passes over, and `branch.json` gains
  nothing.

Rests on clauses 16, 61, 73, 80, 124, 128, 133 and the pushed refs row;
records 0017, 0053 and 0061 (proposed, on the user's card); rules
core.11, core.14 and core.28 (proposed with 0061).

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/lib.rs
crates/sluice-core/src/records.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/ops.rs
crates/sluice-core/src/refused.rs
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/tests/refused.rs
crates/sluice/src/lib.rs
crates/sluice/src/profiles/pushed_refs.rs
crates/sluice/src/profiles/remote_refs.rs
crates/sluice/src/profiles/local_files.rs
crates/sluice/src/profiles/records.rs
crates/sluice/tests/pushed_refs.rs
crates/sluice/tests/remote_branch.rs
crates/sluice/tests/remote_refs.rs
crates/sluice/tests/cli.rs
site/src/content/docs/reference/state.md
site/src/content/docs/reference/commands.md
```
