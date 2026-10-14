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
| 8 | end to end: `e2e.sh` on the scratch workspace | §8 | **done**: `e2e/e2e.sh`, and acceptance test 7 of section 4 (`tests/instance.rs`) |
| 9 | the `sandbox` executor (Seatbelt on macOS, bubblewrap and Landlock on Linux); `[tests] toolchain`; sluice's moon and Rust through proto | §4, *The executor* | **macOS done**: `profiles/checks.rs`, writes and network held, fast tier; sluice's own `#check-fast` passes the runner's way under a localhost-only fence (8 tasks); Linux to come |
| 10 | sluice's own tiers: a workspace `root` project with `format` and `lint` tagged `check-fast`; `e2e/e2e.sh` as `check-e2e`; the conformance suite over the real profiles (the `Rig` split) | §9 | **done**: `moon.yml`, `e2e/check.sh`, `sluice_conformance::shared` (20 scenarios on the fakes and on moon with git worktrees, `cli:tools`) |
| 11 | warm places: `[[checkout.cache]]` (the build cache); the toolchain's evaluation paid per place | §5, §4 | **done**: `Checkout::release(name, passed)`, the cache in `profiles/places.rs`, acceptance test 7, `cli:tools` |
| 12 | `trust` per branch, then sluice's own policy, and sluice landing sluice | §4, §9 | **done**: `Rules::trust`, the verdict reading only trusted results, `RunRequest.replay` (moon's `--force`), `.config/sluice.toml` on `main`; this row landed on `main` through `wt submit`, gated on all three tiers in the sandbox |
| 13 | the culprit's failed tasks' logs | §4, §6 | **done**: `Tests::logs`, `State::keep_logs`, the candidate's `failed[].log`, `Told::Failed.logs`; `wt submit` prints each path |
| 14 | a conventional landing subject: `[landing] subject` | §1 | **done**: `Policy::landing_subject`, `text::landing_message` |
| 15 | the driver removes a landed session's worktree | §7 | **done**: `wt-submit` with `Submit::parse`, `--no-remove` |
| 16 | publishing: a flake, GHA on macOS and Linux, releases, the repository public | | **done**: `flake.nix`, `.github/workflows/ci.yml` green on macOS and Linux, the README's install methods, agentplot/sluice public |
| 17 | no tests of the repository's documents | | **done**: the citations and tool-name checks removed |
| 18 | OpenTelemetry from sluice, and moon's inside sluice's runs | | **done**: ADR 0014, `telemetry.rs`, the core's spans and metric events, `tests/telemetry.rs`; e2e exported to a local receiver, moon's traces carrying the round's `sluice.trace_id` |
| 20 | semantic releases with git-cliff | | **done**: `cliff.toml`, `.versionrc` (convco, the commit-msg hook and CI's check), the `version`, `binaries` and `release` jobs, `SLUICE_VERSION`, `release-preview` |
| 21 | CI: the deeper tiers on Linux, and `target/` kept between runs | | **done**: the `linux` job (`#check-tools`, `#check-e2e`) green, worktrunk 0.80.0 pinned in the devenv shell, `target/` in GitHub's cache |
| 22 | a full stack per candidate, side by side: containers, servers and a browser test | §4 | **done**: ADR 0016, `[checkout] ports`, `[tests] release`, `[executor] docker`, `tests/stack.rs` (open and sandbox, `cli:tools` and `cli:sandbox`); the repository's own move to `SLUICE_PORT_*` and `SLUICE_PLACE` is outside this one |
| 23 | `main` kept in step with the shared remote | C.4 | **done**: ADR 0017, `[branches] remote`, `profiles/pushed_refs.rs` and the router `profiles/branches.rs`; `tests/pushed_refs.rs` (6) and `tests/remote_branch.rs` (6), `cli:tools`; sluice's own `main` lands on `origin` with no hand push |
| 23 | sluice's own identity on GitHub: a GitHub App, the ruleset's only bypass (`github-app`) | C.4 | **done**: ADR 0019, `[forge]` in the host config, `profiles/github/`, `identity.rs` and `sluice credential`, `sluice forge check`; `tests/profiles/forge.rs` against a fake of GitHub's API, `tests/pushed_refs.rs` through the helper; the app and the ruleset on GitHub are the operator's to make |
| 24 | pull requests: one per submission, and one opened by hand submitted | C.5 | designing |

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
- **13, the culprit's failed tasks' logs.** When a run finishes with
  failures, the core asks the check seam for each failed task's output
  (`Tests::logs(run, tasks)`; moon 2.6 keeps it in the place at
  `.moon/cache/states/<project>/<task>/{stdout,stderr}.log`; the `command`
  runner returns what it captured) and writes it through the records seam
  before the place is released, at
  `<common>/sluice/.logs/<branch>/<candidate>/<task>.log`, beside the
  branch's records since a step reads every record of its branch: the
  last 64 KiB of each, the last 20 failed candidates per branch kept
  (core.11). The
  culprit's session is told each log's path with the failing checks and
  the candidate (20); `wt submit` prints them and `sluice status <id>`
  lists them; no other session is told.
- **14, the landing subject** is the policy's, conventional by default:
  `[landing] subject = "chore(land): #{id} {from}"`, with `{id}` (the
  submission), `{from}` (the session's branch), `{agent}` and `{to}` (the
  branch landed on); the `Sluice-*` trailers are unchanged.
- **15, removing a landed session's worktree** is the driver's (C.4;
  core.16 keeps the core off it): `wt-submit`, when its submission lands,
  runs `wt remove` on its own worktree as `wt merge` does, following
  worktrunk's `[merge] remove` setting, with `--no-remove`; the branch
  goes too, `main` holding it.
- **16, publishing.** A root `flake.nix` builds `sluice` and `wt-submit`
  with the Rust `.prototools` names (rust-overlay), so `nix profile
  install github:agentplot/sluice` installs it on macOS and Linux; the
  GHA workflow builds the flake on both, runs the fast tier in the devenv
  shell with agentplot/blueprints checked out for the statement
  (`SLUICE_STATEMENT`), and on a `v*` tag publishes each platform's
  binaries to a release. The history was checked (gitleaks, all
  commits: no leaks; no customer, pricing or employer named). Making the
  repository public is confirmed with the operator first, telling them
  the commits carry their email and the docs absolute paths under their
  home. After each landing, the `main` worktree is brought level with
  `git read-tree -m -u <old> <new>`: sluice moves `main` under the
  worktree that has it checked out.
- **17, no tests of the repository's documents.** sluice's tests test
  sluice. A check that reads the statement or the source to enforce
  core.1 or core.12 is not one: those rules are kept by review.
- **20, semantic releases**, with git-cliff. On each push to `main`, CI
  asks `git cliff --bumped-version` for the next version from the
  conventional commits since the last `v*` tag (the agents' own commits,
  reached through the landing merges). With a new version, it builds the
  flake for aarch64-darwin, x86_64-linux and aarch64-linux with that
  version, tags the commit, and publishes a GitHub release with the
  binaries, named `sluice-<system>.tar.gz` so the README's
  `releases/latest/download/` links hold, and git-cliff's notes for the
  range. Nothing is committed back to `main`, which only sluice moves: the
  version lives in the tag, the flake and `sluice --version` take it from
  there (`SLUICE_VERSION` at build time, `Cargo.toml` staying at `0.0.0`).
  - **`cliff.toml`**: groups Features, Fixes, Performance, Refactors and
    Documentation, with Breaking changes first; `chore(land)` landing
    commits and other `chore` commits skipped, `Co-Authored-By` lines
    stripped; each entry linked to its commit on GitHub, `#<n>` to its
    issue; only `v*` tags counted.
  - **`[bump]`**: `feat` bumps the minor, a breaking change the major,
    and below 1.0 a breaking change the minor; the first release
    `v0.1.0`.
  - **No `CHANGELOG.md` in the repository**: committing one would mean
    landing it after each release; each release carries its range's
    notes, and `git cliff` prints the whole history's.
  - **`release-preview`**, a devenv script: `git cliff --unreleased
    --bump`, the next version and its notes before anything lands.
  - **Conventional commits enforced**: convco's commit-msg hook among the
    prek hooks refuses a message that is not one, and CI checks the
    commits since the last release, since a session can skip a hook;
    `.versionrc` lets a scope name several parts of the tree, `docs(adr,
    architecture)`. Merge commits are not checked.
  - **Which pushes release**: any with a released change; a `docs` or
    `refactor` change bumps the patch, as `fix` does. A push of only
    `chore`, `test`, `ci` or `build` commits releases nothing.
  - Not configured: git-cliff's GitHub remote (pull requests, authors,
    first-time contributors), since sluice's commits come through no pull
    request; path filters, since sluice releases one product.
