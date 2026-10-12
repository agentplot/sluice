# Phase 1, pass 2: building the real profiles

Pass 1 built the core against fakes (`phase-1-core.md`). Pass 2 replaces
the fakes with the real profiles, one seam at a time, until sluice runs
as the merge queue of a real repository on one Mac, and then lands its
own work. How each profile works, with every command and API verified,
is `phase-1-integrations.md`, cited below by section; this file is the
order of work and where it stands.

Work happens in the `phase-2-git-state` worktree, on the branch of that
name, committed as it goes (by explicit paths, Conventional Commits,
never pushed). `devenv shell -- cargo-tests` runs the fast tier;
`devenv shell -- moon run ':#check-fast' ':#check-tools'` runs both.

## The goal

1. sluice lands bolt branches of a scratch repository end to end: `wt
   submit` from a session's worktree, a candidate checked out and tested
   by moon, the branch moved by sluice, the session told (S1 to S4,
   `e2e.sh sluice`, section 8).
2. sluice lands sluice: its own `.config/sluice.toml` gates bolt branches
   on `#check-fast` and `main` on all three tiers (section 9), with
   `main` trusting sandboxed runs while sluice runs on one Mac
   (section 4, *What a branch trusts*).

## Order and status

| # | piece | integrations | status |
|---|---|---|---|
| 1 | branch: gix and git (`profiles/local_refs.rs`) | §1 | **done**: 12 acceptance tests, `cli:tools` |
| 2 | records: local files (`profiles/local_files.rs`) | §6 | **done**: 9 acceptance tests, fast tier |
| 3 | the supervisor and the `open` executor; the instance's bound | §4 | **done**: `runs.rs` (6 tests), `profiles/checks.rs` with the `command` runner (8 tests), fast tier |
| 4 | moon: targets for required tags, `moon exec`, the run report on 2.6.1 | §3 | **done**: `profiles/moon.rs`, 9 acceptance tests, `cli:tools` |
| 5 | checkouts: `git worktree` first, then worktrunk | §5 | **done**: `profiles/places.rs`, 7 acceptance tests, `cli:tools` |
| 6 | the driver: `wt-submit` through worktrunk, `SLUICE_AGENT` | §7 | next |
| 7 | branch protection: the `reference-transaction` hook, installed by `submit`/`step`/`run`; `sluice protect --remove` | §2 | |
| 8 | end to end: `e2e.sh` on the scratch workspace, then crew | §8 | |
| 9 | the `sandbox` executor (Seatbelt on macOS, bubblewrap and Landlock on Linux) | §4, *The executor* | |
| 10 | sluice's own tiers: a workspace `root` project with `format` and `lint` tagged `check-fast`; the conformance suite over the real profiles (the `Rig` split) | §9 | |
| 11 | sluice's own policy, and sluice landing sluice | §9 | |

The `container` executor comes after the goal; on one Mac it is not
needed.

## What each next piece owes the seams

- **The policy** gains `trust` per branch (section 4, *What a branch
  trusts*), read by the verdict from each result's recorded executor
  (28, 29); `Policy::parse` refuses unknown keys, so it comes with its
  parsing tests.
- **6, the driver.** `session` reports the head, who submitted
  (`SLUICE_AGENT`, else the branch) and anything the worktree holds
  beyond its commits; `notify` per section 7, with what the culprit's
  session is shown of its failed tasks' logs (20).
- **8, end to end.** Section 4's acceptance tests 5 and 7 on real
  places: a waiting `wt submit` stopped by Ctrl-C leaves its run going,
  and one `sluice run` with `[runs] max = 2` holds three branches to two
  runs.

## Decisions this pass builds on

- One chain per branch; lanes are gone (statement clauses 8 to 10, S6).
- A waiting `wt submit` runs sluice's rounds on one machine; with
  `--remote` it only submits (ADR 0011; the many-hosts proposal).
- The instance that starts runs caps them; the executor holds a run to
  its limits and reports what it enforced (ADR 0012; clauses 71, 83, 84).
- A branch's gate trusts runs by confinement or executor kind (section
  4); records never claim confinement a run lacked.
- Policy defaults: `parallel = 4`, `batch = false`, `flaky = "fail"`,
  `timeout = "1h"` (`phase-1-questions.md`).
- moon 2.6.1 and proto 0.63.1 from moonrepo's releases in devenv; the
  moon profile's floor is 2.6 (section 3).
- gix 0.89 for refs and objects, the git command for merges and ancestry
  (ADR 0009; section 1).

## Outside this repository

- **crew:** export `SLUICE_AGENT=$CREW_AGENT` in its role environment;
  the conductor brings its worktree level when its bolt's branch moves;
  move to a design where no worktree has a bolt's branch checked out;
  move its devenv to moon 2.6.
- **flywheel-next:** the git hook install fix is on its
  `fix-git-hooks-install` branch (`db9f1dae`), to land with `wt merge`.
- **devenv:** cachix/devenv#3270. When it is fixed, remove the
  `devenv:git-hooks:install` override in `devenv.nix` (here and in
  flywheel-next).
