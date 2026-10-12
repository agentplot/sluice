# sluice rules

The rules that bind wherever a change is made in this repository,
written so a reviewer can point at a diff and say which rule it breaks.
The narrative is the statement in agentplot/blueprints,
`design/sluice/requirements.md`; a rule here names the clause it
enforces and never restates it. There are no area pages yet; a rule
that binds only one part of the tree moves to an area page when that
part exists.

## Rules

### The core and its seams

- **[core.1]** Code outside a profile names no task runner, worktree tool, multiplexer, service manager or forge.
- **[core.2]** Every seam of Part B is reached through exactly one interface, and a profile implements that interface and nothing of the core.
- **[core.3]** A seam with a single profile has an ADR saying why one is enough.

### Branches and landings

- **[core.4]** No code path reachable from a submission writes a branch's ref.
- **[core.5]** A branch is moved only with the expected old revision, and a move whose expectation fails is reported, never retried blind.
- **[core.6]** A candidate is formed from the branch's revision and the entries ahead in its lane, in order, and from nothing else.

### Verdicts and attribution

- **[core.7]** Every check run is recorded with its key, outcome and executor before any verdict is read from it, a fail exactly as a pass.
- **[core.8]** A failure is attributed to one entry, and only after every candidate ahead of it has a verdict.
- **[core.9]** A replay is recorded as a replay, never as a run.

### Places

- **[core.10]** The core starts no service and binds no port; the checks it runs are one process group, ended with their place or their time limit.
- **[core.11]** Every record a reader may need is readable with no sluice process running, from every host that submits to its branch.
- **[core.14]** Nothing the core decides is held between steps outside the records, and no two steps act on one branch at once.
- **[core.15]** A branch's gate is read from the branch's current revision, never from a candidate or a place.
- **[core.16]** The core releases only places it made, and never a session's.

### Design and records

- **[core.12]** A change that bears on a clause cites the clause by number; a clause that no longer holds is changed in the statement first, never worked around here.
- **[core.13]** Nothing commercial, no pricing, tier or customer, appears in this repository.

## Details

- **core.1** Rules out: the strings `moon`, `wt`, `worktrunk`, `devenv`, `herdr`, `gh`, `crew`, `flywheel` in any module outside a profile directory; an import of a profile from the core. Not ruled out: `git`, which section 9 of the statement fixes. Source: statement preamble and clause 52; ADR 0003.
- **core.2** Rules out: a profile calling another profile; a core function with a profile-specific branch. Source: clauses 52, 53; ADR 0003.
- **core.3** Rules out: a seam whose only profile is undocumented as such. Source: ADR 0003.
- **core.4** Rules out: `update-ref`, `push`, a merge or a fast-forward anywhere a session's submit command reaches. Source: clause 1; ADR 0002.
- **core.5** Rules out: `update-ref` without an old value; a landing loop that re-reads and retries on refusal without re-forming the chain. Source: clauses 24, 45, 46.
- **core.6** Rules out: a candidate formed from a cached revision of the branch; a candidate including an entry removed as culprit or conflict; an entry of another lane. Source: clauses 10, 12, 21.
- **core.7** Rules out: reading a verdict from the task runner's cache alone; discarding a failed run's record; writing a record after the verdict. Source: clauses 16, 18; ADR 0004.
- **core.8** Rules out: attributing from a batch with no intermediate verdicts; attributing while a candidate ahead is still running; telling two sessions of one failure. Source: clauses 17, 18, 20.
- **core.9** Rules out: counting a cache hit as an executor's observation. Source: clause 16, 38; ADR 0004.
- **core.10** Rules out: starting an emulator, a server or a process manager from the core; computing a port; a check's process group left running after its run or its place. Source: clauses 31, 32, 66; ADR 0010.
- **core.11** Rules out: state held only in a running process's memory; a database only a server reads, except through the records seam's server profile; records reached other than through the records seam. Source: clauses 26, 51, 61 to 63; C.6; ADR 0008.
- **core.14** Rules out: a decision kept in a loop's memory and not in the records; a step that proceeds without the branch's lock; behavior that differs between `sluice run` and `sluice step`. Source: clauses 26, 51, S10; ADR 0005.
- **core.15** Rules out: reading policy from a candidate's tree or a session's place; a submission whose change to the policy relaxes its own gate. Source: clauses 27, 56; ADR 0007.
- **core.16** Rules out: removing or cleaning a session's worktree on landing; a candidate place left behind after its verdict is read. Source: clauses 25, 34.
- **core.12** Rules out: a comment or commit that reinterprets a clause; a behavior the statement forbids kept behind a flag. Source: ADR 0001.
- **core.13** Rules out: a pricing table, a tier name, a customer name, a hosted-plan reference in any file. Source: ADR 0001.
