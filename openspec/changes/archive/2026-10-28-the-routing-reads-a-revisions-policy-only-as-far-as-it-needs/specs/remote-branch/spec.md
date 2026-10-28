## ADDED Requirements

### Requirement: Deciding which remote a branch is on reads each policy only as far as it needs
To decide which remote a branch is on, sluice SHALL read each revision's
policy only as far as the deciding needs: the `remote` its `[branches]`
table names for the branch, and whether `[signers]` names a signer. Every
other table and field SHALL be passed over, known to sluice or not, so a
revision refused elsewhere in its policy decides the remote as any other
does. A revision whose `remote` or `[signers]` can't be read SHALL refuse
it, naming the ref and revision (124).

#### Scenario: A copy refused elsewhere still says where the branch is
- **WHEN** this host's own `bolt/x` holds a policy with a `[[checkout.cache]]` table, which sluice no longer keeps, and `[branches."bolt/*"] remote = "origin"`, and `origin/bolt/x`, as last fetched, holds the same policy without that table
- **THEN** `bolt/x` is read as origin's branch, and deciding it says nothing of `[[checkout.cache]]`

#### Scenario: The one revision naming the remote is refused elsewhere
- **WHEN** this host's own `bolt/x` holds a policy with a `[[checkout.cache]]` table and `remote = "origin"` for `bolt/*`, the host holds no `origin/bolt/x`, and origin's `bolt/x` holds a clean policy
- **THEN** `bolt/x` is read as origin's branch, not as a branch in this host's own refs, and a round fetches it from origin

#### Scenario: A remote that can't be read refuses, naming the copy
- **WHEN** this host's own `bolt/x` holds a policy whose `[branches."bolt/*"]` says `remote = 1`
- **THEN** `sluice step bolt/x` lands nothing, and its output names `bolt/x`, its revision, and that `remote` must be the name of a git remote

#### Scenario: Two remotes are still refused when one copy is refused elsewhere
- **WHEN** this host's own `main` holds a policy with a `[[checkout.cache]]` table naming `remote = "origin"`, and `upstream/main` holds a clean one naming `remote = "upstream"`
- **THEN** `sluice step main` lands nothing, and its output names `origin` with `main` and `upstream` with `upstream/main`

### Requirement: A policy refused on the branch's remote refuses the branch until a push there clears it
Once a branch is on its remote, sluice SHALL read its policy whole at the
branch's revision on the remote, fetched, for its gate and every move of
it, and SHALL NOT judge the rest of the policy at the host's own copy. A
policy refused there SHALL refuse that branch's round, naming the branch,
the remote's revision and the refusal. A policy pushed to the remote that
sluice reads SHALL clear it at the next round, on every host (56, 124).

#### Scenario: The bolt's own round runs past a copy that lags
- **WHEN** this host's own `bolt/x` holds a policy with a `[[checkout.cache]]` table, origin's `bolt/x` holds a clean policy, and a submission to `bolt/x` is waiting
- **THEN** a round on `bolt/x` reads origin's revision, tests the submission and lands it on origin, and this host's `bolt/x` is levelled to the landing

#### Scenario: A refusal on the remote names the remote's revision
- **WHEN** origin's `bolt/x` holds a policy with a `[[checkout.cache]]` table and a submission to `bolt/x` is waiting
- **THEN** `sluice step bolt/x` lands nothing, runs nothing, and its output names `bolt/x`, origin's revision of it and `[[checkout.cache]]`

#### Scenario: A push to the remote clears the refusal
- **WHEN** origin's `bolt/x` was refused for a `[[checkout.cache]]` table, and a commit removing that table is pushed to origin's `bolt/x` by hand while this host's own `bolt/x` still holds it
- **THEN** the next round on this host reads origin's new revision, tests the waiting submission and lands it
