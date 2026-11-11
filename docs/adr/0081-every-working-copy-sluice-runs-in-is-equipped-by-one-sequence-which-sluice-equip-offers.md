---
number: 81
title: Every working copy sluice runs in is equipped by one sequence, which sluice equip offers for a working copy it did not make
status: accepted
date: 2026-11-05
decision-makers:
- Chuck Swanberg
---

# Every working copy sluice runs in is equipped by one sequence, which sluice equip offers for a working copy it did not make

Status: accepted by the user on 2026-11-05, in approving flume's design of that day for how a working copy is made ready across the tools (flume's change `a-places-readiness-is-one-design-across-the-tools`), whose decision 4 gives `sluice equip` as this record does and whose decision 7 words clause 147, S72 and the amendments of 33, 84, C.1's git worktree row, C.2's moon row and section 9 for agentplot/blueprints. The choices within it below are the construct stage's own for the change `sluice-equips-a-working-copy-as-it-equips-its-own-places`: where the sequence stops for a working copy sluice only reads in, how a run's command is told apart from its runner, the words each step gives, the document `--json` prints beside its steps, and the exit of a refusal. Each is cheap to change until a release carries it, and the user sees them at that change's review.

Amended in part by record 0112, 2026-11-11: `sluice release` after an equip finds nothing of the equip's, and runs the policy's release command as in any worktree sluice holds nothing for. It says there is nothing to release only where the policy names no release command.

Deciders: the user, approving the design named above on 2026-11-05; the construct stage of the change named above, for the choices within it.
Sources: statement clauses 31, 33, 35, 56, 71, 84, 89, 95, 98, 114, 123, 140, 147, C.1's git worktree row, C.2's moon row, section 9, S72, as decision 7 of flume's design words them; that design's "What each tool makes", read at sluice 571c83f6; on sluice's main at 6b29c5ce, `crates/sluice/src/profiles/moon.rs` (`plan`, `kept_out`, `reads`, `install`, `check_version`), `crates/sluice/src/profiles/checks.rs` (`start`, `kept_out`, `reads`, `toolchain`), `crates/sluice-core/src/preview.rs` (`preview_request`) and `crates/sluice-core/src/session.rs` (`session_request`); records 0013, 0016, 0018, 0045, 0050, 0066, 0074; deneb, 2026-11-05: a worktree flume made on a fresh clone of a moon repository failed at `moon setup`, since the moon its `.prototools` pins was never installed, flume's copy of sluice's steps having left the install out.

## Context and problem statement

Before sluice runs anything in a working copy, it prepares it. It runs `[tests] toolchain` there and keeps what it prints as the environment of every command after it. Under the moon runner it then installs what the working copy's `.prototools` pins, refuses a moon below 2.6, and runs moon's setup, which installs the workspace's toolchains. These steps run outside the executor, where a download and a write to the user's home are allowed, so the run inside only uses what they installed.

That preparation is written four times in sluice, and the copies differ:

- a combination's run, and a session's run of the checks: the toolchain, the install, the version check, moon's queries, and the setup where anything is to run (`Moon::plan`);
- a submission's read of which required checks moon keeps out of CI (record 0050): the toolchain, the install, the version check and the query, with no setup (`Moon::kept_out`);
- the affected reading (140): the same, in a tree of its own (`Moon::reads`);
- a preview's command (98) and `sluice exec <branch> -- <command>` (89): the toolchain alone, since each puts the command runner in place of the policy's runner (`preview_request`, `session_request`).

So on a host where nothing has installed what a repository pins, a preview of a moon repository, or `sluice exec main -- moon run app:test`, fails where the branch's own runs pass. And a tool that makes working copies of its own cannot ask sluice for the same preparation. flume copied sluice's steps into its own readiness, left the install out, and its worktree on a fresh clone failed at `moon setup` on deneb on 2026-11-05. The fix copied the install too, deciding from the pins at the base where sluice decides from the working copy, and it still lacks the version check.

How does sluice prepare a working copy, and how does a tool that makes working copies of its own ask for the same?

## Considered options

1. Keep the copies, and write down which steps each path runs.
2. One sequence for every working copy sluice runs anything in, run through one function of the check profile, and offered as a command, `sluice equip <branch> [--at <revision>] [--json]`, for a working copy sluice did not make.
3. The sequence as a library that other tools link.
4. The repository's toolchain installs what the repository pins when it is entered, so sluice and every other tool need only run it.

## Decision outcome

Option 2.

