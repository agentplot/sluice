# Design

## Context

See proposal.md for why. The blueprint is
`agentplot/blueprints:design/sluice/proposals/previews-1.md`, clauses 98
to 103. What exists:

- `profiles::places::Places` keeps a record per place at
  `<common>/sluice/.places/<flat name>.json` (`Made{kind, branch, ports,
  path, caches, block, last_run}`), with `Kind::Made` for a candidate's
  place and `Kind::Session` for an adopted worktree. `materialize` adds a
  detached worktree, restores the declared caches and holds a block of
  ports through `hold_block` (ADR 0016); `release` ends the runs, runs
  `[tests] release`, removes the worktree and frees the block.
- A run is a `RunRequest` started by `Checks` (`Tests::start`) and held by
  `sluice supervise <dir>` under `.runs/`, which writes its outcome;
  `sluice exec -- <command>` already runs an argv with `record: false`
  under the executor and limits (ADR 0018).
- Session runs count against the host's bound from
  `$XDG_STATE_HOME/sluice/sessions/` (`Sessions::begin/end`,
  `Tests::sessions`), started under the branch's lock and the host's
  sessions lock, never into `[runs] reserved`.
- `Sluice::release_abandoned` (ADR 0020) runs before every round and
  `exec`, releasing session places whose worktree is gone, taking each
  branch's lock without waiting (core.14).
- `Branches::contains(tip, rev)` answers whether a revision is in a
  branch's history; `Sluice::policy_at(rev)` reads the policy at any
  revision.

## Goals / Non-Goals

**Goals:**

- One place record, one block of ports, one release path: a preview is a
  third `Kind` of the existing place, not a new seam (core.2, core.3).
- No process holds a preview's state: its record, its run directory and
  the supervisor's outcome are the whole of it (core.11, core.14).
- sluice never names the proxy: names are labels, addresses are
  templates the policy writes (core.1).

**Non-Goals:**

- A cap on previews per host, a lease, or a lapse: the caller ends them.
- Restarting a preview's stack if it dies after `up`: sluice does not
  watch what it handed off; the caller releases and asks again.
- Previews in a container place or on another host.
- Listing previews in `sluice status`: `sluice previews` is their list.

## Decisions

### D1. A preview is a third kind of place record, keyed by revision

`Kind` gains `Preview`. Its record is
`<common>/sluice/.previews/<rev12>.json`, named `preview/<rev12>`, beside
`.places/` rather than in it, holding the
branch it was first asked through, the full revision, the checkout path,
the block, the ports, the names (D3), the filled addresses, the time it
was made, its run directory once started, and the release request
saved at creation (D4). The key is the revision alone: a landing commit
on a bolt is later in main's history too, and one revision has one
preview (100). `Checkout` gains `preview(name, rev, branch, checkout,
names) -> Place`, `preview_of(rev) -> Option<PreviewPlace>`,
`previews() -> Vec<PreviewPlace>` and `release_preview(name)`; the fakes
implement them.

Why: the record, block, caches and release already hang off a place
record; a preview differs only in who releases it. Why a directory of
its own: a binary that predates previews parses every record in
`.places/` and fails on a kind it does not know, which would stop its
rounds; in `.previews/` it never sees one. Rejected: a new
`Previews` profile with its own blocks of ports, which would need its
own collision rules and an ADR under core.3.

### D2. Only a landed revision; the branch is read fresh

`sluice preview <branch> <revision>` resolves the revision, reads the
branch (fetching it first for a branch on a remote, as a round does,
C.4) and refuses unless `Branches::contains(branch tip, rev)`. The core
does this in `Sluice::preview`, before any record is written.

### D3. Names are built by the place profile and checked unique

For each name in `[checkout] ports`, in order: `<port>-<rev7>--<branch
slug>--<repository slug>`, each slug lowercased to `[a-z0-9-]` with
runs of hyphens folded. Over 63 characters, the tail after
`<port>-<rev7>--` is cut so that `-<hash6>` of the whole fits, with
trailing hyphens dropped. The repository slug comes from the common
directory as `devurl` takes it (`<project>.git` names the project;
`.bare` or `.git` names its parent). Before the record is written, the
name is checked against every live record in `.places/` and
`.previews/`; a clash
lengthens the revision to 12 characters. Names enter the place's
environment as `SLUICE_NAME_<NAME>` beside `SLUICE_PORT_<NAME>`, and
`SLUICE_BRANCH` and `SLUICE_REVISION` are set.

Why the profile: names, like blocks of ports, are a fact about places
alive on the host. Why one label: a wildcard certificate and a
`*.localhost` redirect each cover one.

### D4. Which policy is read where

