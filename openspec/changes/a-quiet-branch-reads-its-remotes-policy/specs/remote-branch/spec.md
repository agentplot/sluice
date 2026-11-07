## MODIFIED Requirements

### Requirement: A policy refused on the branch's remote refuses the branch until a push there clears it
Once a branch is on its remote, sluice SHALL read its policy whole at the
branch's revision on the remote, fetched, for its gate and every move of
it, and SHALL NOT judge the rest of the policy at the host's own copy. A
policy refused there SHALL refuse that branch's round, naming the branch,
the remote's revision and the refusal. A policy pushed to the remote that
sluice reads SHALL clear the refusal on every host the next time a
sluice command reads the branch, whether or not anything is queued, so
that submit and status take work again with no round run for it. A
submit SHALL read the branch fetched and judge the submission there, so
the next submit takes the work. A read of the branch outside a round,
by status, the overview, `sluice` with no command, or what a watch or a
wait shows, SHALL read its policy at the remote's revision as this host
last fetched it, never at the revision the branch's state last saw (see
branch-refusal), and SHALL fetch nothing for it, so that it reads the
branch whole once a fetch on this host, a submit's among them, has
brought the pushed revision (56, 124, 129).

#### Scenario: The bolt's own round runs past a copy that lags
- **WHEN** this host's own `bolt/x` holds a policy with a `[[checkout.cache]]` table, origin's `bolt/x` holds a clean policy, and a submission to `bolt/x` is waiting
- **THEN** a round on `bolt/x` reads origin's revision, tests the submission and lands it on origin, and this host's `bolt/x` is levelled to the landing

#### Scenario: A refusal on the remote names the remote's revision
- **WHEN** origin's `bolt/x` holds a policy with a `[[checkout.cache]]` table and a submission to `bolt/x` is waiting
- **THEN** `sluice step bolt/x` lands nothing, runs nothing, and its output names `bolt/x`, origin's revision of it and `[[checkout.cache]]`

#### Scenario: A push to the remote clears the refusal
- **WHEN** origin's `bolt/x` was refused for a `[[checkout.cache]]` table, and a commit removing that table is pushed to origin's `bolt/x` by hand while this host's own `bolt/x` still holds it
- **THEN** the next round on this host reads origin's new revision, tests the waiting submission and lands it

#### Scenario: A quiet branch this host fetched is read whole by status
- **WHEN** `main` is on origin with its state on this host, nothing is queued, `main`'s state last saw `R1`, whose policy has a `[[checkout.cache]]` table, and this host's `origin/main` is at `R2`, which removes it
- **THEN** `sluice status main` and `sluice overview` show `main` whole, nothing is fetched, nothing is written, and no round has run

#### Scenario: A quiet branch this host never fetched takes a submit
- **WHEN** `bolt/x` is on origin with its state on this host, nothing is queued, `bolt/x`'s state last saw `R1`, whose policy has a `[[checkout.cache]]` table, origin's `bolt/x` is at `R2`, which removes it, and this host's `origin/bolt/x` is still `R1`
- **THEN** `sluice status bolt/x` refuses it naming `R1`; `sluice submit bolt/x --no-wait` from a worktree on a branch of its own takes the submission; and `sluice status bolt/x` after it shows the submission queued, with no round run
