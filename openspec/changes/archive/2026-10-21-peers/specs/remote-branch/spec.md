## MODIFIED Requirements

### Requirement: Only the branch reaches the remote
sluice SHALL push nothing to the remote but the branch's landing commits
and, for a branch whose state is on the remote, refs under
`refs/sluice/`: the branch's state, its signers' run records, submitted
heads, open combinations, kept output and hosts' signs of life. It SHALL
push no other ref.

#### Scenario: Nothing else is pushed
- **WHEN** three submissions are queued, tested and landed on a branch with a `remote` whose state is in the host's own files
- **THEN** the remote holds only the branch's new commits, and no other ref on the remote changed

#### Scenario: State on the remote stays under refs/sluice
- **WHEN** three submissions are queued, tested and landed on a branch whose state is on the remote
- **THEN** every ref sluice changed on the remote is the branch or under `refs/sluice/`
