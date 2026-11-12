---
number: 116
title: A failing release is tried its branch's retries, then held until asked again
status: accepted
date: 2026-11-11
decision-makers:
- the brief of 2026-11-11, signal 2026-11-11-agentplot-design-0cdaf45b/01
- the construct of a-failing-release-is-tried-a-few-times-then-held
---

# A failing release is tried its branch's retries, then held until asked again

Status: accepted. Clause 123 and C.7's `release_retries` decide the bound: "sluice tries the release again each round, as many times as the branch's policy allows, a few where it states none, then stops and reports the place as held … with what failed and how to release it", 3 where a branch states none. The choices within it, how a place no round tries counts its tries (D1), what asks a held place's release again (D2), what an unheld worktree's failed release keeps (D3) and the scratch every try runs with (D4), are the construct's own, each cheap to change. It amends record 0112 in part (D3).

Deciders: the brief of 2026-11-11, from signal 2026-11-11-agentplot-design-0cdaf45b/01; and the construct of the change `a-failing-release-is-tried-a-few-times-then-held`, which chose the form below.
Sources: signal 2026-11-11-agentplot-design-0cdaf45b/01 (afterthought/crew-state, `agentplot/main`); agentplot/blueprints `design/sluice/requirements.md` (origin/main a8234b3) clauses 25, 34, 35, 56 and 123 and C.7; `design/sluice/proposals/session-runs-1.md` clause 95 and C.1's session worktree row; `design/flume/requirements.md` clause 66 and C.6's sluice row; sluice at 5c4e580e: `crates/sluice/src/profiles/places.rs` `release_held`, `keep_failure`, `release_unheld`, `release` and `release_preview`, `crates/sluice/src/exec.rs` `release`, `crates/sluice-core/src/release.rs` `release_held` and `release_retries`, `crates/sluice-core/src/preview.rs` `release_preview`; Flume's `flume-core/src/tidy.rs` `tidy_place` at e2973d57; records 0018, 0055, 0063, 0112.

## Context and problem statement

On deneb, sluice held three of boomdrive's worktrees "whose release has failed about 1,600 times since 2026-11-10: boomdrive's release command site-stop is not found (exit 127), the toolchain's setup fails on a gone temporary folder (mktemp under /private/tmp/sluice-501/<id>/ says no such file or directory) and on conflicting .mcp.json and .claude/settings.json. sluice tries again without end and frees nothing". `sluice status` said "its release failed (1603 tries): sluice release runs it again, sluice release --accept-failure frees it without".

sluice at 5c4e580e:

- **A worktree's place is held at its first failure, and run again at every ask.** The place profile's `release_held` counts each failed try on the record and marks it stopped at once, "held at once: no round releases a session's place". `exec::release` reads the policy and calls `release_session` whatever the record's failure says. So "held" bounds nothing, and the count only grows. No round tries a worktree's place (25); its session or driver asks (session-runs-1 95), and Flume's tidy runs the teardown, `sluice release`, again on every pass while the place is kept (flume 66).
- **An unheld worktree's tries are never counted.** Record 0112's D4 lets go of the release's record whether the release passed or failed, so a teardown asking on every pass runs a failing command without end.
- **A held checkout or preview named by its caller runs again at every ask.** `Sluice::release_held` runs a held checkout's release once more, whatever its tries, and `release_preview` runs a held preview's again too. `Sluice::release_retries` reads the branch's bound at its current revision, 3 where it states none or cannot be read, and only a checkout's rounds and a preview's release are counted against it.
- **No release makes the scratch its record names first.** A run does (`adopt`), and so do a gone place's release and an unheld worktree's (rule core.30). A held worktree's release, a checkout's and a preview's run with `TMPDIR` naming whatever is left, which is how boomdrive's `mktemp` found no directory.

## Considered options

1. A held gate for every place: a worktree's tries counted against its branch's `release_retries` whoever asks, and a held worktree, checkout or preview running its release again only at `sluice release … --again`.
2. The gate for worktrees alone, a checkout or a preview named by its caller running again as today.
3. A time between tries, a back-off.
4. Each caller, Flume's tidy, crew's tidy and `wt submit`, bounding its own asks.
5. Record 0112's D4 kept, an unheld worktree's failed release keeping nothing.

