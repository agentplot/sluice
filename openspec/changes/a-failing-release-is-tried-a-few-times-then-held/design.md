# Design

## Context

**The brief.** A place whose release fails is tried again only as often
as its branch's policy allows, a few times where it states none. Then it
is held and reported with what failed and how to release it, and tried
no more on its own. Each try runs with the temporary folder sluice gives
the release in place.

**Sources.**
- Signal 2026-11-11-agentplot-design-0cdaf45b/01 (afterthought/crew-state,
  `agentplot/main`), agentplot-design on deneb: "sluice on deneb holds
  three of boomdrive's places … whose release has failed about 1,600
  times since 2026-11-10: boomdrive's release command site-stop is not
  found (exit 127), the toolchain's setup fails on a gone temporary
  folder (mktemp under /private/tmp/sluice-501/<id>/ says no such file
  or directory) and on conflicting .mcp.json and .claude/settings.json.
  sluice tries again without end and frees nothing", quoting `sluice
  status`: "its release failed (1603 tries): sluice release runs it
  again, sluice release --accept-failure frees it without".
- agentplot/blueprints `design/sluice/requirements.md` (origin/main
  a8234b3): clause 123, quoted in the proposal; C.7's `release_retries
  = 3  # a failing release is tried this often, then its place is held
  (123)` and "one with no `release_retries` tries a failing release
  three times (123)"; clauses 25 ("never releases a session's place …
  releasing its place is its driver's"), 34 and 35 ("the place's record
  names it, its release removes it").
- agentplot/blueprints `design/sluice/proposals/session-runs-1.md`,
  clause 95 and C.1's session worktree row: a session's place is
  released by `sluice release` in it, at its session's or driver's
  asking.
- agentplot/blueprints `design/flume/requirements.md`, clause 66 ("A
  teardown that fails keeps the place, named with the step that
  failed") and C.6's sluice row: Flume's teardown is `sluice release`.
- deneb, 2026-11-11: boomdrive's `main` log holds `released` for the
  three worktrees at 21:39:33Z and 21:39:34Z, after the signal. Their
  failed tries are in no log: a worktree's tries are kept only on its
  record.

**sluice today, read on this branch at 5c4e580e.**
- `crates/sluice/src/profiles/places.rs:870` `release_held`: a
  session's place's release, through `runs.remove_all_with`. A failure
  goes to `keep_failure(name, e, |_| true)` (`:841`), which counts a
  try on the record and marks it `stopped` at once: "held at once: no
  round releases a session's place". Nothing makes the scratch first.
- `crates/sluice/src/exec.rs:666` `release`: `sluice release` in a
  worktree. With a record it reads the policy and calls
  `release_session` whatever the record's failure says. So every ask
  runs the release, and `tries` only grows.
- `:913` `release_unheld` (record 0112): a worktree with no record gets
  one for the release's length, and the record is let go whether the
  release passed or failed (0112 D4). Its tries are never counted.
- `crates/sluice-core/src/release.rs:128` `release_held`: the
  operator's `sluice release sluice/try/<branch>/<n>` runs a held
  checkout's release once more, whatever its tries. `:87`
  `release_retries` reads the branch's bound at its current revision,
  with `step::RELEASE_RETRIES`, 3, where it states none or cannot be
  read.
- `crates/sluice-core/src/preview.rs:261` `release_preview`: a caller's
  `sluice release <preview>` counts against the branch's retries
  (`places.rs:1985`), and runs again at every ask once held.
- `places.rs:1614` `release` (a candidate's checkout) and `:1985`
  `release_preview` run the stored release through `runs.remove`, with
  the environment the last run stored. `release_held` builds a fresh
  one through the check seam (`checks.rs:1385`: `TMPDIR` is
  `place.tmp`, the scratch the record names). None makes the scratch
  first. A run does (`adopt`, `:1875`), and so does a gone place's
  release (`release_gone`, `:1679`) and an unheld worktree's (`:940`).
- Flume's tidy (`flume-core/src/tidy.rs`, `tidy_place`, at e2973d57)
  runs the teardown of every place whose work is over on each settle,
  and keeps it with `Kept::Teardown` when it fails. Its next settle runs
  the teardown again. crew's tidy and `wt submit` keep a worktree whose
  `sluice release` exits non-zero.
- The words: `lib.rs:6351` `write_worktrees`, `lib.rs:6194`
  `release_checkout`, `exec.rs:175` `tell_releases`,
  `preview.rs:41` `tell_dropped`, `preview.rs:145` and `:326`,
  `overview.rs:542` `held_places` and `:619` `write_held_places`,
  `plugins/sluice-board/hooks/board.ts:306`,
  `plugins/herdr/lib/words.jq:148`. Each gives `sluice release <it>`
  as what runs a held place's release again.

## Goals / Non-Goals

**Goals:**
- A worktree's failing release is run at most its branch's
  `release_retries` and once more, whoever asks, then held.
- A held place of any kind runs its release again only when asked
  again in so many words. Every other ask answers held, runs nothing
  and counts nothing.
- An unheld worktree's failed release is counted and held as a held
  one's is.
- Every release runs with the scratch its record names in place.
- Wherever a held place is shown, the command that runs it again is
  the one that does.

**Non-Goals:**
- Changing Flume, crew or `wt submit`.
- Logging a worktree's tries on its branch.
- Boomdrive's own release command and toolchain.
- A time between tries. Tries are counted, never timed (D2).
- A checkout's tries, which rounds already bound as 123 says.

## Decisions

### D1. One bound, whoever asks

A worktree's place counts each failed try of its release on its record,
whoever asked for it, and is held once its tries pass the branch's
`release_retries`: the first try and that many more. The branch is the
one its record names, the branch its runs were held to, or the default
branch for a worktree whose release wrote its record (record 0112 D2).
The bound is read by `Sluice::release_retries`, made public, at the
branch's current revision (56). It falls back to `RELEASE_RETRIES`, 3,
where the branch states none or cannot be read, as a checkout's does,
so a branch that cannot be read never stops a release. `exec::release`
passes it to `Checkout::release_session(name, policy, retries)`, as
`release_preview` is given its retries. The profile's `keep_failure`
then stops at `tries > retries`, as a preview's does.

The tries of a worktree's place are its callers' asks. No round
releases it (25), and its session or driver asks (session-runs-1 95).
This is how the change reads 123's "tries the release again each round,
as many times as the branch's policy allows" for a place no round
tries.

Not taken:
- **Holding a worktree's place at its first failure, as today.** That
  is the incident. "Held" bounded nothing, since every ask ran it.
- **Each caller bounding its own asks.** Flume's tidy, crew's tidy and
  `wt submit` would each need a count of their own, three copies of
  C.7's bound. The family rule asks for one owner (D7).

### D2. A held place runs again only when asked again

A plain `sluice release` of a held place runs nothing, counts no try
and reads no policy, whether it is a worktree (`--worktree`, or the
folder it runs in), a checkout or a preview by name. It answers that
the place is held, with what failed last and its tries. It names the
two commands, `… --again` to run its release once more and `…
--accept-failure` to free it without. It exits 1, so a caller keeps the
worktree. `--again` on a held place runs its release once more and
counts the try. Passing frees the place. Failing keeps it held, with
what failed now. `--again` on a place not held is a plain release, and
`--again` with `--accept-failure` is refused by the parser. `sluice
exec --release` asks as a plain release does.

`--again` is the operator's asking of 123, "running the release once
more". A teardown, a tidy or a skill that asks on every pass is not.

Where the check is made:
- **A worktree.** `exec::release` reads the place's kept failure through
  `Checkout::session_places`, as `accept_session` does, before it reads
  the policy.
- **A checkout.** `Sluice::release_held(branch, place, accept, again)`
  reads its entry in the branch's `failed_releases`. A stopped entry
  asked without `again` answers `Releasing::StillHeld { failed, tries }`
  with nothing run and nothing written.
- **A preview.** `preview::release` reads the preview the seam lists,
  which it reads today to end its session, and answers from its
  `release_failed`.

Not taken:
- **The gate for worktrees alone, a named checkout or preview running
  again.** That gives two rules for one word, "held". A preview is
  released at its caller's asking (previews-1 101), so a caller may ask
  on every pass as Flume does.
- **Reading the name as the operator's asking.** crew's tidy passes
  `--worktree <path>`, as an operator does.
- **A time between tries, or a back-off.** That needs a clock between
  commands (rule core.31). It still runs a release that fails for a
  reason no time mends, as `site-stop` not found did.
- **`--retry`.** `retry` is already sluice's word for a call to a
  remote tried again (rule core.29, `retry.rs`).

### D3. An unheld worktree's failed release keeps its place

Record 0112 D4 let go of the release's record whether it passed or
failed, so an unheld worktree's tries were never counted. A teardown
asking on every pass would run a failing command without end. That is
the incident's own shape, for every worktree no run ever held. Now a
failed release keeps that record, holding no ports block, with what
failed and one try, as a held place's failure is kept. From then on it
is a worktree sluice holds. D1 counts its tries and D2 holds it.
`sluice status` and the overview list it, and `--accept-failure` frees
it. A release that passes lets go of the record and its scratch, as
today. A policy that names no release command still answers `nothing
to release` and keeps nothing.

`sluice exec` in such a worktree adopts the record. `adopt` gives a
session's record with no ports block a block where the policy names
ports, as it gives a record it makes. That answers 0112's own reason
for D4, that a later run would have no ports. 0112's other reason was
that `--accept-failure` would free a hold made only to show a failure.
Under D2, that hold is what keeps the release from running again.

Not taken:
- **0112 D4 as it stands.** It leaves the incident open for an unheld
  worktree.
- **The failures kept apart from the place's record.** That would be a
  second record of one place, and status, the overview and
  `--accept-failure` would read two stores.

### D4. Every try has its scratch

Before a release command runs, the place profile makes the scratch the
place's record names, as `adopt` does for a run. That holds for a
worktree's (`release_held`), a candidate's checkout's (`release`) and a
preview's (`release_preview`). The scratch is made with `make_scratch`,
under the user's directory made the user's own (35 as amended), or
where an older record names it. A record with no scratch named gets the
directory `scratch()` gives it. So the release's `TMPDIR` and
`SLUICE_TMP`, whether built afresh (`checks.rs:1385`) or stored with
the last run, name a directory that exists. It is the same path, made
again. A passing release then removes it with the record, as today. A
failing one leaves it, named by the record, for the next try. A gone
place's release and an unheld worktree's already make it.

Not taken:
- **A fresh scratch path for the try.** The record names one path
  (rule core.30), and the stored environment names that path.
- **Leaving `TMPDIR` out of the release's environment.** The executor's
  sandbox gives the release only its scratch to write.

### D5. The words

- **`sluice release`, held, asked without `--again`.** Plain text:
  `<it> is held: its release failed <n> times: <last line of what
  failed>`, then `` `sluice release <it> --again` runs it again;
  `sluice release <it> --accept-failure` frees it without ``. `<it>` is
  the checkout's or preview's name, or `--worktree <path>` for a
  worktree. `--json`: `{"worktree"|"place"|"preview", "released":
  false, "held": true, "tries": <n>, "failed": "<what failed last>"}`,
  with `"accepted": false` where a checkout's answer carries it today.
  The text goes to stderr. Exit 1.
- **A failed try.** As today, `could not release <it>: <what failed>`,
  then `` `sluice release <it>` runs it again `` while tries remain, or
  the held line's two commands once this try held it. `--json` adds
  `"held"` and `"tries"` to the failure's document.
- **`sluice status`, a worktree.** While tries remain: `its release
  failed (<n> of <m> tries): sluice release runs it again, sluice
  release --accept-failure frees it without`, `<m>` the branch's
  retries and one. Held: `its release failed (<n> tries) and is held:
  sluice release --again runs it again, sluice release --accept-failure
  frees it without`. `--json` keeps `release_failed` as it is.
- **`sluice status <branch>`, its checkouts.** The footer names `sluice
  release <checkout> --again` for a held one.
- **A round's report of a hold** (`tell_releases`), **a preview held by
  a pass** (`tell_dropped`) and **`sluice previews`.** Each gives
  `sluice release <it> --again`.
- **The overview.** A held place's second line gives `sluice release
  <it> --again runs it again; sluice release <it> --accept-failure
  frees it without`. The board's and herdr's held line gives `sluice
  release <it> --again · --accept-failure frees it without`.

### D6. The seam

- `Checkout::release_session(name, policy, retries)`. The profile and
  the fake count a failed try against `retries` and keep an unheld
  worktree's failed release (D3).
- `Checkout::adopt`'s contract says a session's record with no ports
  block is given one where the settings name ports.
- `Releasing::StillHeld { failed, tries }` in `sluice-core`. No round
  makes it. `releases_json` gives it as `held`.
- `Sluice::release_held(branch, place, accept, again)`.

Rules core.1 and core.2 hold: the bound is read by the core, the count
is kept by the profile, and the CLI asks both through their seams.

### D7. The family

A release is tried by sluice's rounds for a checkout and a preview, and
by Flume's tidy, crew's tidy, `wt submit` and a session for a worktree.
The bound on how often is sluice's alone: 123 and C.7's
`release_retries`. Every caller calls `sluice release` and keeps its
place when it exits 1, naming sluice's words (flume 66). Once the place
is held, sluice runs nothing and answers at once. A caller that asks on
every pass, as Flume's tidy does, costs a process a pass, and the
words it keeps name `--again` and `--accept-failure` for the operator.
No caller counts tries of its own, and none changes. The statement
holds as written. 123 bounds the tries, and "the operator's asking …
running the release once more" is `--again`. So nothing is sent to
blueprints.

### D8. The record

A new record in `docs/adr/`, the next free number. 0113, 0114 and 0115
are taken on other branches of this bolt's family, so 0116 at the
time of writing. Title: "A failing release is tried its branch's
retries, then held until asked again". It decides D1 to D4 and amends
record 0112 in part (D4). Its deciders are the brief of 2026-11-11 and
this change's construct, every choice its own and cheap to change.
Record 0112 gains "Amended in part by record NNNN: a failed release of
a worktree sluice holds nothing for keeps its place, held once its
tries are spent."

### Rules relied on

- **core.10**: a place is released only by running its release
  command, and `--accept-failure` frees it with nothing run.
- **core.16**: a session's place is released only by `sluice release`
  at its session's or driver's asking. This change bounds how often
  that asking runs the release.
- **core.30**: a scratch the system emptied is made again at the path
  its record names. D4 does that for a release.
- **core.31**: nothing runs between commands. Tries are counted on the
  record, never timed.
- **core.1, core.2**: D6.

## Risks / Trade-offs

- **An operator used to `sluice release <checkout>` running a held
  release once more** now gets the held answer and must add `--again`.
  The answer names the command.
- **A caller that treats exit 1 as "try again soon"** keeps asking. Each
  ask now costs a read of the place's record and nothing else.
- **A worktree whose release fails for a cause that mends itself** is
  held after four tries and stays held until asked again. That is 123's
  bound. A branch that wants more tries states `release_retries`.
- **Sibling changes on this bolt** touch `places.rs` too:
  `a-place-whose-folder-is-gone-is-freed` (record 0115) and
  `a-previews-containers-end-with-it` (record 0114). The builder takes
  in the bolt before starting and keeps their reading of gone and
  their container sweep. D4's scratch is made before their sweep runs,
  and D2's gate is read before either.

## Migration Plan

Nothing to migrate. A worktree's record held by an older sluice, with
`stopped` true and any number of tries, is held as this change holds
one. Its next plain release runs nothing, `--again` runs it, and
`--accept-failure` frees it. A record with `stopped` false keeps
counting from its tries.

## Task notes

**1.1 The record.** MADR, as 0112 is: front matter (`number`,
`title`, `status: accepted`, `date: 2026-11-11`, `decision-makers`:
"the brief of 2026-11-11, signal 2026-11-11-agentplot-design-0cdaf45b/01"
and "the construct of a-failing-release-is-tried-a-few-times-then-held").
Context: the signal's words and the four facts of "sluice today". Considered
options: D2's held gate for every place; the gate for worktrees alone;
a back-off; each caller bounding its own asks; 0112 D4 kept. Decision
outcome: D1 to D4, each with its "Not taken". Consequences: good, a
teardown asked on every pass runs a failing release at most four times
by default; bad, `sluice release <checkout>` no longer runs a held
release without `--again`; neutral, Flume and crew change nothing.
Record 0112 gets the "Amended in part" line of D8 under its status
line.

**1.2 The rules.** Add to core.16's details, after its "Also rules
out": "Also rules out: a held place's release run by an ask without
`--again`; a worktree's release run more than its branch's
`release_retries` and once, whoever asks, without `--again` (deneb,
2026-11-10 to 2026-11-11: three of boomdrive's worktrees tried 1,603
times by Flume's teardown on every pass, signal
2026-11-11-agentplot-design-0cdaf45b/01; record NNNN). Not ruled out:
a plain ask of a held place answered held, exit 1, with nothing run;
`--again` running it once more whatever its tries." Add to core.30's
details: "Also rules out: a release run while the scratch its record
names is gone (the same signal: `mktemp` under
`/private/tmp/sluice-501/<id>/` finding no directory on every try;
record NNNN)." Add `NNNN` to each rule's Source.

**2.x Tests.**
- `a_failed_session_release_is_held_once_its_retries_are_spent`
  (`places.rs`) is today's
  `a_failed_session_release_keeps_its_place_with_what_failed`, renamed.
  Given `retries` 1: the first failure has tries 1, not stopped, ports
  and scratch kept; the second has tries 2, stopped; `free` lets it go
  and leaves the worktree.
- `an_unheld_worktrees_failed_release_keeps_its_place` (`places.rs`):
  a linked worktree with no record. `release_session` under a policy
  whose release exits 3, retries 0. It fails, and `session_places`
  lists the worktree with the policy's branch, no ports, tries 1,
  stopped, and its scratch exists. A passing release of another
  worktree leaves no record.
- `a_worktrees_place_without_ports_is_given_them_at_its_next_run`
  (`places.rs`): a session record with no block, then `adopt` with
  ports named. The place's env has `SLUICE_PORT_<NAME>`, and the
  record names the block.
- `every_release_runs_with_the_scratch_its_record_names` (`places.rs`):
  for a session's place (fresh release), a candidate's checkout and a
  preview, each with a stored or built release of
  `test -d "$TMPDIR" && mktemp "$TMPDIR/x.XXXXXX"`, the scratch removed
  with `remove_dir_all` before the release. Each release passes.
- `a_worktrees_release_is_tried_its_branchs_retries_then_held`
  (`cli.rs`, `setup_one_retry` with a release command, fakes): `exec`,
  then every release fails. The first `sluice release` exits 1, one
  attempt, not held. The second exits 1, held. The third exits 1 with
  no attempt, saying `is held` and naming `--again` and
  `--accept-failure`, `--json` giving `held: true, tries: 2`. `--again`
  is one attempt, still held, tries 3. With releases passing,
  `--again` releases it, exit 0.
- `release_by_name_releases_a_held_checkout` (`cli.rs`): after
  `held_checkout`, `sluice release <place>` exits 1 with no attempt,
  saying it is held. `--again` is one attempt, then fails. With
  releases passing, `--again --json` gives today's released document.
- `a_held_previews_release_runs_again_only_when_asked_again`
  (`crates/sluice/tests/previews.rs`, fakes): a preview whose release
  fails, with `release_retries = 0`. The first `sluice release
  <preview>` holds it. The second runs nothing and exits 1. `--again`
  runs it.
- Expectations that change: `status_marks_a_worktree_whose_release_failed`
  (one try of four, not stopped); `overview_names_a_held_worktree_under_its_branch`
  and `overview_names_a_held_worktree_of_a_branch_with_no_records`
  (held after its tries, `--again` in the line);
  `overview_names_a_held_checkout_under_its_branch` and
  `a_waiting_submit_says_a_held_checkout_once` (`--again`);
  `release_accepting_the_failure_frees_a_worktrees_place` (unchanged
  but for the signature); the conformance scenario
  `a_held_checkout_is_released_by_its_name`
  (`crates/sluice-conformance/tests/places.rs`, `release_held`'s
  `again`); S27 (`release_session`'s retries);
  `an_unheld_worktrees_release_runs_its_release_command`
  (`crates/sluice/tests/exec.rs`: after the failing release, `sluice
  status` lists the worktree with one try, and `--accept-failure`
  frees it).

**3.x The site.** `reference/commands.md`: the `sluice release` rows
gain `--again`; "Checkouts held because their release failed" and
"Worktrees sluice holds" say D1 and D2 in the reader's words; the
`--json` table adds `held` and `tries`; the overview sample shows
`--again`. `reference/configuration.md`'s `release_retries`: the bound
holds for a checkout, a worktree and a preview. A worktree's tries are
every `sluice release` in it, and a held place runs again only with
`--again`. `guides/herdr.md` and `guides/claude-code.md`: their held
lines show `--again`. The setup skill's `release_retries` bullet says a
worktree's and a preview's release are held the same way, and `sluice
release … --again` runs one again.