- **21, CI's tiers.** sluice's gate for `main` runs every tier before a
  landing, on the macOS host it lands from, so CI does not repeat them
  there. It runs what that host cannot: the flake's build and the fast
  tier on Linux and macOS, and a `linux` job with `#check-tools` and
  `#check-e2e`. The sandbox tests stay the host's (Seatbelt). worktrunk is
  pinned in the devenv shell (0.80.0, its release build), so the tools
  tier and e2e drive one version here and in CI. The fast and `linux`
  jobs keep `target/` and cargo's registry in GitHub's cache, keyed by
  `Cargo.lock` and `.prototools`; moon's own cache is not kept, since it
  holds which tasks passed, not the compiled code, and nearly every push
  changes a task's inputs.
- **Landing on `main`.** `main` moves only through sluice: from
  `phase-2-git-state`, `SLUICE_AGENT=claude PATH="$HOME/.cargo/bin:$PATH"
  wt submit main --no-remove`, then the `main` worktree brought level
  (`git read-tree -m -u <old> main`), then `git push origin main`. After
  a landing that changes sluice, `cargo install --path crates/sluice
  --locked` in the `main` worktree and `sluice protect` upgrade the guard.
- **18, OpenTelemetry** (ADR 0014), as moon does it (`moon --otel`):
  opt-in with `--otel` / `SLUICE_OTEL` (`1`, `true`, `yes`, `on`), the
  service `SLUICE_OTEL_SERVICE_NAME` (default `sluice`), and the standard
  `OTEL_EXPORTER_OTLP_*` variables for endpoint, headers and timeout.
  OTLP over HTTP, protobuf.
  - Spans: `sluice.round` per branch, under it `sluice.recover`,
    `sluice.decide` and `sluice.perform`, and under those `sluice.form`,
    `sluice.start`, `sluice.verdict`, `sluice.blame` and `sluice.land`;
    attributes: branch, submissions, candidate revision, executor and
    confined, outcome, tasks run and replayed. Metrics: queue depth and
    window, verdicts by outcome, tasks run and replayed, culprits,
    conflicts, landings and refused landings, submit-to-landed time, a
    candidate's time under test, the build cache's restores by hit.
  - Moon inside a run: moon 2.6.1 reads no parent trace (`TRACEPARENT` is
    ignored), so its spans are tied to the round by attributes. With
    telemetry on, a run's environment gets sluice's `OTEL_*`, moon gets
    `--otel`, and `OTEL_RESOURCE_ATTRIBUTES` carries `sluice.trace_id`,
    `sluice.span_id`, `sluice.place` and `sluice.revision`; `TRACEPARENT`
    names the start span, for a tool that reads it.
  - Telemetry decides nothing: the records stay the only truth (core.11,
    core.14). A connection to a collector binds no port (core.10). A
    short `sluice step` flushes before it exits. The core emits `tracing`
    spans and events and names no backend; the binary sets up the
    exporter.
