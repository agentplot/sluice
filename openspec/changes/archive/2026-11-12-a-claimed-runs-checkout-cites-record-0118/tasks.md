# Tasks

Work in this change's place on its branch, cut from main. Commit by
pathspec, Conventional Commits, scoped to the part of the tree, citing
clauses by number and rules by id.

## 1. The specs cite record 0118

- [x] 1.1 Archive this change, so `openspec/specs/run-claims/spec.md` and `openspec/specs/run-outcome/spec.md` cite record 0118 where they cited the checkout's record 0117. True when `sluice exec main -- moon run root:specs` passes and `git grep -n "record 0117" -- openspec/specs` finds nothing.
