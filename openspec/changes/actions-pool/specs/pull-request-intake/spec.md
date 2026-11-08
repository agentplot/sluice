## MODIFIED Requirements

### Requirement: A branch turns pull requests on in its policy
A branch's policy SHALL accept a `[branches."<pattern>".pull_requests]`
table with `reviews`, a count of approving reviews, default 0, and
`checks`, a list of check names other systems post, default empty;
sluice's own required checks are read from its results, never listed
there. The table SHALL be refused on a branch with no `remote`. On a
branch whose records each host keeps, the policy naming no
`[signers]`, or in a policy that names no pool, the table SHALL refuse
that branch alone, naming `[signers]` or `[pools]` and `sluice init
actions`, since nothing would take its requests in; every other branch
of the policy SHALL read as before, and a submit to the refused branch
SHALL open and push nothing (56, 137 as amended 2026-11-07, S64). The
conditions SHALL be read from the branch's revision.

#### Scenario: Pull requests need a remote
- **WHEN** a branch's policy has a `pull_requests` table and no `remote`
- **THEN** the policy is refused with a message naming the branch and `remote`

#### Scenario: Pull requests need the branch's records on its remote
- **WHEN** `main` names `remote = "origin"` and a `pull_requests` table, the policy names no `[signers]`, and `bolt/x` names `remote = "origin"` and no `pull_requests`
- **THEN** `sluice status main` shows `main` refused, naming `[signers]` and `sluice init actions`; `wt submit main` exits 2 with those words and nothing is pushed or opened; and `sluice step bolt/x` lands a submission to `bolt/x`

#### Scenario: Pull requests need a pool
- **WHEN** `main` takes pull requests, the policy names `[signers.box]` and no pool
- **THEN** `main` is refused, naming `[pools]` and `sluice init actions`

#### Scenario: A pull request cannot loosen its own conditions
- **WHEN** a pull request changes `.config/sluice.toml` to lower `reviews`
- **THEN** its conditions are still the ones on the branch's revision

#### Scenario: sluice's own check in checks
- **WHEN** `checks` lists a check run sluice itself posts for a required check
- **THEN** the policy is refused, and the message says sluice reads its own checks from its results

## ADDED Requirements

### Requirement: A run on a pull request's head counts toward its combination
A run on a pull request's head by a signer the branch trusts SHALL be
recorded under its keys and systems as any run is, and a combination
that shares a key on a system SHALL read that result without running
the check again (15, 113).

#### Scenario: Passed before it was ready
- **WHEN** a pull request's head passed `cli:build` on three systems on the pool, and it is submitted behind one submission that changes no input of `cli:build`
- **THEN** its combination reads the three passes by key and runs only the checks the submission ahead changes (S40)

#### Scenario: A head run by an untrusted signer
- **WHEN** a pull request's head was run by a signer `main` does not trust
- **THEN** its combination runs every required check as if the head had never been run
