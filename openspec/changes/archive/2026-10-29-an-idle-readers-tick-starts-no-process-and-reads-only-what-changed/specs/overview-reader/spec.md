## ADDED Requirements

### Requirement: Whether the repository has sluice is asked again only once the host's revisions or the records' list changed

A watch that is the reader SHALL ask whether any revision the host holds
has sluice's policy when it opens, and again only once a revision the
host holds, a local branch's or a remote's tracking branch's, or the
records' list of branches, has changed since it last asked; a tick that
finds both as they were SHALL ask nothing of the policy. The question
SHALL start no process (129).

#### Scenario: Set up with nothing submitted, watched for many ticks

- **WHEN** a repository has no records, its `main` holds
  `.config/sluice.toml`, and a watch, the reader, runs in it with git on
  the path counting every start
- **THEN** it prints the overview with `branches` `[]` and no `set_up`
  field, starts git no more after that line, and is still running

#### Scenario: The policy removed while a watch runs

- **WHEN** a watch runs in a repository with no records whose `main`
  alone holds `.config/sluice.toml`, and `main` then moves to a commit
  that removes it
- **THEN** the watch asks again at its next tick, prints the overview with
  `"set_up": false` as its last line and exits 0
