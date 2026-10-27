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

### Requirement: A call refused on the app's token is made once more on a fresh one
A fetch, a listing or a push of sluice's that the remote refuses on the
app's token, as not found, permission denied or unauthenticated, before
it answers for any ref, SHALL be made once more on a token bought afresh
and five seconds old. A push SHALL be made again with the same refspec
and the same lease. This SHALL hold for every push: a landing, a branch
made or removed, a session's branch, a submitted head and the records'
refs (134).

#### Scenario: A landing's push refused on a token just bought
- **WHEN** a submission lands on a branch on GitHub and GitHub answers the push with 403 on a token it gave a moment before, then accepts the next
- **THEN** the push is made once more on a fresh token, the submission reads landed, and nothing reports a failure

#### Scenario: The second push carries the first's lease
- **WHEN** the first push of a landing is refused on its token and the branch moves on the remote before the second
- **THEN** the second push is refused as the branch having moved, the submission is not landed, and the next round forms the chain again on the revision found

#### Scenario: A bolt made on a refused token
- **WHEN** `sluice branch create bolt/x --from main` pushes, GitHub answers `Repository not found.` on the first token and accepts a fresh one
- **THEN** `bolt/x` exists on the remote at `main`'s revision and the command exits 0

#### Scenario: A pull request's branch pushed on a refused token
- **WHEN** a submit pushes the session's branch for its pull request and the first token is refused
- **THEN** the branch is pushed on a fresh token and the submit goes on

#### Scenario: A fetch refused on a token just bought
- **WHEN** a round fetches the branch and the remote answers not found on the first token and serves the branch on a fresh one
- **THEN** the round reads the branch and goes on

#### Scenario: The host's own credentials are not renewed
- **WHEN** the host has no `[forge]` table and the remote refuses a push on the host's own credentials
- **THEN** the push is made once only and fails with the remote's words

### Requirement: A push refused at the ref or left unanswered is not made again
A push the remote refused at the ref, its lease stale, the branch
protected or any ref rejected, SHALL be the move refused and SHALL NOT
be made again, whatever the remote said besides. A push whose answer
git did not read, a connection cut or a server error, SHALL NOT be made
again: what the remote holds decides it (24, 45, 122).

#### Scenario: A protected branch is not pushed twice
- **WHEN** a push to `main` is answered `[remote rejected] (protected branch hook declined)`
- **THEN** one push was made, and the move is reported refused in the remote's words

#### Scenario: A stale lease is not pushed twice
- **WHEN** a landing's push is refused because the remote's branch moved
- **THEN** one push was made, the refusal names the revision the remote holds, and no token is bought afresh

#### Scenario: A server error is not pushed again
- **WHEN** the remote answers a push with a server error
- **THEN** one push was made and the failure is reported in the remote's words

### Requirement: A refusal that stands names each refused token
A call refused on a fresh token too SHALL fail with the remote's own
words, whole, followed by what each refused token was: the app, the
repository, the installation it was bought from, the repositories and
permissions GitHub gave it, and its age, and never the token itself. It
SHALL NOT be made a third time.

#### Scenario: Refused twice
- **WHEN** GitHub answers a push with `Repository not found.` on the first token and on a fresh one
- **THEN** two pushes were made, and the failure carries `Repository not found.`, then says the remote refused the first token and then the fresh one, naming each one's app, repository, installation, scope and age

#### Scenario: No fresh token can be bought
- **WHEN** a push is refused on its token and GitHub refuses to issue another
- **THEN** one push was made, and the failure carries the remote's words, what the refused token was, and why none could be bought afresh
