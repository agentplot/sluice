---
number: 72
title: A check's key holds its executor's grant, and a check that failed on what its executor refused has no outcome
status: accepted
date: 2026-10-29
decision-makers:
- Chuck Swanberg
---

# A check's key holds its executor's grant, and a check that failed on what its executor refused has no outcome

Status: accepted by the user, 2026-10-29, in session.

Deciders: the user, 2026-10-29, in session, on Flume's submission 45: (a) a check's key includes its executor, so runs under executors that differ in what they allow are never compared as one key; (b) a run whose tests fail on a sandbox denial is recorded as could-not-run, not as a fail; both, as the user chose.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 16, 19, 38, 60, 83, 84, 108, 114, 115, 116, 138 and 139 (added 2026-10-29), C.2, S66 (blueprints 50a1c94); records 0004, 0028, 0031, 0032, 0042, 0043; rules core.7, core.9, core.18, core.22; `crates/sluice/src/profiles/checks.rs` (`seatbelt`, `sandbox_profile`, the `Enforced` it builds), `crates/sluice-core/src/verdict.rs` (`read`, `same_limits`), `crates/sluice-core/src/records.rs` (`ResultLine`), `crates/sluice/src/signing/statement.rs`, `crates/sluice/src/profiles/moon.rs` (the hash from moon's `hash-generation` operation), as origin/main held them on 2026-10-29 (f7d7261f); sluice v0.57.1 (#289), which lets a sandboxed command open `/dev/ptmx` (C.3 as amended, blueprints 3990e10).

## Context and problem statement

On 2026-10-29 Flume's fix branch was run with `sluice exec main` by the host's installed sluice 0.56.0. Its Seatbelt profile let no command open `/dev/ptmx`; v0.57.1 allows it. All 29 of `flume:tools`' herdr tests failed with "failed to openpty", so none of them judged anything. sluice recorded the run's fails under each test's key, the hash moon computes from the task's inputs. The queue's run of the same head then passed 45 of 45, under Flume's toolchain's sluice 0.57.1. Each key held a pass and a fail, main reads a flaky key as failed (`flaky = "fail"`, 19), and submission 45 failed. The only ways forward were the operator's set-aside, which the user will not make, or a new commit that changes the tests' inputs.

Two things went wrong. The run's fails were weighed against a pass from an executor that let the run do more: same executor kind, same network, same container engine, so `same_limits` read the two runs as one, though their Seatbelt profiles differed. And a run the sandbox stopped was recorded as a fail at all: record 0031 and rule core.7 already say a run that did not judge its check has no outcome, but the check seam had no way to tell that the executor, not the code, made the test fail.

## Considered options

1. **Set-aside as the only remedy**, as today (116). Each set-aside is meant as a sign that sluice recorded what it should not have; here sluice can tell the cause itself, as record 0043 says of a time limit. And the user will not make it.
2. **A flaky window per host**: weigh a key's results only within one host, or one host's last hours. It separates hosts that run the same executor, which should agree, and keeps together runs on one host whose sluice was upgraded between them, which was this incident's case read the other way round (two sluices on one host).
3. **Run locally with nothing recorded**: a session's runs (`sluice exec`) record nothing. It loses every honest result of a session run, which session-runs-1 recorded on purpose, and the queue's own runs can meet a narrower executor just as well.
4. **The executor's grant in the key, and the executor's own report of what it refused read after a run in which a check did not pass.** The user's choice, both parts.

## Decision outcome

Option 4.

### What the grant holds

A run's record already carries what its executor enforced (`Enforced`: kind, confined, cores, memory, network, container engine), and the gate already weighs a key's results only among runs held to the same limits (19, 83; `same_limits`). The grant is one more field of it, compared the same way, on every branch, signed or not: what the executor let the run reach, named the same wherever the same executor gives the same.

- **Sandbox**: the digest of the Seatbelt profile sluice writes for the run, rendered with every path named by its role instead of its place: `<place>`, `<scratch>`, `<cache>`, `<cache-socket>`, `<engine-socket>`, `<user-tmp>`, the runner's stamp names as written, each rule once. The profile is the precise thing: it is what the kernel enforced, and the incident's two runs differed in nothing else that `Enforced` records. Roles, not paths, because the profile names the place's checkout and scratch, which differ for every place, and the host's cache directories and sockets, which differ for every host; a grant over the raw text would make every run its own key and no replay would ever be proven. Collapsing each role to one rule makes two hosts with two and three cache directories one grant, as a cache changes how fast a check runs and never what it may do (35).
- **Open**: one constant grant. The open executor holds nothing, so two open runs differ in nothing it gives.
- **Container**, when built: its image's digest with the mounts, devices and network it gives, each mount by its role. **Launched**: the grant the job's signed record says.

**Not sluice's version.** Two sluices that write the same profile give the run the same thing, and should be weighed together; a release that changes the profile changes the grant by itself. Putting the version in the key would make every release, even one that touches nothing of the executor, a new key on every host, so no replay is proven across hosts on two patch releases and each upgrade reruns every check. What a sluice release changes outside the executor is not a grant's to catch; the second part of this record catches the case that matters, a run the executor stopped.

Written as `grant` on the recorded `Enforced`, a hex digest with a prefix naming the scheme (`sb1:`, `open:`), so a later change of the rendering is a new grant and never a collision.

### How it fits what is already there

- **Replays (core.9, 108).** A replay counts only when a trusted pass is recorded for its key; the key now holds the grant, so a replay under one grant is proven only by a pass under the same grant. Hosts that run the same executor share their passes as before. A replay with no such pass is unproven and the round runs it again with no replay, as today.
- **Signed runs (core.22, record 0032).** The grant is part of what the executor enforced, so it is in the signed statement's predicate with the other limits, and a signed run's grant is read from the signed statement alone. It needs no allowance of the signer's: it is an observation, as the network is, not a claim such as `confined`. Today's signed path reads a key's results by signer and system without comparing limits at all; the grant's comparison, with clause 19's limits, applies there too.
- **moon's cache keys.** moon's hash stays moon's key, for moon's cache. sluice does not feed the grant into it (an environment variable in the task's inputs): that would split moon's outputs per executor and per host for nothing, since what moon's cache supplies decides nothing (108). The grant lives in sluice's record beside the hash, and the key sluice reads is the two together.
- **Session runs (core.18).** A session run records under its keys as before, with its grant.

### Records keyed the old way

A line written before this record has an `Enforced` with no grant. It is read as run under a grant unknown: weighed with other lines that carry none, and with no line that carries one (138). So an older sluice reading the same records reads them as it always did; a new run's fails are never weighed against an old run's passes, nor its passes proven by an old run's. The cost is one more run of each check the first time each grant meets it, since every old pass stops proving replays. Flume's submission 45 needs no set-aside: its keys hold two lines with no grant; the next run, under a sluice that records grants, reads neither, and the same head passes.

### Recognising what the executor refused

Three ways were weighed:

- **A canary**: sluice runs probes inside the sandbox before the check, opening a pseudo-terminal, binding localhost, writing the scratch. It finds only what sluice already thought to probe. 0.56.0 would have shipped without a pseudo-terminal probe for exactly the reason its profile refused one; a canary catches the needs sluice knows and not the next one.
- **The executor declaring its capabilities**: a list of what the profile allows. It says what sluice believes, never what a test tried, and has the canary's blind spot.
- **The sandbox's own report of its refusals.** The kernel reports each Seatbelt denial to the unified log on macOS: the process, the operation and its path. It is observed (38), it needs no knowledge of what the tests do, and it names the next refusal nobody foresaw. This is the choice.

How it runs: the run's profile gives its denials a mark of the run's own, so a denial is told as this run's and not another's on the same host. After a run in which a check did not pass, under the sandbox only, sluice reads the log for the run's span and the mark, once, and keeps what it read in the run's directory, as it keeps `ready-after` (record 0031). If it found a denial, each check that did not pass is not run (114), with no result under its key, its reason naming what was refused (the operation and its path, the first few and the count); a check that passed keeps its pass. A run with no denial is read as before. If the log cannot be read, the run says so, in the log's words, and is read as one with no denial: the grant still keeps such a fail off any other grant's key.

Seatbelt's way of marking a denial as one profile's (a message the rule carries, or the run's processes) is settled by the change's first task, by probing; if no mark tells two concurrent runs apart, the span alone is used and this record is amended to say so. A denial taken for the wrong run of two at once makes that run's fails not run: they run again, and nothing is recorded wrongly.

**What it costs.** One `log show` over the run's span after a run that did not pass, seconds to tens of seconds, kept, so a run's progress read again pays nothing; a passing run pays nothing.

**Why a real failure is not hidden.** Only a run in which the executor reports a refusal changes; a test failing for its own reasons in a run with none is a fail as before. A run in which a test fails on its own and something else meets a denial makes that fail not run, so it runs again; one that meets a denial in every run, a tool that probes a path it may not write and goes on, has its combination read as not passed on the third such run in a row (115), with each test named and the denials beside it. The fail is told to the culprit, never folded into a pass, and the denial is named so the profile or the host's cache table can grant what the tool needs.

**Out of scope.** The Linux sandbox (bubblewrap and Landlock) and the container executor are not built; each reports its refusals and gives its grant when it is. Reading what a test printed ("failed to openpty") is never done. Failures of the host outside the sandbox, a disk full or a service down, stay record 0031's: `not_run` exits and `[tests] ready`.

### Consequences

- Clauses 138 and 139 are added to the statement; 19, 83 and 114 cite them; the vocabulary gains grant; C.2 says each executor's grant and where the sandbox reads its refusals; S66 (blueprints 50a1c94).
- Rules core.34 and core.35; core.7, core.9 and core.22 name them in their details.
- `Enforced` gains `grant`, written by every executor and read wherever the gate compares limits; `same_limits` compares it, and the signed path compares limits and grant as the unsigned one does.
- The signed statement's predicate carries the grant.
- The sandbox's run report gains the denials it read; `RunReport::not_run` carries the checks they leave not run. The core needs no change for them: it reads `not_run` as it does for record 0031.
- `sluice exec` and a submission's failure name what the sandbox refused and say that sluice's profile, not the test, stopped the run.
- Every check runs once more under its first grant, since no old pass proves a replay.
- The change `a-checks-key-holds-its-executor` builds it.
