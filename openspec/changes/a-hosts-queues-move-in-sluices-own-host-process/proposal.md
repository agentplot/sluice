## Why

Record 0093, accepted by the user on 2026-11-06 ("I approved 93."), has
a host's queues move in sluice's own host process. The statement carries
it in clauses 135, 129, 71 and 150 (agentplot/blueprints a0f0f2f and
88a4cf0), and rules core.31, core.25 and core.49 bind it. The user's
words that set it: "you do `submit`, and if you wanted to wait, you
could"; "if I had 10 agents all submit, then 10 supervisors are running.
They're all going to try to form the next candidate. It just seems
antithetical to the concept of a merge queue"; and, for one process for
the whole host, "The whole point of programming languages like Rust and
Go is to have really great concurrency support so I feel like we give
that up when we have one per queue."

What sluice does at the bolt's base, 68d68a9a:

- `sluice run` (`Command::Run` in `crates/sluice/src/lib.rs`) is one
  repository's driver, held by the lock `readers/<repository>.driver`.
  A host that wants its queues to move runs one per repository, under a
  service manager or by hand.
- Where no `sluice run` lives, every `sluice wait`, waiting `sluice
  submit` and `wt submit` runs rounds between its reads (`wait_on`). Ten
  agents waiting on one branch are ten processes taking turns at the
  branch's lock to form one combination.
- The first watch or wait of a repository becomes its reader (129;
  `reader.rs`), and gives the place to a driver that starts.
- The host's bound is counted across processes through the files of
  `running/` (`host_count.rs`). A round counts its room when it decides
  and admits the run only at its start, so an agent's `sluice exec`
  takes the room between the two (record 0087's 2026-11-06 starts on
  deneb). Nothing starts a waiting combination's run before an agent's.
- The README, the site and the plugin's skills tell a host to keep
  `sluice run` going.

## What Changes

- **sluice's host process.** A hidden `sluice host` command is the one
  process for a user on a host. It holds a lock in sluice's state
  folder and answers on a socket in the user's own short directory. A
  command that needs the host's queues moved reaches it, or starts it
  detached where none answers. Ten commands at once start one.
- **Every repository's queues, each apart.** The host process runs a
  task for each repository with work in play or a follower. Each task
  runs the pass `sluice run` runs today: the remote's one ask, the
  releases, and a round of every branch in play. A refusal or a slow
  fetch in one repository holds no other's round (26).
- **Found again from an index.** Any command of sluice's that finds a
  repository has sluice, records or a policy at a revision the host
  holds, adds it to an index in the host's state. A host process that
  starts finds its repositories there. A repository that is gone, or no
  longer has sluice, leaves the index.
- **Gone when idle, nothing lost.** It ends once no repository it holds
  has work in play, nothing follows it, and no run waits for room. It
  also ends when its state folder is gone. It keeps nothing the records
  do not hold. Runs outlive it, and any command of sluice's that finds
  work in play and no host process starts one again.
- **Started clean.** It starts with the host's own environment and none
  of the starting command's repository, worktree, session or shell. A
  run is equipped from its repository's policy (147).
- **A newer sluice takes over.** A command of a newer sluice has an
  older host process hand its repositories over and end, then starts
  its own.
- **Waits and watches follow.** A `sluice wait`, a waiting `sluice
  submit` or `wt submit` and `sluice overview --watch` follow the host
  process's document and run no round. One that finds no host process
  starts it, and one whose host process stopped starts it again within
  a tick. A submit writes its entry, tells the host process, and returns
  or waits as asked.
- **`sluice step` is the host process's round.** It asks the host
  process for one round of its branch and prints it as before.
  `sluice run` is hidden. It follows the host process's passes of its
  repository, printing what each pass says, and says once that nothing
  needs it running.
