# forge-identity Specification

## Purpose

Gives sluice an identity of its own on the forge, a GitHub App, so the
forge's rules can let sluice move a branch and refuse everyone else,
agents included.

## Requirements

### Requirement: The host names sluice's app
The host's config SHALL accept a `[forge]` table with `app_id` and
`key`, the path of the app's private key on the host. A key file
readable by anyone but its owner SHALL be refused with a message naming
the file and the mode it needs.

#### Scenario: A key others can read
- **WHEN** `key` names a file with mode `0644`
- **THEN** sluice refuses to use it, and the message names the file and `0600`

#### Scenario: No forge table
- **WHEN** the host's config has no `[forge]` table and a branch has a `remote`
- **THEN** sluice fetches and pushes with the host's own git credentials, and `sluice status` says the branch has no identity of its own

### Requirement: sluice's git calls carry the app's token
With a `[forge]` table, sluice's fetches and pushes of a branch whose
remote is on GitHub SHALL authenticate with an installation token minted
from the app's key for that repository. The token SHALL be held in
memory only, never written to a file, a git config or the environment
of any process but that git call.

#### Scenario: A landing pushed as the app
- **WHEN** a submission lands on a branch with a `remote` on GitHub and the host has a `[forge]` table
- **THEN** the push authenticates as the app, and GitHub records the push by the app

#### Scenario: Nothing left behind
- **WHEN** a round has fetched and pushed with the app's token
- **THEN** no file, git config or environment variable on the host holds the token

### Requirement: Agents never hold the app's token
The app's token SHALL be given only to git calls sluice itself makes.
A git command an agent runs in its worktree SHALL get no credential from
sluice.

#### Scenario: An agent pushes to main
- **WHEN** an agent runs `git push origin HEAD:main` in its worktree on a host where sluice has a `[forge]` table
- **THEN** the push carries no token of sluice's, and a ruleset with the app as its only bypass refuses it

### Requirement: forge check verifies the setup
`sluice forge check [<branch>]` SHALL report, for the branch's remote,
whether the key reads, a token mints, the app is installed on the
repository with permission to write contents, and the branch's rules on
the forge let only the app bypass them. It SHALL exit non-zero when any
of these fails, naming which.

#### Scenario: A ruleset that lets the operator through
- **WHEN** the ruleset on `main` lists the operator's account as a bypass beside the app
- **THEN** `sluice forge check main` exits non-zero and names the extra bypass

#### Scenario: All set
- **WHEN** the app is installed with the permissions it needs and is the ruleset's only bypass
- **THEN** `sluice forge check main` exits 0
