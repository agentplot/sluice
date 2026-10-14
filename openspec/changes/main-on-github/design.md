# Design

## Context

The core reaches a branch only through the branch seam (`Branch` in
`crates/sluice-seams`): `read`, `read_file`, `form`, `commit`,
`contains`, `move_to`. Its one profile, `LocalRefs`
(`crates/sluice/src/profiles/local_refs.rs`), reads and moves refs with
gix, which runs no hooks, so the `reference-transaction` hook from
`sluice protect` never sees sluice's own moves. A landing already
writes an intent (from, to, commits) before it moves the branch, and
the next step settles an intent by reading the branch: at `to` it
landed, at `from` it moves again, holding `to` in its history it
landed, elsewhere it was refused (`step.rs`, the landing). `Seams`
holds one `branch` for every branch an instance serves.

## Goals / Non-Goals

**Goals:**
- A pushed refs profile of the branch seam that changes nothing in the
  core: the landing intent, the refusal and core.17 work as they are.
- Local branches keep the local refs profile untouched.

**Non-Goals:**
- Fetching anything but the branch. Submissions' heads are local on
  one host.
- Credentials: `git push` uses whatever the host's git already uses.

## Decisions

**A pushed refs profile wraps local refs.** `PushedRefs` holds a
`LocalRefs` and the remote's name. `form`, `commit`, `contains` and
`read_file` delegate: forming and committing stay local refs (C.4).
`read` runs `git fetch <remote> <branch>`, reads the fetched revision,
moves the local branch to it with gix (no hook, as today), and returns
it. `move_to` runs `git push <remote> <to>:refs/heads/<branch>
--force-with-lease=<branch>:<expected>`; on success it moves the local
branch to `to` and returns `Moved`; on a lease refusal it fetches and
returns `Elsewhere(<fetched>)`; on any other failure it returns an
error, which leaves the intent unsettled for the next step. The git
command, not gix, does fetch and push: gix's push is not complete, and
the git command carries the host's credential helpers and SSH setup.
Alternative: a second seam method `sync` called by the core before a
round. Rejected: the core would learn that some branches are remote,
and `read` is already the one place a round learns the branch.

**The instance routes per branch.** A `Branches` router in the sluice
crate implements `Branch`: for each call it finds the branch's
`remote` and sends the call to that remote's `PushedRefs`, or to
`LocalRefs` when there is none. Methods that take no branch name
(`form`, `commit`, `contains`, `read_file`) go to `LocalRefs`, since
every object is local. The router reads `remote` from the policy on the
local branch's revision, and the round's `read` then keeps the local
branch equal to the remote's, so the next policy read sees what the
remote holds (core.15).

**Fetch once per step.** A step calls `read` more than once. The
profile fetches on the first `read` of a branch in its lifetime and
reuses the fetched revision after, so a step makes one fetch per branch;
`move_to` and an unsettled intent fetch again. A profile lives for one
step, so nothing is held between steps (core.14).

**A failed fetch is an error of the round, not of a submission.**
`read` returns the error; the step lands nothing and the runner reports
it. Runs already started keep running, since their supervisor is not
the step.

**Policy.** `remote` is a string under `[branches."<pattern>"]`, parsed
into `Rules` and validated against `git remote` when the profile is
built. The key, its parser, the test that parses sluice's own policy,
and any use in `.config/sluice.toml` land together, after the installed
`sluice` is reinstalled.

## Risks / Trade-offs

- [The operator's hand push to `main` meets a protected local `main`]
  → the hook guards `refs/heads/main` only; `git pull` on `main` is
  still refused locally. The guide says to pull with `git fetch` or let
  sluice keep the local `main`.
- [GitHub accepts a push the lease allowed, then sluice dies] → the
  intent settles it on the next step (spec: stopped after the push).
- [A fetch on every step against GitHub's rate limits] → git over SSH or
  HTTPS is not rate limited as the API is; `sluice run` steps every few
  seconds, and a fetch with nothing new costs one round trip.
- [Tests need a remote] → a bare repository on disk is a remote to git;
  the lease, the refusal and the fetch behave as against GitHub.
