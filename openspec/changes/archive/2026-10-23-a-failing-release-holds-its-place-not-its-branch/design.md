# Design

## Context

See proposal.md for why. The ruling is clause 123 of
`agentplot/blueprints:design/sluice/requirements.md` (at c872766), with
25 and 34; 81, as amended there, and 122 bear on it, and C.7 names
`release_retries`. What main does today:

- A combination's checkouts to release are listed in the branch's
  records, `BranchDoc.release`. `Sluice::perform_releases`
  (`crates/sluice-core/src/step.rs`) calls `Checkout::release` for each
  and returns at the first error with `?`. `perform` runs it before
  `perform_outbox` and `perform_starts`, and `recover` runs `perform`
  before `decide`, so one release that fails stops every later round of
  the branch before it decides or starts anything. That is swancloud's
  `sluice/try/main/48`.
- `recover`'s release of a checkout left in `Starting`, and `execute`'s
  release of a returning host's leftover checkouts (81), also return at
  the first error.
- `Places::release` (`crates/sluice/src/profiles/places.rs`) ends the
  place's run through `Runs::remove`, which runs the release command
  stored with the run (ADR 0016), then removes the worktree, scratch,
  block of ports and record. A failure anywhere leaves the record, so
  the next round tries the same thing again.
- `Sluice::release_abandoned` (`session.rs`) releases a worktree's place
  whose directory is gone through `release_session`, which builds the
  release command from the policy, running the `[tests] toolchain` in
  the place's path first (`checks::release_request`). For a removed
  worktree that spawn fails ("devenv print-dev-env: No such file or
  directory"), the record stays, and every round on the host tries it
  again and prints the failure: the vanished place on deneb.
- `release_unusable_previews` (`preview.rs`) releases a preview whose
  checkout is gone through `release_preview`, which runs its release
  command, falling back to its scratch directory for a working
  directory.
- `sluice release [preview] [--worktree]` releases a preview by name or
  revision, or the place of the worktree given; a combination's checkout
  is neither, so it answers "nothing to release".

## Goals / Non-Goals

**Goals:**

- No release, failed, retried or held, ever stops a round from
  deciding, landing, telling or starting.
- One path in the core for a combination's checkout's release, used by
  the round's releases, `recover` and a returning host's 81 release, so
  the count, the hold and the log are the same wherever it fails.
- A checkout whose working copy is gone has its stored release run at
  most once, from what sluice holds (its last run's stored command,
  environment and ports, in its scratch directory), enters no toolchain
  and reads no policy, by any path; then it is freed, whatever happened.
- A held checkout stops nothing: not its branch's round, not its host's
  claims (81).
- The operator can end any held checkout on the host by a name sluice
  printed.

**Non-Goals:**

- Releasing a checkout from a host that did not make it.
- Changing which release command a combination's checkout runs: still
  the one stored with its run (ADR 0016). Swancloud's stale command is
  answered by the hold and the operator's release, not by reading the
  policy again.

## Decisions

### D1. The seam frees, and says whether a working copy is gone

Two methods on `Checkout` (`crates/sluice-seams/src/lib.rs`):

- `gone(name) -> Result<bool>`: the place has a record naming a working
  copy, and that path is no directory. A place with no record, or one
  recorded before its checkout was made, is not gone: `release` already
  handles it. Existence is the profile's to judge (I/O), the decision
  the core's (rule core.2), as `session_places` and `previews` already
  report `gone`.
- `free(name) -> Result<()>`: everything `release` does but run the
  release command or save caches: the place's runs' process groups
  ended and their directories removed, the checkout sluice made removed
  (never a worktree's own: rule core.16, 25), its scratch directory, its
  block of ports and its record let go. For a preview it also lets the
  names go and removes the record, keeping no `failed` entry.

- `release_gone(name) -> Result<GoneRelease>`: the release of a place
  whose working copy is gone (123). `GoneRelease { ran: bool, failed:
  Option<String> }`. The profile notes on the place's record that its
  gone release has started, then runs the release command its last run
  stored, once, with the environment stored with it (the place's ports
  and the toolchain's captured environment, `toolchain.sh` in the run's
  directory, among it), with the place's scratch directory as its
  working directory; then it does what `free` does, whatever the command
  did. `ran` is false when no run stored a release command. A place
  whose record already notes a started gone release is freed with
  nothing run, so a free that failed part way and is tried again never
  runs the release a second time. Only a failure to free (I/O) is an
  error; the command's failure is `failed`, with its output's tail.

The stored release, not the policy's, is the one sluice holds: building
it afresh (`checks::release_request`) runs the `[tests] toolchain` in
the place's path, which is what failed on deneb, and reads a policy the
place never ran under.

In the places profile, `Runs` gains `discard(name)` and
`discard_all(place)`: `remove` and `remove_all_with` without the
`release` call; and `release_gone(name, cwd)`: the stored release of the
place's last run that has one, run as `release` runs it but in `cwd`,
its output kept in the run's `release.log`. `remove_worktree` already succeeds on a registered
worktree whose directory is gone: `git worktree remove --force` on a
missing directory exits 0 and drops the registration (checked against
git 2.54 on deneb), so `free` needs no `git worktree prune`, which
would touch other worktrees' registrations (rule core.20).

Alternative rejected: a flag on `release` saying "skip the command".
The core then decides inside a call the profile runs, and the
operator's accept and the gone path read as a release that "worked".

### D2. The branch's state keeps each failed release

`BranchDoc` gains `failed_releases: Vec<FailedRelease>` (serde default,
skipped when empty, so older records read and older sluices ignore it):

```text
FailedRelease { place, failed, tries, at, stopped, host? }
```

`failed` is the error's text, the release command's tail included
(`Runs::release` already puts it in the error); `tries` counts every
attempt, the first included; `stopped` is set once the retries are
spent; `host` is the instance that made it, on a branch whose records
are on its remote. The word `held` is taken by 117's `Candidate.held`,
so the records say `failed_releases` and `stopped`; the user's words in
output are "held".

A place stays in `release` while it is being tried; when its retries
are spent it leaves `release` and stays in `failed_releases` with
`stopped`. Records written before this change, with a place in
`release` that fails forever (swancloud's main today), start at no
tries and reach the hold on their own.

The log gains `release_failed { place, failed, tries, held }` (one per
failed try; `held` true on the last) and `gone { place, worktree?, ran,
failed? }`, `ran` saying whether a stored release ran and `failed` what
it printed when it failed;
`released` gains `accepted: Option<String>`, the failure the operator
accepted.

### D3. One release path for a combination's checkout

`Sluice::release_made(branch, place, passed) -> Result<ReleaseOutcome>`
in the core, called under the branch's lock:

1. `checkout.gone(place)`: `release_gone` it, drop it from `release`,
   `passed` and `failed_releases`, log `gone` with how its release
   went. Done, whatever the release did; it is never tried again.
2. Otherwise `checkout.release(place, passed)`. On success drop it from
   all three lists and log `released`.
3. On error, add a try in `failed_releases`, log `release_failed`;
   when `tries > release_retries`, drop it from `release`, set
   `stopped`, and report it held.

Only a failure to write the records is returned as an error; a failed
release is an outcome. `perform_releases` loops over `release` with it,
skipping nothing but a stopped place; `recover` opens the stale run
whatever its checkout's release did; `execute` (81) runs `release_made`
for each place it left that is not stopped, and then claims up to its
bound whatever those releases did: a place is either released, gone or
held once its release has been tried, and a held place bars no claim
(81). Only a failure to write the records stops the host's claims for
that round. `Round` gains `releases: Vec<ReleaseOutcome>` (failed and
held this round), for the CLI to print.

`release_retries` is a branch rule (`Rules.release_retries: u32`,
`RulesForm.release_retries: Option<i64>`, negative refused), read from
the branch's current revision by `rules_at` (56, rule core.15),
default `RELEASE_RETRIES = 3`, beside `NOT_RUN_BOUND`.

Alternative rejected: counting tries in the place's own record on the
host. A held checkout must be readable from every host that submits to
the branch (26, rule core.11), and the round already writes the
branch's records for each release.

### D4. A gone working copy, everywhere

- `release_abandoned` calls `checkout.release_gone(name)` for a gone
  worktree's place, never `release_policy` or `release_session`, and
  logs `gone` with the worktree and how its release went. `Abandoned`
  carries `freed: bool`, `ran: bool`, `failed: Option<String>` (the
  release's failure) and `error` (a failed free: the profile's I/O
  failing, warned and tried at the next run, which runs nothing, D1).
- `release_unusable_previews` calls `release_gone` for a gone preview
  and logs `gone`.
- `release_made` checks `gone` first (D3).
- `Runs::release` keeps refusing to start a command whose working copy
  is gone, naming the directory: the only path that runs a gone place's
  release is `Runs::release_gone`, given the scratch directory, so a
  path D3 or D4 missed fails loudly rather than running in a fallback.

### D5. A preview's and a worktree's failed release, on their records

A preview and a worktree's place are the host's alone (101; ADR 0035),
so their failure stays on their record in the places profile: `Made`
gains `release_failed: Option<ReleaseFailure>`, a seams type holding
`failed`, `tries`, `at` and `stopped` as `FailedRelease` does, written by the profile when `release_preview` or
`release_session` fails and the record still exists, and shown by
`PreviewPlace.release_failed` and `SessionPlace.release_failed`.
`release_unusable_previews` tries a failed-command preview's release
again under its branch's `release_retries` and stops when they are
spent; no round tries a worktree's place again (25, rule core.16).

### D6. `sluice release <name>` and `--accept-failure`

The positional argument becomes `place: Option<String>`:

- `preview/…` or a revision: a preview, as now.
- `sluice/try/<branch>/<n>`: a combination's checkout on this host. The
  CLI takes the branch's lock waiting (`wait_lock`), then
  `Sluice::release_held(branch, place, accept)`: with `accept`, `free`
  and log `released` with `accepted` the last failure; without it,
  `release_made` once, ignoring `stopped` and the retry count. A place
  not in `made()` and not in `failed_releases` is "nothing to release".
- No argument: the worktree's place, as now; with `--accept-failure`,
  `free` in place of `release_session`, the worktree left.

A release that fails exits 1 and prints what failed; `--json` prints
`{"place", "released", "accepted", "failed"}`. `--accept-failure` on a
place whose release never failed is allowed: the operator's word is the
ruling (123).

### D7. Saying it

- Each try that fails: "could not release <name> (try 2 of 4): <what
  failed>".
- The try that spends the retries: "<name> is held: its release failed
  4 times: <what failed>. `sluice release <name>` runs it again;
  `sluice release <name> --accept-failure` frees it without".
- A gone working copy: "freed <worktree>: it no longer exists; its
  release ran once from its scratch directory" (or "…its release failed
  once from its scratch directory: <what failed>", or "…it had no
  release to run"), then "its runs ended, its ports free".
- `sluice status <branch>`: a "held checkouts" block, one line each
  (name, tries, what failed, since) and the two commands; `--json` adds
  `held: [{place, failed, tries, at, stopped}]`. `sluice status` with no
  argument marks a worktree whose release failed, with what failed.
  `sluice previews` shows a held preview's failure.

`step`, `run` and a waiting `submit` print these through the round's
`releases`, `released` and `previews`, as `tell_abandoned` does now; a
stopped place prints nothing on later rounds.

### D8. Rules and ADR

ADR 0047, "A failing release holds its checkout, not its branch",
records D1 to D4 and the amended 81. Rule core.10's Details gain under
"Not ruled out": a place whose working copy is gone, its stored release
run once in its scratch directory and the place freed whatever it did;
and a place freed at the operator's accepting its failure, with nothing
run (123). core.16's "freeing a session's place whose worktree no longer
exists" says its stored release runs once first, no policy read and no
toolchain entered. core.23's rule says "until each such place is
released or held", and its Details gain under "Not ruled out": a claim
taken while a place the host made is held, named with what failed (81,
123). Each cites 123 and ADR 0047, in `docs/architecture/core.md` and
the site's `architecture/rules.md`.

## Risks / Trade-offs

- [A gone checkout's release written for its working copy] → a release
  command that reads files from its tree finds none in the scratch
  directory and fails; 123 rules it: run once, recorded in `gone` with
  what failed, freed anyway. A release that tears down a stack by name
  or port (containers) still works, which is why it runs.
- [A gone checkout's release that hangs] → held to its run's time limit,
  as every release is, then its group ended.
- [Accepting a failure leaks what the release would have removed] →
  the operator's choice, logged with what failed.
- [A place in `release` from an older sluice and a newer one both
  writing] → the versioned update (62) refuses the stale write; a
  release run twice is safe (ADR 0016).
- [A returning host claims with a held checkout] → 81 as amended: the
  checkout keeps its ports and disk until it is released, so the host's
  claims run in other places beside it.

## Migration Plan

None beyond serde defaults: `failed_releases`, `release_failed`,
`accepted` and the new log events default empty, and an older sluice
reading a branch file with `failed_releases` ignores the unknown field
(`BranchDoc` has no `deny_unknown_fields`). Deneb's vanished worktree
place on swancloud has its stored release run once, and is freed, by the
first round after upgrading. Swancloud's `sluice/try/main/48` was
cleared by hand on the user's word; a place like it is tried and held.
