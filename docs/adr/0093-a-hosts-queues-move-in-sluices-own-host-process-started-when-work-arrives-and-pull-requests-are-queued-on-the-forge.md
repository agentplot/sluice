---
number: 93
title: A host's queues move in sluice's own host process, started when work arrives and gone when idle, and pull requests are queued on the forge
status: accepted
date: 2026-11-06
decision-makers:
- Chuck Swanberg
- agentplot-design
---

# A host's queues move in sluice's own host process, started when work arrives and gone when idle, and pull requests are queued on the forge

Status: accepted by the user on 2026-11-06, in agentplot-design's pane: "I approved 93."

Supersedes record 0092. Amends records 0056, 0068 and 0071, and supersedes records 0088 and 0089, each marked so where it stands.

Deciders: the user, in the research talk `research--a-hosts-one-driver-fits-boomdrive` on 2026-11-06 (agentplot/blueprints `design/sluice/research/2026-11-06-record-0092-beside-boomdrive.md`), approving the design drawn on the page `Boomdrive Work Lifecycle` (https://claude.ai/artifact/671w5zreD9aMqstSLpMQSm): "Okay I approve this." Their words that set it: "What I want is an elegantly designed CLI, and I think it started out that way: you do `submit`, and if you wanted to wait, you could."; "if I had 10 agents all submit, then 10 supervisors are running. They're all going to try to form the next candidate. It just seems antithetical to the concept of a merge queue."; "Should it be one per queue or one system-wide that can also manage the system's concurrency and the host's concurrency rules? The whole point of programming languages like Rust and Go is to have really great concurrency support so I feel like we give that up when we have one per queue."; "I don't know that we want to take it as a given that we have a gatebox."; and, choosing on the question tool, "Index, on any sluice use" for how the host process finds its repositories. agentplot-design for writing it into this repository's records and rules.

Sources: agentplot/blueprints `design/sluice/proposals/host-process-1.md` and `host-process-1.html` (ab5d788), the statement's clauses 71, 120, 129, 135, 137 and 140, S64, S65, the many-hosts table and C.5's pull request row as ratified at 7c2fbf0, and flume's C.6 step row; `design/principles.md` (proposed, from the same talk); records 0056, 0068, 0071, 0088, 0089 and 0092; rules core.25, core.31 and core.33.

## Context and problem statement

Record 0068 kept sluice a plain command line that needs no daemon and let a host run one driver, `sluice run`, by its own choice. A host that wants every queue moving runs one per repository; the gate box runs five under systemd, and the user called that design one that "just doesn't scale". Record 0092 proposed one host driver for every repository, still a service a host keeps. Meanwhile every waiting submit and wait on a host runs rounds where no driver lives, so ten agents' submits are ten processes each trying to form the branch's next candidate, and the host's bound on runs is counted across processes through files. And a branch that takes pull requests is taken in by whichever host holds an intake mark on the forge, with its renewal, its lease, hand-overs and their withdrawals.

## Considered options

1. **One host process, sluice's own.** The first command that needs the host's queues moved starts it where none answers; it holds every repository on the host that has sluice, each repository's queues apart, and the host's bound on runs in one place, and ends once no queue has work in play and nothing follows it. A branch that takes pull requests is queued on its forge by the forge's jobs, so no host takes requests in.
2. Record 0092: one host form of `sluice run`, which a host's service manager keeps. One service in place of one per repository, but still a service each host installs and keeps, and still a host taking pull requests in.
3. One process per queue, started by its submit. Ten submits to ten branches are ten processes, and the host's bound and the forge's allowance are coordinated across them through files.
4. Every command runs rounds itself, as a waiting submit does today. Nothing new runs, but ten waits are ten processes forming one branch's candidate.

## Decision outcome

Option 1.

- **Nothing is installed.** sluice is a binary on the path. A command that needs the host's queues moved, a submit among them, reaches sluice's host process over a socket in sluice's state folder, or takes the lock and starts it when nobody answers. A submit adds its entry and returns at once; `sluice wait` follows the process for whoever wants to watch (135).
- **One process per user on a host.** It holds a task for each repository with work, each repository's queues apart, so a refusal or a slow fetch in one holds no other's round (26); one scheduler keeping the host's bound on runs, a queue's candidates before sessions' runs (71, 150); and one forge client, one allowance. Its concurrency is the language's, inside one process, never processes coordinating through files.
- **Gone when idle, and nothing lost.** It ends once no queue it holds has work in play and nothing follows it. It keeps nothing the records do not hold, so a crash or a sleeping laptop loses nothing: the next command that finds work in play and no host process starts one. It finds its repositories in an index sluice writes in the host's state whenever any command of sluice's finds a repository has sluice, a policy at a revision the host holds or records, the user's choice. A run outlives the process that started it and is equipped from its repository's policy (147). A command of a newer sluice has an older process hand its repositories over and end (135).
- **The reader is the host process.** A watch or a wait that finds no reader starts the host process, which reads, and follows it (129).
- **Pull requests are queued on the forge.** A branch's requests are taken in only where its records are on its remote, by the instance that keeps the branch there: on a forge, the forge's jobs, as a pool, one round at a time. A branch whose records each host keeps takes no requests in. A host's submit opens or updates its request and marks it ready, and the host follows it by the forge's state and sluice's check on its head, its status showing the request's entry there as fields (137, C.5). The intake mark, its renewal and lease, hand-overs and their withdrawals all go (120, 140).
- **sluice calls no other program.** Flume reads each outcome at its settles; its step per settle is nothing (flume C.6).

### Consequences

- Rules core.31 and core.33 are rewritten, each marked proposed; core.25 holds, its reader being the host process.
- The README's "No daemon" paragraph says sluice starts its own process when work arrives and nothing is installed.
- Record 0092 is superseded and rejected; records 0088 and 0089, which keep and hand over the intake mark, are superseded; records 0056, 0068 and 0071 are amended where they name a driver a host runs or a host taking requests in.
- The change `a-queues-test-keeps-the-slot-it-counted`, which kept the slot across processes, is dropped; record 0087, its decision, stands amended in part by this one. The fix `a-hand-overs-withdrawal-holds-no-request` builds what this record removes, and is told so.
- A host where several users run sluice keeps one host process and one bound for each user.
- What each family repository owes, Boomdrive's core.13 and record 0003 and flume's C.6, is in `host-process-1.md`'s last section; the per-repository `sluice run` services on any host retire once a sluice with the host process is released.
