## ADDED Requirements

### Requirement: A submission's status names the revision a submit would send

`sluice submit <branch> --status --json --worktree <w>` SHALL print, with the worktree's last submission to the branch, `sends`: the revision a submit from `<w>` would send now. That SHALL be the worktree's head, or, where `<w>` has checked out a branch sluice lands on, the remote's revision of that branch, read as a submit reads it (124). A read of the branch that fails SHALL fail the status read as any read of the branch does. The status read SHALL write nothing (rule core.4).

#### Scenario: A worktree on its own branch

- **WHEN** a worktree's last submission to `main` is queued, and `sluice submit main --status --json` runs there
- **THEN** it prints the submission's record with `sends` the worktree's head

#### Scenario: A worktree on a branch sluice lands on, its copy behind

- **WHEN** a worktree has checked out `bolt/x`, which sluice lands on and keeps on `origin`, this host's copy of `bolt/x` is behind origin's, and `sluice submit main --status --json --worktree <w>` runs after a submit from it
- **THEN** it prints `sends` origin's revision of `bolt/x`, the `head` of that submission, not the worktree's head
