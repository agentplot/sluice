---
name: using-sluice
description: Land work through sluice, the merge queue for agents in worktrees. Use in a repository with a `.config/sluice.toml` when work is to be submitted, landed, merged or queued, or when a sluice submission failed, conflicted or seems stuck. Covers `sluice submit` and `wt submit`, waiting on or checking a submission, reading a failure's blamed tests and logs, `sluice exec` for tests in the worktree, and `sluice preview`, `previews` and `release`.
---

# Using sluice

sluice is the only way onto a branch that `.config/sluice.toml` names under
`[branches."<pattern>"]`. You never merge into such a branch and never push
to it. You **submit** your worktree's head; sluice tests it combined with
everything queued ahead of it and lands it only if that combination passes.

## The rules

- **Never** `git merge`, `git push`, `git update-ref` or `gh pr merge` onto a
  branch sluice manages. Local updates are refused by sluice's git hook;
  pushes are refused by the forge's ruleset. Submit instead.
- Commit everything first. A worktree holding modified, staged or untracked
  files is refused (exit 2), naming each path.
- Submit from your own worktree, on your own branch. A detached HEAD is
  refused.
- Submitting again from the same worktree replaces your earlier submission.

Which branches sluice manages: `sluice overview` lists the branches with a
queue; the policy's `[branches."<pattern>"]` tables are the whole list.

## Submit

```sh
sluice submit --check main      # would it conflict? writes nothing
sluice submit main              # submit, wait, exit with the outcome
wt submit main                  # the same under worktrunk; removes the worktree on landing
```

Exit codes: `0` landed, `1` failed (this submission was blamed), `2` a
conflict or a refusal, `130` stopped with Ctrl-C (the submission stays
queued).

A waiting submit also runs sluice's rounds, so it is what moves the queue on
a single machine. Prefer it. When you must not block, submit with
`--no-wait` and check later:

```sh
sluice submit main --no-wait --json     # {"id":12,"branch":"main","ahead":[...]}
sluice status 12 --json                 # the submission's record
sluice submit main --status             # this worktree's last submission, or its pull request
```

Nothing moves while nobody waits and no `sluice run` is going. If a
`--no-wait` submission sits at `queued` with nothing testing, move it with
`sluice step main` (one round) in a loop, or `sluice run --rounds N`. Do not
submit again to move it: a second submit from the same worktree replaces the
first. On a pull-request branch, a waiting `sluice submit main` follows the
pull request already open, so there it is the way to wait.

A submission that sits at `queued` while another is testing is waiting its
turn. `sluice status <branch>` says `testing N at once (up to M)`. N is one
on a branch with `batch = true`, which tests one run at a time. It also drops
after failures, halving for each blamed submission and climbing back one per
pass. The host's `[runs] max` caps every branch and every `sluice exec`
together. Submitting again fixes none of these. If unrelated
submissions keep failing the same check, read its log for a cause in the
tests or the host, such as two runs writing the same directory, and tell the
user.

### A branch whose queue is on the remote

When the policy has `[signers]`, a branch with a `remote` keeps its queue on
the remote, shared by every host that submits to it. There, a `--no-wait`
submission goes on without you: push it and you may stop, close the laptop
or end the session. Another host, a box kept running, or a GitHub Actions
job lands it, and `sluice status <id>` reads the outcome from the remote
whenever you look.

- `sluice step <branch>` printing `led` means another host keeps the branch:
  nothing is wrong, and stepping again does not help.
- A failure names its system when the branch requires more than one:
  `failed: cli:build (macos-arm64)`. Reproduce it on a host of that system,
  or read its log; `sluice exec` here runs this host's system only.
