## Why

sluice marks a submission's outcome delivered on the word of whichever
carrier first put it before the session: herdr took the notice into the
pane, the pull request driver posted it, `sluice inbox` printed it, or a
wait reported it. The mark is final. An agent whose session stops after
the notice lands, or after its inbox printed, and before it acts, never
sees the outcome again: every later inbox answers nothing for it. An
outcome nobody collects, the blame of a worktree removed before its
session read it, stays pending in the branch's records with no reader
showing it.

agentplot/weir record 0006, on the user's card of 2026-11-08, gives
Flume and sluice one form for what a tool owes a reader, and weir's
`weir-delivery` crate builds it: a message recorded once under a key
with its readers, owed to each until that reader confirms it naming the
key; a hand-over, by a notice, an inbox or a wait, confirming nothing
by itself unless its reader says in the same call that it confirms on
receipt; an owed message expiring after the host's span and then shown
expired with its reader, never dropped (weir 100 to 113, C.8). sluice
moves onto it here, in the change of its own the record names, and
takes its first dependency on weir for it. The statement's 51 is
amended to cite weir's clauses (agentplot/blueprints 3bb8688, on the
branch `design/sluices-outcomes-are-weirs-deliveries`).

## What Changes

- **An outcome is a delivery.** The round that decides a submission
  landed, blamed or in conflict records one `weir-delivery/1` in the
  branch's records with `weir-delivery`'s `record`: key `<id>:<outcome>`,
  the notification's id today; one reader, the submission's work as it
  recorded it, a worktree's branch or `pull/<n>`; the notice as its
  message; and an expiry by the host's span. A submission by name, and
  a withdrawal, record none.
- **The branch keeps its deliveries.** `deliveries.jsonl` among the
  branch's live records holds the four records of C.8 in JSON, in the
  order added, and passes weir's pack. A done delivery is let go of
  whole once its submission has left the live records; a reader's
  known name stays.
- **Hand-overs confirm nothing.** The round's notice, `sluice inbox`
  and a reporting wait each record a hand-over by its way, `notice`,
  `inbox` or `wait`, on the carrier's word, and leave the outcome owed.
  The pull request driver's post is a hand-over by `pull-request` that
  confirms on receipt. `sluice inbox --confirm` and `sluice wait
  --confirm` confirm on receipt for a caller that says so; `wt submit`
  confirms a landing whose worktree it removes.
- **A reader confirms.** `sluice confirm <key>...` confirms each key for
  the folder's readers; a second confirmation says when the first was;
  a key no branch holds, and one owed to another reader, are refused;
  `--as operator` confirms for the delivery's own reader.
- **Owed until confirmed.** `sluice inbox` hands over every outcome
  still owed each time it runs, each with its key and how to confirm
  it, an expired one saying so; `--new` only what no way has handed its
  reader. The round pushes a notice at most once, and none for an
  outcome already handed to its reader or confirmed. The notice names
  the key and `sluice confirm`.
- **Expired outcomes are shown.** `sluice status <branch>` shows what
  is owed on the branch: each reader, whether any reader has confirmed
  under its name, and each delivery owed or expired with its
  hand-overs. `sluice status <id>`, `sluice wait` and `sluice inbox`
  give a submission's `delivery`.
- **The host sets the span.** `[deliveries] expire_after` in
  `~/.config/sluice/config.toml`, `"168h"` where unset; the flake's
  module declares it.
- **BREAKING:** a submission's record and the inbox's document lose
  `delivered` and `delivered_to`, which sluice no longer writes or
  reads, for `delivery`. The seam's `Delivery::Delivered` becomes a
  hand-over with its way and whether its reader confirmed on receipt.
- The board collects with `sluice inbox` at its first overview and
  `sluice inbox --new` on a new outcome; the session note and the
  using-sluice skill tell an agent to confirm each outcome once it has
  acted on it.

## Capabilities

### New Capabilities

- `outcome-deliveries`: an outcome recorded as a weir delivery in the
  branch's records, its key, reader, message and span; the store
  holding the pack; a done delivery let go of; none for a submission by
  name or a withdrawal.
- `outcome-confirm`: `sluice confirm`, by key, for the folder's readers
  or as the operator.
