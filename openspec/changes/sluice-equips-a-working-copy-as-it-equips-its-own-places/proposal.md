## Why

sluice makes a working copy ready before it runs anything there. It runs
the toolchain the policy names, then, under moon, installs what
`.prototools` pins, checks moon's version and runs `moon setup`. That
sequence is written four times, and the copies differ. A combination's
run and a session's run of the checks get all of it (`Moon::plan`). A
submission's read (record 0050) and the affected reading (140) get all
of it but the setup (`Moon::kept_out`, `Moon::reads`). A preview's
command and `sluice exec <branch> -- <command>` get the toolchain alone,
since each writes the command runner over the policy's
(`preview_request`, `session_request`). On a host where nothing has
installed what a moon repository pins, a preview or `sluice exec main --
moon run app:test` fails where the branch's own runs pass.

Nor can a working copy sluice did not make ask for the sequence. flume
copied three of its steps into its own readiness and left the install
out. Its worktree on a fresh clone failed at `moon setup` on deneb on
2026-11-05. The user approved flume's design of that day, which makes
sluice the one owner of how a working copy is equipped, with flume
asking it through `sluice equip` (its decision 4) and nothing of
sluice's steps copied anywhere (its decisions 2 and 6). This change is
sluice's part. Its clauses are the design's decision 7 for sluice: 147,
the amendments of 33, 84, C.1's git worktree row, C.2's moon row and
section 9, the command, and S72, which a sibling chore is writing into
agentplot/blueprints. Record 0081 sets the decision down, rule core.46
binds it, and the details of rules core.37 and core.38 say where it
sits beside them. All three are written with this change.

## What Changes

- **One sequence.** Every working copy sluice runs a check or a command
  in is equipped by one function of the check profile, in this order:
  the toolchain the policy names, run in the working copy, its output
  kept as the environment of what follows; then the runner's own steps,
  under moon `proto install --config-mode upwards` where the working
  copy holds a `.prototools`, `moon --version`, refused below 2.6, and
  `moon setup`. Each step runs outside the executor with a run's
  environment, gives its outcome and words, and the sequence stops at
  the first that fails (147; rule core.46).
- **Where it stops.** A working copy sluice only reads in is equipped up
  to the version check: a submitter's worktree at `sluice submit`, the
  affected reading's tree, and a combination's or a session's working
  copy while its plan is read. One a check or a command runs in is
  equipped through the setup. A combination that affects no required
  check runs nothing, and is never set up, as today.