## Decision outcome

Option 1.

- **D1. One bound, whoever asks.** A worktree's place counts each failed try of its release on its record, whoever asked, and is held once its tries pass the `release_retries` of the branch its record names, read at the branch's current revision (56), 3 where the branch states none or cannot be read: the first try and that many more. `Sluice::release_retries` is public, and `exec::release` passes it to `Checkout::release_session(name, policy, retries)`, as `release_preview` is given its retries; the profile's `keep_failure` stops at `tries > retries`, as a preview's does. This is the construct's reading of 123's "tries the release again each round, as many times as the branch's policy allows" for a place no round tries: its tries are its callers' asks.
  Not taken: holding a worktree's place at its first failure, which bounded nothing since every ask ran it; each caller bounding its own asks, three copies of C.7's bound where the family asks for one owner.
- **D2. A held place runs again only when asked again.** A plain `sluice release` of a held worktree, checkout or preview runs nothing, counts no try and reads no policy. It says the place is held, with what failed last and its tries, names `sluice release … --again` and `sluice release … --accept-failure`, and exits 1, so a caller keeps the worktree. `--again` runs the release once more and counts the try: passing frees the place, failing keeps it held with what failed now. `--again` on a place not held is a plain release, and `--again` beside `--accept-failure` is refused. A held checkout asked without `--again` is answered `Releasing::StillHeld`, with nothing run or written. `--again` is the operator's asking of 123, "running the release once more"; a teardown, a tidy or a skill asking on every pass is not.
  Not taken: the gate for worktrees alone, two rules for one word, "held", while a preview's caller may ask on every pass as Flume does; reading a name as the operator's asking, since crew's tidy passes `--worktree <path>` as an operator does; a time between tries, which needs a clock between commands (rule core.31) and still runs a release that fails for a reason no time mends; `--retry`, already sluice's word for a call to a remote tried again (rule core.29).
- **D3. An unheld worktree's failed release keeps its place.** Amending record 0112's D4: a failed release keeps the record it wrote, holding no ports block, with what failed and one try, as a held place's failure is kept, and from then on the worktree is one sluice holds. D1 counts its tries, D2 holds it, `sluice status` and the overview list it and `--accept-failure` frees it. A release that passes lets go of the record and its scratch, as before, and a policy that names no release command still answers `nothing to release` and keeps nothing. `sluice exec` later in that worktree adopts the record, and `adopt` gives a session's record with no ports block a block where the policy names ports, as it gives a record it makes, which answers 0112's reason that a later run would have no ports.
  Not taken: 0112's D4 as it stood, which leaves the incident open for every worktree no run ever held; the failures kept apart from the place's record, a second record of one place that status, the overview and `--accept-failure` would read as two stores.
- **D4. Every try has its scratch.** Before a release command runs, a worktree's, a candidate's checkout's or a preview's, the place profile makes the scratch the place's record names with `make_scratch`, as `adopt` does for a run: under the user's directory made the user's own (35 as amended), or where an older record names it, and where a record names none, the directory `scratch()` gives it. So the release's `TMPDIR` and `SLUICE_TMP`, built afresh or stored with the last run, name a directory that exists. A passing release removes it with the record; a failing one leaves it, named by the record, for the next try.
  Not taken: a fresh scratch path for the try, since the record names one path (rule core.30) and the stored environment names that path; leaving `TMPDIR` out of the release's environment, since the executor's sandbox gives the release only its scratch to write.

### Consequences

- Good: a teardown asked on every pass runs a failing release at most four times by default, then sluice answers held at once and runs nothing.
- Good: every try's `TMPDIR` names a directory that exists, so a release fails on its own command and never on a scratch the system emptied.
- Bad: `sluice release <checkout>` no longer runs a held checkout's release; the operator adds `--again`, which the held answer names.
- Bad: a worktree whose release fails for a cause that mends itself is held after its tries and stays held until asked again; a branch that wants more tries states `release_retries`.
- Neutral: Flume's teardown, crew's tidy, `wt submit` and `sluice exec --release` change nothing, since each already keeps its worktree when `sluice release` exits 1.
