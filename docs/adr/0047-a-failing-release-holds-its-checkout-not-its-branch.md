---
number: 47
title: A failing release holds its checkout, not its branch
status: accepted
date: 2026-10-23
decision-makers:
- Chuck Swanberg
---

# A failing release holds its checkout, not its branch

Deciders: the user, through clause 123 of the statement and the design of the change `a-failing-release-holds-its-place-not-its-branch`, 2026-10-23.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 123, 25, 34, 81, 122; ADRs 0016, 0020, 0045; rules core.2, core.10, core.11, core.16.

## Context and problem statement

A round released the checkouts of combinations whose result it had read, one after another, and stopped at the first release that failed. The round released before it decided or started anything, so one release that failed stopped every later round of its branch. On swancloud's `main`, the checkout `sluice/try/main/48` kept a release command the branch had since changed; it failed on every round, `main` tested nothing, and no command could clear it: `sluice release --worktree` on it answered "nothing to release". A stale checkout left by a round that stopped before its run started, and the checkouts a returning host releases before it claims (81), failed the same way.

On 2026-10-23, on deneb, a worktree of swancloud removed by hand, `swancloud/places/every-session-starts-in-bypass-whatever-file-claude-reads`, had its release tried on every round of an unrelated submission, #146 from `box-op-token`. The release ran the policy's toolchain in the worktree's directory, which no longer existed, failed, and printed the same failure each time.

Clause 123 rules both: a release that fails leaves its checkout held, named with what failed, tried again a bounded number of times and then reported; a held checkout never holds its branch; the operator releases it by name, or accepts its failure; and a checkout whose working copy is gone is freed at once, with nothing run in its name, and never tried again (25, 34). How does sluice hold a checkout, and free one, without holding its branch?

## Considered options

1. A failed release is an outcome the branch's state keeps: one release path counts each try, holds the checkout once the branch's `release_retries` are spent, and lets the round go on; the checkout seam says whether a working copy is gone and frees a checkout with nothing run.
2. As option 1, with a flag on `release` saying to skip the release command, in place of a seam method that frees.
3. As option 1, with each try counted in the checkout's own record on the host that made it, in place of the branch's state.
4. A failed release is passed over and tried again on every round, without a bound.
5. A held checkout's release command is read again from the branch's current policy, so a changed command clears it.

## Decision outcome

Option 1, in four parts.

**The seam frees, and says whether a working copy is gone.** `Checkout::gone(name)` is true when the checkout's record names a working copy that is no directory; a checkout with no record, or recorded before its working copy was made, is not gone. `Checkout::free(name)` does everything a release does but run the release command or save a cache: it ends the checkout's runs and removes their directories, removes the working copy sluice made, never a worktree's own (25, rule core.16), and lets its scratch directory, ports, record and, for a preview, names go. Whether a directory exists is the profile's to judge, and what to do about it the core's (rule core.2).

**The branch's state keeps each failed release.** The branch's state gains the checkouts whose release failed, each with what failed, its tries, when it last failed, whether it is held, and, on a branch whose records are on its remote, the host that made it, so a held checkout is readable from every host that submits to the branch (26, rule core.11). The log gains `release_failed`, one a failed try, and `gone`; `released` says what failure the operator accepted. A preview's checkout and a worktree's own are the host's alone (101; ADR 0035), so their failure stays on their record in the checkout profile.

**One release path for a combination's checkout.** The round's releases, a stale checkout's in recovery and a returning host's (81) all go through one function: a checkout whose working copy is gone is freed and logged `gone`; one released is logged `released`; one whose release fails counts a try, and once its tries pass the branch's `release_retries`, read from its current revision (56), 3 where it names none, it leaves the checkouts to release and is held. Only a failure to write the state is an error. The round goes on to land, tell outcomes and start combinations in other checkouts, whatever a release did (122). A stale checkout's run is opened again whatever its release did. A host holding a checkout it made whose release failed claims nothing, as 81 says, and the branch goes on through the other hosts.

**A gone working copy, everywhere.** A removed worktree's checkout, a deleted combination's checkout and a removed preview's checkout are freed with no policy read, no toolchain and no release command, and are never tried again. A release command whose working copy is gone is never started, by any path: where one would have run in the checkout's scratch directory, it is refused, naming the directory, so a path this decision missed fails loudly rather than running elsewhere.

The operator ends a held checkout by the name sluice printed: `sluice release sluice/try/<branch>/<n>` runs its release once more, whatever its tries, and `--accept-failure` frees it with nothing run. In a worktree, `sluice release --accept-failure` frees what sluice holds for the worktree and leaves the worktree.

Option 2 has the core decide inside a call the profile runs, and logs an accepted failure and a gone working copy as releases that worked. Option 3 keeps a held checkout readable only on the host that made it, where 26 asks for every host that submits to the branch, though the round already writes the branch's state for each release. Option 4 keeps swancloud's `main` moving but prints the same failure forever and never tells the operator the checkout needs them; 123 bounds the tries. Option 5 changes which release command a checkout runs, which ADR 0016 fixes as the one stored with its run, and answers only the stale command, not a release that fails for any other reason.

### Consequences

- No release, failed, retried or held, stops a round from deciding, landing, telling or starting.
- What a removed worktree's checks handed off, a container among them, is no longer torn down by sluice: with no working copy its release command cannot run as it was written to. The log's `gone` names the checkout, for the operator to look.
- Accepting a failure leaves behind whatever the release would have removed; it is the operator's choice, logged with what failed.
- A returning host with a held checkout claims nothing until the operator releases it, as 81 is written; whether 123 lifts 81 for a held checkout is the design's to say.
- State written before this change reads with no failed releases; a checkout already failing in it is counted from its next try and held on its own.
