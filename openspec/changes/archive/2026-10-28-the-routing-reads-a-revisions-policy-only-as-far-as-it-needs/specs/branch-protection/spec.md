## MODIFIED Requirements

### Requirement: The protection reads only what guarding needs
Of the host's settings, the protection SHALL read only `[forge]`, how
sluice reaches a branch's remote. Of the policy it SHALL read only the
`[branches]` patterns, each one's `remote`, and whether `[signers]`
names a signer: which branches are guarded and where their records are.
It SHALL pass over every other table and field, known to it or not, so
a host whose settings a newer sluice wrote is guarded by an older hook
as before. Every other command reads the host's settings in full, and
reads the policy in full at the revision its gate is read from; to decide
which remote a branch is on, every command reads the policy as the
protection does (124; record 0069).

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

#### Scenario: Every other command still refuses a policy field at the branch's revision
- **WHEN** the policy at `bolt/x`'s revision, in local refs, holds a `[[checkout.cache]]` table and a submission to `bolt/x` is waiting
- **THEN** `sluice step bolt/x` lands nothing and names the table, as it did before
