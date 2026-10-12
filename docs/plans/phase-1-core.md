# Phase 1, pass 1: the core and its conformance suite

A brief for one agent, written so it needs nothing from the conversation
that produced it. Read it whole before starting.

## What sluice is

A merge queue for AI agents working in git worktrees. Agents submit their
work to a target branch and never merge it. sluice queues submissions, tests
each one combined with everything queued ahead of it exactly as it would
land, merges it only if that combination passes, and when a combination
fails names the one submission that broke it and tells only that agent.

## Read first, in this order

1. `docs/examples/README.md` (this repo): what using sluice looks like. The
   config, the CLI output, the state files and the landing commit there are
   the target. Where this brief and the walkthrough differ, ask.
2. `agentplot/blueprints:design/sluice/requirements.md` (local:
   `~/Code/github_agentplot/blueprints/main/design/sluice/requirements.md`):
   the statement of invariants. Every behaviour you build satisfies a
   numbered clause; cite it.
3. `blueprints/main/design/sluice/models/reconstruction-1.md` and
   `reconstruction-1-answers.md`: a model of sluice derived from the
   statement, and the operator's rulings on its 25 questions. Use the model's
   structure (records, lifecycles, one step function, outbox, landing intent)
   unless a ruling or the walkthrough says otherwise.
4. `CLAUDE.md`, `docs/architecture/core.md` (rules core.1 to core.16) and
   `docs/adr/0001` to `0010`. The rules bind every diff.

The statement's vocabulary is for design (line, entry, place, candidate,
culprit, seam). Everything a user sees, config keys, CLI output,
state file fields, error messages, uses the walkthrough's words: branch,
submission, checkout, combination, blame. Code may use either; be
consistent within a module.

## Scope of this pass

Build the core and prove it with a conformance suite, against in-memory
test doubles of every external tool. No real git, moon or worktrunk is
called in this pass. Alongside, write down exactly how each real
integration will work, so pass 2 can be split among several agents.

In scope:

- A Rust workspace (edition 2021, like flywheel-next) with at least:
  - `sluice-core`: the domain types, the policy, the step function, the
    chain's order, attribution, landing choice, window. Pure: no I/O but
    through the seam traits.
  - `sluice-seams`: one trait per seam, exactly five: checkout (the place:
    materialize, release), tests (the check: affected, run), branch (the
    line: read, form, commit, move), driver-facing operations (submit,
    check/forecast, withdraw, amend, state, notify), and state (the records:
    read with a version, write conditional on it). Rule core.2: a profile
    implements a trait and nothing of the core.
  - `sluice-fakes`: in-memory implementations of all five, scriptable per
    test (this combination fails this task; this merge conflicts on this
    path; the branch was moved by hand; the process crashed after the
    landing intent).
  - `sluice-conformance`: the suite.
  - `sluice` (binary): the CLI skeleton, `submit`, `step`, `run`, `status`,
    `amend`, `withdraw`, with `--json`, wired to the core but only to fakes
    or stubs for now. `wt-submit` as a second binary forwarding to
    `sluice submit`.
- The policy: parse `.config/sluice.toml` exactly as in the walkthrough
  (`[tests]`, `[checkout]`, `[branches."<pattern>"]` with `required`,
  `parallel`, `batch`, `flaky`, `timeout`), read from the target branch's
  revision through the branch trait (56).
- The state files: the JSON and JSONL shapes in the walkthrough section 7,
  with a schema version in every document, a version number bumped on every
  write, and the per-repository results log (69). Serialize and parse them
  for real (to an in-memory store in the fakes, but the bytes are the real
  format), so pass 2's file store only writes them to disk.

Out of scope for this pass: real git, gix, moon, worktrunk, process
groups, the protection hook, several machines, the shared line's trust
rules beyond `required` and `flaky`.

## The conformance suite

Each scenario is a test that cites the clauses it exercises in a
`satisfies:` list (a doc comment or an attribute), and the suite has a check
that fails when a cited number names no clause in the statement or when a
phase-1 clause is cited by no scenario. Phase-1 clauses are those of Part A
and B except 57 to 63 (several hosts) and 28, 29 (trust).

Cover at least the statement's S1 to S4, S7 (bolt branch half), S8, S10,
S11, S16, plus:

- a resubmission withdraws the earlier submission (67);
- priority never displaces a combination under test (6);
- unrelated work is tested in one chain and not tested again when the
  submission ahead lands (8, S6);
- batch mode: the longest passes and all land on its result; the longest
  fails and the shorter ones are tested to find the culprit (14, S9, 23);
- a timeout is recorded as timed out and read as failed (66);
- a crash between the landing intent and the move is finished correctly by
  the next step (24, 59);
- a notification repeated after a crash carries the same id and outcome
  (51);
- a conflict names every submission it is contested with (65);
- `amend` changes order only for a submission not under test (47b, 6).

Run `cargo test --workspace` and `cargo clippy --workspace --all-targets
-- -D warnings` green before you report.

## Lining up pass 2

Write `docs/plans/phase-1-integrations.md`: for each real profile, the exact
calls and the facts they rest on, verified against the installed tools
(`git 2.54`, `moon` on PATH, `wt 0.80`), not from memory. Each section must
be a task one agent can take alone.

- **Branch, through gix and git.** Which gix 0.87 APIs read refs, write a
  commit with two parents and trailers, and move a ref only from an expected
  old value (flywheel-next's `crates/flywheel-store-git` uses gix 0.87; read
  it). `git merge-tree --write-tree` for forming: its output on success and
  on conflict, and how conflicted paths are parsed (ADR 0009).
- **Branch protection.** A `reference-transaction` hook that refuses
  updates to protected branches unless made by sluice: how sluice marks its
  own updates, how the hook is installed without clobbering an existing
  one, what it prints.
- **moon.** How to ask which tasks a revision range affects (dependents
  included), how to run a set of targets in a checkout, where the run report
  and per-task run records are and what fields give each task's outcome,
  hash and whether it replayed. Probe a real run in crew
  (`~/Code/github_afterthought/crew/main`, tag `#suite`) and record the
  actual JSON.
- **Checkouts.** `wt switch --create sluice/try/<n> --base <commit> --no-cd
  --yes` and `wt remove`, `git worktree add --detach`/`remove`, `git
  archive`; what each leaves behind, and the env vars sluice sets
  (`SLUICE_CHECKOUT`, `SLUICE_TMP`).
- **Running tests.** One process group per run (setsid), killed with TERM
  then KILL on exit, timeout or checkout removal; how the timeout is
  enforced.
- **State on disk.** `<git common dir>/sluice/<branch>/`, atomic writes by
  rename, the per-branch lock (flock), the version check.
- **The CLI and `wt-submit`.** Waiting, exit codes (0 landed, 1 failed, 2
  conflict), `--no-wait`, `--check`, the dirty-worktree refusal (2).
- **End to end.** A scratch repository script that sets up a bolt branch,
  three worktrees and a moon workspace, and drives S1 to S4 for real; then
  the same against a throwaway clone of crew.

## Working rules

- Work in a worktree off `main` of this repository (`wt switch --create
  phase-1-core`), never in `main` itself.
- Commit as you go, by explicit paths, Conventional Commits, scopes by
  crate (`feat(core): …`), citing clauses and rules in the body. Never push.
  End each commit message with the attribution line your session provides.
- A behaviour the statement does not settle is a question, not a guess:
  write it to `docs/plans/phase-1-questions.md` with the clause it sits
  beside, choose the reading the walkthrough implies, and carry on.
- When done, report: what was built, the test count and output, the
  integrations doc, and the questions file.