- A result counts only when a host the branch trusts signed it. If your
  host has no `[signing]` key, or the branch does not trust your host's
  signer (as `main` often does not trust agents' hosts), your runs do not
  decide it; the branch's own executors run the checks.
- `sluice status <branch>` saying the remote could not be reached shows the
  queue as last fetched; it moves on when the remote is back.

On a branch with `[branches."<pattern>".pull_requests]`, `submit` pushes your
branch and opens or updates its pull request instead; the pull request joins
the queue once its reviews and checks hold. See
[references/submit.md](references/submit.md).

## When it fails

Read what sluice printed, then the log it names. `failed: <task>` means
your submission broke that test **in combination with what was ahead of
it**; sluice has already checked the entries ahead passed on their own.

1. Open the log path printed after `log of <task>:` (also
   `failed[].log` in `sluice status <id> --json`).
2. Reproduce in your worktree with `sluice exec <branch>`, the same tests
   under the same limits.
3. Fix, commit, submit again.

A **conflict** names the paths and the submission it conflicts with. Merge
or rebase the target branch into your worktree, resolve, commit, submit
again. Details and every state: [references/verdicts.md](references/verdicts.md).

## Seeing the queue

```sh
sluice overview                 # every branch: depth, what is testing, latest landings and blames
sluice overview --json          # sluice/overview@1, for scripts
sluice status main              # one branch's queue; --all for every submission it has had
```

A submission's `from.path` is the worktree it came from: filter
`branches[].submissions[]` on your worktree's path to find yours.

## Tests while you work: `sluice exec`

Pick the form by what you want from the run:

1. **While iterating**, run the targets you touched through sluice:

   ```sh
   sluice exec main -- moon run catalog:test     # or: -- moon run :test --affected, -- npm test
   ```

   It covers uncommitted edits, uses moon's cache, records nothing, and is
   the fast loop. Going through `sluice exec` rather than straight to moon
   shares the host's bound with the queue's runs and gives your worktree
   ports no other worktree holds.
2. **Before submitting**, to get the gate's own verdict early: commit, then

   ```sh
   sluice exec main
   ```

   It runs the branch's required tasks that your commits since the branch's
   tip affect, with the tasks that depend on them, forced past moon's cache
   so the run is observed. That is the whole gate: every task your change
   does not touch has the same key (the task plus a hash of its inputs) it
   passed under when the tip landed, so you do not need to run everything.
3. **Whether that run saves the submission any work** depends on the
   branch's `trust` in `.config/sluice.toml`. It is recorded, as an `open`
   run, only when the worktree holds exactly its head. A submission reuses
   it only when the branch trusts `"open"` and has not moved since. Without
   `"open"` (sluice's own `main`, for one) it is feedback only, and the
   submission runs the affected tasks again.

Name targets as `project:task` or a single tag, `#task-tag`.

`sluice release` ends what your runs left up and frees the ports; release
before removing a worktree by hand (`sluice exec <branch> --release` runs
and releases in one go). See [references/exec.md](references/exec.md).

## Previews

`sluice preview <branch> <revision>` brings up a revision the branch has
**landed** on the repository's own stack, at addresses a person opens, until
`sluice release <preview>`. Queued heads and combinations under test are
refused. See [references/previews.md](references/previews.md).

## Branches

A branch sluice lands on, such as a bolt, is made and deleted by sluice,
never pushed or deleted by hand:

```sh
sluice branch create bolt/x --from main   # at main's revision, on its remote if the policy names one
sluice branch delete bolt/x               # once it has landed into main and nothing is queued on it
```

Each refusal says why; `--into <branch>` names another branch the bolt
landed into, and `--json` gives the revision made or deleted.

## Amend and withdraw

```sh
me="${SLUICE_AGENT:-$(git branch --show-current)}"
sluice amend 14 --after 12 --as "$me"     # land after #12
sluice amend 14 --priority 1 --as "$me"   # higher goes first; the default is 0
sluice withdraw 14 --as "$me"
```

`--as` must match the agent recorded on the submission (`from.agent`):
`SLUICE_AGENT` when it was set at submit, else the branch name. A
submission's order is fixed once it is under test.