- **A preview's and a session's command gain the install and the
  setup.** A run request carries the command it runs in place of the
  runner's checks, and the working copy is equipped by the runner the
  branch's policy names. `sluice preview` and `sluice exec <branch> --
  <command>` of a moon repository install what the checkout pins, check
  moon's version and run its setup before the command (S72).
- **`sluice equip <branch> [--at <revision>] [--json]`.** It equips the
  working copy it runs in, by the policy at `<revision>`, or at the
  branch's current revision where none is named, through the setup. It
  prints each step and its outcome. `--json` prints `{"branch",
  "revision", "steps": [{"step", "passed", "words"}]}`. It exits 0 when
  every step passed, none included, and 1 when one failed. A refusal is
  `{"error": {"kind", "message", "next"}}` with no steps, exiting 2. It
  adopts nothing (no ports, no record, no part of the host's bound) and
  keeps nothing once it answers (147, the command; S72).
- **A step's words.** Each step's words are what it printed, its output
  then its error stream; the toolchain's error stream alone. A step that
  failed is given sluice's reason first. A combination whose working
  copy could not be equipped could not start, and is recorded so with
  the step's name and words, every check not run and nobody blamed
  (114).
- **The words a reader sees.** The commands and configuration reference
  pages, the README's command table and the plugin's skills name `sluice
  equip` and say that every checkout, worktree and preview is equipped
  so.
- **A release.** Once the change lands, the release CI makes from it
  carries `sluice equip`, and the tag is given to the orchestrator for
  flume's pin.

## Capabilities

### New Capabilities

- `equip`: how sluice makes a working copy ready for its branch's
  runner: the one sequence and its steps, where it stops for a working
  copy sluice only reads in, every path that runs it, and `sluice
  equip` for a working copy sluice did not make.

### Modified Capabilities

None. No requirement of `preview`, `moon-runner`, `run-outcome` or
`checkout` changes. A preview's command still runs once under the
executor, a submission's read still warns when moon cannot be asked, and
a run that could not be started is still recorded with why. What each
now gains, the equipping before it, is `equip`'s.

## Out of scope

- **flume's readiness through `sluice equip`.** It is flume's change
  `a-places-readiness-is-one-design-across-the-tools`, groups 2 to 4. It
  waits on this change's release and on flume's chore that pins it.
- **crew's, Boomdrive's and the kit standard's amendments** (flume's
  design, decision 7), each in its own repository.
- **The release command.** It still runs under the toolchain alone (95):
  the working copy was equipped by the run it releases, and a gone
  working copy's release enters no toolchain (123).
- **A step's own time limit.** No step has one today. A setup that hangs
  holds its run's start as it does now. It waits on a host that meets
  one.
- **A failed start's exit from `sluice exec` and `sluice preview`.** They
  still exit 3 when their run cannot start, now naming the step and its
  words. A failed exit of their own waits on a change that weighs it for
  every cause of a failed start.
- **`--worktree` for `sluice equip`.** It equips the working copy it runs
  in, as decision 4 gives it. A caller elsewhere runs it there.

## Impact

- `crates/sluice-seams/src/lib.rs`: `Through`, `Equip`, `EquipStep` and
  `Equipped`; `Tests::equip`, defaulting to nothing to equip;
  `RunRequest::command`.
- `crates/sluice/src/profiles/checks.rs`: `Equipping`, the sequence and
  its stops; `Checks::start`, `kept_out` and `reads` through it;
  `Tests::equip`; a command's run told apart from its runner; the
  module's doc comment.
- `crates/sluice/src/profiles/moon.rs`: the runner's steps, `install`,
  `check_version` and `setup`, given to `Equipping` in order with the
  stop each belongs to; `plan`, `kept_out` and `reads` no longer install
  or check for themselves; the module's doc comment.
- `crates/sluice-core/src/equip.rs` (new), `crates/sluice-core/src/lib.rs`:
  `Sluice::equip`, the policy at the revision and the refusals.
- `crates/sluice-core/src/session.rs`, `crates/sluice-core/src/preview.rs`:
  `session_request` and `preview_request` carry the command and leave
  `[tests]` the policy's; `crates/sluice-core/src/step.rs` gives a
  combination's run no command.
- `crates/sluice-fakes/src/lib.rs`: the fake's start reads the request's
  command; `FakeTests::equip`, scripted, and what it was asked.
- `crates/sluice/src/lib.rs`, `crates/sluice/src/equip.rs` (new): the
  command, its output, its refusals and exits;
  `crates/sluice/src/main.rs` and `crates/sluice/src/profiles/mod.rs`:
  the repository opened at the top of the worktree `sluice equip` runs
  in.
- `crates/sluice/moon.yml`: `tools` runs `--test equip_tools`.
- Tests: `crates/sluice/tests/checks.rs`,
  `crates/sluice/tests/profiles/moon.rs`, `crates/sluice/tests/equip.rs`
  (new), `crates/sluice/tests/equip_tools.rs` (new); the tests that read
  a preview's or a session's command from the request's `[tests]`
  (`crates/sluice-conformance/tests/previews.rs`,
  `crates/sluice/tests/previews.rs`, `crates/sluice/tests/cli.rs`); and
  every test that builds a `RunRequest`.
- Docs: `site/src/content/docs/reference/commands.md`,
  `site/src/content/docs/reference/configuration.md`, `README.md`,
  `plugins/sluice/skills/using-sluice/references/exec.md`,
  `plugins/sluice/skills/setting-up-sluice/references/profiles/moon.md`,
  `plugins/sluice/skills/setting-up-sluice/references/profiles/previews.md`.
- Records and rules: record 0081; rule core.46; the details of core.37
  and core.38; the site's rules page, written from them.
- Sources: clauses 31, 33, 35, 56, 71, 84, 89, 95, 98, 114, 123, 140,
  147, C.1's git worktree row, C.2's moon row, section 9, S72, as flume's
  design of 2026-11-05 words them (decision 7); records 0013, 0018,
  0045, 0050, 0066, 0074, 0081; rules core.1, core.2, core.10, core.14,
  core.16, core.18, core.19, core.21, core.26, core.37, core.38, core.46.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice/src/profiles/checks.rs
crates/sluice/src/profiles/moon.rs
crates/sluice-core/src/equip.rs
crates/sluice-core/src/lib.rs
crates/sluice-core/src/session.rs
crates/sluice-core/src/preview.rs
crates/sluice-core/src/step.rs
crates/sluice-fakes/src/lib.rs
crates/sluice/src/lib.rs
crates/sluice/src/equip.rs
crates/sluice/src/main.rs
crates/sluice/src/profiles/mod.rs
crates/sluice/moon.yml
crates/sluice/tests/**
crates/sluice-conformance/tests/previews.rs
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/configuration.md
README.md
plugins/sluice/skills/using-sluice/references/exec.md
plugins/sluice/skills/setting-up-sluice/references/profiles/moon.md
plugins/sluice/skills/setting-up-sluice/references/profiles/previews.md
docs/adr/0081-every-working-copy-sluice-runs-in-is-equipped-by-one-sequence-which-sluice-equip-offers.md
docs/architecture/core.md
site/src/content/docs/architecture/rules.md
openspec/changes/sluice-equips-a-working-copy-as-it-equips-its-own-places/**
```
