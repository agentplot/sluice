---
number: 62
title: A push the remote refused on the credential before any ref was negotiated is made once more
status: proposed
date: 2026-10-26
decision-makers:
- Chuck Swanberg
---

# A push the remote refused on the credential before any ref was negotiated is made once more

Status: proposed to the user on 2026-10-26, as `briefs.15` has it; accepted only on their yes.

Deciders: madswan-design, on madswan-ops's signal 2026-10-26-madswan-ops-16f1e125/01 and madswan-planner's question of 2026-10-26; the user's yes is asked on the record's card.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 24, 45, 46, 104, 105, 122, 124, 134, C.5, C.6, C.7 pushed refs; ADRs 0017, 0026, 0053; rules core.4, core.5; `crates/sluice/src/profiles/pushed_refs.rs` `leased`, `make`, `remove`, `push_branch`, `crates/sluice-core/src/step.rs` `perform_landing` as origin/main held them on 2026-10-26 (ef6d414b); slc-2's chore `a-fetch-with-sluices-token-is-never-silently-refused` (c5fca2f8).

## Context and problem statement

sluice reaches a forge as an app, with an installation token bought per process and handed to git at once. GitHub's git servers refuse such a token on its first use now and then, a private repository reading as not found and a push as permission denied, and accept the same token moments later. On 2026-10-26 a submit on afterthought/paymaster failed twice this way, flume's bolt saw one 403, and bolt creation failed at random. slc-2's chore runs a refused fetch or listing once more on a credential bought afresh and settled five seconds, and reports a refusal that stands in the remote's words with what each refused credential was; it never runs a push again, citing rule core.5.

Every push sluice makes is leased or writes a ref of its own: a landing and a branch removed are leased on the revision expected, a branch made is leased on its absence (45, 104, 105), a session's branch for its pull request is the session's own, and the records' refs are content-named or leased on the commit read (C.5, C.6). A refusal of the credential comes before any ref is negotiated, so the remote is as it was. Is such a push made once more, and how does that sit with a move that is never retried blind?

## Considered options

1. Made once more, on a credential bought afresh and settled, with the lease and the expectation of the first, when the remote refused the credential before any ref was negotiated; a push refused at the ref is the move refused, and one whose answer was not read is never made again blind: the branch is read and what the remote holds decides (clause 134).
2. Never made again; the next round settles a landing from its recorded intent, and a submit or a branch's creation is run again by hand, which is what was built.
3. Every failed push made once more.

## Decision outcome

Option 1.

**The lease makes the second push as safe as the first.** Rule core.5 and clause 46 are about the expectation: a move is made only from the revision expected, and two moves from one expectation never both succeed. A credential the remote refused before it read a single ref left the branch where the first push expected it; the second push carries the same lease, so if anything moved the branch between the two, the remote refuses the second at the ref as it would have refused the first. Nothing is retried blind: what is retried carries its expectation.

**Only the credential's refusal, and once.** A refusal at the ref, the lease stale or the branch protected, is the move refused (24, 45) and is reported with what the remote holds, as before. An answer that was not read, a connection cut after the push may have been applied, is not pushed again: the branch is read, and a landing's intent is settled from what the remote holds (122, `perform_landing`), as ADR 0017 has it. One second try, as the chore gives a read: the forge's lag is seconds, and a credential refused twice is refused for a reason the operator must read, reported in the remote's words with what each credential was.

**Every push alike.** A landing's push is already made again by the next step from the landing's intent, but the process that met the refusal exits as sluice itself failed, and a `sluice branch create`, a `branch delete` or a submit's push has no next round of its own: it fails in the asker's face and crew's bolt give with it. One rule for every push, in the profile that makes them, leaves the core unchanged.

Option 2 is today's random failure of submits and bolt creation. Option 3 would push again after an answer that was not read, which the lease still guards but which turns a refusal to be reported into a success to be explained.

### Consequences

- The pushed refs and remote refs profiles run a push refused on the credential once more on a fresh credential, with the same lease and refspec, and report a second refusal in the remote's words with what each credential was, as the chore reports a read's.
- `--force-with-lease` is on every push that moves a branch sluice lands on, as before (rules core.4, core.5), and the session's own branch and the content-named refs stay as they were.
- Rule core.5's details say what is not ruled out; rule core.29 is drawn from this record.
