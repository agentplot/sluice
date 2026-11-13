---
number: 118
title: A claimed run's checkout is made at its combination, and a checkout not made is a run that never began
status: accepted
date: 2026-11-12
decision-makers:
- the fix main--a-claimed-runs-place-is-made-at-its-combination
---

# A claimed run's checkout is made at its combination, and a checkout not made is a run that never began

Status: accepted on 2026-11-12, as the brief of the fix `main--a-claimed-runs-place-is-made-at-its-combination` decided it. The statement's 60, 82, 114 and C.6 decide it, and it holds no choice of the user's. It is cheap to change.

Sources: the statement's 60, 82, 114, 115 and C.6; record 0031; `crates/sluice-core/src/step.rs` `start_run`, `start_claimed` and `read_not_run`, and `crates/sluice/src/profiles/places.rs` `make_worktree`, at `045cb7d0`; switchboard-kit's state of `bolt/a-person-without-the-app-is-told-why`, 2026-11-11 and 2026-11-12.

## Context and problem statement

A combination's checkout is made with `git -C <common> worktree add --detach <path> <combination>`. On a branch whose state is on its remote, the combination may be formed on another host. The host that makes the checkout fetches every open combination of the branch once a round, before it claims. Nothing fetches the one combination the checkout is made at:

- A pool's job starts its claimed run with no round before it.
- A round's claim reads the state after its fetch of the combinations, so a combination opened between the two is claimed but not here.

On 2026-11-12 switchboard-kit's `bolt/a-person-without-the-app-is-told-why` tried checkouts 20, 21 and 22 at `05b3ce00`, each failing in seconds with `fatal: invalid reference: 05b3ce00…`. Each counted as a run whose checks could not run (114). The third read the combination as not passed (115) and blamed #61, every check named not run. Its resubmission, #62, at the same head, was blamed the same way at checkouts 23 to 25. On 2026-11-11 the same bolt's `3d2fd11b` failed twice so, and ran once its combination reached the host.

A checkout that cannot be made says nothing of the work. Its run started no check and no executor. 115's count exists so that a check that always says it could not run cannot hold the queue (record 0031). A host that cannot make a checkout is the host's trouble, not the submission's.

## Considered options

1. **The combination fetched by its own ref where the host lacks it, and a checkout not made is a run that never began, counting toward nothing** (chosen).
2. **Fetch every open combination of the branch before each start.** It asks the remote more than the start needs, once a run, where most starts need nothing fetched (82; rule core.32's way).
3. **The checkout's profile fetches the revision it is given.** A checkout profile reaching a remote calls another seam's profile (rule core.2).
4. **Keep counting a checkout not made toward 115.** A host's missing object, full disk or broken repository blames the submission, as #61 and #62 were.

## Decision outcome

- **A claimed run's checkout is made at its combination, however its claim reached the host.** Before the checkout is made, a host that lacks the combination fetches `refs/sluice/combinations/<branch>/<id>` from the branch's remote (C.6) into the ref its fetch of every combination fills. A host that holds it fetches nothing. This holds for a round's own start, a claim after the round's fetch, and a pool's job.
- **A checkout that cannot be made is a run that never began.** The run has no outcome (60): it is open again for any host, nobody is blamed, and it counts toward no runs in a row (115). Its record names each check with the error's words, as a run with no outcome does (16), until its next run has a result. Whatever of the checkout was made is released.
- **A start that fails once its checkout is made counts as before.** A toolchain that fails to enter, an engine not answering or an executor's refusal at the start still count toward 115.
- **A remote that cannot answer** the fetch refuses the round, as any read the round could not make (133).

### Consequences

- A checkout that can never be made is tried again each round, bounded by the branch's time limit (66) as a run no host can take is (87). A branch with no time limit tries it until the cause is mended or the submission is withdrawn.
- The statement holds as written: 114's checks that could not be started are those of a run that began; a run whose checkout was never made began no check, and is 60's run with no outcome. Should 114 or 115 name a checkout not made, that is the statement's to amend.
- Rule core.58 says it for a reviewer.
