# Spec Delta

## ADDED Requirements

### Requirement: The protection reads only what guarding needs
Of the host's settings, the protection SHALL read only `[forge]`, how
sluice reaches a branch's remote. Of the policy it SHALL read only the
`[branches]` patterns, each one's `remote`, and whether `[signers]`
names a signer: which branches are guarded and where their records are.
It SHALL pass over every other table and field, known to it or not, so
a host whose settings a newer sluice wrote is guarded by an older hook
as before. Every other command reads both in full, as before (124).

#### Scenario: A host setting the hook does not know changes nothing it guards
- **WHEN** the host's `~/.config/sluice/config.toml` holds `[runs] cores = 12` and a table `[later]` with `x = 1`, which no sluice yet reads, and someone runs `git update-ref refs/heads/bolt/x <unit/a's revision>`
- **THEN** git fails with sluice's refusal naming the command, `bolt/x` is where it was, its log has one `protected` line, and sluice prints no warning about the settings

#### Scenario: A move that changes nothing still passes under such settings
- **WHEN** the host's settings hold that same unknown table and someone runs `git reset --hard` in a worktree on `bolt/x`
- **THEN** the reset succeeds, `bolt/x` is where it was, and sluice prints nothing

#### Scenario: A policy field the hook does not know changes nothing it guards
- **WHEN** the policy at `bolt/x`'s revision gives `[branches."bolt/*"]` a field `later = true` and has a table `[later]`, neither of which any sluice yet reads, and someone runs `git branch -f bolt/x unit/a`
- **THEN** git fails with sluice's refusal, `bolt/x` is where it was, and its log has one `protected` line

#### Scenario: A branch on a remote is still judged as sluice's copy
- **WHEN** `bolt/x` names `remote = "origin"`, the policy and the host's settings each hold a field no sluice yet reads, the host's `bolt/x` is at `10e4791` and origin's at `8079db3`, and someone runs `git update-ref refs/heads/bolt/x 8079db3`
- **THEN** git fails, the refusal says sluice levels this host's `bolt/x` at each landing, and the host's `bolt/x` is still `10e4791`

#### Scenario: Every other command still refuses a field it does not know
- **WHEN** the host's settings hold `[runs] later = 1` and someone runs `sluice status`
- **THEN** it fails naming the settings file and the field, as it did before

### Requirement: Settings or a policy the protection cannot read pass the transaction, said once
Host settings or a policy the protection cannot read at all, whether not
TOML or holding a field guarding needs that is of the wrong kind, SHALL
be a reason of its own: the transaction SHALL pass, with one warning
naming what could not be read and why, and nothing SHALL be refused or
recorded for it (124).

#### Scenario: Host settings that are not TOML pass the transaction
- **WHEN** the host's `~/.config/sluice/config.toml` is not TOML and someone runs `git update-ref refs/heads/bolt/x <unit/a's revision>`
- **THEN** the update succeeds, git's error output holds exactly one warning from sluice saying its branch protection let the transaction through and naming the settings file, and `bolt/x`'s log has no `protected` line

#### Scenario: A remote of the wrong kind passes the transaction
- **WHEN** the policy at `bolt/x`'s revision gives `[branches."bolt/*"]` `remote = 1` and someone runs `git update-ref refs/heads/bolt/x <unit/a's revision>`
- **THEN** the update succeeds with exactly one warning from sluice naming the policy and `remote`, and `bolt/x`'s log has no `protected` line

### Requirement: The hook runs the sluice that installed it
The hook SHALL run the sluice that installed it, by the path it was
installed with, never a `sluice` found where the command that ran git
looks for one, since what the hook refuses may not turn on who ran git.
Every command sluice runs in a repository SHALL re-install the hook
there for itself where sluice's hook stands for another sluice; a
repository with no sluice hook gets one only from `sluice protect`,
`submit`, `wait`, `step` and `run` (1, 124).

#### Scenario: A sluice on the caller's PATH is not the one that judges
- **WHEN** the hook was installed by `sluice protect`, and someone whose `PATH` holds first another program called `sluice` that exits 0 having read nothing runs `git update-ref refs/heads/bolt/x <unit/a's revision>`
- **THEN** git fails with the installed sluice's refusal and `bolt/x` is where it was

#### Scenario: A newer sluice's next command re-installs the hook
- **WHEN** the hook names a sluice at one path, and a sluice at another path runs `sluice status` in the repository
- **THEN** the hook names the second sluice's path, the hook chained beside it is unchanged, and `sluice status` says nothing of the hook

#### Scenario: A removed hook is not put back by a reading command
- **WHEN** `sluice protect --remove` took the hook out and someone runs `sluice status` in the repository
- **THEN** the repository still has no sluice hook
