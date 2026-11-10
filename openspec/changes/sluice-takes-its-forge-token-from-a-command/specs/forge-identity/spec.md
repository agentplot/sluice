# Spec Delta

## MODIFIED Requirements

### Requirement: The host names sluice's app
The host's config SHALL accept a `[forge]` table with `app_id` and one
of two ways to the app's token: `key`, the path of the app's private key
on the host, or `credential`, a command that gives the token (record
0108). A key file readable by anyone but its owner SHALL be refused with
a message naming the file and the mode it needs.

#### Scenario: A key others can read
- **WHEN** `key` names a file with mode `0644`
- **THEN** sluice refuses to use it, and the message names the file and `0600`

#### Scenario: No forge table
- **WHEN** the host's config has no `[forge]` table and a branch has a `remote`
- **THEN** sluice fetches and pushes with the host's own git credentials, and `sluice status` says the branch has no identity of its own

### Requirement: sluice's git calls carry the app's token
With a `[forge]` table, sluice's fetches and pushes of a branch whose
remote is on GitHub SHALL authenticate with a token for that repository
alone: an installation token minted from the app's key, or the token
the host's credential command gives (record 0108). The token SHALL be
held in memory only, never written to a file, a git config, a log or the
environment of any process but that git call.

#### Scenario: A landing pushed as the app
- **WHEN** a submission lands on a branch with a `remote` on GitHub and the host has a `[forge]` table
- **THEN** the push authenticates as the app, and GitHub records the push by the app

#### Scenario: Nothing left behind
- **WHEN** a round has fetched and pushed with the app's token
- **THEN** no file, git config or environment variable on the host holds the token

### Requirement: A refusal that stands names each refused token
A call the remote refused on sluice's token on every try until the
retry policy's time is spent SHALL fail with the remote's own words,
whole, followed by what each refused token was, in the order tried, and
never the token itself. A minted token SHALL be named by its app, its
repository, the installation it was bought from, the repositories and
permissions GitHub gave it, and its age.

#### Scenario: Refused twice
- **WHEN** GitHub answers a push with `Repository not found.` on the token held and on a fresh one, and the policy's time is spent before another try
- **THEN** two pushes were made, and the failure carries `Repository not found.`, then says the remote refused the first token and then the fresh one, naming each one's app, repository, installation, scope and age

#### Scenario: No fresh token can be bought
- **WHEN** a push is refused on its token and GitHub refuses to issue another
- **THEN** one push was made, and the failure carries the remote's words, what the refused token was, and why none could be bought afresh

### Requirement: forge check verifies the setup
`sluice forge check [<branch>]` SHALL report, for the branch's remote,
whether every ruleset on the branch lets only the app through, and
SHALL exit non-zero when any check fails, naming which. With a key, it
SHALL also report whether the key reads, a token mints, and the app is
installed on the repository with permission to write contents,
workflows, checks and pull requests.

#### Scenario: A ruleset that lets the operator through
- **WHEN** the ruleset on `main` lists the operator's account as a bypass beside the app
- **THEN** `sluice forge check main` exits non-zero and names the extra bypass

#### Scenario: All set
- **WHEN** the app is installed with the permissions it needs and is the ruleset's only bypass
- **THEN** `sluice forge check main` exits 0

#### Scenario: No permission to post checks
- **WHEN** the app is installed with Contents and Workflows write but only Checks read
- **THEN** `sluice forge check main` exits non-zero and names the Checks permission

## ADDED Requirements

### Requirement: A forge credential is a command written as a list
`[forge] credential` SHALL be written as a list of the command's
program and then each argument, and run with no shell. A `credential`
that is a string, an empty list, or a list whose program is a relative
path SHALL be refused, naming the setting. Branch protection's reading
of the host's settings SHALL take `credential` as it takes `key` (124;
records 0059, 0108).

#### Scenario: A command in place of a key
- **WHEN** `[forge]` names `app_id = 42` and `credential = ["/opt/forge/bin/token-helper", "--app", "sluice"]`
- **THEN** sluice reads the table, with the command's program and arguments as written and no key, and branch protection's reading gives the same command

#### Scenario: A command written as a shell line
- **WHEN** `[forge]` names `credential = "token-helper --app sluice"`
- **THEN** sluice refuses the table, saying the command is a list of its program and arguments, run with no shell

### Requirement: A forge table names a key or a command, never both
A `[forge]` table that names both `key` and `credential`, or neither,
SHALL be refused, naming both (record 0108).

#### Scenario: Both a key and a command
- **WHEN** `[forge]` names both `key` and `credential`
- **THEN** sluice refuses the table, naming `key` and `credential` and saying to name one

### Requirement: The credential command is asked as git asks a helper
sluice SHALL ask the host's credential command for a repository's token
as git asks a credential helper: it runs the command with `get` appended
to its arguments and writes `protocol=https`, `host=github.com` and
`path=<owner>/<name>` on its standard input. It then reads `password`
and `password_expiry_utc` from the command's standard output (record
0108).

#### Scenario: The command asked as a credential helper
- **WHEN** the host's `[forge]` names a credential command and sluice needs credentials for `https://github.com/o/r.git`
- **THEN** the command was run once with `get` as its last argument, its standard input held `protocol=https`, `host=github.com` and `path=o/r`, and the credentials carry the `password` it printed

### Requirement: A command's token is named by its command
Where a refusal that stands, or the credentials, say what a token a
credential command gave is, they SHALL name its app, its repository, the
command, what GitHub said it reaches when sluice got it, its age, and
the end the command gave or that it gave none, and never the token's
text (134 as amended 2026-11-09; record 0108).

