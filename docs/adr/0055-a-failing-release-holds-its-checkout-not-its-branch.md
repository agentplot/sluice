---
number: 55
title: A failing release holds its checkout, not its branch
status: accepted
date: 2026-10-23
decision-makers:
- Chuck Swanberg
---

# A failing release holds its checkout, not its branch

Deciders: the user, through clause 123 of the statement and the design of the change `a-failing-release-holds-its-place-not-its-branch`, 2026-10-23.
Sources: `agentplot/blueprints:design/sluice/requirements.md` at c872766, clauses 123, 25, 34, 81, 122 and C.7; ADRs 0016, 0020, 0045; rules core.2, core.10, core.11, core.16, core.23.

## Context and problem statement

A round released the checkouts of combinations whose result it had read, one after another, and stopped at the first release that failed. The round released before it decided or started anything, so one release that failed stopped every later round of its branch. On swancloud's `main`, the checkout `sluice/try/main/48` kept a release command the branch had since changed; it failed on every round, `main` tested nothing, and no command could clear it: `sluice release --worktree` on it answered "nothing to release". A stale checkout left by a round that stopped before its run started, and the checkouts a returning host releases before it claims (81), failed the same way.

On 2026-10-23, on deneb, a worktree of swancloud removed by hand, `swancloud/places/every-session-starts-in-bypass-whatever-file-claude-reads`, had its release tried on every round of an unrelated submission, #146 from `box-op-token`. sluice built the release afresh from the policy, which ran the policy's toolchain in the worktree's directory, which no longer existed; it failed with `devenv print-dev-env: No such file or directory`, and printed the same failure each time.

Clause 123 rules both: a release that fails leaves its checkout held, named with what failed, tried again as many times as the branch's `release_retries` allows (three where it names none, C.7) and then reported; a held checkout never holds its branch; the operator releases it by name, or accepts its failure; and a checkout whose working copy is already gone is no one's: sluice runs its release once, from what it itself holds for the checkout and with the checkout's ports, records how that went, frees what it holds whatever the outcome, records it gone, and never tries it again (25, 34). Clause 81 says a held checkout bars no claim of its host. How does sluice hold a checkout, and free one, without holding its branch or its host?

## Considered options

1. A failed release is an outcome the branch's state keeps: one release path counts each try, holds the checkout once the branch's `release_retries` are spent, and lets the round go on; the checkout seam says whether a working copy is gone, runs a gone checkout's stored release once from its scratch directory, and frees a checkout with nothing run.
2. As option 1, with a flag on `release` saying to skip the release command, in place of a seam method that frees.
3. As option 1, with each try counted in the checkout's own record on the host that made it, in place of the branch's state.
4. A failed release is passed over and tried again on every round, without a bound.
5. A held checkout's release command is read again from the branch's current policy, so a changed command clears it.

## Decision outcome

Option 1, in four parts.

**The seam frees, and says whether a working copy is gone.** `Checkout::gone(name)` is true when the checkout's record names a working copy that is no directory; a checkout with no record, or recorded before its working copy was made, is not gone. `Checkout::free(name)` does everything a release does but run the release command or save a cache: it ends the checkout's runs and removes their directories, removes the working copy sluice made, never a worktree's own (25, rule core.16), and lets its scratch directory, ports, record and, for a preview, names go. `Checkout::release_gone(name)` releases a checkout whose working copy is gone from what sluice holds for it: the release command its last run stored, with the environment stored with it, its ports and what its toolchain gave the run among it, run once with the checkout's scratch directory as its working directory; then it frees the checkout as `free` does, whatever the command did, and says whether a command ran and what it printed when it failed. It notes on the checkout's record that the release has started before it starts it, so a free that fails part way and is tried again runs nothing. Whether a directory exists is the profile's to judge, and what to do about it the core's (rule core.2).

**The branch's state keeps each failed release.** The branch's state gains the checkouts whose release failed, each with what failed, its tries, when it last failed, whether it is held, and, on a branch whose records are on its remote, the host that made it, so a held checkout is readable from every host that submits to the branch (26, rule core.11). The log gains `release_failed`, one a failed try, and `gone`, saying whether the gone checkout's stored release ran and what it printed when it failed; `released` says what failure the operator accepted. A preview's checkout and a worktree's own are the host's alone (101; ADR 0035), so their failure stays on their record in the checkout profile.

**One release path for a combination's checkout.** The round's releases, a stale checkout's in recovery and a returning host's (81) all go through one function: a checkout whose working copy is gone has its stored release run once and is freed, logged `gone` with how its release went; one released is logged `released`; one whose release fails counts a try, and once its tries pass the branch's `release_retries`, read from its current revision (56), 3 where it names none (C.7), it leaves the checkouts to release and is held. Only a failure to write the state is an error. The round goes on to land, tell outcomes and start combinations in other checkouts, whatever a release did (122). A stale checkout's run is opened again whatever its release did. A held checkout whose working copy is gone since is released once by the next round, as any gone checkout is. A returning host releases each checkout it left, then claims up to its bound whatever those releases did: once each is released, gone or held, a held checkout bars no claim (81), and keeps its ports and disk beside the checkouts the claims run in.

**A gone working copy, everywhere.** A removed worktree's checkout, a deleted combination's checkout and a removed preview's checkout each have the release their last run stored run once, from the checkout's scratch directory with its ports, with no policy read and no toolchain entered, and are freed whatever it did; none is tried again. The stored release is the one sluice holds: one built afresh from the policy enters the toolchain in a working copy that is gone, which is what failed on deneb, and reads a policy the checkout never ran under. Only that path runs a gone checkout's release: a release asked of a checkout whose working copy is gone by any other path is refused, naming the directory, so a path this decision missed fails loudly rather than running in a fallback.

The operator ends a held checkout by the name sluice printed: `sluice release sluice/try/<branch>/<n>` runs its release once more, whatever its tries, and `--accept-failure` frees it with nothing run. In a worktree, `sluice release --accept-failure` frees what sluice holds for the worktree and leaves the worktree.

Option 2 has the core decide inside a call the profile runs, and logs an accepted failure and a gone working copy's release as releases of a checkout that was there. Option 3 keeps a held checkout readable only on the host that made it, where 26 asks for every host that submits to the branch, though the round already writes the branch's state for each release. Option 4 keeps swancloud's `main` moving but prints the same failure forever and never tells the operator the checkout needs them; 123 bounds the tries. Option 5 changes which release command a checkout runs, which ADR 0016 fixes as the one stored with its run, and answers only the stale command, not a release that fails for any other reason.

### Consequences

- No release, failed, retried or held, stops a round from deciding, landing, telling or starting.
- A gone checkout's release that tears down a stack by name or port, a container among them, still does, since it runs with the checkout's ports and stored environment. One that reads files from its working copy finds none in the scratch directory and fails; it is run once all the same, `gone` records what it printed, and the checkout is freed.
- A gone checkout's release that hangs is held to its run's time limit, as every release is, then its group is ended.
- Accepting a failure leaves behind whatever the release would have removed; it is the operator's choice, logged with what failed.
- A returning host with a held checkout claims as any other host does; the held checkout keeps its ports and disk until it is released.
- State written before this change reads with no failed releases; a checkout already failing in it is counted from its next try and held on its own.
