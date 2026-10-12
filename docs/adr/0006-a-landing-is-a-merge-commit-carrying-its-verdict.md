---
number: 6
title: A landing is a merge commit carrying its verdict
status: accepted
date: 2026-10-08
decision-makers:
- Chuck Swanberg
---

# A landing is a merge commit carrying its verdict

Deciders: the user, in the requirements interview of 2026-10-08.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 23, 25, 44, C.4 and C.6.

## Context and problem statement

Clause 23 allows one commit per entry with the branch as its parent, which admits a squash, a merge, or a replay of the session's commits. The records of why an entry landed are local to the host and may be pruned. What does a landed entry look like in the branch's history, and what of its verdict survives?

## Considered options

1. A squash: one parent, the candidate's tree.
2. A merge commit: the branch as first parent, the session's head as second, the candidate's tree.
3. Either, by the branch's policy.

And for the verdict: trailers on the commit, a git note, or the local records alone.

## Decision outcome

Option 2, with the verdict as `Sluice-*` trailers on the commit: the entry, its submitter, the candidate revision, and each required check with its key and executor. The session's commits stay reachable, so the worktree tool and `git branch --merged` see the place's branch as landed, and the first-parent history of a branch is one commit per landing. The trailers travel with the history wherever it is pushed, so the local records can be pruned without losing why anything landed.

A squash drops the session's commits and leaves its branch looking unmerged. A policy choice doubles every landing path for no case anyone has. A git note is not pushed by default and is lost silently.

### Consequences

- Clause 23 and the local refs profile of C.4 say so.
- Since a landing names its verdict, the verdict is final before the commit is written (core.7).