- `owed-outcomes`: what a branch's status and a submission's documents
  show of what is owed and expired.

### Modified Capabilities

- `outcome-delivery`: the round's push, the pull request's post and a
  wait are hand-overs recorded by their way, confirming nothing but on
  receipt where the call says so; the notice names its key.
- `inbox`: `sluice inbox` hands over what is owed until its reader
  confirms it, with `--confirm` and `--new`; its document gives
  `delivery`.
- `submission-wait`: a wait hands over only its own outcome.
- `submit-by-name`: a submission by name records no delivery; a request
  opened by name is owed to the request.
- `plugin-board`: the board collects with `--new` on a new outcome, and
  a reload repeats only what is still owed.
- `plugin-usage`: the session note has an agent confirm each outcome.
- `host-settings-module`: the module declares `deliveries.expire_after`.

## Out of scope

- **Flume's merge-queue profile.** Flume reads `sluice inbox --json` in
  a place and takes each outcome into the place's own records, so it is
  a reader that confirms on receipt and passes `--confirm`, in Flume's
  change `flumes-inbox-is-weirs-delivery-form`. Until it does, each of
  its settles reads its places' outcomes again; the release that
  carries this change waits on that change (design D11).
- **The records sluice wrote before.** No code reads `delivered`,
  `delivered_to`, `told` or `told_to` into the form or records a
  delivery for an outcome decided before the upgrade (principle 12,
  proposed). An agent moves them, the step design D12 names.
- **The overview.** It shows the queues, not what is owed; an expired
  outcome is shown by the inbox and the branch's status, where weir 104
  asks.
- **weir's form itself.** The records, the calls and the pack are
  weir's, at agentplot/weir 52b9552; a change to them is weir's.

## Impact

- Code: `Cargo.toml`, `Cargo.lock` and `flake.nix` (the weir git
  dependency and its hash); `crates/sluice-seams` (`Delivery`,
  `Driver::notify`); `crates/sluice-core` (the deliveries store, the
  round's record and push, `Pending`, the notice's words, the submission
  record's fields); `crates/sluice-fakes`; `crates/sluice` (`inbox`,
  `confirm`, `status`, the wait's hand-over, `wt submit`, the host's
  settings, the worktrunk and pull request profiles); the conformance
  suite.
- Plugins: `plugins/sluice-board/hooks/register.tsx`,
  `plugins/sluice/scripts/session-context.sh`,
  `plugins/sluice/skills/using-sluice/SKILL.md`.
- Nix: `nix/modules/options.nix` and its tests' files.
- Docs: `site/src/content/docs/reference/commands.md`,
  `reference/state.md`, `reference/configuration.md`,
  `guides/claude-code.md`.
- Records: record 0102; rules core.31 and core.50 amended, core.52
  added, with the site's rules page; the statement's vocabulary, 50,
  51, 124, C.5, S76 and S77 at agentplot/blueprints 3bb8688.
- Sources: agentplot/weir record 0006, `weir-delivery` and `weir-pack`
  at 52b9552; weir 100 to 113, C.7, C.8, I9, S22 to S25; principle 12.

## Touches

```
Cargo.toml
Cargo.lock
flake.nix
crates/sluice-seams/src/lib.rs
crates/sluice-core/Cargo.toml
crates/sluice-core/src/**
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/**
crates/sluice/src/**
crates/sluice/tests/**
plugins/sluice-board/hooks/register.tsx
plugins/sluice-board/hooks/board.ts
plugins/sluice-board/tests/**
plugins/sluice-board/README.md
plugins/sluice-board/.claude-plugin/sluice.json
plugins/sluice/scripts/session-context.sh
plugins/sluice/tests/hooks.sh
plugins/sluice/skills/using-sluice/SKILL.md
nix/modules/options.nix
nix/tests/**
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/state.md
site/src/content/docs/reference/configuration.md
site/src/content/docs/guides/claude-code.md
docs/adr/0102-a-submissions-outcome-is-a-weir-delivery-owed-to-its-work-until-it-confirms-it.md
docs/architecture/core.md
site/src/content/docs/architecture/rules.md
openspec/changes/sluices-inbox-is-weirs-delivery-form/**
```
