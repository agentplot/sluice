---
number: 62
title: Every call to a remote runs under one retry policy, a crate's, that tries again only what the remote never judged
status: proposed
date: 2026-10-27
decision-makers:
- Chuck Swanberg
---

# Every call to a remote runs under one retry policy, a crate's, that tries again only what the remote never judged

Status: proposed to the user on 2026-10-27, as `briefs.15` has it; accepted only on their yes. Revised from the proposal of 2026-10-26 on the user's note to its card.

Deciders: madswan-design, on madswan-ops's signal 2026-10-26-madswan-ops-16f1e125/01 and madswan-planner's question of 2026-10-26, revised on the user's note of 2026-10-27; the user's yes is asked on the record's card.

The user's note, on the first proposal's card: "can't this just be done generically the way any http client would with backoff/retry? should we have a crate that supports this behavior so we don't have to invent it"

Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 24, 45, 46, 104, 105, 122, 133, 134, C.5, C.6, C.7 pushed refs; ADRs 0017, 0026, 0053, 0061; rules core.4, core.5; `crates/sluice/src/profiles/pushed_refs.rs` `remote_git`, `leased`, `push_branch`, `crates/sluice/src/profiles/github/app.rs` `issue` and `failed`, `crates/sluice-seams/src/lib.rs` `Unanswered`, `crates/sluice/src/lib.rs` `Unread` and `UNANSWERED_FOR`, `crates/sluice-core/src/release.rs` `release_retries`, as origin/main held them on 2026-10-27 (9cb1a69b); slc-2's chore `a-fetch-with-sluices-token-is-never-silently-refused` (c5fca2f8); crates.io read on 2026-10-27 for backon 1.6.0, backoff 0.4.0, retry 2.2.0, tokio-retry 0.3.2 and exponential-backoff 2.1.1, and backon's README.

## Context and problem statement

sluice reaches a remote two ways: git, run as a child process for every fetch and push, and the forge's HTTP API, for pull requests, checks and the app's tokens. A call fails for reasons the remote never judged, a connection not made, a server answering 5xx, a token GitHub refuses in its first seconds until all its servers know it, and for reasons that are the remote's answer: a lease stale, a branch protected, a pull request not found.

What is built tries again in three places, each written by hand with its own figure. A wait reads through a forge's 5xx answers for five minutes at its own pace (`Unread`). A fetch or a listing the remote refused on sluice's token runs once more on a token bought afresh and settled five seconds (the chore, c5fca2f8). A preview's release that fails is run once a round up to the branch's `release_retries`. A push is never run again, by rule core.5's words, so on 2026-10-26 a submit on afterthought/paymaster failed twice on a fresh token's first push, a bolt's creation failed at random, and a round the forge answered 5xx failed outright. The first proposal of this record added a fourth hand-written retry: a push the credential was refused on, made once more.

The user asked, on that proposal's card, whether this is not what any HTTP client does with backoff and retry, and whether a crate should carry it so sluice does not invent it.

## Considered options

1. One retry policy for every call to a remote, carried by a maintained crate: exponential backoff with jitter, bounded by what called it; each failure classed as the remote never having judged the call, which is tried again, as the remote's answer, which is not, or as not known, on which a read is tried again, since reading changes nothing, and a push is not, the branch being read instead; a credential the remote refused bought afresh for the next try.
2. Hand-written retries where each failure was met, as built, plus a push the credential was refused on made once more: the first proposal.
3. No push tried again; the next round settles a landing from its intent, and a submit or a branch's creation is run again by hand.

## Decision outcome

Option 1. The user's reading is right: this is an HTTP client's retry policy, and the one thing sluice adds is which failures of a git push count as the remote's answer.

**What the remote never judged is tried again; what it answered is not.** An HTTP client retries a request the server never processed: a connection refused, a timeout before anything was sent, a 5xx, a 429. It does not retry a 4xx that is the answer. The same line runs through sluice's calls: a 5xx (`Unanswered`), a connection not made and a credential refused before any ref was negotiated, not found, permission denied or unauthenticated on a token just bought, are tried again; a ref `[rejected]` under its lease, a branch protected and a pull request not found are the remote's answer and are reported as before (24, 45; rule core.5).

**A read is tried again whatever happened; a push only when the remote never judged it.** The HTTP client's other rule is idempotency: a GET is safe to repeat even when its answer was lost; a POST is repeated only when it is known not to have been applied. A fetch, a listing and a forge read are sluice's GETs. A push whose answer was not read, the connection cut after the pack was sent, is not pushed again: the branch is read, and what the remote holds decides (122, `perform_landing`), as ADR 0017 has it. The lease would make a second push as safe as the first (46), but reading is the cheaper and the clearer settlement, and it is what a landing's next step does already.

**A credential refused is bought afresh for the next try.** GitHub refuses an installation token in its first seconds, as the chore found and as Octokit's auth-app allows for. The policy's first pause for that class is the settle the chore gives a fresh token, and the next try carries a token bought afresh, never the one refused. A refusal that stands when the policy's time is spent is reported in the remote's words with what each refused credential was, as the chore reports today.

**One policy, in one place, from a crate.** Every call that reaches a remote, by git or by HTTP, goes through the policy at the seam that makes the call; a sleep-and-try loop written at a call site is ruled out, and the three built today fold into the policy when it comes. The backoff, the jitter, the bound and the retry predicate are a maintained crate's, not sluice's. Of those on crates.io on 2026-10-27, `backon` 1.6.0 (October 2025; blocking and async; `when` for the class of failure; `notify` for the log; no runtime needed) fits sluice's synchronous code; `backoff` 0.4.0 has not moved since 2021; `tokio-retry` is async only; `retry` 2.2.0 is simpler and would do; `exponential-backoff` only generates the delays. The crate is the construct's to choose within that class; a loop of sluice's own is not.

**Bounded by what called it.** A wait gives the policy the five minutes it reads through a forge's outage today, and says it could not read when they are spent. A round, a submit's push and a branch command give it seconds, not minutes: the round comes again on the timer, the asker is waiting at the prompt, and a refusal that stands is the round's event in the branch's log (133; ADR 0061), once per cause and host, never once per try.

Option 2 is what the user's note names: a fourth hand-rolled retry beside three others, each with its own figure and its own blind spot. Option 3 is the random failure of 2026-10-26.

### Consequences

- The pushed refs and remote refs profiles and the forge's HTTP client make every call under the one policy; a leased push tried again carries the lease and the refspec of the first (45, 46; rule core.5), and a push whose answer was not read is settled by reading the branch, never by pushing again.
- The wait's five minutes through 5xx and the chore's second read on a fresh token become the policy's bound and its credential class; `release_retries`, a release run once a round under the branch's policy, is another thing and stays as it is.
- Rule core.29 is drawn from this record, in place of its first proposal's words; rule core.5's details name what is not ruled out.
- Clause 134 of the statement says the same, as amended with this revision.
