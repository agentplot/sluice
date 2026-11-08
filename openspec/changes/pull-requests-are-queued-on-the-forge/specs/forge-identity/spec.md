## MODIFIED Requirements

### Requirement: forge check verifies the setup
`sluice forge check [<branch>]` SHALL report, for the branch's remote,
whether the key reads, a token mints, the app is installed on the
repository with permission to write contents, workflows, checks and
pull requests, and the branch's rules on the forge let only the app
bypass them. For a branch the forge keeps, it SHALL also report whether
the repository's default branch, on which the round job of every
branch the forge keeps runs, holds the round job `sluice init actions`
would write at this version, whether the app may dispatch it, and
whether the `sluice` environment exists and admits only the default
branch. It SHALL exit non-zero when any of these fails, naming which.

#### Scenario: A ruleset that lets the operator through
- **WHEN** the ruleset on `main` lists the operator's account as a bypass beside the app
- **THEN** `sluice forge check main` exits non-zero and names the extra bypass

#### Scenario: All set
- **WHEN** the app is installed with the permissions it needs and is the ruleset's only bypass
- **THEN** `sluice forge check main` exits 0

#### Scenario: No permission to post checks
- **WHEN** the app is installed with Contents and Workflows write but only Checks read
- **THEN** `sluice forge check main` exits non-zero and names the Checks permission

#### Scenario: No round job on the branch
- **WHEN** `main` is the forge's and holds no `.github/workflows/sluice.yml`
- **THEN** `sluice forge check main` exits non-zero and names the workflow and `sluice init actions`

#### Scenario: The app may not dispatch it
- **WHEN** `main` is the forge's and the app has Actions read only
- **THEN** `sluice forge check main` exits non-zero and names the Actions permission

#### Scenario: An environment any branch may use
- **WHEN** `main` is the forge's and the `sluice` environment has no branch rule
- **THEN** `sluice forge check main` exits non-zero and names the environment

#### Scenario: Another branch the forge keeps
- **WHEN** `release/1` is the forge's, `main` is the default branch and holds the round job, and the `sluice` environment admits `main` alone
- **THEN** `sluice forge check release/1` reads the round job on `main`, passes the environment, and names neither as failing