- **8, end to end.** Section 4's acceptance test 7 on real places
  (`tests/instance.rs`, `cli:tools`): one `sluice run` with `[runs] max =
  2` serving three branches of window four has two runs under test at
  most, reaches two, and gives each half the processors.

- **22, a full stack per candidate.** A repository's checks bring up
  everything a browser test needs: a container stack on a port, stand-ins
  for outside services, API and dev servers, then Playwright against
  them, for every candidate under test at once.
  What sluice owes the checks:
  - Ports, assigned by sluice. `[checkout] ports` names the ports a
    candidate's stack uses. The place seam gives each place a slot, held
    on the host across repositories and checked free before the place is
    handed over, and each name a fixed position in the slot's block:
    `port = ports_from + slot × names + position`, `ports_from` 20000 by
    default, above worktrunk's `hash_port` range. The place carries
    `SLUICE_PLACE` and `SLUICE_PORT_<NAME>` into its runs, and the checks
    name their containers, deployments and addresses from them.
    Separate systems find each other by these numbers, not by discovery.
    This replaces the part of ADR 0010 and core.10 that kept ports out
    of sluice, through the statement first (core.12) and a new ADR.
  - A release command, `[tests] release`, run by the check seam in the
    place under the same executor, limits and environment as the
    checks, whenever the core releases a candidate's place: after a
    pass, a failure, a time limit, or a run found dead by a later round.
    It removes what the checks started outside the run's process group,
    containers above all, which a check's own cleanup cannot do once
    sluice has killed it. The core ends a place's run, release command
    included, before the place seam removes the place.
  - `[executor] docker = true` lets a sandboxed run reach the container
    daemon's socket. Such a run is recorded as not confined: a container
    it starts can mount any path, so the sandbox no longer holds it. The
    daemon pulls images itself, so a run's own network stays as the
    policy states.
  - The tools from the repository's toolchain: the container client and
    Playwright with its browsers from its devenv shell; the container
    daemon is the host's. The sandbox lets a run reach the daemon's
    socket, and the policy says whether a run may reach the network
    beyond localhost.
  - moon runs the checks and devenv brings up the stack: a moon task
    that depends on one bringing the stack up and waiting until each
    service answers, with devenv's processes and their readiness probes
    doing the starting.
  - Earthly is not used: it is unmaintained, reports only an exit code,
    and moves every check into privileged containers outside devenv and
    the sandbox.
  - The acceptance test, in the tools tier, with Docker and Playwright
    on every host that runs it, CI's Linux job included: a fixture of a
    container on a port, a server, and a browser test against them; two
    candidates under test at once neither collide nor share a container;
    both land; a candidate ended by its time limit leaves no container
    and no process.
