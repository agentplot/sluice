---
number: 73
title: The readiness probe is the branch's, run at the branch's current revision and never in the candidate's tree
status: proposed
date: 2026-10-30
decision-makers:
- Chuck Swanberg
---

# The readiness probe is the branch's, run at the branch's current revision and never in the candidate's tree

Status: proposed to the user on 2026-10-30, as `briefs.15` has it; accepted only on their yes.

Deciders: swancloud-design's reading of 2026-10-30, sent through madswan-operator and recorded as signal 2026-10-30-madswan-operator-880be251/01: "The failure is the candidate's, not the host's, and the words say so (a module not found under the repository's own path); sluice could read a ready failure whose words name the repository's tree as the newest submission's, or run ready against the base's tree." The choice between those two is madswan-design's; the user's yes is asked on this record's card.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 16, 17, 18, 19, 56, 60, 66, 84, 114, 115, 124, 139, the policy's `[tests] ready` and its comment, S43 and S67 (blueprints 0edc91c); records 0031, 0043, 0066, 0072; rules core.7, core.15, core.20; `crates/sluice/src/profiles/checks.rs` (`ready_after`, run in the request's working directory, the run's place, with the run's environment; the `unpassed` filter before it), as origin/main held it on 2026-10-30 (d452b1db); WilldanGroup/switchboard-kit on chuck-herdr-alpha under sluice 0.58.0, bolt a40c598 removing `infra/ministack/host-ready.ts` and editing `.config/sluice.toml` in one change, entry #1005 and its combination.

## Context and problem statement

Record 0066 gave the readiness probe one job: after a run in which a check did not pass, ask whether the host still has what the checks need, so that a host's fault is told from the code's; a host that lost it makes the run one with no outcome, run again and blamed on nobody (84, 114), and three such runs in a row read the candidate as not passed (115). The probe is the repository's own command, `[tests] ready`, read with the policy at the branch's current revision (56), and today it runs in the run's place: the candidate's tree.

On 2026-10-30 in wldn a bolt removed the file main's probe runs and named a new probe in the same change. Every combination holding that bolt failed a check, and the probe then failed with a module not found under the repository's own path, because the candidate's tree no longer held it. sluice read that as the host not ready: nobody blamed for three runs, the other submissions in the combination held with it, and the words on every run naming the host. The failure was the candidate's. A probe that runs in the candidate's tree answers a question about the candidate as often as one about the host.

## Considered options

1. **Read the probe's words.** A readiness failure whose words name the repository's tree is read as the newest submission's. Against 139's principle and record 0066: sluice infers nothing from what a check printed, and a probe's words are its own, different for every repository and tool.
2. **Run the probe at the branch's current revision** (chosen). The probe asks about the host, so its command and the tree it runs in are the branch's, as its policy already is (56): the run's place is brought to the branch's revision once the run is over and before the probe, keeping the toolchain the place entered, so what the probe needs installed is there. A candidate that changes or removes what the probe runs changes no attribution before it lands, and the new probe is asked from the next landing on, as a policy change is.
3. **Run the probe twice**, in the candidate's tree and then the branch's, blaming the candidate when only the first fails. Two runs to learn what one at the branch's revision says, and a second tree to make.
4. **Declare the probe's exits**, the policy naming which exits mean the host is not ready, every other exit being the probe's own failure. Another field to keep, and a probe's own failure still has to be read as somebody's: the candidate's only by inference.
5. **Leave it**: 115 converges after three runs, attributing the candidate. Three runs and misleading words for a fault the candidate's own change made.

## Decision outcome

Option 2.

**The probe is the branch's.** Its command is read with the policy at the branch's current revision (56), and it runs in a tree of that revision: the run's place, brought to the branch's revision once the run is over and before the probe, the toolchain the place entered kept (84 as amended). A place is sluice's own, made for the candidate, and its run is over when the probe is asked; moving its head then breaks nothing the run recorded and makes no second tree. What the probe says of the host is then the branch's own probe's word, the one the policy's author wrote for this branch, whatever the candidate did to it.

**A candidate that changes the probe changes nothing before it lands.** Removing, moving or rewriting the probe's file, or naming a new probe in the policy, takes effect at the landing, as every policy change does (56, core.15). Until then the branch's probe answers, found in the branch's tree. S67 shows wldn's case under this record: the fail is recorded under its key and the submission attributed at the first run.

**A session's run keeps its own tree.** A session's worktree is not sluice's to move (124, core.20), and a session's run decides no verdict (S43): its probe runs in the session's tree as today, and a session that broke its own probe reads its fails as not recorded, which is the safe side for a run nothing lands on.

**Why not the others.** Option 1 infers from words. Option 3 pays twice. Option 4 adds a field and still infers. Option 5 keeps the fault's cost at three runs and blames the host in every word.

### What could go wrong, and what holds it

- **A place brought to the branch's revision with the candidate's build outputs in it.** The probe asks the host, not the tree's build; outputs the candidate left under ignored paths change nothing the probe reads of the host. A probe that reads the tree's outputs is asking about the candidate, which is a check's job (0066).
- **The branch moved while the run went.** The branch's current revision is the one the candidate was formed on (6, 10), the revision the policy was read at; the place is brought to that revision, not to a newer one, so the probe and the policy agree.
- **The probe's own dependencies.** The place's toolchain stays entered, and the branch's revision holds the probe the toolchain was set up for; a probe whose dependencies the candidate removed from the toolchain's manifest is asked at the branch's revision all the same, where the manifest is the branch's.

### Consequences

- Clauses 84 and 114, the policy's `ready` comment (which also loses the probe before a run, gone since record 0066) and scenario S67 are amended in the statement at blueprints 0edc91c; rule core.36 is added, marked proposed; `_open.md` names this record until it is built.
- One unit on the yes, citing this record: the readiness probe runs at the branch's current revision, the candidate's place brought to it after the run, a session's run keeping its tree.
- wldn's bolt keeps its old probe as a stub until main's policy names the new one, as wldn-design ruled; under this record no stub is needed.
