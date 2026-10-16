# Spec Delta

## Purpose

The bare repository with worktrees beside it, and safe conversions to it.

## ADDED Requirements

### Requirement: Conversions move nothing in use and delete nothing

A conversion to the layout SHALL print its plan and wait for the user's
word, SHALL skip any worktree a process works in or that holds changes, and
SHALL NOT run `rm`, `git worktree remove` or `git worktree prune`.

#### Scenario: A normal clone with changes and a worktree elsewhere

- **WHEN** a clone with a modified file, an untracked file and a worktree
  outside it is converted by the plain-clone procedure
- **THEN** `main/` holds the same changes on the same branch, the other
  worktree still works, and `git worktree list` shows nothing prunable
