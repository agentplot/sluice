# Spec Delta

## ADDED Requirements

### Requirement: The pause names the installation by its account
The installation a pause is kept for SHALL be named by the account it
is on, the repository's owner, since an app has one installation on an
account. This SHALL hold whether the token was minted from the app's
key or given by the host's credential command, which tells no
installation id, so a process holding either honours a pause the other
met (134; record 0108).

#### Scenario: A command's token honours the pause
- **WHEN** a process whose token came from the app's key is refused with the REST allowance spent for `o/r` until 14:20, and a process whose `[forge]` names a credential command for the same app reads `o/r`'s pull requests at 14:05
- **THEN** the second process makes no call to GitHub's REST API for `o/r`, and the pause is kept under the app and `o`
