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
| 6 | the driver: `wt-submit` through worktrunk, `SLUICE_AGENT` | §7 | **done**: `profiles/worktrunk/`, 9 tests through `wt submit`, `cli:tools` |
| 7 | branch protection: the `reference-transaction` hook, installed by `submit`/`step`/`run`; `sluice protect --remove` | §2 | **done**: `protect.rs`, 9 acceptance tests, `cli:tools` |
| 8 | end to end: `e2e.sh` on the scratch workspace, then crew | §8 | **done** on the scratch workspace, both place methods: `e2e/e2e.sh`; crew once it is on moon 2.6 |
| 9 | the `sandbox` executor (Seatbelt on macOS, bubblewrap and Landlock on Linux); `[tests] toolchain`; sluice's moon and Rust through proto | §4, *The executor* | **macOS done**: `profiles/checks.rs`, writes and network held, fast tier; the self-probe below to rerun; Linux to come |
| 10 | sluice's own tiers: a workspace `root` project with `format` and `lint` tagged `check-fast`; `e2e/e2e.sh` as `check-e2e`; the conformance suite over the real profiles (the `Rig` split) | §9 | next |
| 11 | warm places: the build cache and the toolchain's evaluation carried into a new place | §5, §4 | |
| 12 | `trust` per branch, then sluice's own policy, and sluice landing sluice | §4, §9 | |

The `container` executor comes after the goal; on one Mac it is not
needed.

## What each next piece owes the seams

- **10, sluice's own tiers.**
  - A `root` moon project at the workspace root (`moon.yml` beside
    `.moon/`, listed in `.moon/workspace.yml`) with `format` (`treefmt
    --fail-on-change`) and `lint` (`cargo clippy --workspace --all-targets
    -- -D warnings`), both tagged `check-fast`: a candidate is held to what
    the commit hooks hold a commit to. Their inputs name everything they
    read; `lint` is cacheable, `format` too.
  - `check-e2e`: a task running `e2e/e2e.sh <scratch dir> sluice` with the
    built `sluice` and `wt-submit` first on `PATH`; `cache: false` unless
    its inputs can be named whole. Tag `check-e2e` only.
  - The conformance suite over the real profiles: split the conformance
    harness into a `Rig` trait (what every scenario needs: a branch to
    land on, sessions with heads, a submit, rounds, the records) and
    fake-only faults (crashes, interference, lost runs) kept on the fake
    rig. A real rig builds a scratch repository with git-worktree places
    and the `command` runner, or moon on a scratch workspace; the
    scenarios that need no fault run on both, in `check-tools`.
- **The self-probe.** `scratchpad/self-probe.sh` (in this session's
  scratchpad; Claude Code's safety check will not run a sourced script
  holding `rm`, so the operator runs it with `!`) runs sluice's fast tier
  the profile's way under the bare Seatbelt fence; its worktree is at
  `34640ae`. It last failed on `/bin/bash` 3.2, since fixed; it is to be
  rerun and pass (`fenced run: exit 0`, `Tasks: 6 completed`).
- **11, warm places.** Every candidate gets a new place, so when moon does
  not replay a task, it pays a cold build and a fresh devenv evaluation.
  - **The build cache.** flywheel-next carries `target/` into each new
    worktree with worktrunk's `wt step copy-ignored` as a named
    `[post-start]` hook, bounded by `.worktreeinclude` (an entry crosses
    only if gitignored *and* listed): worktrunk clones where the
    filesystem allows (16 GiB in 7 s for 36 MB of disk), and a build took
    87 s instead of 134 s cold. Not more, because cargo's fingerprints
    hold each workspace crate's absolute path, so the workspace's own
    crates rebuild; the dependencies, the larger part, survive. Read
    `flywheel-next/main/.config/wt.toml` and `.worktreeinclude` (and the
    commits `084a5b20` to `ec7ab014`). For sluice: a place runs no `post-*`
    hook (section 5), so the place profile runs `wt step copy-ignored`
    itself after `pre-start`, for both methods (git-worktree has no
    worktrunk hooks but the copy is worktrunk's command; or a plain
    `cp -c` clone of `target/` from the main worktree). Measure cold and
    warm under the sandbox. Check whether `CARGO_TARGET_DIR` or remapping
    paths (`--remap-path-prefix`, `-Zbuild-dir`) keeps the workspace
    crates too.
  - **The toolchain's evaluation.** devenv caches its evaluation in the
    worktree's own `.devenv/` (flywheel-next measured 219 MB, holding
    absolute paths into that worktree, so it is not copied), and the nix
    store already holds the built shell, so a new place re-evaluates but
    rebuilds nothing. Measure `devenv print-dev-env` in a new place, again
    in the same place, and in a second new place. If the cost is the
    evaluation, keep the printed toolchain per revision of what it reads
    (`devenv.nix`, `devenv.lock`, `devenv.yaml`, `.prototools`), under the
    common dir, with the place's own paths written in at use; or find
    whether devenv can share its evaluation cache across roots.

- **The policy** gains `trust` per branch (section 4, *What a branch
  trusts*), read by the verdict from each result's recorded executor
  (28, 29); `Policy::parse` refuses unknown keys, so it comes with its
  parsing tests.
- **The culprit's session** is shown its failed tasks' logs (20), which
  stay in the place until it is released.
- **8, end to end.** Section 4's acceptance test 7 on real places: one
  `sluice run` with `[runs] max = 2` holds three branches to two runs.

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
- devenv holds the system tools, proto 0.63.1 (moonrepo's release build)
  among them. moon 2.6.1 and Rust 1.98.1 are application tools, their
  versions written in `.prototools` and installed by proto: in a
  developer's shell by the devenv task `sluice:proto:install`, never by
  `enterShell` (the toolchain script sluice sources inside the executor
  holds `enterShell`, so it only activates); in a check run by the moon
  runner, which runs `proto install` outside the executor before
  `moon --version` and `moon setup`. Both use
  `PROTO_HOME` and `MOON_HOME` of the repository's own under the common
  dir (moon puts both homes' `bin` ahead of everything on a task's
  `PATH`). `.moon/toolchains.yml` reads the Rust version from it, so each task's hash
  carries the toolchain's version. The moon profile's floor is 2.6
  (section 3).
- A repository's toolchain is printed outside the executor (`[tests]
  toolchain`, e.g. `["devenv", "print-dev-env"]`) and sourced inside it
  by the `bash` on the run's `PATH`; the moon runner runs `moon setup`
  outside too. The sandbox lets a run write only its place, its scratch
  directory and proto's `.last-used` stamps. `moon exec --no-actions` is
  never used: it runs whatever is on `PATH` under a hash naming the
  configured toolchain.
- The run's environment keeps `PROTO_HOME` and `MOON_HOME` with `PATH`: a
  proto shim on `PATH` resolves its tool under its home.
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
