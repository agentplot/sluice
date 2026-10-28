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
repository with permission to write contents, workflows, checks and
pull requests, and the branch's rules on the forge let only the app
bypass them. It SHALL exit non-zero when any of these fails, naming
which.

#### Scenario: A ruleset that lets the operator through
- **WHEN** the ruleset on `main` lists the operator's account as a bypass beside the app
- **THEN** `sluice forge check main` exits non-zero and names the extra bypass

#### Scenario: All set
- **WHEN** the app is installed with the permissions it needs and is the ruleset's only bypass
- **THEN** `sluice forge check main` exits 0

#### Scenario: No permission to post checks
- **WHEN** the app is installed with Contents and Workflows write but only Checks read
- **THEN** `sluice forge check main` exits non-zero and names the Checks permission

### Requirement: A push refused at the ref or left unanswered is not made again
A push the remote refused at the ref, its lease stale, the branch
protected or any ref rejected, SHALL be the move refused and SHALL NOT
be made again, whatever the remote said besides. A push whose answer
git did not read, a connection cut or a server error after the push was
sent, SHALL NOT be made again: what the remote holds decides it (24, 45,
122).

#### Scenario: A protected branch is not pushed twice
- **WHEN** a push to `main` is answered `[remote rejected] (protected branch hook declined)`
- **THEN** one push was made, and the move is reported refused in the remote's words

#### Scenario: A stale lease is not pushed twice
- **WHEN** a landing's push is refused because the remote's branch moved
- **THEN** one push was made, the refusal names the revision the remote holds, and no token is bought afresh

#### Scenario: A server error is not pushed again
- **WHEN** the remote answers a push with `RPC failed; HTTP 500` after the push was sent
- **THEN** one push was made, and what the remote holds of the branch decides whether the move was made

### Requirement: A server error before the push is sent is no answer to it
A server error the remote gave before the push was sent, while git
asked it for its refs, is no answer to the push, and the push SHALL be
tried again under the retry policy (134).

#### Scenario: A server error before the push is tried again
- **WHEN** the remote answers git's request for its refs with `The requested URL returned error: 503`, and accepts the next try
- **THEN** the move is made

### Requirement: A refusal that stands names each refused token
A call the remote refused on sluice's token on every try until the
retry policy's time is spent SHALL fail with the remote's own words,
whole, followed by what each refused token was, in the order tried: the
app, the repository, the installation it was bought from, the
repositories and permissions GitHub gave it, and its age, and never the
token itself.

#### Scenario: Refused twice
- **WHEN** GitHub answers a push with `Repository not found.` on the token held and on a fresh one, and the policy's time is spent before another try
- **THEN** two pushes were made, and the failure carries `Repository not found.`, then says the remote refused the first token and then the fresh one, naming each one's app, repository, installation, scope and age

#### Scenario: No fresh token can be bought
- **WHEN** a push is refused on its token and GitHub refuses to issue another
- **THEN** one push was made, and the failure carries the remote's words, what the refused token was, and why none could be bought afresh
