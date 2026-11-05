# Design

## Context

See proposal.md for why, and record 0081 for the decision and the
options weighed. flume's design of 2026-11-05 ("What each tool makes")
read this code at 571c83f6. It reads the same at main 6b29c5ce, where
this change was cut.

- **The toolchain.** `crates/sluice/src/profiles/checks.rs`, `toolchain`
  (line 516): `[tests] toolchain` run in the working copy with the run's
  environment, stdin from `/dev/null`, its output written to a script.
  `Context::argv` sources that script before each later command as
  `bash -c 'source "$0" >&2; exec "$@"'`. The environment is
  `environment` (line 572): `KEEP`, the locale's `LC_*`,
  `SLUICE_CHECKOUT`, `SLUICE_TMP`, `SLUICE_JOBS`, the place's ports and
  names, and the host's caches' variables (35).
- **The runner's steps.** `crates/sluice/src/profiles/moon.rs`:
  `install` (line 353) runs `proto install --config-mode upwards` where
  the working copy holds a `.prototools`; `check_version` (line 388)
  refuses a moon below `FLOOR`, 2.6; and `plan` runs `moon setup` (line
  122) only once it has targets. Every command goes through `Moon::run`,
  which fails with `<argv> exited <status>: <stderr>`.
- **Four callers, each with its own copy.**
  - `Checks::start` (line 1242), a combination's run and a session's:
    admitted against the bound, then the toolchain into the run's
    directory, then `Moon::plan`, which installs, checks the version,
    queries, and sets up where it has targets.
  - `Checks::kept_out` (line 1640), a submitter's read (record 0050):
    under moon only, the toolchain into `.runs/kept-out-<pid>/`, then
    `Moon::kept_out`, which installs, checks the version and queries.
  - `Checks::reads` (line 1696), the affected reading (140): under moon
    only, a `ReadingTree` under `.runs/reading-<pid>/`, the toolchain,
    then `Moon::reads`, which installs, checks the version and queries.
  - A preview (`crates/sluice-core/src/preview.rs`, `preview_request`)
    and a session's command (`crates/sluice-core/src/session.rs`,
    `session_request`) write `runner = "command"` and their command into
    `[tests]`, so `Checks::start` gives them the toolchain and then the
    command.
- **A failed start.** `Checks::start` returns the error. The core
  records a combination's run as could not start, every check not run
  with `could not start: <error>`, its last 2048 bytes kept
  (`crates/sluice-core/src/step.rs`, `could_not_start`), and blames
  nobody (114). `sluice exec` and `sluice preview` return it, and exit 3.
- **The seam.** `Tests` (`crates/sluice-seams/src/lib.rs`, line 743) has
  `start`, `kept_out` and `reads`, and nothing a caller can ask to equip
  a working copy with. It has four implementors: `Checks`, `FakeTests`,
  the conformance wrapper `RecordedChecks`, and the test host in
  `crates/sluice-core/src/ops.rs`.

## Goals / Non-Goals

**Goals:**

- One sequence for every working copy sluice runs anything in, run by
  one function, so a step is added or changed in one place (147; rule
  core.46).
- A preview's command and a session's command equipped by the branch's
  runner, the install and the setup included (S72).
