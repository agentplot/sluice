# A failing release is tried a few times, then held

## Why

On deneb, sluice held three of boomdrive's worktrees whose release had
failed about 1,600 times since 2026-11-10: `sluice status` said "its
release failed (1603 tries): sluice release runs it again, sluice
release --accept-failure frees it without" (signal
2026-11-11-agentplot-design-0cdaf45b/01, 21:36:53Z). Each try failed on
boomdrive's own command, `site-stop` not found (exit 127), and on the
toolchain's setup. Part of that setup failed on sluice's account:
`mktemp` under `/private/tmp/sluice-501/<id>/` found no such directory,
since the scratch the release is given as `TMPDIR` was gone.

Clause 123 bounds the tries: "sluice tries the release again each
round, as many times as the branch's policy allows, a few where it
states none, then stops and reports the place as held … with what
failed and how to release it". C.7's `release_retries` is that bound, 3
where a branch states none. sluice keeps it for a combination's
checkout, which rounds try. A worktree's place is tried by no round
(25). It is tried by whoever runs `sluice release` in it: its session,
`sluice exec --release`, `wt submit`, crew's tidy and Flume's teardown,
which Flume's tidy runs again on every pass while the place is kept
(flume 66). sluice counts each failed try on the place, marks it held
at the first, and runs the release again at every ask
(`crates/sluice/src/profiles/places.rs`, `release_held`). So nothing
bounds the tries, and the count only grows. A preview named by its
caller and a held checkout named by the operator run again at every
ask too. A worktree sluice holds nothing for keeps nothing of a failed
release (record 0112), so its tries are not even counted.

No release path makes the scratch its record names before the command
runs. A run does, and so does a gone place's release (rule core.30: "a
scratch the system emptied made again at the path its record names").
A held worktree's release, a checkout's and a preview's run with
`TMPDIR` naming whatever is left.

## What Changes

- **One bound for every place.** A worktree's release, asked by
  anyone, counts each failed try against the `release_retries` of the
  branch its runs are held to, read at the branch's current revision,
  3 where it states none or cannot be read. It is held once its tries
  pass that bound: the first try and three more by default. A
  checkout's tries are the rounds' as today, and a preview's are the
  rounds' and its caller's as today (123, C.7).
- **A held place runs again only when asked again.** `sluice release`
  of a held worktree, checkout or preview runs nothing and counts no
  try. It says the place is held, with what failed last and its tries.
  It names `sluice release … --again`, which runs the release once more,
  and `sluice release … --accept-failure`, which frees it without. It
  exits 1. So a teardown run on every pass asks, and sluice runs
  nothing. `--again` is the operator's asking of 123. Passing frees the
  place, failing keeps it held with one more try.
- **An unheld worktree's failed release keeps its place.** The record
  the release wrote stays, with the failure and its one try, so the
  bound counts that worktree's tries too. `sluice status` and the
  overview list it, and `--accept-failure` frees it. A passing release
  keeps nothing, as today. This amends record 0112's D4. `sluice exec`
  later in that worktree gives the kept place its ports, as a first run
  does.
- **Every try has its scratch.** Before any release command runs, a
  worktree's, a checkout's or a preview's, sluice makes the scratch the
  place's record names, as a run does. So `TMPDIR` and `SLUICE_TMP` name
  a directory that exists (35; rule core.30).
- **The words.** `sluice status`, the overview, a round's report,
  `sluice previews`, the board's and the herdr plugin's held lines and
  each `--json` answer give the held place's `--again` command. A
  failed answer carries `held` and `tries`.
- The decision is recorded as a new record, the next free number,
  amending record 0112 in part. Rule core.16's and core.30's details
  cite it. The site's commands, configuration, herdr and Claude Code
  pages, and the setup skill, say it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `place-release`: a worktree's failed release is tried its branch's
  retries, then held; a held place's release runs again only at `sluice
  release --again`; an unheld worktree's failed release keeps its place;
  the overview gives `--again`.
- `place-scratch`: every release runs with the scratch its record
  names, made again where it is gone.
- `preview-release`: a held preview's release at its caller's asking
  runs only with `--again`.
- `held-worktrees`: a worktree's place with no ports is given them at
  its next run.

## Impact

- `crates/sluice-seams/src/lib.rs`: `Checkout::release_session` takes
  the retries, as `release_preview` does.
- `crates/sluice-core/src/release.rs`: `release_retries` is public;
  `release_held` takes `again`, and answers a held checkout asked
  without it with a new `Releasing::StillHeld`, running nothing.
- `crates/sluice/src/profiles/places.rs`: `release_held` and
  `release_unheld` count against the retries, `release_unheld` keeps a
  failed release's record, `adopt` gives a portless record its block,
  and `release_held`, `release` and `release_preview` make the scratch
  first.
- `crates/sluice/src/lib.rs`, `exec.rs`, `preview.rs` and `overview.rs`:
  `--again`, the held answer, and the words.
- `crates/sluice-fakes/src/lib.rs`: the fake place seam keeps the same
  contract.
- `plugins/sluice-board/hooks/board.ts` and `plugins/herdr/lib/words.jq`:
  the held line gives `--again`.
- Tests: new tests in `crates/sluice/tests/profiles/places.rs`,
  `crates/sluice/tests/cli.rs` and `crates/sluice/tests/previews.rs`.
  The tests whose expectations change are named in the tasks.
- `docs/adr/` (a new record; record 0112 amended in part),
  `docs/architecture/core.md`,
  `site/src/content/docs/architecture/rules.md`,
  `site/src/content/docs/reference/commands.md`,
  `site/src/content/docs/reference/configuration.md`,
  `site/src/content/docs/guides/herdr.md`,
  `site/src/content/docs/guides/claude-code.md`,
  `plugins/sluice/skills/setting-up-sluice/SKILL.md` and the two
  plugins' READMEs.
- Callers: Flume's teardown, crew's tidy, `wt submit` and `sluice exec
  --release` change nothing. Each already keeps its worktree when
  `sluice release` exits 1. Once the place is held, sluice answers them
  at once and runs nothing.

### Out of scope

- **What boomdrive's release and toolchain need.** `site-stop` not
  found and the conflicting `.mcp.json` and `.claude/settings.json` are
  boomdrive's own failures. This change holds such a place after its
  tries and names what failed. Fixing the failures is boomdrive's.
- **A worktree's tries logged on its branch.** A checkout's tries are
  logged as `release_failed`. A worktree's are kept on its record and
  shown by status and the overview, as today. Logging them waits on a
  report that asks for it.
- **Flume's tidy asking a held place less often.** Once held, sluice
  answers with nothing run, and the bound is sluice's alone (D7). A
  change to Flume's tidy is Flume's.
