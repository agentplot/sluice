# Proposal

## Why

When a checkout's release fails, sluice today either stops the branch or
repeats the failure forever. On swancloud's main, a combination's
checkout (`sluice/try/main/48`) kept a release command the branch had
since changed; the release failed every round, and because the round
gives up at the first failed release, main's queue tested nothing, and
no command could clear it (`sluice release --worktree` on it answered
"nothing to release"). On 2026-10-23 a worktree removed by hand,
`swancloud/places/every-session-starts-in-bypass-whatever-file-claude-reads`,
had its release attempted on every round of an unrelated submit (#146,
from `box-op-token` on deneb): sluice built the release afresh from the
policy, entering the `[tests] toolchain` in a directory that no longer
exists, and printed the same failure each time.

Clause 123 of `design/sluice/requirements.md` in agentplot/blueprints
(at c872766, with 25 and 34) rules both: a failed release leaves the
checkout held and named with what failed, tried again as many times as
the branch's `release_retries` allows (three where it states none, C.7),
then reported; a held checkout never holds its branch; the operator
releases it by name, or accepts its failure. A checkout whose working
copy is already gone is no one's: sluice runs its release once, from
what it itself holds for the checkout and with the checkout's ports,
records how that went, frees what it holds whatever the outcome, records
the checkout gone, and never tries it again. Clause 81, as amended in
the same commit, says a held checkout bars no claim of its host.

## What Changes

- A combination's checkout whose release fails stays held, and its
  branch's state names it with what failed and how many times it was
  tried. The next rounds try it again, as many times as the branch's
  `release_retries` allows (3 when the policy names none), then stop and
  report it held. The round goes on whatever a release does: it lands,
  tells outcomes and starts combinations in other checkouts (123, 25).
- The same holds for a combination's checkout left from a run that never
  started, and for one a returning host releases before it claims. A
  host whose leftover checkout's release fails holds it and claims runs
  as usual (81).
- A failed release of a preview, or of a worktree at its own `sluice
  release`, leaves it held and named with what failed, listed by
  `sluice previews` and `sluice status`. sluice tries a preview's release
  again as a combination's is; a worktree's place it never releases on
  its own while the worktree exists (25, rule core.16).
- `sluice release <name>` releases a held checkout by its name
  (`sluice/try/main/48`), as it already releases a preview, running the
  release once more. `--accept-failure` frees what sluice itself holds
  for it, without running the release command: its runs ended, the
  checkout sluice made removed, its scratch directory, ports and record
  let go. In a worktree, `sluice release --accept-failure` does the same
  for the worktree's place and leaves the worktree.
- A checkout whose working copy is already gone (a worktree removed by
  hand, a combination's or a preview's checkout deleted) is released
  once by the next run or round: the release command its last run
  stored, with the environment and ports stored with it, run in the
  checkout's scratch directory. No toolchain is entered and no policy is
  read for it. Whatever that release does, sluice then frees what it
  holds: runs ended, scratch, ports and record let go. The branch's log
  records it `gone` with how the release went, and it is never tried
  again. A checkout whose runs stored no release command is freed with
  nothing run.
- `sluice status <branch>` and `--json` list the branch's held
  checkouts with what failed, how many tries, and how to release one.
- Rules core.10 and core.16 say how a checkout whose working copy is
  gone is released and freed, and that an accepted failure frees with
  nothing run; core.23 says a held checkout bars no claim. ADR 0047
  records why.
- Docs: the commands page (`sluice release <name>`, `--accept-failure`,
  `sluice status`), the configuration page (`release_retries`), the
  state page (the branch's failed releases, the `release_failed` and
  `gone` events).

Out of scope:

- Releasing a checkout held on another host from this one: a checkout is
  released on the host that made it.
- Revising the unarchived `held-worktrees` change's scenario that
  releases a removed worktree with the policy's release command built
  afresh; it waits on that change's archive, which should take this
  change's requirement instead.
- The overview and herdr's views showing held checkouts; `sluice status`
  is where they are listed.

## Capabilities

### New Capabilities

- `place-release`: how a checkout sluice made or adopted is released:
  what a failed release leaves, how often it is tried again, that it
  never stops its branch or its host's claims, how the operator releases
  or gives up on it by name, and how a checkout whose working copy is
  gone is released once and freed.

### Modified Capabilities

- `preview-release`: a preview whose checkout no longer exists has its
  stored release run once and is freed whatever it did, never tried
  again.

## Impact

- The place seam (`sluice-seams`): `Checkout::gone`, `Checkout::free`
  and `Checkout::release_gone`; the places profile runs a gone place's
  stored release in its scratch directory once and then frees it, and
  keeps a preview's and a worktree's failed release on its record;
  `Runs` ends and removes runs without running their release, and runs a
  stored release in a given working directory.
- The core (`sluice-core`): one release path for a combination's
  checkout used by the round's releases, `recover` and a returning
  host's 81 release; the branch's records gain the failed releases and
  the policy gains `release_retries`; `release_abandoned` and
  `release_unusable_previews` release a gone checkout once; a held
  checkout no longer stops the host's claims; `Round` reports what
  failed and what was held; the log gains `release_failed` and `gone`.
- The CLI (`sluice`): `sluice release <name>` and `--accept-failure`;
  `status` lists held checkouts; rounds and runs say what they freed,
  what failed and what is now held.
- The fakes and the conformance suite: a release that fails, a checkout
  removed from under sluice.
- Docs: ADR 0047, `docs/architecture/core.md` and the site's rules page,
  the commands, configuration and state pages, the setup skill.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/**
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/src/**
crates/sluice-conformance/tests/places.rs
crates/sluice-conformance/tests/previews.rs
crates/sluice-conformance/tests/scenarios.rs
crates/sluice/src/lib.rs
crates/sluice/src/exec.rs
crates/sluice/src/preview.rs
crates/sluice/src/runs.rs
crates/sluice/src/profiles/places.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/exec.rs
crates/sluice/tests/previews_tools.rs
crates/sluice/tests/profiles/places.rs
crates/sluice/tests/profiles/conformance.rs
docs/adr/0047-*.md
docs/architecture/core.md
plugins/sluice/skills/setting-up-sluice/SKILL.md
site/src/content/docs/architecture/rules.md
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/state.md
```
