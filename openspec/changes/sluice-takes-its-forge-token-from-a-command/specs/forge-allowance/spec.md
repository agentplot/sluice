# Spec Delta

## MODIFIED Requirements

### Requirement: Every process on the host honours the pause
The pause SHALL be written in the host's state, for the app's
installation on the repository and for the allowance GitHub names
(`x-ratelimit-resource`: REST's apart from GraphQL's). The installation
SHALL be named by the account it is on, the repository's owner, since
an app has one installation on an account. This holds whether the token
was minted from the app's key or given by the host's credential command,
which tells no installation id (record 0108). Every process of sluice on
the host SHALL read it before it calls GitHub. It SHALL make no call
under that installation and allowance while the pause stands, and none
to find out whether it has ended. The pause SHALL hold no token (134).

#### Scenario: One process meets it, another honours it
- **WHEN** `sluice run` is refused with the allowance spent until 14:20, and a `sluice submit main` starts in a worktree on the same host at 14:05
- **THEN** the submit makes no call to GitHub's REST API before 14:20

#### Scenario: GraphQL's allowance is its own
- **WHEN** the REST allowance is paused and the GraphQL one is not
- **THEN** a GraphQL query is made

#### Scenario: The pause outlives the process
- **WHEN** the process that met the pause is stopped and started again before the pause ends
- **THEN** the new process makes no call to GitHub before the pause ends

#### Scenario: A command's token honours the pause
- **WHEN** a process whose token came from the app's key is refused with the REST allowance spent for `o/r` until 14:20, and a process whose `[forge]` names a credential command for the same app reads `o/r`'s pull requests at 14:05
- **THEN** the second process makes no call to GitHub's REST API for `o/r`, and the pause is kept under the app and `o`
