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
| 5 | checkouts: a detached `git worktree` (ADR 0013) | §5 | **done**: `profiles/places.rs`, acceptance tests, `cli:tools` |
| 6 | the driver: `wt-submit` through worktrunk, `SLUICE_AGENT` | §7 | **done**: `profiles/worktrunk/`, 9 tests through `wt submit`, `cli:tools` |
| 7 | branch protection: the `reference-transaction` hook, installed by `submit`/`step`/`run`; `sluice protect --remove` | §2 | **done**: `protect.rs`, 9 acceptance tests, `cli:tools` |
| 8 | end to end: `e2e.sh` on the scratch workspace, then crew | §8 | **done** on the scratch workspace: `e2e/e2e.sh`; crew once it is on moon 2.6 |
| 9 | the `sandbox` executor (Seatbelt on macOS, bubblewrap and Landlock on Linux); `[tests] toolchain`; sluice's moon and Rust through proto | §4, *The executor* | **macOS done**: `profiles/checks.rs`, writes and network held, fast tier; sluice's own `#check-fast` passes the runner's way under a localhost-only fence (8 tasks); Linux to come |
| 10 | sluice's own tiers: a workspace `root` project with `format` and `lint` tagged `check-fast`; `e2e/e2e.sh` as `check-e2e`; the conformance suite over the real profiles (the `Rig` split) | §9 | **done**: `moon.yml`, `e2e/check.sh`, `sluice_conformance::shared` (20 scenarios on the fakes and on moon with git worktrees, `cli:tools`) |
| 11 | warm places: `[[checkout.cache]]` (the build cache); the toolchain's evaluation paid per place | §5, §4 | **done**: `Checkout::release(name, passed)`, the cache in `profiles/places.rs`, acceptance test 7, `cli:tools` |
| 12 | `trust` per branch, then sluice's own policy, and sluice landing sluice | §4, §9 | **`trust` and the policy done**: `Rules::trust`, the verdict reading only trusted results, `RunRequest.replay` (moon's `--force`), `.config/sluice.toml`; next: the policy on `main` by `wt merge`, then sluice landing sluice |

The `container` executor comes after the goal; on one Mac it is not
needed.

## What each next piece owes the seams

- **11, warm places.** Every candidate gets a new place, so when moon does
  not replay a task, it pays a cold build and a fresh devenv evaluation.
  sluice thinks of it as CI does: the executor is the runner, a
  `container` image the custom runner, the toolchain and `proto install`
  the setup steps, and `[[checkout.cache]]` the `actions/cache` step
  (section 5, *Caches*).
  - **The build cache**: `[[checkout.cache]]`, restored into a place
    before its hooks and saved from a passed one, keyed by the content of
    the files it names; sluice's own declares `target`, keyed by
    `Cargo.lock` and `.prototools`. A place never takes anything from
    another worktree: `.worktreeinclude` is worktrunk's, read by the `wt
    step copy-ignored` that sluice's `wt.toml` runs at `post-start` for
    the worktrees people make.
  - **The toolchain's evaluation is paid per place**, as a CI job pays
    it: `devenv print-dev-env` takes 23 s in a new place at normal load
    and 0.1 s again in the same place, the cost evaluating nixpkgs, and
    devenv keys its evaluation cache by the root. A repository that wants
    it gone brings a container image, or keeps its devenv in a pure flake
    (`devenv.root` fixed, `nix print-dev-env path:./nix`: the same shell
    from any directory, 9 s; devenv's processes, tasks and own evaluation
    cache are lost). sluice's toolchain finds its worktree when entered,
    never at evaluation (`devenv.nix`: `git rev-parse` in its scripts and
    `enterShell`, `treefmt.config.projectRootFile`), so a toolchain
    printed elsewhere never checks another tree.

- **The conformance scenarios with faults** (crashes, interference,
  lost and held runs) stay on the fakes. Each shared scenario names what
  differs on a real rig where timing matters: a run cut short once the
  candidate ahead fails, a busy round taken again.

- **12, `trust`** (section 4, *What a branch trusts*): per branch,
  `confined`, executor kinds and `cache`; a replay counts only under
  `cache`, and without it the runner is asked for none; a table without
  `trust` trusts only `confined`; a gate the executor can never satisfy
  stops the step before any run. Then sluice's own `.config/sluice.toml`
  (section 9) lands on `main` by `wt merge`, since a branch's gate is
  read from the branch (core.15) and `main` has none yet; after that,
  sluice lands sluice.
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
- A place is a plain `git worktree` (ADR 0013; C.1 as changed in the
  statement): nothing of the candidate's runs outside the executor, so
  no hook; setup is the toolchain step and the checks' own
  dependencies, the build cache `[[checkout.cache]]`. worktrunk stays
  the driver (`wt submit`) and people's tool, never a place's.
- A place holds the candidate and the caches its branch's policy declares
  (`[[checkout.cache]]`), each keyed by the content of named files,
  restored before any hook and saved only from a passed candidate's
  place (35, as changed in the statement for this). The toolchain's
  evaluation is not cached: it is paid per place, as a CI job pays it.
- A branch's policy says how much network a run may reach (83, 84):
  `[executor] network = false` (none), `"localhost"` (only 127.0.0.1 and
  ::1, so tests can start servers there and reach them), or `true`
  (any). The sandbox holds all three and records which it held. sluice's
  own runs use `"localhost"`: its integration tests listen on
  localhost.
- Every moon task says what network its tests need, by tag:
  `needs-localhost` or `needs-network`; a task with neither needs none.
  `moon query tasks --tags needs-localhost` finds them, and a task needing
  more than its tier's policy allows is split so the rest run with less.
  Of sluice's own tasks only `cli:sandbox` needs localhost; none needs
  more.
- The tests of the `sandbox` executor start a sandbox themselves, which
  macOS refuses inside another one. They are the task `cli:sandbox` (every
  test whose name holds `sandbox`, skipped by `cli:test` and
  `cli:tools`), tagged `starts-a-sandbox` and `cargo-test` and no tier, so
  no branch checked under the sandbox requires them; the `wt merge`
  pre-merge hook runs them, outside any sandbox. Until the `container`
  executor exists, a change to the sandbox code is gated by that hook
  alone.
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
