# Spec Delta

## ADDED Requirements

### Requirement: A round that read the state last fetched advances nothing
When a round's read of a branch's state on its remote could not be
made and was answered from the state last fetched, the round SHALL
form, start and land nothing, and SHALL fail saying the read failed.
Every other read SHALL still show the state last fetched and say it
could not refresh it (80, 133).

#### Scenario: A round with the state's fetch refused
- **WHEN** `main`'s state is on `origin` and a round cannot fetch `refs/sluice/state/main`, a fetch made earlier being kept on the host
- **THEN** the round forms, starts and lands nothing and fails saying the read failed, while `sluice status main` shows the state last fetched and says it could not refresh it
