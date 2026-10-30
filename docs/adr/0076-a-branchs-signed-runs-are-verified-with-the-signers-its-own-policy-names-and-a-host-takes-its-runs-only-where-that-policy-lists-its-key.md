---
number: 76
title: A branch's signed runs are verified with the signers its own policy names, and a host takes its runs only where that policy lists the host's key
status: accepted
date: 2026-10-30
decision-makers:
- Chuck Swanberg
---

# A branch's signed runs are verified with the signers its own policy names, and a host takes its runs only where that policy lists the host's key

Status: accepted with the change that builds it, `a-branchs-run-records-are-verified-with-the-signers-its-own-policy-names`, on 2026-10-30. It decides how the code holds clauses 78 and 87 and rule core.22, and moves no clause.

Deciders: the change's design (its decisions D1 to D4), on batch 44's reviewer reading of 2026-10-28, which found the shared list in `crates/sluice/src/profiles/records.rs`, and on clause 87 as amended 2026-10-30 (agentplot/blueprints c078b45), which says what a host whose key a branch does not list takes.
Sources: statement clauses 27, 28, 56, 66, 78, 87 (as amended 2026-10-30, agentplot/blueprints c078b45), 124 and S18 in agentplot/blueprints `design/sluice/requirements.md`; rule core.22; ADRs 0032, 0037, 0069; `crates/sluice/src/profiles/records.rs` (`route`, `open`, `signers`), `crates/sluice/src/profiles/remote_refs.rs` (`RemoteRefs::new`, `signed`, `results`), `crates/sluice-seams/src/lib.rs` (`State::results`), `crates/sluice-core/src/step.rs` (`claimable`, `counts_here`), `crates/sluice-core/src/verdict.rs` (`read`), as the bolt held them before this change.

## Context and problem statement

A branch whose policy names signers and a remote keeps its records on that remote (C.6; ADR 0037), and a run counts for its gate only when a signer its policy names signed the run's record, verified from the state and the policy alone (78, 28). The gate and its signers are the branch's own policy (27), read at the branch's current revision (56), which for a branch on a remote is the remote's revision, never the host's copy (124).

As built, the profile that keeps a branch's state opened one `remote refs` profile per remote, and the first branch routed there in a process gave that profile its signers and their keys, read from that branch's policy. Every later branch on the same remote read its runs through the same list. A second branch whose policy lists another key for a signer had its runs judged by the first branch's key, so a run signed with a key its own policy never names counted for its gate; a signer only the second branch names was never read at all. Which branch came first depended on what the process happened to read first: a round's routing, a tick's ask, a lookup by number. The gate already filtered signed lines by signer name per branch; what was shared across branches was which names were read and which key verified each.

The list was also read once a process. A long-running `sluice run` kept the key a branch's policy listed when it started, after a landing on the remote added or removed one.

Once each branch verifies with its own list, a second gap shows. A round claimed a run on a signed branch when the host's signer name was one the gate trusts, whatever key the host signs with. Two hosts that both sign as `box`, `main` listing one's key and `release` the other's, each claimed both branches' runs; the result a host signed for the branch that does not list its key was never read back, and the branch read its checks as failed, blaming the submission for a host's key. Clause 87, as amended 2026-10-30, says such a host takes none of that branch's runs, that a run no host the branch would count is live to take stays open until the combination's time limit reads it not passed (66), and that a result signed with a key the branch does not list is missing, never a fail (28, S18).

## Considered options

Where a branch's signers and keys come from:

1. As built: one list per remote, from the first branch routed there, for the life of the process.
2. Keep the read of the results log branch-free; verify each signer's runs with the union of the keys every routed branch lists, tag each result with the key that verified it, and let the gate count a result only when its key is one its own policy lists.
3. One `remote refs` profile per branch, each with its branch's list.
4. The read of the results log names the branch whose gate reads it; the profile that keeps the state keeps one list per branch, read from the branch's own policy at its revision on its remote and read again only when that revision moves; the `remote refs` profile keeps no list and verifies with the one it is handed.

What a host whose key a branch does not list does with that branch's runs:

5. As built: claim by the signer's name, and read a result the branch does not count as a fail.
6. The core reads the host's key and the branch's allowed signers, and claims only where they match.
7. Ask the seam that keeps the state once per run whether the branch would count what this host signs.
8. Ask the seam that keeps the state once a round, where the round reads the gate, whether the branch at that revision lists the key this host signs with; claim, start and launch only where it does; read a check the branch counts no result for as missing, and open its run again.

## Decision outcome

Options 4 and 8.

