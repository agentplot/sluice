## MODIFIED Requirements

### Requirement: A policy refused on the branch's remote refuses the branch until a push there clears it
Once a branch is on its remote, sluice SHALL read its policy whole at the
branch's revision on the remote, fetched, for its gate and every move of
it, and SHALL NOT judge the rest of the policy at the host's own copy. A
policy refused there SHALL refuse that branch's round, naming the branch,
the remote's revision and the refusal. A read of the branch outside a
round SHALL read its policy at the remote's revision as this host last
fetched it, never at the revision the branch's state last saw (see
branch-refusal). A policy pushed to the remote that sluice reads SHALL
clear the refusal on every host, at the first round, read or tick after
that host holds the pushed revision, whether or not anything is queued,
with no round run for it. That host holds the pushed revision after
the tick's ask where the branch's state is on that remote, after
intake's fetch at its next discovery where the branch takes pull
requests, and after a submit's own read of the branch, which fetches it
(56, 124, 129, 136).

#### Scenario: The bolt's own round runs past a copy that lags
- **WHEN** this host's own `bolt/x` holds a policy with a `[[checkout.cache]]` table, origin's `bolt/x` holds a clean policy, and a submission to `bolt/x` is waiting
- **THEN** a round on `bolt/x` reads origin's revision, tests the submission and lands it on origin, and this host's `bolt/x` is levelled to the landing

#### Scenario: A refusal on the remote names the remote's revision
- **WHEN** origin's `bolt/x` holds a policy with a `[[checkout.cache]]` table and a submission to `bolt/x` is waiting
- **THEN** `sluice step bolt/x` lands nothing, runs nothing, and its output names `bolt/x`, origin's revision of it and `[[checkout.cache]]`

#### Scenario: A push to the remote clears the refusal
- **WHEN** origin's `bolt/x` was refused for a `[[checkout.cache]]` table, and a commit removing that table is pushed to origin's `bolt/x` by hand while this host's own `bolt/x` still holds it
- **THEN** the next round on this host reads origin's new revision, tests the waiting submission and lands it

#### Scenario: A quiet branch whose state is on its remote clears at the next tick
- **WHEN** `main`'s state is on origin, nothing is queued, `main`'s state last saw a revision whose policy has a `[[checkout.cache]]` table, the overview watch is reading, and a commit removing that table is pushed to origin's `main` by hand
- **THEN** the watch's line after its next tick shows `main` with no `unread`, `sluice status main` shows its queue, and no round has run: `main`'s state still names the revision it last saw as `at`

#### Scenario: A quiet branch that takes pull requests takes them in after intake's fetch
- **WHEN** `main` is on origin with its state on this host and takes pull requests, nothing is queued, `main`'s state last saw `R1`, whose policy has a `[[checkout.cache]]` table, origin's `main` is at `R2`, which removes it, a pull request is open against `main`, and `sluice run` passes once `main`'s discovery is due
- **THEN** that pass takes the request in as a submission with no round run before it, and `sluice status main` shows the submission queued

#### Scenario: A quiet branch this host fetched is read whole by status
- **WHEN** `main` is on origin with its state on this host, nothing is queued, `main`'s state last saw `R1`, whose policy has a `[[checkout.cache]]` table, and this host's `origin/main` is at `R2`, which removes it
- **THEN** `sluice status main` and `sluice overview` show `main` whole, nothing is fetched, and nothing is written

#### Scenario: A quiet branch this host never fetched takes a submit
- **WHEN** `bolt/x` is on origin with its state on this host, nothing is queued, `bolt/x`'s state last saw `R1`, whose policy has a `[[checkout.cache]]` table, origin's `bolt/x` is at `R2`, which removes it, and this host's `origin/bolt/x` is still `R1`
- **THEN** `sluice status bolt/x` refuses it naming `R1`; `sluice submit bolt/x --no-wait` from a worktree on a branch of its own takes the submission; and `sluice status bolt/x` after it shows the submission queued, with no round run
