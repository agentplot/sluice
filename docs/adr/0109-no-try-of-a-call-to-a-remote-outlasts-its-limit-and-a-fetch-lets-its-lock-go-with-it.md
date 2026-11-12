---
number: 109
title: No try of a call to a remote outlasts its limit, and a fetch lets its lock go with it
status: accepted
date: 2026-11-10
decision-makers:
- the code stage of fix/main--a-hung-fetch-holds-no-lock
- the code stage of change/no-try-of-a-call-to-a-remote-outlasts-its-limit
- the code stage of chore/sluice-takes-weir-bounded
---

# No try of a call to a remote outlasts its limit, and a fetch lets its lock go with it

Status: accepted. This is the code stage's own decision under clause 134 as written and rules core.28 and core.29, which it keeps. It is cheap to change later.

Deciders: the code stage of `fix/main--a-hung-fetch-holds-no-lock`, which decided it on main at 69bc9934, and the code stage of `change/no-try-of-a-call-to-a-remote-outlasts-its-limit`, which built it on main beside record 0110. The amendment of 2026-11-11: agentplot-design's ruling of 2026-11-10 for its direction, and the code stage of `chore/sluice-takes-weir-bounded` for the choices within it.
Sources: agentplot-ops's signal `2026-11-10-agentplot-ops-e19d134d/01`; `agentplot/blueprints:design/sluice/requirements.md` clauses 122, 133 and 134 at 60c25ef; the `remote-retry` spec's "The policy's time is set by what called it" and "One fetch at a time in a repository", and the OpenSpec change `no-try-of-a-call-to-a-remote-outlasts-its-limit`, which carries its delta; rules core.10, core.28, core.29; records 0017, 0062, 0086, 0108, 0110; `crates/sluice/src/profiles/pushed_refs.rs` `call`, `fetching`, `git_as` and `under_own_fetch`, `crates/sluice/src/retry.rs` `Policy::run`, and `crates/sluice/src/identity.rs` `CredentialCommand`, as main held them at 5554f215, once record 0110 had landed. The amendment of 2026-11-11: `agentplot/blueprints:design/weir/requirements.md` C.3 and section 9 as amended at ad3a1ae; agentplot/weir record 0008 and its crate `weir-bounded` at weir 9cba691; `crates/sluice/src/bounded.rs`, `runs::end_group` and `identity.rs` `CredentialCommand::spawn`, as the bolt `sluice-takes-weirs-bounded-calls` held them at 47fa1fcc.

## Context and problem statement

On deneb on 2026-11-10, sluice 0.73.1's `status <branch> --all --json` and `overview --json` on agentplot/sluice each hung 10 to 17 minutes, repeatedly. Six processes were ended by hand from 16:50Z. Each had `sluice-fetch.lock` open and a GitHub socket in CLOSE_WAIT. Every other sluice read of the repository, and the runner's acts behind them, waited.

The same picture, made on deneb with sluice 0.78.1 against a remote that takes the connection and never answers, is as follows. The status's main thread is in `Branches::read`, `PushedRefs::fetch`, `fetch_specs`, `call` and `Command::output`, polling git's pipes. It holds `sluice-fetch.lock`. Its git is in `cmd_fetch`, reading from its helper. The helper, `git-remote-http`, is in `discover_refs`, in `select` on the socket, with no limit. A second sluice read of the repository is in `File::lock`, in `flock`, also with no limit. The sluice process itself holds no socket to the remote. The CLOSE_WAIT socket in the signal was therefore an idle connection of the forge's API client. The API's calls already end at thirty seconds. The hang was the git call.

Clause 134 bounds every call to a remote by what called it: a wait's minutes, a round's or a command's seconds. Two things broke that bound:

- A try had no limit. The backoff bounded only the pauses between tries, and a git try waited on its remote for as long as the remote kept the connection.
- A call's tries could outlast its bound. The thirty seconds were a sum of pauses, so a remote that answered each try slowly held a command for one slow try per pause.

The fetch lock of record 0086 held for a try, which record 0086 took to be seconds. A fetch waited for it with no limit. Record 0110 has a sluice under sluice's own fetch take no lock and run no git, so a fetch's hook never waits on the lock its ancestor holds. Every other fetch still waited for the lock with no limit, behind a try with none.

## Considered options

