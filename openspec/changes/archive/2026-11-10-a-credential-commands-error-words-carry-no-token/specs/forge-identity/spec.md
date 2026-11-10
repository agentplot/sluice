# Spec Delta

## ADDED Requirements

### Requirement: A credential command's words carry no token
A credential command's failure SHALL carry its standard error with
every string shaped as a GitHub token replaced by `[redacted]`, the rest
of its words as the command wrote them. The failure, and every report,
log line, kept state and status made from it, SHALL hold no token
(record 0108).

#### Scenario: The command's words hold a token
- **WHEN** the command exits 1 writing `the cached ghs_0123456789abcdefghijABCDEFGHIJ012345 was refused` on its standard error
- **THEN** the call fails, naming the command and `exited 1`, and its failure carries `the cached [redacted] was refused` and none of the token

#### Scenario: A command that prints no password names a token
- **WHEN** the command exits 0 printing only `username=x-access-token`, and writes a `github_pat_` token on its standard error
- **THEN** the call fails, saying the command gave no `password`, and its failure carries `[redacted]` in place of the token

#### Scenario: forge check tells the command's words without its token
- **WHEN** `sluice forge check main` runs a command that exits 1 writing `the cached ghs_0123456789abcdefghijABCDEFGHIJ012345 was refused` on its standard error
- **THEN** the check fails, saying `the cached [redacted] was refused`, and nothing it says holds the token

### Requirement: sluice reads a GitHub token by the module's prefixes
Wherever sluice redacts what reads as a secret, a string SHALL be shaped
as a GitHub token where one of the prefixes the host settings module
refuses in `forge.credential` is followed by sixteen or more letters,
digits, `_` or `-`. Those prefixes are `ghs_`, `ghp_`, `gho_`, `ghu_`,
`ghr_` and `github_pat_`. The gate SHALL fail where the module's list
and sluice's differ (record 0108; rule core.51).

#### Scenario: Each prefix's token redacted
- **WHEN** text holds one of the six prefixes followed by sixteen letters and digits
- **THEN** that string becomes `[redacted]`, and the text around it is left as it is

#### Scenario: A prefix with no token after it
- **WHEN** a credential command's words say `expected a ghs_ token`, or name a prefix followed by fifteen characters
- **THEN** those words are carried as written

#### Scenario: The module's list and sluice's differ
- **WHEN** the module's `prefixes` in `nix/modules/options.nix` names a prefix sluice's list does not, or sluice's names one the module's does not
- **THEN** the crate's tests fail, naming the prefixes each list holds

### Requirement: A credential command's words cut at their limit end at a whole line
Where a credential command writes more than 64 KiB on its standard
error, its failure SHALL carry only the whole lines of the first 64 KiB,
and none of its words where those hold no line's end. A token cut at
the limit SHALL leave none of itself in the failure (record 0108).

#### Scenario: A token across the limit
- **WHEN** the command exits 1 writing 65,520 `x` and a line's end on its standard error, then `ghs_0123456789abcdefghijABCDEFGHIJ012345`
- **THEN** the call fails, naming `exited 1`, and its failure holds neither `ghs_` nor `0123456789a`