- **The host's room in one place.** The host process holds the host's
  bound. A round holds a combination's room from the count that chose
  it to its start, and the check seam gains `hold`, whose room `start`
  takes. `sluice exec` and `sluice preview` ask the host process for
  their room and wait in its order. A combination's run starts first,
  taking turns with sessions' runs, and sessions' and previews' runs
  start in the order they came to wait (150, S75; record 0087). The
  overview's `room` names the runs waiting and whose turn is next, and
  its `host` names the host process.
- **The docs stop naming a service.** The README's "No daemon"
  paragraph, the site's pages and the plugin's skills say sluice starts
  its own process when work arrives and nothing is installed. They
  drop every instruction to keep `sluice run` going, except the pull
  request pages the sibling change rewrites (principle 11).

## Capabilities

### New Capabilities

- `host-process`: sluice's one host process for a user, how commands
  reach and start it, what it holds, when it ends, and how a newer
  sluice takes over (135, 129).

### Modified Capabilities

- `host-driver`: `sluice run` is no repository's driver. It follows the
  host process, and the requirements of a per-repository driver go.
- `overview-reader`: the reader is the host process. A watch or wait
  that finds none starts it, and a stopped one is started again within
  a tick (129).
- `submission-wait`: a wait runs no round. It follows the host
  process's document to the outcome, and a landing stands whatever the
  host process's round failed after it.
- `reader-tick`: the idle tick is the host process's pass, and a watch
  starts no git of its own.
- `run-bound`: the count, each run's room from its count to its start
  and the runs waiting for room are kept in the host process, the
  combination's run first by turns (71, 150; core.49).
- `standing-executor`: the host process claims the runs its signer
  counts for, in place of `sluice run`.

## Impact

- **Code.** A new module `crates/sluice/src/host_process/` holding the
  lifecycle, socket, index, environment, room and a repository's pass.
  Also `crates/sluice/src/lib.rs` (run, step, wait, submit, overview),
  `exec.rs`, `preview.rs`, `reader.rs`, `host_count.rs`,
  `profiles/checks.rs` and `profiles/mod.rs`. The check seam gains
  `hold` (`crates/sluice-seams`), the core's starts hold room
  (`crates/sluice-core/src/step.rs`), and the fakes and the tests'
  `FakeHost` take passes when asked.
- **Tests.** A new binary test target `tests/host_process.rs` on real
  git with the command runner. `cli.rs` and `reader_tick.rs` keep their
  commands and read rounds from the host process.
- **Docs.** README, the site's commands, state, walkthrough, overview,
  executors and telemetry pages, the plugin's using and setting-up
  skills, the details of rules core.4, core.25 and core.31, and record
  0093.
- **Hosts.** Nothing to install. A host's `sluice run` units keep
  running harmlessly until removed, and record 0093 names them retired
  once a release with the host process is out.

## Out of scope

- **Pull request intake.** The change
  `pull-requests-are-queued-on-the-forge`, on the same bolt, removes a
  host's intake, the intake mark, its lease and hand-overs (137, 120,
  140, C.5; core.33). Until it lands, the host process's pass takes
  requests in as `sluice run`'s does today. The pull request guides and
  the setting-up skill's pull request reference are its to rewrite.
- **One forge client for the host.** Each repository's task keeps its
  own forge client. The allowance is already decided in one place, the
  pause in the host's state that every task reads (134; core.29). One
  client across tasks would save token mints alone, and waits on the
  forge profile being shareable across threads.
- **Waking a host that runs no sessions.** A box that only takes runs
  on branches whose records are on a remote has its host process
  started by any command of sluice's there, or by the wake its
  deployment names (59, 111). Nothing new is built for it here.
- **What other repositories owe.** agentplot/boomdrive's core.13 and
  record 0003, flume's C.6 step, which is now nothing, and the
  per-repository `sluice run` units on the hosts afterthought/swancloud
  manages, as `host-process-1.md` lists them. Each changes when its host
  moves to the release with the host process (principle 11).
- **Whether `sluice submit` waits by default.** Unchanged. A waiting
  submit is a submit and a wait. Rules core.4 and core.50 keep it.
