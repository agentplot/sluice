## MODIFIED Requirements

### Requirement: A tick fetches a branch only to act on it

A tick SHALL read a branch on a remote at its revision as last fetched,
and SHALL fetch it only to run a round of it, or, on a branch the forge
keeps, only to claim its runs. A host's tick SHALL fetch no branch for
intake; a round job fetches its branch for intake once its discovery is
due (121, 129, 136, 137).

#### Scenario: A branch on a remote moved there and nothing in play

- **WHEN** `main` lands on `origin`, nothing is queued, and another host
  lands on `origin`'s `main`
- **THEN** a watch shows `main` at its revision as last fetched, and the
  next submission's round fetches `main` before it forms anything (121)

#### Scenario: A branch the forge keeps, nothing to claim

- **WHEN** `main` is the forge's, a pull request into it waits for a
  review, and no combination of `main` is open to claims
- **THEN** a host's ticks for a minute fetch `main` no more than its
  records' one ask a tick brings it, and ask GitHub nothing about
  `main`'s open pull requests
