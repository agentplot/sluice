# Verify: sluices-inbox-is-weirs-delivery-form

Verified on `change/sluices-inbox-is-weirs-delivery-form` at `3dc72bc7`,
cut from `bolt/sluices-outcomes-are-owed-until-confirmed` at `4dbe34df`,
on 2026-11-08.

**Verdict: not ready to archive.** The bolt's gate fails on two tests the
change broke, and several scenarios of its specs are held by no test.
The code does what every requirement asks; nothing found is a
requirement left unbuilt.

## The whole checks

`sluice exec bolt/sluices-outcomes-are-owed-until-confirmed` (sluice
0.73.1), the bolt's gate `#check-fast`, run once: **failed, exit 1**.
19 tasks passed, 1 failed; sluice recorded 20 results under their keys
for `3dc72bc`.

| Task | Result |
|---|---|
| `cli:test` | **failed**: 211 passed, 2 failed in `--test cli` |
| `core:test`, `seams:test`, `fakes:test`, `conformance:test` | passed |
| `nix:lint`, `nix:test`, `nix:typecheck` | passed |
| `plugin-board:test`, `plugin-board:typecheck`, `plugin-board:validate` | passed |
| `plugin-sluice:hooks`, `plugin-sluice:lay-out`, `plugin-sluice:validate` | passed |
| `root:adrs`, `root:changes`, `root:format`, `root:lint` | passed |
| `site:prose`, `site:rules` | passed |

The two failures are in `crates/sluice/tests/cli.rs`, which the change
does not touch:

- `status_lists_the_queue` (`cli.rs:1275`, assertion at 1334)
- `status_and_overview_show_a_hold_while_it_lasts` (`cli.rs:7254`,
  assertion at 7265)

Each compares the exact lines of `sluice status <branch>`. The change
adds the `OWED` section (owed-outcomes, D13) after the queue, and the
fixtures in these tests hold outcomes nobody confirmed. So `sluice
status` now also prints, for example:

```text
OWED
  unit/a · never confirmed
    1:landed #1  owed until 2026-10-18T09:00:12Z · handed over by notice at 2026-10-11T09:00:16Z, wait at 2026-10-11T09:00:19Z
confirm one with: sluice confirm <key>, in its worktree, or sluice confirm <key> --as operator
```

The output is what owed-outcomes asks for. The tests' expectations
predate it and must take the section in, or confirm the fixtures'
outcomes first. Each task of 4.x ran only its named test, so the whole
`--test cli` suite never ran before this stage.

## Tasks

All 27 tasks are ticked. Every test a task names exists under its
name, and every test it renames or replaces is gone.
`openspec validate --strict` passes. Every MODIFIED and REMOVED
requirement's header matches its main spec, and no ADDED header
clashes with one, so the archive would apply.

## Specs

The REMOVED requirements are gone. `Submission` holds no `delivered`,
`delivered_to`, `told` or `told_to`, and an older record is written
back without them (`records.rs:1913`). `Pending::mark`, `pending_one`
and `Delivery::Delivered` are gone. The log's `delivered` events are
read as history alone.

Every requirement is built. The scenarios below are built, but no test
asserts the THEN clause named:

### Gaps the code stage closes

**plugin-board**
1. "A reload after a turn" passes without testing a reload
   (`register.test.tsx:1724`). A second `$.session.start` never resets
   the module's `following` (`register.tsx:396`, `:450`), so no first
   overview is taken and no `sluice inbox` runs.
2. "A reload after a turn the agent did not confirm" has no test.
3. "The board collects at its first overview", after a watch said the
   repository has no sluice, has no test.
4. "Landed while no watch was open": the tests at `:1511` and `:1702`
   count two collects but never check the second was `sluice inbox
   --new`.

**inbox**

5. "An expired outcome says so": no test runs `sluice inbox` on an
   expired delivery, so `Expired at <t>, still owed.` is asserted
   nowhere.
6. "--new hands over only what no way has handed over" is held in the
   core alone (`sluice-conformance/tests/inbox.rs:165`). No CLI test
   passes `--new`.
7. "The git directory collects nothing of a worktree's": 13 staying
   owed with no hand-over is checked only before the inbox runs, never
   after (`worktrunk.rs:2072`).

**outcome-confirm**

8. "A confirmation's document": no test checks a first confirmation's
   `--json` (`first` true, `branch`, `reader`). The only check is a
   second confirmation's `first` false (`worktrunk.rs:1511`). The
   not-found error document's `kind` and `next` are untested.
9. "A key no branch holds is not found": exit 3, naming a branch it
   could not read, has no test. Neither does the order of exits across
   mixed keys, 3 over 2 over 1 (`lib.rs:4378-4413`).

**owed-outcomes**

10. "A landing pushed and not confirmed" is run through `sluice wait
    --json` (`worktrunk.rs:650`), not `sluice status <n> --json`, and
    asserts neither `key`, `reader` nor `confirmed` null. `recorded_at`
    is asserted in no `delivery`, and `known` in none either (only in
    the status's `owed` list).
11. "A landing confirmed and let go of": no test asserts that the
    `delivery` is `key` and `state` `done` alone.
12. "A queued submission": no test runs `sluice status <n> --json` on a
    queued or withdrawn submission. A waiting `sluice submit --json`'s
    `delivery` is untested.

**outcome-deliveries**

13. "A day's span" asserts only the parsed setting
    (`instance.rs:759`), never `expires_at` 24 hours after
    `recorded_at`.
14. "A line that does not read" is held by `Line::load`
    (`line.rs:688`). Nothing runs `sluice status main` on such a file.
15. "A branch whose state is on its remote": no test has a second host's
    inbox read the delivery.

**submit-by-name**

16. "No worktree on the branch collects its outcome": the outcome
    "shown in `main`'s status" is not asserted (`pull_requests.rs:4339`).
    `sluice confirm` finding nothing under a by-name key has no test.

**site**

17. `reference/state.md:419-426`: the example notice lacks the closing
    `sluice confirm 13:failed once you have read it.` that the next
    sentence says it ends with.

### Seen, not blocking

- `wt submit` decides to pass `--confirm-landing` before the wait, from
  `wt`'s `merge.remove` setting (`wt_submit.rs:45`). Where that removal
  then fails and the worktree stays, its landing is confirmed already.
  Record 0102 chose this ("where it will remove the worktree").
- outcome-delivery's first requirement says the notice ends with
  `sluice confirm <key>`. D10, and the code, end it with `sluice confirm
  <key> once you have read it.`. The scenarios ask only that the line
  name it, which holds.
- Doc comments still say "marked delivered": `crates/sluice/src/lib.rs`
  at 2989 and 4667-4670, and `pulls.rs` near 4027.
- The weir `outputHashes` value in `flake.nix` is evaluated by
  `nix:typecheck` but first built at the release.
- The board's `from` `0.79.0` holds while this is the next `feat` to
  release: the latest tag is `v0.78.1`.

## Archive

Not archived: the gate fails, and items 1 to 17 are the code stage's.
