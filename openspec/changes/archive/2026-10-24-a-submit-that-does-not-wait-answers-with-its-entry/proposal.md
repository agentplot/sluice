# Proposal

## Why

On 2026-10-23 crew landed a bolt on swancloud's main, a branch sluice
lands through pull requests, with `wt submit main --no-wait --json`.
sluice answered `{"pull_request": 168, "branch": "main", "state": ...}`
and no submission number. It answers so whatever the submit's own intake
did with the request, even when it took it into the queue. crew
found nothing to hand `sluice wait`, read the answer as sluice queueing
nothing, and withdrew its own landing. C.5's pull request driver now says
that a submit that does not wait answers with the entry intake took for
the request, taking it then where its conditions hold, and otherwise with
the request and what it waits for, and that `sluice wait` follows a
request as it follows an entry.

## What Changes

- `sluice submit <branch> --no-wait` and `wt submit <branch> --no-wait`,
  to a branch with pull requests, push the branch and open or update the
  request as now, then run one intake of the branch, as now. When that
  intake took the request in on the head just pushed, the submit answers
  with that submission: under `--json` the document a no-wait submit
  prints on any branch, `{"id","branch","ahead"}`, with `pull_request`,
  the request's number, beside them; without `--json`, the line
  `submission <id> to <branch> from #<n> · <what is queued ahead>` after
  the `pushed … · opened #<n>` line.
- When the request was not taken in, waiting for a review, a label, a
  check, or anything else its branch's policy names, or because intake
  could not read it, the submit answers as now with the request and what
  it waits for: `{"pull_request","branch","state"}`, `state` the words
  sluice posts on it, such as `waiting: needs 1 review`.
- Either way the submit exits `0`, as a no-wait submit does today.
- `sluice wait <branch>`, from the worktree or with `--worktree`, follows
  the worktree's request, and once the request has a submission on its
  current head, follows that submission exactly as `sluice wait <id>`
  does: under `--json`, out of time or stopped, it prints the
  submission's record rather than the request's document; without
  `--json` it says `still waiting: sluice wait <id>`. Before the request
  is taken in it answers with the request and what it waits for, and
  exits `5` when `--timeout` runs out, as now. A waiting `sluice submit`
  stopped with Ctrl-C does the same.
- The commands reference, the pull-requests guide and the using-sluice
  skill say what a no-wait submit answers on a pull-request branch and
  what to hand `sluice wait` in each case.

No clause changes: this makes C.5's pull request row true of the
commands, with 120 (a submit stands for the head it pushed) and 51 (the
state read is the delivery). Intake still submits the request (rule
core.4: the submit writes no ref, and the submission is intake's, from
the place `pull/<n>`, as today).

Out of scope:

- Naming a request to `sluice wait` by its number, from anywhere but its
  worktree: the design names no form for it, and `sluice wait <branch>
  --worktree <path>` already reaches it.
- The group submit under pull requests, every entry or every request
  with what it waits for (125): its own change, which waits on this one
  for the single request's answer.
- How crew reads the answer (`plan.py` SUBMIT), which is crew's: it
  already takes an `id` first and a `pull_request` second.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-submit`: a no-wait submit answers with the submission
  intake took for the request, or with the request and what it waits
  for.
- `submission-wait`: `sluice wait <branch>` on a pull-request branch
  follows the request's submission as `sluice wait <id>` follows one,
  once the request has one.

## Impact

- **The pull-request seam:** `Followed::Open` carries the submission
  intake took for the request on its current head, when there is one,
  read from the records as `follow` reads the request's latest entry
  today.
- **The CLI:** `submit_pull`'s no-wait answer and its `still` ending, in
  `crates/sluice/src/lib.rs`.
- **Tests:** `crates/sluice/tests/profiles/pull_requests.rs`, real
  binaries against the GitHub fake.
- **Docs:** `site/src/content/docs/reference/commands.md`,
  `site/src/content/docs/guides/pull-requests.md`, the using-sluice skill
  and its submit reference.

## Touches

```
crates/sluice/src/lib.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/tests/profiles/pull_requests.rs
crates/sluice/tests/cli.rs
site/src/content/docs/reference/commands.md
site/src/content/docs/guides/pull-requests.md
plugins/sluice/skills/using-sluice/SKILL.md
plugins/sluice/skills/using-sluice/references/submit.md
```
