---
number: 92
title: A host's one driver may drive every repository on the host that has sluice, and nothing requires it
status: proposed
date: 2026-11-06
decision-makers:
- Chuck Swanberg
- agentplot-design
---

# A host's one driver may drive every repository on the host that has sluice, and nothing requires it

Status: proposed to the user on 2026-11-06, as `briefs.15` has it; accepted only on their yes.

Deciders: the user, in swancloud-ops's pane at about 22:29Z on 2026-11-06, on hearing that the gate box runs one `sluice run` under systemd per repository (afterthought/crew-state, agentplot/main, signal 2026-11-06-agentplot-operator-287e6227/01; madswan/main, signal 2026-11-06-madswan-operator-72b5d835/01): "This is a terrible [...] design for sluice. [...] it just doesn't scale: having to install it and run a system service for each repo." and "We have a fixed number of repos for right now so we could keep it in place if it's working. We need to [...] figure out the right way to build sluice", which settles that a host's driving of its queues must not take one installed service per repository, and that the services standing today stay until the new shape is built. agentplot-design for the shape: the host form of `sluice run`, how it finds its repositories, and the intake mark naming a copy, each with the path not taken below.

Sources: `agentplot/blueprints:design/sluice/research/2026-11-06-how-a-host-drives-the-queues-of-the-repositories-it-keeps.md` (e7c9ad1), read against sluice's code on main and deneb's processes on 2026-11-06; statement clauses 26, 58, 71, 129, 135 and 137; rules core.25, core.31 and core.33; records 0056, 0068 and 0071; `crates/sluice/src/reader.rs` (the reader and driver locks, keyed by a repository's git directory) and `crates/sluice/src/profiles/github/pulls.rs` (a copy judges the mark its own by `mark.host == self.peer.name`) at 0b15d250; Boomdrive's rules core.13 and core.14; swancloud's `lib/sluice-driver.nix`.

## Context and problem statement

Record 0068 made sluice a plain command line that needs no daemon, and let a host run one driver, `sluice run`, by its own choice; clause 135 and rule core.31 hold it. That driver drives the repository of its working directory alone, and its reader and driver locks are keyed by that repository's git directory. So a host that wants every queue moving while nobody waits runs one `sluice run` for each copy of each repository: five services on the gate box, sixteen on deneb for the person's copies and twenty-three more would be needed for the runner's. Each new repository waits on a deploy before its queue moves, and two lists of repositories, the host's and the services', have to be kept in step.

deneb also keeps two copies of most repositories, the person's clone and the runner's. sluice knows a repository by its git directory, so each copy is a sluice of its own, with its own queue of the same GitHub branch. Both copies read the branch's intake mark as theirs, since a mark names a host and both run on one host: on 2026-11-05 at 21:50Z the runner's copy of swancloud took pull requests 314 and 315 into its own queue of main while the person's copy held the mark.

## Considered options

1. **A host form of `sluice run`** that drives every repository on the host holding sluice's records, found from an index sluice writes in the host's state.
2. One `sluice run` service per repository the runner keeps, rendered from the runner's list. No sluice change; it is the shape the user ruled out.
3. The runner or Flume steps each undecided queue on its pass or settle. A round at most every pass makes each landing wait minutes longer, it drives only the runner's repositories and never a person's own submits, and it makes the runner a second driver beside Flume's step (Boomdrive core.1, core.13).
4. Records kept on the remote (the peers profile, record 0037), so every copy and host reads one queue. Every repository's policy must name signers and every write is a push under the forge's allowance; it is the answer once several hosts submit to one branch, and too heavy for one host.
5. One queue per remote branch per host, its records in the host's state keyed by remote and branch, whatever the copies. It moves where every record lives (C.6) for a case that one copy per host ends.

## Decision outcome

Option 1, with the intake mark naming the copy that holds it.

**The host's driver.** `sluice run` gains a host form, one process that drives every repository on the host holding sluice's records. For each repository it does what `sluice run` in that repository does: it holds that repository's reader and driver locks, so a wait or a watch there follows it and runs no round (129, 135); a refusal or a failure in one repository is that repository's, said once while it stands, as one branch's is today (26); and one repository's slow fetch holds no other's round. The host's bound on runs (71) is this one process's to keep. A `sluice run` started in one repository still drives it, and the host driver passes over a repository whose driver lock another process holds. Whether the host form keeps a thread per repository or a child `sluice run` for each is the construction's; to the host it is one process.

**How it finds the repositories.** sluice writes a repository into an index in the host's state when it first writes that repository's records, at a submit or a `sluice branch create`, and the host driver reads the index at each tick, as a watch reads its records list (129): a repository gone, or one that holds no sluice, leaves the index, and one newly set up is driven from its first submission, with no deploy and no list to keep. A list the host's configuration renders was the other path; it covers only what the list names, and a person's own copy, or a repository set up after the last deploy, would wait on someone to add it.

**Nothing requires it.** Record 0068 stands whole: nothing of sluice's need run, no policy, configuration or document of sluice's tells a host to run the host driver, and a host with nothing running moves its queues by its waiting submits and waits as today. A host that runs it chooses to, from its service manager, by hand or from cron; starting it on a host is the host's, and on Boomdrive's hosts the flake's (Boomdrive core.13), never the runner's and never sluice's.

**A mark names the copy that holds it.** The intake mark names the copy of the repository that holds it as well as the host, and a copy judges the mark its own only when it names that copy, so a second copy on the host never takes the branch's requests in (137). Boomdrive's rule core.14 makes the runner's clone the host's one copy, and swancloud's change that retires the person's clones carries it out; until it lands, two copies on one host keep two queues of one branch, and this keeps them from both taking its requests in.

### Consequences

- Clause 135 of the statement gains the host form: a host's one driver may be a process driving every repository on the host that holds sluice's records, found by the index sluice keeps, each repository's reader where it runs. Clause 137 names the copy on the mark. Rules core.31 and core.33 are amended, each marked proposed until the user's yes.
- The README's "No daemon" paragraph gains one sentence: a host that wants every repository's queues moved may run one `sluice run` for all of them.
- swancloud's per-repository driver services stay as the user said, while the repository count is fixed, and retire once the host driver is released and the host's service manager starts it (swancloud's, and the Boomdrive flake's for Boomdrive's hosts).
- One copy of each repository per host is Boomdrive's (core.14) and swancloud's to carry out; this record decides none of it.
- No unit is queued from this record while it is proposed.