**One sequence.** Equipping a working copy is these steps, in order. First the toolchain the policy names is run in the working copy, and what it prints is kept as the environment of the steps after it. Then come the runner's own steps. Under moon they are `proto install --config-mode upwards` where the working copy holds a `.prototools`, `moon --version`, refused below 2.6, and `moon setup`. Under the command runner there is nothing more. Each step runs outside the executor, with a run's environment: what sluice keeps of its own, the working copy's paths and the host's caches' variables (35). Each gives its outcome and its words, and the sequence stops at the first that fails (147). The sequence belongs to the check profile, since the toolchain and the runner are `[tests]`'s settings. The place profile makes a working copy and runs nothing in it (record 0013), and the core names no step (rule core.1).

**Where it stops.** A working copy sluice only reads in, to ask the runner a question, is equipped up to the version check. That covers a submitter's worktree at `sluice submit` (record 0050), the affected reading's tree (140), and a combination's or a session's working copy while its plan is read. A working copy sluice runs a check or a command in is equipped through the setup: a combination's run and a session's run of the checks before they run anything, a preview's command and `sluice exec -- <command>` before the command, and a caller's `sluice equip`. A combination that affects no required check runs nothing, so it is read and never set up, as today.

**A command is equipped for by the branch's runner.** A preview's command and a session's command run in place of the runner's checks. The working copy is still equipped by the runner the policy names, so the command finds what a check would find. A run request carries its command beside the policy's `[tests]`, and no longer writes the command runner over the policy's.

**`sluice equip`.** Run in a working copy, `sluice equip <branch>` equips that working copy by the policy at the branch's current revision. With `--at <revision>` it uses the policy at that revision, as a preview reads the previewed revision's `[tests]` (record 0045), and reads nothing of the branch itself. Equipping decides nothing, so the revision need not be one the branch has landed.

It adopts nothing. It holds no ports, writes no record of the working copy, takes no part of the host's bound and waits for no room in it, since it runs no check (71). Adopting a worktree is `sluice exec`'s alone (record 0018), and a `sluice release` after an equip says there is nothing to release. It keeps nothing: the toolchain's script is in a directory of its own, removed when the command answers.

It prints each step and its outcome, each on its own line. With `--json` it prints `{"branch", "revision", "steps": [{"step", "passed", "words"}]}`, `revision` being the one whose policy it read. It exits 0 when every step passed, and when there was none to run, and 1 when one failed. A refusal prints `{"error": {"kind", "message", "next"}}`, with no steps, and exits 2, as `sluice exec`'s refusals do. It refuses a branch or a revision it cannot read, a revision that holds no policy, and a folder in no working copy. At the branch's current revision it refuses a policy that does not manage the branch. With `--at` it does not: it equips by the revision's `[tests]` whether or not its `[branches]` names the branch, on the words of 2026-11-05 that approved the change's design: "Under --at, refuse only a revision with no policy, and equip from [tests] whether or not [branches] names the branch: Flume readies places on bases the policy may not manage, and its tools test writes a policy with no [branches]."

**The words.** A step's words are what it printed, its output then its error stream, trimmed. The toolchain's words are its error stream alone, since its output is the environment it hands on. A step that failed is given sluice's reason before them: `exited <code>`, `ended by signal <n>`, `could not be run: <why>`, or the version check's own words. A combination whose working copy could not be equipped could not start, and is recorded so, with the step's name and words, every check not run and nobody blamed (114), as a toolchain that fails is today.

**Not readiness.** Equipping is no readiness command (84; record 0066; rule core.37). What it runs is the toolchain the policy names, which prints the environment, and the runner's own install, version check and setup. It never runs a command the repository writes to ask whether the host can run its checks, and it runs before every run whatever it would answer. What a check needs beyond its tools, such as a stack, a database or a container engine, the check brings up itself (31, 84). A container engine the policy asks for stays a requirement the executor reads from the host (record 0074; rule core.38).

Option 1 keeps three copies inside sluice and a fourth outside it, each found wrong only when a working copy fails, as deneb's worktree was. Option 3 would build sluice's internals into another tool's binary at whatever version its build resolved, where every other use of sluice by another tool goes through the command line of a release that tool pins. Option 4 makes the install a command the repository writes, which 84 rules out, and the toolchain's script is sourced inside the executor, where nothing may be installed.

### Consequences

- `sluice equip` is new. The check seam gains `Tests::equip`, and a run request gains the command a run runs in place of its runner's checks.
- A preview and `sluice exec -- <command>` of a moon repository gain the install, the version check and the setup. Each takes longer by them: on a host where everything is installed, the time `proto install` and `moon setup` take to find nothing to do.
- A combination whose working copy could not be equipped is recorded as could not start, naming the step with its words, where today it names the command line that failed.
- The release command still runs under the toolchain alone (95), since the working copy was equipped by the run it releases, and the release of a working copy that is gone enters no toolchain (123).
- Rule core.46 binds the sequence, and the details of rules core.37 and core.38 say it is neither a readiness command nor a requirement of the host.
- A tool that makes working copies of its own runs `sluice equip` in each, at the revision it made it from, and reads its steps.
