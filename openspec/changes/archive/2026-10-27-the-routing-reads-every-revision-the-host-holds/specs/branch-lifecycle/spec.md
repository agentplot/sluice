# Spec Delta

## MODIFIED Requirements

### Requirement: sluice makes a branch on the remote its policy names
`sluice branch create` SHALL make the branch on the remote the policy
at `<other>`'s revision names for it, by a push leased on the branch
being absent, or in local refs when it names none. Which remote
`<other>` is on, and so its revision, SHALL be read as for any branch,
from every revision of it the host holds (124).

#### Scenario: With no remote, the same in local refs
- **WHEN** the policy names no `remote` for `bolt/*`
- **THEN** `bolt/x` is made in local refs and nothing reaches any remote

#### Scenario: Made from a main this host holds from before the policy file
- **WHEN** this host's `main` is a revision from before `.config/sluice.toml`, `origin/main` holds a policy naming `remote = "origin"` for `main` and `bolt/*`, and `sluice branch create bolt/x --from main` runs
- **THEN** `bolt/x` is made on origin at origin's `main`, the command prints that revision, and this host's `main` is where it was
