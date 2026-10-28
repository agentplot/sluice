## ADDED Requirements

### Requirement: The signers a branch's records count are read from its revision on the remote
Where a branch's records are on its remote, the signers whose runs count,
and the keys their runs are verified by, SHALL be read from the policy,
whole, at the branch's revision on that remote, fetched, and when the
remote can't be reached, as last fetched. They SHALL NOT be read from the
host's own copy of the branch (27, 124).

#### Scenario: A host whose own copy is refused counts the remote's signers
- **WHEN** two hosts share origin, `main`'s policy on origin names `[signers.box]` with box's key and `remote = "origin"`, and the second host's own `main` holds that policy with a `[[checkout.cache]]` table added
- **THEN** on the second host `sluice status main` reads `main`'s state from origin, and a round there counts the runs box signed and lands the submission they pass
