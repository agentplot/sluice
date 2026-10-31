## MODIFIED Requirements

### Requirement: A result counts only when a trusted signer signed it
A result SHALL count for a gate only when its record is signed by a
signer the gate trusts and the signature verifies from the records and
the policy alone. A record with no signature, or one by a signer the
gate does not trust, SHALL never satisfy the gate, whatever its outcome
(78, 28). The signature SHALL verify with a key the policy of the
gate's own branch lists for that signer; a key another branch's policy
lists for a signer of the same name SHALL count for nothing on it.

#### Scenario: Signed by a key the policy does not name
- **WHEN** a result claiming a confined pass is signed by a key `main`'s policy does not list
- **THEN** `main` reads the check as missing until a result its signers signed settles it (S18)

#### Scenario: A record's executor field alone
- **WHEN** an unsigned record on the remote says its executor was a confined container
- **THEN** it counts for no gate

#### Scenario: A key only another branch lists
- **WHEN** `release` trusts `box`, `release`'s policy lists key B for `box`, `main`'s lists key A, and a pass for `release`'s candidate is signed as `box` with key A
- **THEN** `release` reads the check as missing until a pass signed with key B settles it, while the same pass counts for `main`

#### Scenario: A branch in the host's own files
- **WHEN** `scratch` keeps its records in the host's own files, its policy at its revision names no signers, and `main` on origin names `[signers.box]`
- **THEN** none of box's signed runs on origin is read for `scratch`'s gate

## ADDED Requirements

### Requirement: A shared branch's run is taken only by a host whose signature its gate would count
On a branch whose records are on its remote, a run SHALL be taken only by
a host signing as a signer the branch's policy names, with a key that
policy lists for it at the revision the round read; the name alone SHALL
take nothing. A run no such host takes SHALL stay open until the time
limit reads its candidate not passed (87, 66). A result signed with an
unlisted key SHALL be missing, never a fail, and its run open again
(28, S18). The records seam, never the core, SHALL read the key.

#### Scenario: A host whose key the branch does not list takes no run
- **WHEN** host a signs as a signer `main`'s gate trusts, with a key `main`'s policy does not list, and a submission waits on `main`
- **THEN** host a's rounds leave the run open and unclaimed, and host b, whose key `main` lists, claims it and the submission lands

#### Scenario: A run no listed host takes is not passed at the time limit
- **WHEN** the only live host signs with a key `main`'s policy does not list, and `main`'s time limit passes with the run still open
- **THEN** the candidate reads as not passed, and no run of it was started

#### Scenario: A result signed with a key the branch does not list is missing (S18)
- **WHEN** host a runs `main`'s candidate, `main`'s policy stops listing host a's key before the result is read, and the run reports
- **THEN** its checks are missing, not failed: the submission is not failed, the run is open again, the log records the checks as uncounted for host a, and once host b's rounds run it the submission lands