1. **A limit on each try, the same for every call to a remote, and none past the call's own time** (chosen). Each try of a git call runs in a process group of its own and is ended at `retry::TRY_FOR`, thirty seconds: asked to end, given two seconds, then killed, the remote's helper with it. A fetch waits for the repository's lock no longer than that. The forge's API calls and a host's credential command already end at thirty seconds, and now take the same constant. No try starts once the call's bound has passed since its first try, or since a spent allowance's pause ended.
2. **git's own stall limit, `http.lowSpeedLimit` and `http.lowSpeedTime`, given on each call.** It leaves a transfer that still moves alone, and git words its own failure. It covers HTTP's transfer alone. It does not cover a connection stalled in its TLS handshake, which curl allows five minutes, or a remote reached by SSH. It does not cover a fetch waiting for the lock. A backstop would still be needed.
3. **A limit on the call alone, the try ended at the call's end.** One number. But a wait's five minutes would let one stalled try hold the lock for five minutes, with every other read of the repository behind it.

## Decision outcome

Option 1.

- **A try that is stopped** is one whose answer was not read. A read is tried again, since it changes nothing. A push is never made again blind: the ref it set is read, and what the remote holds decides (122; record 0017). Its words are sluice's own, `git <fetch|push|ls-remote> with <remote> was still going after 30s, the most a try may take, and was stopped`, after anything git printed. A round logs a fetch's as a read it could not make, once per cause and host (133; rule core.28).
- **The fetch lock, beside record 0110.** A sluice under sluice's own fetch waits for no lock at all: it is asked first, before any try, and takes no lock and runs no git (record 0110). Every other fetch waits for the lock no longer than a try may take, trying it every 20 ms until `TRY_FOR` has passed. A lock held past that is a try the remote never judged, tried again after a pause. Where the lock is held every time the call tries, the call fails as no refusal of the remote's, naming the lock: another fetch of the repository held it. That is logged on no branch, as a race lost is (record 0086). The holder's own limit ends it. The lock held is named to git in `SLUICE_FETCH_HELD`, as record 0110 has it, so the `reference-transaction` hook git runs still finds it, and it is ended with the try at the limit. The lock is let go as soon as the try returns, never held over a pause.

  Not taken: waiting for the lock with no limit, on the grounds that the holder's own try ends at its limit. A holder that is no try of this sluice's, an older sluice or a hand-held lock, would hang every read again. Also not taken: a wait for the lock whose limit is the call's time, which is five minutes under a wait.
- **One owner.** `crates/sluice/src/bounded.rs` runs a process to a limit in a group of its own, each pipe read on a thread of its own as it goes. At the limit it ends the group through `runs::end_group`, giving two seconds' grace, so git, asked, removes the lock files it holds, then kills what is left. A git call to a remote and a host's credential command both use it. The credential command's group is now asked to end before it is killed, as a check's is. It says of each pipe whether what it kept is less than all the pipe gave: bytes past the part kept passed over, the pipe failing past it, or the pipe not read to its end when the group was ended. The credential command's words, so cut, still end at their last whole line, as record 0108 as amended 2026-11-10 has them, so no token cut at the limit leaves part of itself; and a stopped command's words are read so too, since its pipe may be held by a process outside its group.

  Not taken: a second copy of the group's ending in `pushed_refs.rs`, beside the credential command's. One behaviour would have two owners that drift apart, as the credential command's kill at once had already drifted from a check's grace.
- **Ctrl-C.** git, in a group of its own, no longer takes the terminal's Ctrl-C with sluice. A wait, which stops on Ctrl-C, lets its try in flight run to its end or its limit, then stops at its next pause, as an API call in flight already did. Any other command ends at once, and the kernel lets its locks go with it: the fetch lock's descriptor is closed on exec, so no git holds it. Its git then ends by itself, holding none of sluice's locks, as a credential command and a check already do. Under record 0110, a hook of that git finds the lock free and takes it as any fetch does.

  Not taken: passing Ctrl-C on to the group of the try in flight. Every command would need a signal handler, though Ctrl-C ends a wait and nothing else, and the try holds nothing of sluice's once sluice has gone.

### Consequences

