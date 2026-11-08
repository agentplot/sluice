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

## Amendment, 2026-11-07: what the build of the host process chose

Deciders: the construct stage of the change `a-hosts-queues-move-in-sluices-own-host-process`, on choices the written rules leave to the build, each cheap to change later. Sources: that change's design, D1 to D14; clauses 26, 71, 129, 135, 150; rules core.1, core.12, core.25, core.31, core.49; record 0063.

What it decides stands: one host process for each user on a host, started by the command that needs the host's queues moved and gone when idle, holding every repository's queues apart and the host's bound in one place. The build chose the following, each with the path it did not take.

1. **The socket is in the user's own short directory**, `/tmp/sluice-<uid>/`, which record 0063 makes and checks, as `host-` and a digest of the state folder's real path, beside the host process's files in `host/` under sluice's state folder; where this record said "a socket in sluice's state folder", it is there. A socket's path is at most 104 bytes on macOS, and a state folder in a test's or a sandbox's temporary path is deep (the core.30 incident of 2026-11-03). Not taken: the socket under `host/`, refused where its path is too long, which fails every test whose scratch is deep.
2. **`sluice run` is kept, hidden, as a follower** of the host process's passes of its repository, printing each and saying once that nothing needs it, so a host's unit that runs it keeps working until it is retired. Not taken: removing the command, which would turn each such unit into a restart loop.
3. **`sluice step` is the host process's round**: it asks the host process for one round of the branch, which runs between its passes, and prints it as before (rule core.14). Not taken: a step running the round in its own process, a second process forming the branch's combinations.
4. **Under `SLUICE_ROUND_SLEEP=record`**, the tests' setting, the host process passes only when asked: a follower asks a pass at each of its ticks, `sluice step` asks a round, and a serve holds the repository for the next ask, so a test's rounds are those its commands asked for, in its own order. The tests over the fakes ask the same of a host process in the test's own process, through the same pass. Not taken: a setting that keeps the host process from starting, with waits running rounds as before, which keeps behind a flag what clause 135 removes (rule core.12); and a serve that passes at once, which moves a queue a test's `--no-wait` submit leaves for its next command.
5. **Each repository's task keeps its own forge client** for now; the allowance is decided in one place already, the pause in the host's state every task reads (134). Not taken: one client across the tasks, which saves token mints alone and waits on the forge profile being shareable across threads.
6. **The overview names the host process as `host_process`**, `{version, pid, since}` or `null` where none answers, beside `host`, which stays the host's name: a tool that watches the host, the board and the herdr plugin among them, reads `host` as the name, and the overview's document only gains fields. Its `room` gains `waiting`, by kind, and `turn`. Not taken: `host` as the host process, which breaks what reads it.
7. **A wait relays what the host process's rounds of its branch said** of the checkouts they released, and what its passes freed, as its own rounds said them; a round's failure on its branch it says once on stderr and goes on, and a policy it cannot read itself it reads as taking no pull requests, the rounds' to say. Not taken: a wait silent on its branch's releases, which leaves a held checkout said nowhere a waiting agent looks.
8. **The host process emits the rounds' telemetry**, keeping `SLUICE_OTEL` and `OTEL_*` of the command that started it, since the rounds it runs are the host's (ADR 0014).
9. **The host process's look at its index**, at its start and every three seconds, reads each repository it holds no task for as an idle tick reads it, each remote that keeps its records asked once, then its records and its previews, and starts a task only where work is in play, a preview to drop among it (129). A repository whose task could not start, its binding refused or its systems not this host's, is said once in the log and passed by until a command asks for it. Not taken: a task started for every listed repository at each look, which runs a pass of each idle one every three seconds and says each refusal again; and a refused repository's task kept running idle, which keeps the host process from ever ending.