**The read names its branch.** `State::results` takes the branch whose gate, session run or submission's result reads the log. Every caller in the core has it in hand: a round's run and landing, a session run, the view of a submission's result. A profile that reads no signed run, the host's own files, ignores it. Setting a result aside names no branch, so it looks for the result among every branch with a state, each read for itself and joined once; it is an operator's command, and a read per branch is its cost.

**Each branch's own list, by its revision.** The records profile keeps, per branch, the revision its list was read at and the list. A branch on its remote takes its revision from the remote's tracking ref as last fetched; the first time a process reads it, and only where no tick of that process asked the remote, it fetches the branch first, as every read of a branch on its remote does (C.7), and reads the tracking ref as last fetched when the remote can't be reached. While that revision stands the list is used as read (rule core.31); once it moves, the policy is read whole at the new revision before any run is verified for the branch. A remote that holds no revision of the branch, and a host with no tracking ref of it, give no list: the host's own copy is never read for one (124). A branch in the host's own files reads its list from its own policy at the revision its routing read, so the rule is the same for every branch, and a policy there naming no signer reads no signed run.

**The remote's profile verifies what it is handed.** `remote refs` keeps no signer and no key. It reads each named signer's runs and verifies them with the key the list gives for that signer, and remembers what it verified by the statement, the signer and a digest of the key's allowed-signers lines together, so a statement verified with one branch's key is never read as verified with another's.

**A host takes a run only where the branch would count it.** The records seam answers whether a branch's own policy, at the revision a round read, lists the key this host signs with for the signer it signs as (`State::verifies_own`). The records profile answers from the branch's list at that revision, with no fetch, and reads the public half of its own signing key once a process; a host that signs nothing, and a binding for guarding, answer no. A profile that verifies no signed run answers yes. A round asks once, where it reads the gate. On a signed branch a run counts here only when the answer is yes and the signer may claim what the gate needs, as before; a round claims, starts and launches on its pools only runs that count here. The signer's name alone takes nothing.

**A run no listed host takes, and a result not counted.** The time limit reads an open run as not passed once it is over when it does not count here, as it already did for a run of a system the host is not, so a run no listed host is live to take stays open and the combination is not passed at its limit (66). A check a run ran on a signed branch with no counted result for its key is missing, and a run with a missing check has not passed; a check whose key the tree did not bear out still fails (15). When a finished run leaves checks missing, none failed and no time out, the round opens the run again, its place released, and logs the checks as uncounted with the host that ran them, so a host whose result counts takes it and the branch's log says whose work went uncounted (28, S18).

**Why not option 2.** The gate would need the keys' identities in its rules, and the core cannot read a key: a key may be a file at the revision, read by the profile. It also keeps a list made of other branches' keys in the read path, which is the shape of the defect.

**Why not option 3.** The id counter, the hosts' signs of life, the tick's ask and the stale reads are the remote's, not a branch's; a profile per branch would hold each of them once per branch.

**Why not option 1.** It breaks 78 and core.22 whenever two branches on one remote name different keys or signers, and keeps a key the remote's policy has since dropped for as long as the process runs.

**Why not option 5.** It claims runs whose results the branch will never count, and reads them as fails, which blames a submission for a host's key; 28 and S18 forbid it, and 87 as amended takes nothing on the name alone.

**Why not option 6.** Keys and allowed-signers files are the profile's to read (core.2, core.22); the core would have to read files at a revision it reaches only through the seam.

**Why not option 7.** The answer depends only on the branch and the revision the round read, so once a round is enough, and asking per run would only repeat it.

### Consequences

Good: a branch's runs count by its own policy, whichever branch a process read first; a key added or removed on the remote takes effect at the next fetch of the branch, in a running driver too, and the policy is read for it once per move. Two hosts signing under one name each take only the runs of the branches that list their key, and a result a branch does not count waits for a host whose result it does, never failing the submission.

Bad: `State::results` changed its signature and `State::verifies_own` is new; every implementor is in this repository and follows them. A branch in the host's own files whose policy names no signer no longer reads another branch's signed runs, and may run checks it reused before. A branch no listed host serves reads its combinations not passed at the time limit, naming the checks missing. A run opened again for an uncounted result can be taken again by the same host, where its key is listed but its signed write is not read back; the time limit ends it, and the log's uncounted lines name the host.

Neutral: which results a branch in the host's own files counts, and which remotes' runs a branch reads, are as before; only the keys that verify the signed ones change. Rule core.22's details gain lines ruling the shared list, a claim on the name alone, an uncounted result read as a fail and the core reading a key out.

## More information

Left for the planner, each named in the change's proposal: whether a branch in the host's own files should count other hosts' signed runs at all, and whether a branch should read signed runs from remotes other than its own.