- `sluice equip` for a working copy sluice did not make, answering each
  step's outcome and words in the shape flume's change reads (the
  command; flume's design, decision 4).
- Nothing adopted, counted or kept by an equip (71; rules core.16,
  core.18).

**Non-Goals:**

- flume's, crew's, Boomdrive's and the kit standard's sides (flume's
  design, decision 7).
- The release command, which keeps the toolchain alone (95, 123).
- A time limit on a step, which no step has today.
- A new exit for `sluice exec` or `sluice preview` when their run cannot
  start.

## Decisions

### The sequence is the check profile's, run by one `Equipping`

`checks.rs` gains `Equipping`, which holds the working copy, `[tests]`,
the run's environment, the directory its toolchain's script goes in, the
steps run so far and how far it has got. `Equipping::through(stop)` runs
each step not yet run, up to `stop`, in order, and records each as an
`EquipStep { step, passed, words }`. It stops at the first that fails.
`Equipping::context()` then gives the `Context` the runner's queries,
plan and run use, with the toolchain's script sourced when one ran.

The steps are, in order:

1. `toolchain`, where `[tests] toolchain` is set: run as `toolchain`
   runs it today, its output kept as the script, belonging to the stop
   `Reading`.
2. The runner's steps, which the runner gives in order, each with the
   stop it belongs to. `Moon` gives `proto install`, where the working
   copy holds a `.prototools`, and `moon --version`, both `Reading`, and
   `moon setup`, `Running`. The command runner gives none.

`Through` is `Reading` or `Running`, in that order, and `Running`
includes every step of `Reading`. An `Equipping` asked through `Reading`
and later through `Running` runs only the steps it had not run, so a
combination's run is one sequence whose middle is the runner's queries.

`moon.rs` keeps `install` and `check_version` as the steps' bodies and
gains `setup`. Each answers an `EquipStep`, not an error, through a
`Moon::step` that runs a command and forms its words. `plan`, `kept_out`
and `reads` stop calling `install`, `check_version` and the setup
themselves. `plan` still decides whether anything runs. Its caller
equips through `Running` only where the plan's `argv` is `Some`.

**The seam.** `sluice-seams` gains:

- `Through { Reading, Running }`;
- `Equip { working_copy: PathBuf, tests: Settings, through: Through }`;
- `EquipStep { step: String, passed: bool, words: String }`;
- `Equipped { steps: Vec<EquipStep> }`, with `failed()`;
- `Tests::equip(&self, equip: &Equip) -> Result<Equipped>`. Its default
  answers no steps, for a check profile with nothing to equip, the core's
  test host among them. Its doc comment cites 147 and says it takes no
  place, admits nothing against the bound and keeps nothing once it
  answers. `Err` is sluice's own failure, such as a directory it could
  not make, or a `[tests]` setting of the wrong kind. A step that failed
  is an `Equipped` whose last step failed, never an `Err`.

`Checks::equip` makes a directory `.runs/equip-<pid>/` with a `tmp`
under it, builds the run's environment for a place named `equip` at the
working copy with that `tmp` and no ports, as `kept_out` builds one,
runs an `Equipping` through what was asked, and removes the directory
before it answers.

The paths not taken:

- *The sequence in the place profile*, since 33 calls a materialized
  place "equipped (147)". The toolchain and the runner are `[tests]`'s,
  which the check profile reads. A place profile that ran them would call
  the check profile (rule core.2), and the place profile runs nothing in
  a working copy (record 0013).
- *The core asking `Tests::equip` before `Tests::start`.* A
  combination's run is admitted against the bound before anything of it
  runs (71), the toolchain included, and the run's directory holds the
  script the run sources. Split across two calls of the seam, the
  admission and the script would cross it. `start` equips inside itself
  through the same `Equipping`, and `Tests::equip` is the one door for a
  caller that is no run.
- *Each step as a free function called in turn by each caller.* That is
  today's code, whose four callers drifted.

### Where each working copy stops

| working copy | equipped through | then |
|---|---|---|
| a combination's run, a session's run of the checks | `Reading` | `Moon::plan`; `Running` where the plan runs anything; the run |
| a preview's command, a session's command | `Running` | the command |
| a submitter's worktree at `sluice submit` | `Reading` | `moon query tasks` |
| the affected reading's tree | `Reading` | the queries |
| `sluice equip` | `Running` | nothing |
| a checkout's or a worktree's release | not equipped: the toolchain alone, for its environment, or nothing for a gone working copy (123) | the release command |

Under the command runner, `Reading` and `Running` both run the toolchain
alone. `kept_out` and `reads` still return at once under it, running
nothing, since they ask the runner nothing.

A combination that affects no required check is read and runs nothing,
so it is never set up, as today. That keeps the cheapest pass the queue
gives free of `moon setup`'s time on a host that has not set up the
workspace. The path not taken: always through `Running` before the
queries, which would also be one sequence with no middle, and would run
the setup in every combination's checkout, a combination that runs
nothing among them.

### A run's command beside its runner

`RunRequest` gains `command: Option<String>`: "a command the run runs in
place of the runner's checks (89, 98), its working copy equipped by the
runner `tests` names (147)". `session_request` and `preview_request` set
it and leave `[tests]` as the policy has it. `session_request` still
removes `not_run`, since a command's exit is its own. A combination's
run, in `step.rs`, sets `None`.

`Checks::start` equips by `tests`' runner and runs by `command` where it
is set. A run with a command plans as the command runner plans today.
Its one check is the command, it is never replayed, and its `stamps` are
proto's where the working copy holds a `.prototools`. `Meta::runner` is
`"command"`, so its results, progress, logs and keys are read as before.
The release command is built as before.

`FakeTests::start` reads `request.command` where it reads the runner and
command settings today. The tests that assert the overwritten runner
assert the command and the policy's runner instead.

The path not taken: the policy's runner kept under another `[tests]` key
beside the written `runner = "command"`. `[tests]` is the runner's
settings, which the core passes on unread (52), so a policy could write
that key too. Every reader of `[tests]` would then have to tell sluice's
own key from the repository's, and the table would carry a run's own
state.

### `sluice equip`

**The core.** `crates/sluice-core/src/equip.rs` gains
`Sluice::equip(branch, at: Option<&Rev>, working_copy: &Path) ->
Result<Result<Equipment, EquipRefusal>>`:

- With `at`, it reads the policy at that revision, and nothing of the
  branch. Without it, it reads the branch's current revision as
  `session_plan` does, and refuses `NoBranch` where there is none.
- It refuses `NotManaged` where the revision has no policy, or its
  policy names no `[branches]` table for the branch.
- It asks `seams.tests.equip` through `Running` with the policy's
  `[tests]`, and answers `Equipment { branch, revision, equipped }`.

It writes nothing and takes no lock: it is no step of a branch (rule
core.14), and nothing it does is the records' (rule core.11). It need
not check that the branch has landed the revision. A preview must (rule
core.19) because it shows a person what landed, while an equip decides
and shows nothing.

**The command.** `crates/sluice/src/equip.rs`, dispatched from
`lib.rs`'s `Command::Equip { branch, at: Option<String>, json }`:

1. The working copy is the top of the git worktree that holds the
   folder the command runs in, so `.prototools` and the toolchain are
   looked for where the repository keeps them. `exec::Host` gains
   `working_copy(dir) -> Result<Option<String>>`. `Bound` answers it
   with `git rev-parse --show-toplevel`, run in the folder with `GIT_DIR`
   and `GIT_WORK_TREE` removed, canonical as `Host::worktree` gives a
   path. The default answers `Host::worktree(dir)`, so the tests' hosts
   need nothing new. A folder in no worktree, a bare repository's among
   them, is refused: `refused: <folder> is in no worktree`.
2. `--at` is resolved with `Host::revision`. A name that is no commit
   here is refused: `refused: <at> names no revision here`.
3. `Sluice::equip`. `NoBranch` is refused with `crate::no_branch`'s
   words and `sluice overview` as the next step. `NotManaged` is
   refused with `refused: no [branches] table in .config/sluice.toml at
   <revision> names <branch>`.
4. Each refusal goes through `told::tell` as `Kind::Refused`. That is
   `{"error": {"kind": "refused", "message", "next"}}` under `--json`,
   with no `steps`, and exits 2 (`CONFLICT`), as `sluice exec`'s
   refusals do. An `Err` is sluice's own failure and exits 3 through
   `main`'s `tell_json`, its document's `message` holding the error.

Without `--json` it prints `equipping <working copy> for <branch> at
<short revision>`, then a line per step, `<step>: passed` or `<step>:
failed`. A failed step's words follow on the lines after it, indented
two spaces. Where there was no step it prints `nothing to equip:
<branch>'s policy names no toolchain, and its runner nothing to set
up`.

With `--json` stdout holds one document, `{"branch": …, "revision":
<full>, "steps": [{"step", "passed", "words"}, …]}`, and stderr nothing.
flume's change reads `steps` alone. `branch` and `revision` beside it
say which policy was read.

It exits 0 (`LANDED`) when every step passed, none included, and 1
(`FAILED`) when one failed.

It goes through `Bound::open` as every command does, under the
command's retry policy. No `Host::lock_sessions`, no `begin_session`,
no `Checkout::adopt` and no admission are reached from it (rule
core.18).

The path not taken: refusals exiting 1 like a failed step. flume reads
an answer with no `steps` as one failed step, whatever the exit, so the
exit only serves a person or a script. A refusal exits 2 throughout
sluice (`told::Kind::Refused`).

### A step's words

- What the step printed, its output and then its error stream, each as
  text, trimmed. The toolchain's are its error stream alone, since its
  output is the script.
- A step that failed is given sluice's reason first, then `: ` and what
  it printed where it printed anything. The reason is `exited <code>`, or
  `ended by signal <n>`, or `could not be run: <the error>` where the
  program could not be started. The version check's reasons are its own:
  `moon --version said <what>` where the version cannot be read, and
  `<version> is below moon 2.6, the oldest the moon runner reads`.
- Nothing is cut. flume cuts a step's words as it cuts any step's, and a
  could-not-start reason keeps the end of what it is given (`START_SAID`).

A run whose equipping failed fails its start with `<step>: <words>`. A
combination's run then reads, through `could_not_start`, `could not
start: toolchain: exited 1: error: flake.lock is not up to date`.
Today's reads `could not start: [tests] toolchain nix develop exited
exit status: 1: error: flake.lock is not up to date`. The step's name
replaces the command line, which for the runner's steps was the
`bash -c` line that sources the script.

### Tests, and where each scenario is held

- **The profile, with stand-ins.** `crates/sluice/tests/checks.rs`, in
  the fast tier, on a directory with a stand-in `proto` and `moon`
  written by the test. The stand-in `proto install` writes the stand-in
  `moon`, and each logs its arguments to a file the test reads. A
  toolchain the test writes puts both on the path. These need neither
  real tool nor git. They hold: the order and the stop at a failed
  step; `Reading` running no setup; the command runner's toolchain
  alone; no steps at all; the words; the environment cleared; the script
  gone once `equip` answers; and through `Checks::start`, a
  combination's run set up only where it affects a check, and a run
  with a command equipped through `Running`.
- **moon itself.** `crates/sluice/tests/profiles/moon.rs`, in the tools
  tier: `an_old_moon_is_refused` reads the version step's words.
- **The command over the fakes.** `crates/sluice/tests/equip.rs`, in the
  fast tier: the refusals and their exits, the revision read with and
  without `--at`, both forms of output, and the request the fake was
  asked.
- **S72 on the binary.** `crates/sluice/tests/equip_tools.rs`, in the
  tools tier, as `exec.rs` is, since it needs git's worktrees. A
  repository is committed with a `.prototools` and a policy whose
  toolchain is a script outside it that puts the stand-ins on the path,
  the stand-in `moon` absent until the stand-in `proto install` writes
  it. On it run `sluice exec main -- moon run app:test`, a `sluice
  preview`, and `sluice equip main` in a worktree sluice did not make.
  Each log shows the install, the version check and the setup before
  anything else, and after `sluice equip` the host holds no record and
  no ports for the worktree, and `sluice release` there says `nothing to
  release`. `crates/sluice/moon.yml`'s `tools` task gains `--test
  equip_tools`.

## Where the code shows the design wrong

The design is built as it stands, with these four points read as
follows. Each is for the orchestrator to carry to the blueprints chore
writing the clauses.

1. **147 is placed in B.1, the place.** Decision 7 adds 147 to B.1 after
   35, and 33's materialize gives a place "equipped (147)". In the code
   the sequence is the check profile's: the toolchain and the runner are
   `[tests]`'s, which the check profile reads, and the place profile runs
   nothing in a working copy (record 0013). A place profile that ran them
   would call another profile (rule core.2). This change equips through
   the check seam. 33's "equipped" holds as its "environment the
   repository declares" holds today: the core materializes, and the
   check equips before anything runs. 147 would sit truer in B.2, or 33
   could read "equipped by the check (147)".
2. **147's stop for a working copy only read.** 147 has "a submitter's,
   which last is read before any setup". The code reads two more working
   copies before any setup: the affected reading's tree (140), and a
   combination that affects no required check. The design's table lists
   the reading tree as equipped by "sluice's equip" with no stop. This
   change reads 147 as "a working copy sluice only reads in is equipped
   up to the runner's version check", and the clause could say so.
3. **84's amendment calls the toolchain the runner's own.** Decision 7
   has "What equips a place is the runner's own (147): the toolchain the
   policy names, then the runner's install and setup. It is never a
   command the repository writes, and never a probe." `[tests] toolchain`
   is a command the repository writes. This change reads the sentence as
   ruling out a command the repository writes to ask whether the host can
   run its checks, which is 84's own subject. The clause could read:
   "Beyond the toolchain the policy names, which prints the environment
   and decides nothing, what equips a place is the runner's own (147)
   …".
4. **There are four variants, not three.** "What each tool makes" names
   `Moon::plan`, `kept_out` and `preview_request` as three variants of
   one sequence. `Moon::reads`, the affected reading (140), is a fourth,
   installing and checking the version for itself. This change routes it
   through `Equipping` with the rest. This one changes no clause.

## Risks / Trade-offs

- **Every preview and `sluice exec -- <command>` of a moon repository
  takes longer**, by the time `proto install` and `moon setup` take to
  find nothing to do on a host that has them. → The build measures it
  on this repository and says so in the group's commit. The time counts
  against the bound as the run's start always has (71).
- **A preview or a session's command that passed may now fail at a
  step**, where a repository's `.prototools` pins what proto cannot
  install, or its setup fails. Its combinations already fail so, so the
  failure is the branch's own, said with the step and its words.
- **The step names are the moon profile's words.** flume shows them as
  its readiness steps. Renaming one changes what a person reads there,
  and the JSON's shape does not change with it. → The names are written
  once, in `moon.rs`, and the tools tier holds them.
- **Two equips install into one proto home at once**, in two working
  copies of one repository. That is what two runs' starts do today,
  and proto's install is its own to keep whole.
- **The start error's words change** for a combination whose toolchain
  fails, from the command line to the step's name. A person reading
  status loses the toolchain's command, which the policy names.
  `crates/sluice/tests/checks.rs`'s test of those words is updated.

## Migration Plan

Nothing stored changes shape. A run started by an older sluice keeps its
`Meta`, whose `runner` is read as before. A request is built and
started in one process, never stored. The release that carries this
change is what flume's pin moves to. Until flume's change is built,
flume's own copied steps keep making its worktrees ready, and `sluice
equip` serves any caller that runs it.

## Open Questions

None for the build. The four points above are the statement's to
settle, and the build goes on under the readings given there.
