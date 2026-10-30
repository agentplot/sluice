---
number: 75
title: A branch's signed runs are verified with the signers its own policy names, at its revision on its remote
status: accepted
date: 2026-10-30
decision-makers:
- Chuck Swanberg
---

# A branch's signed runs are verified with the signers its own policy names, at its revision on its remote

Status: accepted with the change that builds it, `a-branchs-run-records-are-verified-with-the-signers-its-own-policy-names`, on 2026-10-30. It decides how the code holds clause 78 and rule core.22, and moves no clause.

Deciders: the change's design (its decisions D1 to D3), on batch 44's reviewer reading of 2026-10-28, which found the defect in `crates/sluice/src/profiles/records.rs`.
Sources: statement clauses 27, 28, 56, 78 and 124 in agentplot/blueprints `design/sluice/requirements.md`; rule core.22; ADRs 0032, 0037, 0069; `crates/sluice/src/profiles/records.rs` (`route`, `open`, `signers`), `crates/sluice/src/profiles/remote_refs.rs` (`RemoteRefs::new`, `signed`, `results`), `crates/sluice-seams/src/lib.rs` (`State::results`), as the bolt held them before this change.

## Context and problem statement

A branch whose policy names signers and a remote keeps its records on that remote (C.6; ADR 0037), and a run counts for its gate only when a signer its policy names signed the run's record, verified from the state and the policy alone (78, 28). The gate and its signers are the branch's own policy (27), read at the branch's current revision (56), which for a branch on a remote is the remote's revision, never the host's copy (124).

As built, the profile that keeps a branch's state opened one `remote refs` profile per remote, and the first branch routed there in a process gave that profile its signers and their keys, read from that branch's policy. Every later branch on the same remote read its runs through the same list. A second branch whose policy lists another key for a signer had its runs judged by the first branch's key, so a run signed with a key its own policy never names counted for its gate; a signer only the second branch names was never read at all. Which branch came first depended on what the process happened to read first: a round's routing, a tick's ask, a lookup by number. The gate already filtered signed lines by signer name per branch; what was shared across branches was which names were read and which key verified each.

The list was also read once a process. A long-running `sluice run` kept the key a branch's policy listed when it started, after a landing on the remote added or removed one.

## Considered options

1. As built: one list per remote, from the first branch routed there, for the life of the process.
2. Keep the read of the results log branch-free; verify each signer's runs with the union of the keys every routed branch lists, tag each result with the key that verified it, and let the gate count a result only when its key is one its own policy lists.
3. One `remote refs` profile per branch, each with its branch's list.
4. The read of the results log names the branch whose gate reads it; the profile that keeps the state keeps one list per branch, read from the branch's own policy at its revision on its remote and read again only when that revision moves; the `remote refs` profile keeps no list and verifies with the one it is handed.

## Decision outcome

Option 4.

**The read names its branch.** `State::results` takes the branch whose gate, session run or submission's result reads the log. Every caller in the core has it in hand: a round's run and landing, a session run, the verdict view. A profile that reads no signed run, the host's own files, ignores it. Setting a result aside names no branch, so it looks for the result among every branch with a state, each read for itself and joined once; it is an operator's command, and a read per branch is its cost.

**Each branch's own list, by its revision.** The records profile keeps, per branch, the revision its list was read at and the list. A branch on its remote takes its revision from the remote's tracking ref as last fetched; the first time a process reads it, and only where no tick of that process asked the remote, it fetches the branch first, as every read of a branch on its remote does (C.7), and reads the tracking ref as last fetched when the remote can't be reached. While that revision stands the list is used as read (rule core.31); once it moves, the policy is read whole at the new revision before any run is verified for the branch. A remote that holds no revision of the branch, and a host with no tracking ref of it, give no list: the host's own copy is never read for one (124). A branch in the host's own files reads its list from its own policy at the revision its routing read, so the rule is the same for every branch, and a policy there naming no signer reads no signed run.

**The remote's profile verifies what it is handed.** `remote refs` keeps no signer and no key. It reads each named signer's runs and verifies them with the key the list gives for that signer, and remembers what it verified by the statement, the signer and a digest of the key's allowed-signers lines together, so a statement verified with one branch's key is never read as verified with another's.

**Why not option 2.** The gate would need the keys' identities in its rules, and the core cannot read a key: a key may be a file at the revision, read by the profile. It also keeps a list made of other branches' keys in the read path, which is the shape of the defect.

**Why not option 3.** The id counter, the hosts' signs of life, the tick's ask and the stale reads are the remote's, not a branch's; a profile per branch would hold each of them once per branch.

**Why not as built.** It breaks 78 and core.22 whenever two branches on one remote name different keys or signers, and keeps a key the remote's policy has since dropped for as long as the process runs.

### Consequences

Good: a branch's runs count by its own policy, whichever branch a process read first; a key added or removed on the remote takes effect at the next fetch of the branch, in a running driver too, and the policy is read for it once per move.

Bad: `State::results` changed its signature; every implementor is in this repository and follows it. A branch in the host's own files whose policy names no signer no longer reads another branch's signed runs, and may run checks it reused before.

Neutral: which results a branch in the host's own files counts, and which remotes' runs a branch reads, are as before; only the keys that verify the signed ones change. Rule core.22's details gain a line ruling the shared list out.

## More information

Left for the planner, each named in the change's proposal: whether a branch in the host's own files should count other hosts' signed runs at all, and whether a branch should read signed runs from remotes other than its own.
