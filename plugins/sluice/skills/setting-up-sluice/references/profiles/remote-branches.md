# A branch on a remote (GitHub)

**Choose it when** the gated branch's real copy lives on GitHub (or any git
remote) and people pull it from there. Everything still runs on this host.

## Keys

```toml
[branches.main]
remote = "origin"
required = ["#check-fast", "#check-tools"]
```

- Every round fetches the branch from the remote first and forms every
  combination on what the remote holds. The local branch stays put until a
  landing.
- A landing is `git push --force-with-lease`: the remote takes it only if
  the branch has not moved since the combination was formed. It is always a
  fast-forward, so a ruleset that blocks force pushes lets it through. Then sluice
  moves the local branch to match. "landed" is reported only once the remote
  accepted.
- A refused push (someone else pushed, a dropped connection) is logged as
  `refused` and the next round tests again on the new tip. Nothing in the
  queue is lost.
- Only the branch is pushed; the queue, results and logs stay in
  `$(git rev-parse --git-common-dir)/sluice/` on the host.

## Turning it on

sluice reads the policy from the branch before a landing, so the change that
adds `remote` lands on this host only. Push the branch to the remote by hand
that once; sluice pushes every landing after it.

A branch made later under a pattern with `remote`, such as a bolt, is never
pushed by hand: `sluice branch create bolt/<b> --from main` makes it on the
remote as sluice's app, and `sluice branch delete bolt/<b>` deletes it once it
has landed into `main`. A submission to a branch the remote lacks is refused
with the `sluice branch create` to run.

## GitHub settings

A branch ruleset on the branch (**Settings › Rules › Rulesets**): restrict
updates on, block force pushes on, bypass list sluice's GitHub App alone
([github-app](github-app.md)). The ruleset on a bolt pattern also restricts
creations and deletions, since sluice makes and deletes bolts;
`sluice forge check bolt/<b>` checks it before any bolt exists. Leave GitHub's merge queue off and require no
status checks on the branch: sluice's gate runs before the push.

Until the app is set up, sluice pushes with the host's own git credentials,
the bypass must hold the user's account, and `sluice status <branch>` says
sluice has no identity of its own. Agents' local updates are then refused
only by the git hook `sluice protect` installs.

## Traps

- A round that cannot fetch lands nothing and says why. Run again once the
  remote answers; running tests keep running.
- Never press GitHub's merge button on the branch: sluice treats it as the
  branch moving under its queue and retests everything queued.

Reference: <https://agentplot.github.io/sluice/guides/github/>.
