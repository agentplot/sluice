# Design

## Context

A round (`sluice-core/src/step.rs`) recovers, decides in one write, then
performs. Decide polls finished runs, arranges the chain (forming each
candidate on `base`), chooses a landing or the starts. The affected
reading exists only inside a run: the moon runner narrows the required
tasks with `moon query tasks --affected` between `since` and the
candidate in the candidate's place (`profiles/moon.rs`). Nothing asks it
outside a run. Record 0075 and clause 140 settle the behaviour; this
design settles how it is built and what the record left open.

## Goals / Non-Goals

**Goals:** clause 140 and S68 as the record decides them; the amended
8, 10, 23 and 24; rule core.39 and the details of core.6 and core.17.

**Non-Goals:** a signer's attested reading; path patterns in the policy;
any change to how a candidate is narrowed in its run.

## Decisions

### D1. The reading is a query of the check seam

`Tests::reads(&Reading) -> Result<Vec<Reads>>`: the required checks,
`[tests]`, and pairs of revisions `(from, to)`; for each pair either the
required checks the change reads (`Reads::Checks`, empty for none) or
`Reads::Every`. The default answers `Every`, so a runner that cannot say
qualifies nothing (the command runner keeps the default). The core
qualifies work only when every pair answers `Checks([])`.

A runner's own settings are the runner's to recognise: a pair whose
change touches them answers `Every`. The moon runner names `.moon/`, any
`moon.yml` and `.prototools` (as `plan` already reads `.moon/` and
`moon.yml` as changing the graph), and the files a toolchain is set up
from at the tree's root: `devenv.nix`, `devenv.yaml`, `devenv.lock`,
`flake.nix`, `flake.lock`, `.envrc`, `.tool-versions`, `mise.toml`,
`rust-toolchain` and `rust-toolchain.toml`. The record is silent on how
"the toolchain the place enters" is named; this list is the choice. The
place profile, a git worktree, has no setting of its own outside the
policy, which the core already excludes.

The core excludes the policy itself by comparing `.config/sluice.toml`
at `from` and `to` of the branch's pair (`Branch::read_file`), so no new
branch method is needed.

### D2. The moon runner reads in a scratch tree, not a place

moon reads its project graph from a working tree. "Takes no place" is
held so: no place seam call, no ports, no release, no run and no room in
the bound. The check profile makes a detached worktree of the first
pair's `to` under its own run directory (`sluice/.runs/reading-<pid>`,
which the scratch sweep of `sluice/.tmp/` never touches), sets up
`[tests] toolchain` there as `kept_out` does, runs `moon query tasks
--affected --downstream deep` with `MOON_BASE`/`MOON_HEAD` for each
pair, and removes the tree. A pair whose `from` differs from the tree's
revision in a setting of the runner's own answers `Every`, since the
graph read would not be that pair's; work that changes none has the same
graph at `from` and `to`. git runs with hooks off and the run's
environment, as a place's checkout does.

### D3. Where in the round

After `poll` and before `arrange`, with `base` read and `verified`: the
round considers each queued entry in no candidate on `base` whose
after-entries have landed, in submission order (priority orders no
landing beside). For each it checks, in order:

1. the policy unchanged and the work formed cleanly onto `base`;
2. merged cleanly onto the work of each entry ahead of it (higher
   priority, or equal priority and submitted earlier), each formed alone
   on `base`: a conflict waits behind that entry, `why: "conflict"`;
3. merged cleanly onto each candidate on `base` as it would land now, in
   chain order: a conflict waits behind its newest entry;
4. one `reads` call over the pair at `base` and the pair at each
   candidate. A check read at `base` gives no `beside`; one read at a
   candidate waits behind its newest entry, `why: "read"`.

A group submit records no `after` between its heads; their order is
their ids. So a submission made in an act of several records `act`, the
id of the act's first head, and a head is read only once every earlier
head of its act still in the records has landed or qualified this
round: the leading heads that qualify land in the act's order, and from
the first that does not, the rest take the chain (125).

An entry that qualifies is given a landing commit (parent the virtual
base, tree its form on it) and stands as landed for the rest of the
loop: the virtual base and each candidate's tree advance by it, so a
group's next head, whose after is this one, is read against them. A
reading that fails is warned and takes the chain.

When any qualifies, the round writes one landing intent marked `beside`
and nothing else: no arrange, no landing of the chain, no starts and no
states derived. The next round forms on the new revision. A round's
records thus hold either a landing beside or the chain's work, never
both.

### D4. The candidate carries the work

`Candidate.carries: Vec<Rev>`, the heads landed beside since it was
formed, in order. Performing a landing beside moves every candidate on
the old revision to the new one (`on`) and appends the heads; `revision`
stays the tested one, so its runs, keys, published refs and results
stand. The chain's landing commits with `tree_of` the candidate's
revision formed with each carried head in turn. Heads are kept rather
than the formed revision so that any instance can form it again: a
landed head is reachable from the branch. Should that form conflict,
which the reading's merge checks rule out, the candidate is dropped as
obsolete and formed again next round, never landed without the work.

A candidate is named by `on` and its submissions (core.17), so moving
`on` names it as the new revision extended by its entries.

### D5. Records and words

- `Submission.beside`: `true` once landed beside, or `{behind, why}`
  while it waits; written once, the round it is first read, and kept.
  Its record is the JSON `submit`, `wait` and `status` print, so the
  field is the JSON the record asks for.
- `Submission.act`: the first head of a group submit's act (D3).
- `LandingIntent.beside`, `Event::Landed.beside`,
  `DecidedRecord::Landed.beside`, `Decided::Landed.beside` and
  `Standing::Landed.beside`, each defaulting to false.
- The landing commit's trailers name no tested combination:
  `Sluice-Result: landed beside the queue (no required check reads its
  changes)`.
- Text: the waiting submit and the notice as the record words them;
  `detail` in status and wait appends the waiting words.
- GitHub: the check's title "landed as <rev> beside the queue", its
  summary and the report's heading "landed on <branch> as <rev> with no
  check run".

## Risks / Trade-offs

- [The scratch tree's toolchain is slow on its first set-up] → asked
  only for entries in no candidate, once, and every pair in one tree.
- [A host that runs rounds lacks the runner] → the reading fails and the
  work takes the chain as before.
- [moon's reading is wrong] → every candidate's narrowing is already
  wrong (15); nothing new is trusted.
