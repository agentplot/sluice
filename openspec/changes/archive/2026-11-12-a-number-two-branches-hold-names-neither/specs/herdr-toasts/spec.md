## ADDED Requirements

### Requirement: A culprit's worktree is asked of sluice with its branch

Where the plugin asks sluice for a blamed or conflicting submission's
worktree or failed tests, for `jump` or for a blame's toast, it SHALL
run `sluice status <id> --branch <branch> --json`, `<branch>` the branch
the overview's event names, so a number another branch holds is never
refused or read as that branch's (26; record 0114).

#### Scenario: Jump to a culprit whose number another branch holds

- **WHEN** `#9` on `main` was blamed most recently, `bolt/a` holds a
  `#9` too, its worktree is not yet learned, and the user runs `jump`
- **THEN** the plugin runs `sluice status 9 --branch main --json`, and
  focuses the pane of `main`'s `#9`'s worktree

#### Scenario: A blame's toast learns its tests

- **WHEN** `#9` is blamed on `main` and its tests are not in the
  overview's event
- **THEN** the plugin runs `sluice status 9 --branch main --json`, and
  the toast names the failed tests it gives
