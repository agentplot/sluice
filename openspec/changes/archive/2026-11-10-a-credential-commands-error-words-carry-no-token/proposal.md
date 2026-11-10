## Why

A host may name in `[forge] credential` a command that gives the app's
token, asked as git asks a credential helper (record 0108). When the
command gives no token, the call fails, naming the command and giving
what it wrote on its standard error. Today `CredentialCommand::get`
passes that standard error on trimmed and nothing more. A helper that
prints a token on its error output, a refused token it names or a
response body it echoes, puts the token in the failure's words. From
there it reaches every reader of the failure: `sluice forge check`'s
finding, a round's report, the branch's log, the host's kept state, and
a status.

Record 0108 says nothing of the token is kept, and no token is written
to a file or a log. Rule core.28 logs a refusal on the branch with its
words. The configuration reference already says the failure gives the
command's standard error, never a token. Batch 42's review of
`sluice-takes-its-forge-token-from-a-command` noted that the failure
passes the command's error output on trimmed: it holds no token from
the command's standard output, but nothing keeps one out of its
standard error. This change closes that gap. It needs that
change's work on main, which it is (`69bc9934`).

## What Changes

- **A credential command's words carry no token.** `get` passes the
  command's standard error through sluice's scrub before its failure
  carries it. Every string shaped as a GitHub token becomes
  `[redacted]`: one of the prefixes the flake's module refuses in
  `forge.credential`, `ghs_`, `ghp_`, `gho_`, `ghu_`, `ghr_` or
  `github_pat_`, then sixteen or more letters, digits, `_` or `-`.
  The rest of the words stand as the command wrote them. So the failure,
  and every report, log line, kept state and status made from it, holds
  no token.
- **The scrub reads a GitHub token by the module's prefixes.** The scrub
  already redacts GitHub's tokens in telemetry and in a failed check's
  output posted on a pull request (records 0022, 0044). Its GitHub
  shapes are now built from one list, the module's six prefixes, and a
  test fails if the module's list and the scrub's differ. A prefix
  followed by sixteen or more of a token's characters reads as a token,
  as flume's design 7 reads one.
- **Words cut at their limit end at a whole line.** sluice keeps the
  first 64 KiB of the command's standard error. Where the command wrote
  more, the failure carries only the kept part's whole lines. A token
  cut at the limit would be too short to have a token's shape, so this
  keeps the start of one out of the words.
- The configuration reference says the failure gives the command's
  standard error with what reads as a GitHub token redacted. Record 0108
  gets an amendment recording this change's choices, and rule core.28's
  details say a credential command's words are carried redacted.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `forge-identity`: a credential command's words, carried in its
  failure, hold no string shaped as a GitHub token, and are cut at a
  whole line where the command wrote more than sluice keeps.

## Out of scope

- **No clause changes.** Record 0108's "nothing of the token is kept"
  already asks this, and clause 134 names the command, never its words.
  The redaction is a build choice within record 0108, recorded in its
  amendment.
- **The command's standard output.** A failure never gives it (record
  0108 as amended 2026-11-09), and this change leaves that as it is.
- **The value of the `password` a failed command printed.** sluice reads
  no standard output of a command that failed, and the app's token is
  GitHub's, whose shape the scrub knows.
- **The scrub's other shapes and its variables.** The command's words
  pass through every shape the scrub knows. The values of variables
  whose names say they are secret are not redacted from them, as
  design D1 says.
- **One family list of credential shapes.** flume refuses a credential's
  shape in a program's settings (flume design 7,
  `crates/flume-core/src/own.rs`). sluice's scrub redacts text and its
  module refuses a store value. All three read the same six GitHub
  prefixes. Giving the family's shapes one owner is a question for the
  statement's principles, and this change doesn't wait on it.

## Impact

- `crates/sluice/src/scrub.rs`: `GITHUB_TOKEN_PREFIXES`, the GitHub
  shapes built from it, and `Scrub::shapes()`.
- `crates/sluice/moon.yml`: `cli:test` reads the module's
  `nix/modules/options.nix`, whose list a unit test reads.
- `crates/sluice/src/identity.rs`: `get`'s standard error scrubbed and,
  where it was cut, carried to its last whole line. `drain` says whether
  it cut.
- Tests: `crates/sluice/src/scrub.rs` and `crates/sluice/src/identity.rs`
  (their own), `crates/sluice/tests/profiles/forge.rs`.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `docs/adr/0108-sluices-forge-token-may-come-from-a-credential-command-the-host-names.md`,
  `docs/architecture/core.md` (core.28's details).
- Sources: record 0108 (accepted 2026-11-09, as amended that day);
  records 0022 and 0044; clauses 133 and 134; rules core.1, core.26 and
  core.28; flume design 7; batch 42's review of
  `sluice-takes-its-forge-token-from-a-command`.

## Touches

```
crates/sluice/src/scrub.rs
crates/sluice/moon.yml
crates/sluice/src/identity.rs
crates/sluice/tests/profiles/forge.rs
site/src/content/docs/reference/configuration.md
docs/adr/0108-sluices-forge-token-may-come-from-a-credential-command-the-host-names.md
docs/architecture/core.md
openspec/changes/a-credential-commands-error-words-carry-no-token/**
```