#### Scenario: A command's token described
- **WHEN** the host's credential command gives a token for `o/r` that ends in an hour, and GitHub answers that it reaches `o/r`
- **THEN** the credentials say they are app 4242's token for `o/r` from the command, naming its program and arguments, reaching `o/r`, with its age and its end, and never hold the token's text in what they say of it

### Requirement: forge check verifies a credential command
With a credential command, `sluice forge check [<branch>]` SHALL also
report whether the command gives a token for the repository, whether
that token is the app's that `app_id` names, and whether the token may
push to the repository. Whether it may push SHALL be read from GitHub's
answer to the receive-pack advertisement that git's push asks for first
(record 0108).

#### Scenario: A command, all set
- **WHEN** the host's credential command gives a token for the repository, GitHub names that token's bot as app 4242's, the receive-pack advertisement answers 200, and the ruleset's only bypass is app 4242
- **THEN** the check says the command gives a token, the token is app 4242's and may push, and the ruleset lets only app 4242 through, and fails nothing

#### Scenario: A command that gives another app's token, one that may not push
- **WHEN** the host's `[forge]` names `app_id = 4242`, its command gives a token GitHub names as app 99's, and the receive-pack advertisement answers 403
- **THEN** the check fails, naming app 99 and app 4242, and saying that the token may not push and that the app needs Contents read and write

### Requirement: A command's token is held until five minutes before its end
sluice SHALL hold a token the credential command gave for a repository
in memory and use it for that repository's calls until five minutes
before the end the command gave in `password_expiry_utc`, and SHALL then
run the command again. A token given with no end SHALL be held until
GitHub refuses it. A token held SHALL be used for its repository alone
(134; record 0108).

#### Scenario: A token got once an hour
- **WHEN** the command gives its n-th token with an end n hours after sluice first needs one, and sluice needs a token for `o/r` each minute for two hours
- **THEN** the command ran three times, at the first minute and at 55 and 115 minutes, five minutes before the end of the token it gave last

#### Scenario: A token with no end
- **WHEN** the command gives a token with no `password_expiry_utc`, and sluice needs a token for `o/r` each minute for two hours with GitHub refusing none
- **THEN** the command ran once

### Requirement: A command's token refused is got afresh from the command
Wherever sluice would buy a fresh token for a call GitHub refused the
held one on, it SHALL run the credential command with `erase`, writing
`protocol`, `host`, `path`, `username` and the refused token as
`password`, then with `get`. It SHALL NOT try again with the refused
token. A `get` that gives the refused token back SHALL be no fresh
token. A token got afresh SHALL be handed over once five seconds have
passed since it was got, as a token bought afresh is (134; rule
core.29).

#### Scenario: Refused, erased and got afresh
- **WHEN** GitHub answers a read with `401 Bad credentials` on the token the command gave first, and takes the second it gives
- **THEN** the command ran with `get`, then `erase` for the first token, then `get`, the read was made twice, the second time with the second token, and sluice holds the second token after

#### Scenario: The command gives the refused token back
- **WHEN** GitHub refuses the token held, and the command, run with `erase` and then `get`, gives the same token again
- **THEN** the read is not made again, and the failure carries GitHub's words and says no token could be got afresh because the command gave the refused one again

### Requirement: A command that gives no token fails the call
A credential command that exits non-zero, prints no `password`, or
gives nothing within thirty seconds SHALL fail the call that needed its
token, as a key that cannot sign fails it. The failure SHALL name the
command and give its own words from its standard error. It SHALL never
hold a token. A command still running at thirty seconds SHALL be
stopped. What the command answers to `erase` SHALL NOT be read, as git
reads none (record 0108).

#### Scenario: The command refuses
- **WHEN** the command exits 1 saying `no drive on this host serves o/r` on its standard error
- **THEN** the call fails, and its failure names the command's program and arguments and carries `no drive on this host serves o/r`

#### Scenario: The command prints no password
- **WHEN** the command exits 0 printing only `username=x-access-token`
- **THEN** the call fails, naming the command and saying it gave no `password`

#### Scenario: The command hangs
- **WHEN** the command gives nothing within its limit
- **THEN** it is stopped, and the call fails, naming the command and its limit

### Requirement: A command's token is asked what it reaches
When the credential command gives a token for a repository, sluice
SHALL ask GitHub once, with that token, which repositories it reaches,
and SHALL refuse a token GitHub says does not reach the repository, as
it refuses a minted one, naming what it reaches. The read SHALL be
counted against the installation's allowance and not made while its
pause stands, and a refusal of it SHALL NOT be a reason to get another
token (134 as amended 2026-11-09; record 0108).

#### Scenario: A token for another repository
- **WHEN** the command gives a token for `o/r` that GitHub says reaches `o/other` alone
- **THEN** no call is made with it for `o/r`, and the failure names the command, `o/r` and `o/other`

### Requirement: The app's bot is read from the token under a command
Under a credential command, the app's bot, the author of every pull
request sluice opens, SHALL be read once in a process from GraphQL's
`viewer` with the repository's token, since GitHub answers the app's
own record to its key alone (C.5 pull request, 120; record 0108).

#### Scenario: The bot under a command
- **WHEN** sluice asks twice who the app's bot is, under a command whose token GraphQL's `viewer` names `sluice-test[bot]`
- **THEN** both answers are `sluice-test[bot]`, GraphQL was asked once, and `GET /app` was never called
