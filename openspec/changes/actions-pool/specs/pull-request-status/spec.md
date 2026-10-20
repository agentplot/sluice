## ADDED Requirements

### Requirement: sluice posts a check per required check per system
Beside its `sluice` check, sluice SHALL post on a pull request's head one
check run per required check per system the branch names, named
`sluice / <check> (<system>)`, with the result's outcome and the signer
that signed it, and update it when a result is recorded for that key.

#### Scenario: A build on three systems
- **WHEN** `main` needs `cli:build` on three systems and a pull request's combination passed it on two and is running the third
- **THEN** the pull request's head shows `sluice / cli:build (linux-x64)` and `(linux-arm64)` as success naming their signers, and `(macos-arm64)` in progress

#### Scenario: A result read by key
- **WHEN** a pull request's combination reads a pass recorded for the same key from an earlier run
- **THEN** its check run reads success and names the signer of that pass
