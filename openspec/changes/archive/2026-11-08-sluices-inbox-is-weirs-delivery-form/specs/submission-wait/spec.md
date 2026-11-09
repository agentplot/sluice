## MODIFIED Requirements

### Requirement: A wait marks delivered only its own outcome

A waiting `sluice submit` and `sluice wait` that follow a later
submission or a pull request SHALL hand over only the outcome they
report as their own. The outcome of the submission they followed SHALL
stay owed, with no hand-over by them, for its own carriers, the round's
notice or `sluice inbox` (51; weir 103).

#### Scenario: The resubmission's landing stays pending

- **WHEN** a session submits `<a>`, then a commit on top of it as `<b>`,
  and `sluice wait <a>` runs in its worktree until `<b>` lands
- **THEN** `sluice status <b> --json` gives `<b>`'s `delivery` with no
  hand-over by `wait`, and `sluice inbox` in that worktree then hands
  over `<b>`'s landing
