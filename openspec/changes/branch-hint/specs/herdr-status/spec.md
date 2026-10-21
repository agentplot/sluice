## MODIFIED Requirements

### Requirement: The branches keep the overview's order

The popup and the tab bar SHALL list a repository's branches in the order
of its overview's `branches[]`, and SHALL name no branch in their code;
scrolling and the line counting branches with nothing queued SHALL hold
in that order. When the focused pane has a branch named by its
`sluice_branch` token, the popup and the dashboard SHALL show that branch
first, even when it has nothing queued and no recent event, and leave it
out of the count; a branch no document lists SHALL be one line,
`<branch> · no records`.

#### Scenario: The policy's order

- **WHEN** the overview lists `release`, an idle `bolt/a`, then `main`
- **THEN** the popup shows `release` above `main` and one other branch
  with nothing queued

#### Scenario: The named branch first

- **WHEN** the overview lists `release`, an idle `bolt/a`, then `main`,
  and the focused pane has `sluice_branch=bolt/a`
- **THEN** the popup shows `bolt/a`, then `release`, then `main`, and no
  line counting branches with nothing queued

#### Scenario: A named branch with no records

- **WHEN** the focused pane has `sluice_branch=hotfix` and no document of
  its repository lists `hotfix`
- **THEN** the popup's first branch line is `hotfix · no records`