- No try of a call to a remote takes longer than thirty seconds, and nothing it holds, the fetch lock among them, is held longer.
- Against a remote that never answers, a command's call ends after about thirty seconds, where 0.73.1 waited for as long as the remote kept the connection. Measured on deneb against a listener that never answers: a `status main --all --json` ends in 31 seconds, naming the stopped fetch; 0.78.1's does not end. A second read started three seconds later ends within the same thirty seconds.
- A call can end no later than its bound and one more try, about a minute for a command.
- A wait stops up to thirty seconds after Ctrl-C when a try is in flight. Before, it stopped at once, since git took the Ctrl-C too.
- A transfer that takes longer than thirty seconds is stopped too. The largest repository on deneb is 21 MiB, and sluice's fetches are incremental. A host whose fetches need longer raises `TRY_FOR`, one value for every call.
- Flume and the Boomdrive runner run their own git fetches with no limit: Flume's `fetch_branch` in `crates/flume/src/git.rs`, and the runner's in `crates/boomdrive/src/clone.rs`. Neither holds a lock that a sluice read waits on. Neither is changed here. The family has no shared owner of a git call's limit to cite, so this is told to the orchestrator.

## Amendment, 2026-11-11: the limit is weir-bounded's

agentplot-design ruled on 2026-11-10 that weir owns the code of a call's limit, since sluice, Flume and Boomdrive already take weir's crates. weir's statement, section 9 as amended at agentplot/blueprints ad3a1ae, has each of them take `weir-bounded` in place of a copy of its own, in a change of its own, and weir record 0008 built the crate from this record's, at weir 9cba691. The chore `sluice-takes-weir-bounded` takes it. The limit, its grace and a stopped call's words are as decided above. These are the code stage's own choices within the ruling, each cheap to change, with the path not taken.

- **weir-bounded is the one owner.** sluice pins weir at 9cba691 and depends on `weir-bounded`, and `crates/sluice/src/bounded.rs` is gone. A git try to a remote runs through `weir_bounded::output` to the policy's `try_for`, and a stopped try fails in `weir_bounded::stopped`'s words, which are the words above. `retry::TRY_FOR` is `weir_bounded::TRY_FOR`, which the forge's API calls and a credential command's limit take through it. Not taken: `bounded.rs` kept as a wrapper over weir's, a second place to read one behaviour from.
- **A check's group ends by the same call.** `runs::end_group`, by which a check's run, its release and `Runs::end` end a group, is gone too: they call `weir_bounded::end_group`, which weir record 0008 copied from it, asking, its grace, the kill and the five seconds after it unchanged. Not taken: keeping it for the checks, which are no call to a remote. One behaviour would again have two owners, which is this record's reason for one, and weir record 0008 names it among what sluice takes.
- **What a try prints is read from unnamed files once it has ended, with no thread.** weir-bounded writes a try's standard output and error to unnamed temporary files, and reads them once its leader has exited and no process of its group is left. A process outside the group can no longer hold a try open by holding its pipe, since there is no pipe to hold. What is kept, and whether it is cut, are as before, so a credential command's words cut at 64 KiB still end at their last whole line (record 0108 as amended 2026-11-10).
- **What a credential command is asked is written into a pipe before it starts.** weir-bounded starts the command itself and leaves its standard input to the caller. `CredentialCommand` writes `get`'s or `erase`'s lines into a pipe whose write end never waits, closes that end and hands the other to the command as its standard input. The lines are a few short ones, which a pipe holds whole with no reader; what it could not hold would fail before anything started. A start the system refuses because the program is still open for writing is tried again as before, each try with a pipe of its own. Not taken: an unnamed temporary file, as weir-bounded keeps what a try prints, which would put the token `erase` is told on disk, where record 0108 keeps nothing of a token. Not taken either: a thread writing the pipe as the command reads it, which weir-bounded's reason for files rules out.
- **No call of weir-branch's is sluice's.** sluice depends on no `weir-branch`, so weir-branch's fetches and pushes, which weir-bounded bounds at 9cba691, are none of sluice's calls to a remote. Should sluice take weir-branch, its calls are bounded by the same crate.

### Consequences of the amendment

- sluice's calls to a remote end as before: each try within thirty seconds and its two of grace, in the same words. A test of weir-bounded's, not of sluice's, holds the group's ending, the kept part and the cut.
- The family's owner of a call's limit is `weir-bounded`. Flume's `fetch_branch` and the runner's `boomdrive-bounded` take it in changes of their own.
- Rule core.29's details read with it.
