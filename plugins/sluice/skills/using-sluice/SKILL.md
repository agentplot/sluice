---
name: using-sluice
description: Land work through sluice, the merge queue for agents in worktrees. Use in a repository with a `.config/sluice.toml` when work is to be submitted, landed, merged or queued, or when a sluice submission failed, conflicted or seems stuck. Covers `sluice submit` and `wt submit`, waiting on or checking a submission, reading a failure's blamed tests and logs, `sluice exec` for tests in the worktree, a dev server or stack kept up in the worktree (in place of `wt step tether`), and `sluice preview`, `previews` and `release`.
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
- A branch whose policy requires a task the task runner keeps out of CI
  (moon's `runInCI: false`) can never pass, so every submission to it is
  refused (exit 2), naming the task and the policy's line. Let the task run
  in CI in your worktree, or drop it from the branch's policy in the same
  submission, and submit again; if neither is your change to make, tell the
  user.

Which branches sluice manages: `sluice overview` lists the branches with a
queue; the policy's `[branches."<pattern>"]` tables are the whole list.

## Submit

```sh
sluice submit --check main      # would it conflict? writes nothing
sluice submit --check main --head bolt/x   # the same for a branch no worktree holds
sluice submit main              # submit, wait, exit with the outcome
wt submit main                  # the same under worktrunk; removes the worktree on landing
```

Exit codes: `0` landed, whatever failed after the landing (what failed is
on stderr, after `sluice: after the landing:`), `1` failed (this
submission was blamed), `2` a conflict or a refusal, `3` sluice itself
failed and the submission has not landed, or it could not read the forge
for five minutes and whether it landed is not known (the cause is on
stderr), `4` withdrawn, `5` still waiting (`sluice wait --timeout` ran out),
`130` stopped with Ctrl-C (the submission stays queued).

A waiting submit or `sluice wait` reads again through a forge's 5xx
answers, a pause apart, for five minutes. Past them it says `could not read
#12: sluice wait 12` and exits `3`; under `--json`, the error document with
that command in `next`. That is no outcome: never take it for a failure.
Run the command it names, which reads the submission's record again.

A waiting submit also runs sluice's rounds, so it is what moves the queue on
a single machine. A verdict takes from a minute to most of an hour, and an
agent whose submission holds its session hears nothing meanwhile: no
message, no word from the user. So an agent submits with `--no-wait` and
waits with `sluice wait`, which runs the rounds the same way, in its
harness's background:

```sh
sluice submit main --no-wait --json        # {"id":12,"branch":"main","ahead":[...]}
sluice wait 12 --timeout 9m --json         # in the background, again for as long as it exits 5
sluice wait main --timeout 9m --json       # this worktree's last submission to main
```

Start each `sluice wait` as a background command (in Claude Code, the Bash
tool's `run_in_background`): it brings you back when it exits, and a
message or the user reaches you while it runs. Start it again the same way
for as long as it exits `5`. Only where your harness runs nothing in the
background, run it in the foreground, its timeout a minute under your tool
call's limit, and end no turn while the submission is undecided. A
waiting `sluice submit main` is for a person at a terminal or a script,
which nothing else needs to reach.

Under worktrunk, wait with `wt submit` run again, the same way:

```sh
wt submit main --no-wait --json            # submit, as above
wt submit main --timeout 9m --json         # in the background, again for as long as it exits 5
```

Run again where the worktree's submission at its head stands, queued,
under test or landed, or its pull request is open and ready there, `wt
submit` submits nothing: it follows that submission as `sluice wait main`
does, with the same output, record and exits, and once it lands it
releases the worktree and removes it, at once when it had already landed.
`sluice wait` removes nothing, so never wait with it and then remove the
landed worktree by hand. `--timeout` only follows: where nothing stands, or
with `--no-wait`, `--check`, `--status`, `--after` or `--priority`, it exits
`64` and submits nothing. A new commit, or a submission that was blamed,
conflicted or was withdrawn, is submitted again by `wt submit main
--no-wait`.

Each `sluice wait --json` prints the submission's record however it ends:
read its `state` and the exit. Exit `5` leaves it `queued`, `waiting`, `testing`, `passed` or
`landing`; run `sluice wait` again. Never search `sluice status <branch>`'s
listing for `landed` or `failed`: a decided submission leaves that listing,
and a loop waiting for it there never ends. `sluice status 12 --json` reads
the record once without waiting, and `sluice submit main --status` this
worktree's last submission or its pull request.

Several worktrees whose heads must go in together, such as a set of
approved bolts, are submitted as one act: `sluice submit main --worktree
../a --worktree ../b`. sluice queues all of them, in that order, or none,
and forms no combination on any of them until every one is queued. Once
queued, each is its own submission: blamed, withdrawn and waited on
(`sluice wait <id>`) alone. See [references/submit.md](references/submit.md).

Nothing moves while nobody waits and no `sluice run` is going, so a
`--no-wait` submission with nobody waiting on it sits at `queued`; `sluice
wait` moves it, as do `sluice step main` (one round) and `sluice run
--rounds N`. Do not submit again to move it: a second submit from the same
worktree replaces the first. On a pull-request branch, `--no-wait --json`
answers `{"id","branch","ahead","pull_request"}` when sluice queued the
pull request at once, its conditions holding: hand `id` to `sluice wait`.
Otherwise it answers `{"pull_request","branch","state"}`, `state` what the
pull request waits for: run `sluice wait main` in the worktree, which
follows the pull request and, once sluice queues it, its submission, whose
record it prints under `--json` from then on.

A submission that sits at `queued` while another is testing is waiting its
turn. `sluice status <branch>` says `testing N at once (up to M)`. N is one
on a branch with `batch = true`, which tests one run at a time. It also drops
after failures, halving for each blamed submission and climbing back one per
pass. The host's `[runs] max` caps every branch, every `sluice exec` and
every preview together, every repository's on the host: another
repository's queue can hold yours back. Submitting again fixes none of
these. If unrelated
submissions keep failing the same check, read its log for a cause in the
tests or the host, such as two runs writing the same directory, and tell the
user.

### A branch whose queue is on the remote

When the policy has `[signers]`, a branch with a `remote` keeps its queue on
the remote, shared by every host that submits to it. There, a `--no-wait`
submission goes on without you: push it and you may stop, close the laptop
or end the session. Another host or a box kept running
lands it, and `sluice status <id>` reads the outcome from the remote
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

## Agents you start in worktrees

A background agent you start to work in a worktree, with the Agent tool's
`isolation: "worktree"` or in one you made, knows only the prompt you give
it. Tell it there to commit its work when it is done and submit it through
sluice from that worktree, with `sluice submit <branch>` naming the branch
it lands on, waiting as above (`--no-wait --json`, then `sluice wait <id>
--timeout 9m --json`, or under worktrunk `wt submit <branch> --timeout 9m
--json`, in its background, again for as long as it exits `5`), and to
report the outcome to you. The rest of the prompt is yours, written for
the work.

Work left committed in a worktree whose agent has ended is submitted by an
agent you start in that worktree, told the same. It is the one that can
fix a failure or a conflict there. Never ask the user to run `sluice
submit`, in that worktree or any other.

A way on that sluice prints, such as `the agent working in unit/a fixes it
there and submits again`, is for the agent it names: tell that agent, or
start one in that worktree, and never hand it to the user.

## When it fails

Read what sluice printed, then the log it names. `failed: <task>` means
your submission broke that test **in combination with what was ahead of
it**; sluice has already checked the entries ahead passed on their own.

1. Open the log path printed after `log of <task>:` (also
   `failed[].log` in `sluice status <id> --json`).
2. Reproduce in your worktree with `sluice exec <branch>`, the same tests
   under the same limits. If `sluice exec` itself goes wrong, find out why
   and tell the user; don't route around it.
3. Fix, commit, submit again.

A **conflict** names the paths and the submission it conflicts with. Merge
or rebase the target branch into your worktree, resolve, commit, submit
again. Don't wait for the submission to find it: `sluice exec` says
`would conflict: …` before it runs, and when told your work would conflict
(by the board, or a forecast), merge the branch in then, while it is small. Details and every state: [references/verdicts.md](references/verdicts.md).

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
   shares the host's bound with every repository's queue and gives your worktree
   ports no other worktree holds. Run suites this way, so the sessions on
   one host take turns rather than all running at once; a quick, narrow
   call, such as a type check or one unit test, can run directly.
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

## Servers in your worktree

A dev server or stack you keep up while you work is started by
`sluice exec <branch> -- <script>`, which brings it up on your ports, hands
it off and exits, and stopped by `sluice release`; never by
`wt step tether` or a worktrunk hook. Your worktree gets ports and
`SLUICE_PLACE` but no `SLUICE_NAME_<NAME>`, which only a preview gets:
default each name, `${SLUICE_NAME_WEB:-web-$SLUICE_PLACE}`. See
[references/servers.md](references/servers.md).

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
