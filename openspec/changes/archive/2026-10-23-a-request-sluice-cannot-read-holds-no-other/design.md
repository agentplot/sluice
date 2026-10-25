# Design

## Context

See proposal.md for why. The design rests on clause 128, C.5's pull
request row and S63 in `agentplot/blueprints:design/sluice/requirements.md`,
as they stand after 18e4e7f and bab7b59, with 7, 64 and 120, which 128
cites.

Where the code stands (`crates/sluice/src/profiles/github/pulls.rs` on
main):

- `Pulls::take_in` reads the branch's target, takes intake's lock, lists
  the open pull requests, withdraws the submissions of changed ones,
  reads the log and each place's latest submission, and opens the
  pushed refs. Then it loops over the repository's own pull requests,
  calling `consider`, then `verdict_of` and `show_report`, each with
  `?`. The first error from any of them ends the loop and `take_in`.
- `consider` reads, for a sent-back submission, the timeline
  (`forge.readied`); for an out-of-draft request, its reviews and, when
  the policy names checks, its checks; then calls `fetch_head`
  unconditionally, a draft's included, before submitting or forecasting.
- `fetch_head` fails with "pull/<n> has no head on <remote>" when
  `refs/pull/<n>/head` is not served, and with "pull/<n> moved to …"
  when the head fetched is not the one the list gave.
- `intake` in `crates/sluice/src/lib.rs` turns an error from the whole
  pass into one `sluice: warning: taking in <branch>'s pull requests: …`
  line through `telemetry::warn`, and the round goes on landing what is
  queued. That warning line is the round's report this change writes
  to.

## Goals / Non-Goals

**Goals:**

- A failure in one pull request's step is reported by name and ends
  only that step (128).
- A draft never needs its head for anything but its forecast (128, 64).

**Non-Goals:**

- Posting the failure on the unreadable pull request itself.
- Changing what ends a branch's whole intake.

## Decisions

### Each pull request's step is isolated

The body of `take_in`'s loop for one pull request (`consider`,
`verdict_of`, `show_report`) runs as one fallible step. An error from it
is printed as `sluice: warning: taking in <branch>'s pull requests:
pull/<n>: <error with its causes>` through `crate::telemetry::warn`,
the same channel and prefix the whole pass's failure already uses, and
the loop goes on to the next pull request. `take_in` returns `Ok(true)`
after the loop, whatever failed in it.

The steps before the loop stay outside the isolation: the target, the
lock, `forge.pulls`, `s.status`, the withdraw loop, the log,
`latest_of` and `pushed`. Each is about the branch, not a request, and
128 governs "what sluice reads of one request".

Errors from the GitHub calls inside the step get a context naming what
was read (`reading pull/<n>'s reviews`, `… checks`, `… timeline`), so
the warning says what failed and not only the HTTP error.

Alternatives considered: isolating only `fetch_head`. That fixes S63
but leaves a failed reviews or checks read, which 128 names too
("conditions it cannot tell"), stopping the loop. Collecting failures
and returning them through the `PullRequests` trait to `lib.rs`: it
changes the trait for the same line on the same channel, so it is not
worth it.

### A head that cannot be read gives no forecast, and the rest still shows

Inside `consider`, a `fetch_head` failure is caught. When the request
is a draft, or is missing a review or a check, it is shown as waiting on
what it lacks, `waiting(&missing, None)`, so a draft reads `waiting: a
draft` with no forecast line (128's last sentence), and the failure is
still reported. When it lacks nothing, nothing is submitted and nothing
is posted; the failure is reported. A failed reviews, checks or
timeline read is reported and nothing is posted for that request.

So `consider` hands back two things: what to post, if anything, and the
failure to report, if any. The loop posts what it is given and prints
the failure. The exact shape (a small struct, or `Option<Shown>` with
an error) is the code stage's.

The same rule is applied to an out-of-draft request still missing a
condition as to a draft, since both are waiting and neither needs its
head except for the forecast. Posting nothing on a ready request sluice
could not fetch keeps its earlier check, which says no more than the
last round knew.

Alternatives considered: posting a `waiting: sluice could not read its
head` check on it. The design names only the round's report, so this
change does not invent a posting; it is left as a question for the
design (proposal, out of scope).

### Nothing is stored

The failure lives only in the warning. The next round lists the pull
requests again and reads each afresh, so "read again each round until
it can be, or it closes" holds with no record (core.14, ADR 0021's
"nothing of a waiting pull request is stored"). A closed request is no
longer listed, so it is no longer named.

## Risks / Trade-offs

- [A branch-wide failure that shows up inside one request's step, such
  as the records being unwritable during `s.submit`] → each later
  request fails the same way and is named; the round still goes on to
  land what is queued, as it did when the whole pass failed. Nothing is
  half-written: a submission is written whole or not at all.
- [A warning every round for a request whose branch is gone for good]
  → this is what 128 asks: it is named until it closes. Closing it
  stops the warnings.
- [`Pulls::session` calls `fetch_head` again when the core submits] →
  only for a request whose head `consider` just fetched, so it reads
  the local ref and fetches nothing.
