# Spec Delta

## Purpose

What the branch protection `sluice protect` installs refuses and lets
through on a host, judged by each branch's own value there, and what a
refusal tells whoever ran the command and the branch's records, in the
repository git names wherever the command runs (1, 124).

## ADDED Requirements

### Requirement: A write is judged by the branch's own value on the host
The protection SHALL judge a write to a branch it protects by the
value that branch's ref holds on the host before the write and the
value it would hold after. On a branch whose policy names a remote, it
SHALL fetch nothing and SHALL NOT compare the write with the remote's
revision. A write that leaves every protected branch at the value it
held SHALL pass; one that changes a protected branch's value, or
deletes it while anything still needs it, SHALL be refused (1, 124).

#### Scenario: A reset to where the copy stands passes
- **WHEN** `bolt/x` names `remote = "origin"`, the host's `bolt/x` is at `2111d21` while origin's is at `8079db3`, and someone runs `git reset --hard` in a worktree that has checked out `bolt/x`
- **THEN** git's reset succeeds, the host's `bolt/x` is still `2111d21`, and the branch's log has no `protected` line

#### Scenario: git's packing passes on a copy apart from the remote
- **WHEN** the host's `bolt/x` is at `2111d21`, origin's is at `8079db3`, and `git pack-refs --all --prune` runs
- **THEN** it succeeds, `bolt/x` is packed at `2111d21`, its loose file is gone, and the branch's log has no `protected` line

#### Scenario: A move of the copy to the remote's revision is refused
- **WHEN** the host's `bolt/x` is at `10e4791`, origin's is at `8079db3`, and someone runs `git update-ref refs/heads/bolt/x 8079db3`
- **THEN** git fails, the host's `bolt/x` is still `10e4791`, and the branch's log has one `protected` line from `10e4791` to `8079db3` naming the `git update-ref` command

#### Scenario: A branch with no remote is judged the same way
- **WHEN** `bolt/x` has no `remote` and someone runs `git reset --hard` in a worktree on it, then `git reset --hard unit/a` there
- **THEN** the first passes, the second is refused, and `bolt/x` is where it was

### Requirement: The protection judges the repository git names
The protection SHALL judge a transaction in the repository it is of, as
git names it, wherever the command was run from: the repository
`GIT_DIR` names when git sets it, and otherwise the one holding the
folder the command ran in. It SHALL refuse nothing of a ref it does not
guard. A transaction it cannot read for a reason of its own SHALL pass,
with one warning saying why, rather than aborting git's write (124).

#### Scenario: A fetch into a bare repository run from outside it
- **WHEN** sluice's protection is installed in the bare repository `x.git`, whose `bolt/x` sluice lands on, and from a folder that is no repository someone runs `git --git-dir=<path to x.git> fetch origin` that updates a remote-tracking ref and a branch sluice does not guard
- **THEN** the fetch succeeds, every ref it fetched is updated in `x.git`, sluice prints no warning, and `bolt/x`'s log has no `protected` line

#### Scenario: A write to a protected branch through `--git-dir` from outside is refused
- **WHEN** from that same outside folder someone runs `git --git-dir=<path to x.git> update-ref refs/heads/bolt/x <another revision>`
- **THEN** git fails, `x.git`'s `bolt/x` is where it was, and `bolt/x`'s log in `x.git` has one `protected` line naming the command

#### Scenario: Run from inside another repository, the named one is judged
- **WHEN** a second repository `y`, whose policy protects `main`, holds the folder the command runs in, and from there someone runs `git --git-dir=<path to x.git> update-ref refs/heads/main <another revision>`, where `x.git`'s policy guards no `main`
- **THEN** the update succeeds in `x.git`, `y`'s `main` is untouched, and neither repository's log has a `protected` line

#### Scenario: A transaction the protection cannot read passes, said once
- **WHEN** the policy at `bolt/x`'s revision and at `bolt/y`'s cannot be parsed, and someone runs `git update-ref --stdin` with one transaction moving both
- **THEN** the update succeeds, git's error output holds exactly one warning from sluice saying its branch protection let the transaction through and why, and no log has a `protected` line

### Requirement: A refusal is recorded with the command and both revisions
A refused write SHALL be recorded in the branch's log as `protected`,
with the branch's ref, the value it held on the host, the value the
write would have set or none for a deletion, and the command that tried
it (1, 124).

#### Scenario: The log line of a refused move
- **WHEN** the host's `bolt/x` is at `10e4791` and `git branch -f bolt/x unit/a` is refused
- **THEN** the branch's log has a `protected` line with `ref` `refs/heads/bolt/x`, `from` `10e4791`, `to` `unit/a`'s revision, and `by` naming `git branch -f bolt/x unit/a`

### Requirement: A refusal tells whoever ran it in sluice's words
Whoever ran a refused command SHALL be told, before git's own error,
that the branch is protected by sluice, the command that tried, the
revision the branch holds on the host and the one the write would have
set, and how to bring the worktree level again. On a branch whose
policy names a remote, it SHALL also say that sluice levels the host's
copy of the branch itself, at each landing (1, 124).

#### Scenario: The refusal names the command and both revisions
- **WHEN** the host's `bolt/x` is at `10e4791`, origin's at `8079db3`, and a tool runs `git update-ref refs/heads/bolt/x 8079db3`
- **THEN** git's error output starts with sluice's lines saying `bolt/x is protected by sluice`, naming `git update-ref refs/heads/bolt/x 8079db3`, `10e4791` and `8079db3`, and saying sluice levels this host's `bolt/x` at each landing; git's own `fatal` line comes after them

#### Scenario: A deletion's refusal says what still needs the branch
- **WHEN** `bolt/x` holds submission #1 in its queue and someone runs `git branch -D bolt/x`
- **THEN** git fails, sluice's lines name the command, the revision `bolt/x` holds, and #1, and the branch's log has a `protected` line with no `to`
