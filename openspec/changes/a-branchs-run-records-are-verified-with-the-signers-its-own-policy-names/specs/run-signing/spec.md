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
