---
number: 104
title: sluice's own branches count the runs deneb signs as sessions, and keep their state on origin
status: accepted
date: 2026-11-09
decision-makers:
- the brief of chore/sluices-main-names-denebs-signer
- the code stage of chore/sluices-main-names-denebs-signer
---

# sluice's own branches count the runs deneb signs as sessions, and keep their state on origin

Status: accepted. That sluice's own policy names deneb's signer `sessions`, with the public half of the key deneb's host file signs with, is the brief's, so that sluice's own pull requests are still taken in under the release of the bolt `sluice-runs-one-host-process`. D2 and D3 are the code stage's own, each cheap to change by a landing of the policy.

Deciders: the brief of `chore/sluices-main-names-denebs-signer` for D1; its code stage for D2 and D3.

Sources: clauses 78, 79, 108, 137 and C.6; records 0076 and 0103; afterthought/swancloud `vars/per-machine/deneb/sluice-signing/key.pub/value` and `docs/architecture/testing.md`, which say deneb signs its user's runs as `sessions`; `crates/sluice-core/src/policy.rs` (`Policy::parse`, `Rules::trusts_signed`).

## Context and problem statement

Record 0103 D2 refuses a pull request table on a branch whose state each host keeps, naming `[signers]`, and its consequences say sluice's own `main` is refused on the release that carries it until its policy names signers. sluice's `main` takes its work as pull requests (C.7), so that release would stop it.

Naming any signer keeps the state of every branch with a `remote` on that remote (C.6), and on such a branch `trust` lists signers alone (78). sluice's bolts name `remote = "origin"`, so their state moves with `main`'s.

Which signers does sluice's own policy name, how is a signer's key written, and what does each branch trust?

## Considered options

**D1. Which signers.**

1. **`sessions` alone, deneb's key** (chosen, the brief). deneb is where sluice's sessions work and its queues run, and its host file signs as `sessions`.
2. `sessions` and `mac`, mac-studio's gate, confined. The gate's runs would count, but which repositories it runs is the host's configuration, and the brief names deneb alone.

**D2. How the key is written.**

1. **One allowed-signers line in the policy** (chosen). One host signs as `sessions`, and its key written in the policy needs no other file.
2. A file of lines, `.config/sluice.sessions`, as a signer with many hosts has. It is a new file the gate's formatters must own or exclude by name (rule core.21). A second host moves the key there.

**D3. What each branch trusts.**

1. **`["sessions"]` on `bolt/*` and on `main`** (chosen). The policy refuses an executor kind in `trust` on a branch whose state is on its remote (78), so `main`'s `"confined"` and `"sandbox"` and the bolts' `"open"`, `"sandbox"` and `"container"` go. Confinement is a signer's claim (79), and `sessions`, on a host that runs agents, makes none.
2. The bolts without `remote`, their state left on each host. Bolts live on GitHub so that agents on any host share them, and their landings are leased pushes there.

## Decision outcome

Option 1 of each of D1 to D3. The tests `sluices_own_policy_reads` (`crates/sluice-core/src/policy.rs`) and `sluices_own_signers_each_read_as_one_key` (`crates/sluice/src/signing/ssh.rs`) hold the policy to them.

### Consequences

- `main`'s state is on origin, and the host that keeps it there takes its pull requests in (137).
- Every bolt's state and `main`'s move to origin under `refs/sluice/` at the first round of a sluice that reads this policy at the branch's revision; the first host to lead a branch pushes its own queued submissions as the starting state.
- A run counts on sluice's branches only where it was signed with deneb's key. A host whose `[signing]` key the policy does not list takes no runs of them (record 0076), mac-studio's gate among them.
- A replay counts only through a pass `sessions` signed for its key (108).
