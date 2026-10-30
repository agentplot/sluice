## MODIFIED Requirements

### Requirement: The signers a branch's records count are read from its revision on the remote
Where a branch's records are on its remote, the signers whose runs count,
and the keys their runs are verified by, SHALL be read from the policy,
whole, at the branch's revision on that remote, fetched, and when the
remote can't be reached, as last fetched. They SHALL NOT be read from the
host's own copy of the branch (27, 124).

They SHALL be that branch's alone: the runs read for one branch's gate
SHALL be verified with the signers and keys its own policy names, never
with those another branch on the same remote names, whichever branch a
process read first (78, 27). When the branch's revision on its remote
moves, they SHALL be read again at the new revision before any run is
verified for it, and while that revision stands they SHALL NOT be read
again (56). While its remote holds no revision of the branch and the host
no tracking ref of it, no signer's runs SHALL count for it.

#### Scenario: A host whose own copy is refused counts the remote's signers
- **WHEN** two hosts share origin, `main`'s policy on origin names `[signers.box]` with box's key and `remote = "origin"`, and the second host's own `main` holds that policy with a `[[checkout.cache]]` table added
- **THEN** on the second host `sluice status main` reads `main`'s state from origin, and a round there counts the runs box signed and lands the submission they pass

#### Scenario: Another branch's key for the same signer
- **WHEN** `main` and `release` keep their records on origin, `main`'s policy on origin lists key A for `[signers.box]` and `release`'s lists key B, and one process reads `main` first and then `release`
- **THEN** a pass signed with key A counts for `main` and is missing for `release`, and a pass signed with key B counts for `release` and is missing for `main`; read in the other order, the same

#### Scenario: A signer only the second branch names
- **WHEN** `main`'s policy on origin names `[signers.box]` alone, `release`'s names `[signers.box]` and `[signers.ci]` and trusts `ci`, and one process reads `main` first
- **THEN** a pass `ci` signed for `release`'s candidate counts for `release`

#### Scenario: A key added while a driver runs
- **WHEN** `sluice run` is driving `main`, whose policy on origin lists key A for `box`, and a landing on origin adds key B to `box`
- **THEN** once that driver has fetched `main`'s new revision, a pass signed with key B counts for `main`, with no restart, and the driver reads `main`'s policy for its signers no more often than `main`'s revision moves

#### Scenario: A branch its remote does not hold yet
- **WHEN** `release`'s policy routes it to origin, origin holds no `release` and the host no tracking ref of it, and the host's own `release` names `[signers.box]`
- **THEN** no run counts for `release` until origin holds it, and the host's own `release` is not read for its signers