- **23, `main` kept in step with the shared remote.** The branch's
  pushed-refs profile (C.4): fetch before a round reads the branch, form
  on the remote's revision, land with a push leased on the revision
  read, the forge's rules letting only sluice's identity move the branch.
  sluice's identity is a GitHub App, the ruleset's one bypass, whose
  token reaches only sluice's own git calls, so agents pushing with the
  operator's credentials are refused (OpenSpec change `github-app`,
  built after `main-on-github` and before `pull-requests`).
  Only sluice's identity moves `main` on GitHub; a person or a merged
  pull request moving it is an outside move, which sluice takes as the
  branch moving under its queue. Only the branch is pushed: on one
  machine the records stay local files.
  Built as ADR 0017 says: a branch whose policy names a `remote` is
  fetched on every read, never moving the local branch, and landed by a
  push leased on the revision read, after which the local branch is set
  to the landing. `tests/pushed_refs.rs` covers the profile and the router
  against a bare repository as the remote; `tests/remote_branch.rs`
  covers rounds end to end: landing through `sluice run`, a push from
  elsewhere refused and tested again, a push stopped before it was
  recorded, a push that never reached the remote, a failed fetch, and an
  unknown remote.
  The app is built as ADR 0019 says: the host's `[forge]` names the app
  and its key, a file only its owner may read; sluice buys an hour's
  installation token for the repository and gives it to its own fetch
  and push through `sluice credential`, a helper named on that git
  call's command line, the token in that call's environment alone.
  Without `[forge]` sluice pushes with the host's credentials and
  `sluice status` says the branch has no identity of its own. `sluice
  forge check` says whether the app may write the repository and is
  every ruleset's only bypass on the branch, reading the rulesets with
  the operator's own token, since GitHub shows a bypass list only to
  the ruleset's admins. No fetch or push sluice makes ever prompts for
  a credential. The guide is "Give sluice its own GitHub identity";
  making the app and setting the ruleset are the operator's.
- **24, pull requests.** A pull request is a submission, from a branch
  in the repository (never a fork), opened by `submit` or by hand. It
  waits until it is ready: out of draft, and every condition the
  branch's policy names holds, such as an approving review or a check
  another system posts (a QA team's approval on a test deployment).
  Only then does it enter the queue, sluice's ordered list of what is
  ready to land, which sluice tests in combination and lands by pushing
  the branch (23); the forge then marks the request merged because its
  head is in the branch. sluice posts each pull request's state as a
  check on its head, and on a waiting one a forecast each round: whether
  it would merge cleanly behind the queue, with no checks run. The
  conditions are in sluice's policy for the branch, read from the
  branch's revision (core.15), never from the forge's branch rules.
  None of this needs a GitHub Action: sluice's checks
  run where sluice runs them. GitHub as the executor and the stepper is
  a separate deployment, designed apart from this.

## Decisions this pass builds on

- A key reads as flaky only across ran results held to the same limits
  (19): the executor, memory, network and container engine, not the
  processors. A fail with no container engine and a pass with one are
  two experiments, not one check failing at random.
- The branch protection hook calls an installed sluice at a stable path
  (`~/.cargo/bin/sluice` now, the nix profile's once published), never
  one built from a candidate: a submission never changes the guard it is
  judged by (as core.15 keeps it from its gate). A new sluice guards
  `main` once it is installed, a step of its own; the candidate's code is
  tested by its checks, the e2e tier building sluice from the tree.
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

- **Where sluice's telemetry goes**, set up by each host's own
  repository from the brief in `docs/telemetry.md`: a receiver on
  `localhost:4318` and `SLUICE_OTEL=1` where sluice runs. The Mac Studio:
  otel-tui or otel-desktop-viewer. crew's EC2 host: the Datadog Agent.
  swancloud and its crews: a viewer first, a hosted backend later through
  a collector on each host. GHA sends nothing.

- **crew:** export `SLUICE_AGENT=$CREW_AGENT` in its role environment;
  the conductor brings its worktree level when its bolt's branch moves;
  move to a design where no worktree has a bolt's branch checked out;
  move its devenv to moon 2.6.
- **The repository with the stack:** key its stack on sluice's place
  rather than its branch: every port from `SLUICE_PORT_*` instead of
  `hash_port`; container names and labels, service names and scratch
  directories from `SLUICE_PLACE`; the stand-ins never killing whatever
  holds a port. Its policy names the ports, `[tests] release` as its
  teardown, `docker = true`, and `network = true` for the suites that
  reach an outside service.
- **flywheel-next:** the git hook install fix is on its
  `fix-git-hooks-install` branch (`db9f1dae`), to land with `wt merge`.
- **devenv:** cachix/devenv#3270. When it is fixed, remove the
  `devenv:git-hooks:install` override in `devenv.nix` (here and in
  flywheel-next).