From the previewed revision (`policy_at(rev)`): `[preview] command`,
`[preview] addresses`, `[checkout] ports`, `[[checkout.cache]]` and
`[tests] release`, because they describe that revision's stack and its
teardown. From the branch's current revision: `[executor]`, the limits
and the time limit (`[preview] timeout` if the revision's table sets
one, else the branch's), because what a run may use and reach is the
branch's trust now (27, 56; core.15). The release request is built once
at creation and saved on the record, so a release never reads a policy
and works when the branch is gone.

### D5. Policy shape

```toml
[preview]
command   = ["moon", "run", "app:preview"]
addresses = { web = "https://{name.web}.localhost", api = "https://{name.api}.localhost/docs" }
timeout   = "10m"   # optional
```

`command` is an argv as `[tests] release` is. Each address may name
`{name.<port>}` and `{port.<port>}` only for ports `[checkout] ports`
lists; anything else is refused when the policy is read, naming the
address and the port.

### D6. Asking: make, wait for room, start, return

`Sluice::preview` under the branch's lock (core.14):

1. `preview_of(rev)`: a record whose run has started is returned as it
   is (100). A record with no run (an asker died while waiting) is taken
   up with its names and ports.
2. Otherwise `Checkout::preview` writes the record first, then makes the
   checkout, restores caches, holds the block and builds names.
3. The wait for room is `exec`'s: the host's sessions lock, the bound
   less the reserve, less candidates under test and live session runs;
   while full it prints "waiting: the host is at its bound" and pauses
   3 s, and Ctrl-C leaves the record for the next asking.
4. With room, `Tests::start` with the command argv, `record: false`, the
   executor and limits of D4; then `Sessions::begin` with the preview's
   checkout path, so the command counts while its supervisor runs
   (102). The supervisor's end drops the count as for any session run.
5. Print name, state, ports and addresses, or one JSON document with
   `--json`. `--wait` polls the run directory until the supervisor has
   written its outcome.

State is read, never kept: no run directory is `waiting`; a supervised
run is `starting`; outcome 0 is `up`; anything else, a time limit
included, is `failed`.

Why block the asker for room: there is no daemon to start the command
later (ADR 0010), and a waiting `sluice preview` is the same shape as a
waiting `wt submit` or `sluice exec`.

### D7. Release, and the sweep

`sluice release <preview>` calls `Checkout::release_preview`: ends the
run's process group, runs the saved release request under the
executor in the checkout, `git worktree remove`, frees the block, and
removes the record. No record: "nothing to release", exit 0.

`Sluice::release_unusable_previews` runs beside `release_abandoned`, at
the start of every round, `sluice exec`, `sluice preview` and `sluice
previews`. For each preview whose outcome is failed, or whose checkout
directory is gone, it takes the branch's lock without waiting, releases
as above, and rewrites the record as `failed` with the command's output
tail and no checkout, block or run; such a record holds nothing and is
listed until `sluice release <preview>` removes it. It logs `released`
in the branch's log, as a candidate's release is.

Why keep the failed record: a caller polling `sluice previews` must see
why its preview never came up.

### D8. Commands and surface words

`Command::Preview{branch, revision, --wait, --json}`,
`Command::Previews{--json}`, and `Command::Release` gains an optional
positional `preview` (with it, a preview; without, `--worktree` as
today). The site and CLI say preview, checkout, ports, names,
addresses; never place, candidate or line.

### D9. Rules and the ADR

core.16 becomes: "The core releases only places it made; a session's
place is released only by `sluice release` at its session's or
driver's asking, and a preview only by `sluice release` at its caller's
asking or when its checkout is gone or its command failed." core.10's
"Not ruled out" gains "a preview's command handing off a stack that
stays until the preview is released (previews-1 clause 98)". A new
core.19: "A preview is made only at a revision its branch has landed,
and decides nothing: its command is never recorded as a run, read by a
gate, or waited on by a submission." ADR 0021 records the decision with
the options: a kept bolt worktree, a separate tool, a preview as a
place.

## Risks / Trade-offs

- [A caller never releases a preview] → its stack runs until someone
  does; `sluice previews` shows it with its age; the site guide tells a
  caller to reconcile against its own state.
- [A preview's stack dies after `up`] → still listed `up`: sluice does
  not watch what it handed off. Documented; the caller releases and
  asks again.
- [The preview command is killed at the time limit while its stack is
  half up] → read as failed and released, so the release command must
  be safe on a half-started stack, as ADR 0016 already requires.
- [A name clash after hashing] → checked against live records; the
  longer revision makes it unique.
- [Executor from the branch, command from the revision] → a landed
  revision's command may need something the branch's executor now
  forbids; it fails, says so, and is released.

## Migration Plan

New files only: `.previews/` records and their run directories, which an
older sluice never reads, and blocks of ports under the host's
`ports/` that name a `.previews/` record as holder; an older binary
treats such a block as held while its record exists, as it does any
block. No rollback beyond `sluice release` of each preview.
