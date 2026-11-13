---
number: 93
title: A host's queues move in sluice's own host process, started when work arrives and gone when idle, and each branch's queue runs whole in one of four ways
status: accepted
date: 2026-11-06
decision-makers:
- Chuck Swanberg
- agentplot-design
---

# A host's queues move in sluice's own host process, started when work arrives and gone when idle, and each branch's queue runs whole in one of four ways

Status: accepted by the user on 2026-11-06, in agentplot-design's pane: "I approved 93."

Supersedes record 0092. Amends records 0056, 0068 and 0071, and supersedes records 0088 and 0089, each marked so where it stands.

Deciders: the user, in the research talk `research--a-hosts-one-driver-fits-boomdrive` on 2026-11-06 (boomdrivedev/blueprints `design/sluice/research/2026-11-06-record-0092-beside-boomdrive.md`), approving the design drawn on the page `Boomdrive Work Lifecycle` (https://claude.ai/artifact/671w5zreD9aMqstSLpMQSm): "Okay I approve this." Their words that set it: "What I want is an elegantly designed CLI, and I think it started out that way: you do `submit`, and if you wanted to wait, you could."; "if I had 10 agents all submit, then 10 supervisors are running. They're all going to try to form the next candidate. It just seems antithetical to the concept of a merge queue."; "Should it be one per queue or one system-wide that can also manage the system's concurrency and the host's concurrency rules? The whole point of programming languages like Rust and Go is to have really great concurrency support so I feel like we give that up when we have one per queue."; "I don't know that we want to take it as a given that we have a gatebox."; and, choosing on the question tool, "Index, on any sluice use" for how the host process finds its repositories. agentplot-design for writing it into this repository's records and rules.

Sources: boomdrivedev/blueprints `design/sluice/proposals/host-process-1.md` and `host-process-1.html` (ab5d788), the statement's clauses 71, 120, 129, 135, 137 and 140, S64, S65, the many-hosts table and C.5's pull request row as ratified at 7c2fbf0, and flume's C.6 step row; `design/principles.md` (proposed, from the same talk); records 0056, 0068, 0071, 0088, 0089 and 0092; rules core.25, core.31 and core.33.

## Context and problem statement

Record 0068 kept sluice a plain command line that needs no daemon and let a host run one driver, `sluice run`, by its own choice. A host that wants every queue moving runs one per repository; the gate box runs five under systemd, and the user called that design one that "just doesn't scale". Record 0092 proposed one host driver for every repository, still a service a host keeps. Meanwhile every waiting submit and wait on a host runs rounds where no driver lives, so ten agents' submits are ten processes each trying to form the branch's next candidate, and the host's bound on runs is counted across processes through files. And a branch that takes pull requests is taken in by whichever host holds an intake mark on the forge, with its renewal, its lease, hand-overs and their withdrawals.

## Considered options

1. **One host process, sluice's own.** The first command that needs the host's queues moved starts it where none answers; it holds every repository on the host that has sluice, each repository's queues apart, and the host's bound on runs in one place, and ends once no queue has work in play and nothing follows it.
2. Record 0092: one host form of `sluice run`, which a host's service manager keeps. One service in place of one per repository, but still a service each host installs and keeps.
3. One process per queue, started by its submit. Ten submits to ten branches are ten processes, and the host's bound and the forge's allowance are coordinated across them through files.
4. Every command runs rounds itself, as a waiting submit does today. Nothing new runs, but ten waits are ten processes forming one branch's candidate.

## Decision outcome

Option 1.

- **Nothing is installed.** sluice is a binary on the path. A command that needs the host's queues moved, a submit among them, reaches sluice's host process over a socket in sluice's state folder, or takes the lock and starts it when nobody answers. A submit adds its entry and returns at once; `sluice wait` follows the process for whoever wants to watch (135).
- **One process per user on a host.** It holds a task for each repository with work, each repository's queues apart, so a refusal or a slow fetch in one holds no other's round (26); one scheduler keeping the host's bound on runs, a queue's candidates before sessions' runs (71, 150); and one forge client, one allowance. Its concurrency is the language's, inside one process, never processes coordinating through files.
- **Gone when idle, and nothing lost.** It ends once no queue it holds has work in play and nothing follows it. It keeps nothing the records do not hold, so a crash or a sleeping laptop loses nothing: the next command that finds work in play and no host process starts one. It finds its repositories in an index sluice writes in the host's state whenever any command of sluice's finds a repository has sluice, a policy at a revision the host holds or records, the user's choice. A run outlives the process that started it and is equipped from its repository's policy (147). A command of a newer sluice has an older process hand its repositories over and end (135).
- **The reader is the host process.** A watch or a wait that finds no reader starts the host process, which reads, and follows it (129).
- **No intake mark.** The intake mark, its renewal and lease, hand-overs and their withdrawals all go (120, 140). Whatever executes a branch's queue takes its requests in. A host's submit opens or updates its request and marks it ready, and the host follows it by the forge's state and sluice's check on its head, its status showing the request's entry there as fields (137, C.5).
- **Where a queue runs.** Each branch's queue runs whole in one of four ways, behind one contract with Flume, as the amendment of 2026-11-08 below has it.
- **sluice calls no other program.** Flume reads each outcome at its settles; its step per settle is nothing (flume C.6).

### Consequences

- Rules core.31 and core.33 are rewritten, each marked proposed; core.25 holds, its reader being the host process.
- The README's "No daemon" paragraph says sluice starts its own process when work arrives and nothing is installed.
- Record 0092 is superseded and rejected; records 0088 and 0089, which keep and hand over the intake mark, are superseded; records 0056, 0068 and 0071 are amended where they name a driver a host runs or a host taking requests in.
- The change `a-queues-test-keeps-the-slot-it-counted`, which kept the slot across processes, is dropped; record 0087, its decision, stands amended in part by this one. The fix `a-hand-overs-withdrawal-holds-no-request` builds what this record removes, and is told so.
- A host where several users run sluice keeps one host process and one bound for each user.
- What each family repository owes, Boomdrive's core.13 and record 0003 and flume's C.6, is in `host-process-1.md`'s last section; the per-repository `sluice run` services on any host retire once a sluice with the host process is released.

## Amendment of 2026-11-08: one contract with Flume, and a queue runs whole in one of four ways

sluice keeps one contract with Flume and the Boomdrive runner however a queue runs, and each branch's queue runs whole in one of four ways: with no daemon, by a daemon on this machine, by a daemon on another machine, or by a GitHub Action. Status: accepted by the user on 2026-11-08, on their card ("Approved"), and point 3 as it reads now on their card the same day ("Yes: machines choose how sluice runs, the repository says what must pass").

Deciders: the user, 2026-11-08, in agentplot-ops's pane, as agentplot-ops relayed it (signal 2026-11-08-agentplot-ops-134a1434/01). They asked for sluice's direction to be handled as two separate things. The first is how sluice works with Flume and the Boomdrive runner: `sluice submit` starts the background processes, and sluice reports to Flume when the work is done, so Flume marks it and launches a fix. The second is local against remote execution, which doesn't change the first: "It seems like the GitHub job idea, just for doing the pushes to main and not for running the test, is a [...] band-aid from a design perspective. In my mind there's going to be local execution and then there's going to be remote execution but that doesn't change."; "I think we need to support either a daemon running on the same machine or a different machine, or a GitHub action". The same day, refining it (signal 2026-11-08-agentplot-ops-77e3a93c/01-sluice-runs-four-ways): "I'd argue that it's really four options because sluice runs on the same machine as a background process that gets kicked off. We did that to avoid the daemon but let's face it: a remote executor is a daemon. What stops you from running a daemon on the same host or a remote host? It's kind of the same thing but sluice should also be able to run without a daemon". Reading the plan that followed, the same day: "Why would it be the responsibility of a repository to know how its surrounding infrastructure is actually set up?" agentplot-design for writing it into this record and the statement, and for the choices marked as its own.

1. **Two concerns, kept apart.** How sluice works with Flume and the Boomdrive runner is one concern, and where a branch's queue executes is another. A change to one never changes the other.
2. **One contract, wherever the queue runs.** `sluice submit` adds its entry and returns at once, having started whatever background work the way that executes its branch needs. Once the submission is decided, sluice reports its outcome to its submitter as this record has it (the notice, `sluice inbox` and `sluice wait`), and Flume reads that report, marks its place, and launches the kind's landing fix on a kick-back (flume record 0056; weir record 0005). sluice still calls no program of Flume's. The user's "sluice report" is this report. Whether it is also a command of its own is the build's choice, and it changes nothing of the contract.
3. **A queue executes whole, in one of four ways, and the machines choose it.**
   - **No daemon.** The submitting machine's own sluice process executes it: `sluice submit` starts it, and it ends when idle (135, as decided above). sluice works this way with nothing set.
   - **A daemon on this machine.** The machine keeps `sluice daemon` running. That is the same host process, which never ends when idle.
   - **A daemon on another machine.** That machine keeps `sluice daemon` running, and it runs the queues of the repositories it holds.
   - **A GitHub Action.** GitHub Actions' jobs execute it: they take the requests in, run the checks on GitHub's machines and push the landing. The repository carries the workflow GitHub runs. This is the one thing any way puts in a repository, since GitHub runs only jobs a repository defines.

   No setting chooses a way. A daemon is a command a machine keeps running by its own choice. The repository says only what a landing must pass and whose test results it accepts: its gate and its signers. Those stay in the repository because a machine can't vouch for itself, and the branch's own revision is what is read, so no change loosens its own checks (core.15, 78). A shared branch's queue runs where its results count. A machine whose signature the branch does not accept leads none of its rounds and runs none of its checks. It submits, and leaves the queue to one whose signature the branch accepts.

   Whatever executes a branch's queue takes its requests in, forms and tests its candidates, and lands them. Nothing does one part of that while another does the rest. No request is taken in by two ways, two machines or two copies of one repository.
4. **Not taken.** A forge's jobs taking requests in and landing them while machines run the checks, the split the user called a band-aid. A branch on GitHub Actions runs its checks there too, and the change that built the split is reverted whole on its bolt (`sluice-runs-one-host-process--takes-out-the-forge-round`).
5. **What stands.** Everything else above stands: one host process per user, started when work arrives and gone when idle, as the no-daemon way, with nothing installed on a machine that uses it. So does the removal of the intake mark, its renewal, its lease and its hand-overs. Whatever executes a branch takes its requests in.
6. **Consequences.**
   - sluice runs standing only as a daemon a machine keeps running by its own choice, on this machine or another (point 3). That settles wldn's question of a machine whose signature a branch's tests need: that branch executes by a daemon on that machine, which stays up.
   - Rules core.31 and core.33 are amended to match, each marked proposed.
   - The statement's clauses 135 and 137 and C.8's table follow, marked proposed (boomdrivedev/blueprints, design/sluice/requirements.md).
   - Left to the build, within points 2 and 3 (agentplot-design's choice): how a machine's service manager keeps `sluice daemon` running, and what a machine whose signature a branch does not accept starts when it submits. Units are queued from this amendment.
