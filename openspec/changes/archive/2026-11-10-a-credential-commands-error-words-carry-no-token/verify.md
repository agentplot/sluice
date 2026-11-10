# Verify: a-credential-commands-error-words-carry-no-token

Verified 2026-11-10 on `change/a-credential-commands-error-words-carry-no-token`
at `eec263c8`, cut from `bolt/a-credential-commands-error-words-carry-no-token`
(`69bc9934` on `origin`). The work holds. The whole checks pass, and
every requirement and scenario of the `forge-identity` delta is met by
the code and by a test that passed. Every task is ticked, and the specs
the change makes pass strict validation once it is archived.

## The whole checks

`sluice exec bolt/a-credential-commands-error-words-carry-no-token`, the
bolt's gate `#check-fast` (`.config/sluice.toml`), run once with sluice
0.78.1:

```
running the checks eec263c affects against bolt/a-credential-commands-error-words-carry-no-token (69bc993)
waiting: the host is at its bound
...
passed: 28 tasks
recorded 28 results under their keys for eec263c
```

Exit 0. The tasks were `cli:test`, `conformance:test`, `core:test`,
`fakes:test`, `seams:test`, `nix:test`, `nix:typecheck`,
`plugin-board:test`, `plugin-board:validate`, `plugin-herdr:test-dash`,
`plugin-herdr:test-sh`, `plugin-sluice:hooks`, `plugin-sluice:lay-out`,
`plugin-sluice:validate`, `root:adrs`, `root:changes`, `root:format`,
`root:lint`, `root:marketplace`, `root:openspec-claude`, `root:specs`,
`root:toolchain`, `site:prose` and `site:rules`, among the 28. Every
`cli:test` binary passed, with none failing. `site:prose` gave 0 errors,
0 warnings and 37 suggestions in 19 files.

One kind of warning appears in the run's output. It comes from neither
this change nor a failing check. moon could not write its shared store
under the host's moon home (`Failed to create
.../sluice.git/moon-home/cache/shared/blobs/temp/...`). It also could
not archive a task's outputs in the place's `.moon/cache`. That is the
sandbox not yet granting a run the store its check profile reads
(record 0105, rule core.35). This tree does not build that grant yet.
The results were recorded under their keys all the same.

The bolt's gate does not run the `tools` feature, so task 3.3's test
was run once by name, as the task says:
`sluice exec bolt/a-credential-commands-error-words-carry-no-token --
cargo test -p sluice --features tools --test forge
forge_check_tells_a_commands_words_without_its_token`, which gave `1
passed; 0 failed`, exit 0.

## The specs once archived

`root:specs` reads `openspec/specs/` alone. The gate can't see what
this change makes of them until it is archived. They were checked in a
copy of `openspec/` in a scratch directory, with the devenv's pinned
OpenSpec 1.14.3:

- `openspec validate a-credential-commands-error-words-carry-no-token
  --strict --no-interactive`: valid.
- `openspec archive a-credential-commands-error-words-carry-no-token
  --yes`: `forge-identity` + 3 added, nothing modified, removed or
  renamed.
- `openspec validate --specs --strict --no-interactive`: 74 passed, 0
  failed.

The three added requirements sit beside "A command that gives no token
fails the call", which still asks that the failure "give its own words
from its standard error" and "never hold a token". The additions narrow
how. They contradict nothing there.

## Requirements and scenarios

### A credential command's words carry no token

`CredentialCommand::get` (`crates/sluice/src/identity.rs`) builds
`words` from the command's standard error. It passes them through
`crate::scrub::Scrub::shapes().scrub(...)` before the `failed` closure
makes any failure, so every failure `get` returns carries the scrubbed
words. `App::get_token`, `App::renewed` and `check::commanded` pass the
failure on unchanged, so a report, a log line, kept state and a status
made from it carry the same words (design D1).

- **The command's words hold a token.** Met by
  `identity::tests::a_credential_commands_words_carry_no_token`, first
  case. The script exits 1 writing `the cached
  ghs_0123456789abcdefghijABCDEFGHIJ012345 was refused`. The test
  asserts the failure starts `[forge] credential `, names the program
  and `exited 1`, carries `the cached [redacted] was refused` and holds
  no `0123456789abcdefghij`. Passed in `cli:test`.
- **A command that prints no password names a token.** Met by the same
  test's second case. The script prints only `username=x-access-token`,
  writes `github_pat_0123456789abcdefghij_ABCDEFGHIJ` on its standard
  error and exits 0. The test asserts `gave no password`, `[redacted]`
  and no `0123456789abcdefghij`. Passed in `cli:test`.
- **forge check tells the command's words without its token.** Met by
  `forge_check_tells_a_commands_words_without_its_token`
  (`crates/sluice/tests/profiles/forge.rs`, `tools` feature). `sluice
  forge check main` exits 1, says `the cached [redacted] was refused`
  and holds no `0123456789abcdefghij`. With `--json` it exits 1 and
  holds none of it either. Passed by name, above.

### sluice reads a GitHub token by the module's prefixes

`crates/sluice/src/scrub.rs` has `pub const GITHUB_TOKEN_PREFIXES: [&str;
6]` set to `ghs_`, `ghp_`, `gho_`, `ghu_`, `ghr_` and `github_pat_`, in
the module's order. `github_token()` builds
`(?:<escaped prefixes>)[A-Za-z0-9_-]{16,}` from that list, and
`Scrub::new` compiles it before the other shapes, which are unchanged.
The old `gh[pousr]_…{20,}` and `github_pat_…{20,}` shapes are gone.
`Scrub::shapes()` is `Scrub::new` of no variables, so it scrubs every
shape and no variable's value (design D2).

- **Each prefix's token redacted.** Met by
  `scrub::tests::each_github_token_prefix_is_redacted`. For each prefix,
  `before <prefix>0123456789abcdef after` scrubs to `before [redacted]
  after`. `ghp_0123456789abcdef_ghij-klm` scrubs to `[redacted]` whole.
  Passed in `cli:test`.
- **A prefix with no token after it.** Met by the same test.
  `<prefix>0123456789abcde`, fifteen characters, is left as written for
  each prefix, and so is `expected a ghs_ token`. Passed in `cli:test`.
- **The module's list and sluice's differ.** Met by
  `scrub::tests::the_github_token_prefixes_are_the_modules`. It reads
  `nix/modules/options.nix` with `include_str!` and takes the quoted
  strings between `prefixes = [` and the next `];`. It compares them
  with `GITHUB_TOKEN_PREFIXES` as sets, and its failure names both
  lists. `crates/sluice/moon.yml` adds `/nix/modules/options.nix` to
  `test`'s inputs, so a change to the module's list alone runs the test
  again. The module's list today, lines 87 to 94, is the same six.
  Passed in `cli:test`.

The scrub's other users only gain redaction. The existing telemetry and
pull request scrub tests passed in `cli:test`. Every GitHub-shaped
fixture in them, such as
`ghp_0123456789abcdefghijABCDEFGHIJ012345` and `ghs_fakeInstallationToken0123456789`,
matched both the old shape and the new.

### A credential command's words cut at their limit end at a whole line

`drain` now returns `Kept { bytes, cut }`. `cut` holds where the kept
part fills `OUTPUT_LIMIT` and the rest of the pipe gave anything but a
clean end of 0 bytes. Where `err.cut` is set, `get` takes the kept text
up to its last `\n`, or nothing where it holds none, then trims and
scrubs it. The standard output is read as before, and `drain`'s and
`OUTPUT_LIMIT`'s docs say so (design D3).

- **A token across the limit.** Met by
  `identity::tests::a_credential_commands_words_cut_at_their_limit_end_at_a_whole_line`.
  The script writes 65,520 `x`, then
  `\nghs_0123456789abcdefghijABCDEFGHIJ012345\n`, on its standard error
  and exits 1. The kept 64 KiB ends with `ghs_0123456789a`. The test
  asserts `exited 1` and the 65,520 `x`, and asserts neither `ghs_` nor
  `0123456789a`. Passed in `cli:test`.

## Tasks

All eight are ticked, and each is done as it says:

- 1.1: record 0108's second amendment, "Amendment, 2026-11-10: a
  command's words carry no token". It records D1 to D3, each with the
  paths not taken. Rule core.28's details gain the "Also rules out" and
  "Not ruled out" design D4 gives them, citing record 0108 as amended
  2026-11-10, and their sources gain ADR 0108. `root:adrs` passed.
- 2.1, 2.2: `scrub.rs` and `crates/sluice/moon.yml` as above. The
  module doc names a credential command's words as the scrub's third
  use.
- 3.1, 3.2: `identity.rs` as above. The module doc and `get`'s doc say
  the words are scrubbed, citing record 0108, and name no forge (rule
  core.1).
- 3.3: `OnGitHub::give_forge_command_running(body)`, with
  `give_forge_command()` calling it with the earlier body, and the new
  `tools` test beside `forge_check_runs_the_hosts_command`.
- 3.4: `site/src/content/docs/reference/configuration.md` says the
  failure gives the command's standard error "with anything shaped as a
  GitHub token replaced by `[redacted]`". Where the command wrote more
  than 64 KiB, the failure "gives its whole lines within the first 64
  KiB". It names no host or tool of ours. `site:prose` passed with no
  warning.

The commits touch only the paths the proposal's Touches names.

## Notes

- In `a_credential_commands_words_carry_no_token`, the two scripts'
  string literals hold real line breaks and a `\` line continuation
  where the other tests in the module write `\n`. The text the script
  gets is the same, `cat > /dev/null\necho … >&2\nexit 1\n`, and the
  test passes. It is a matter of style, and it fails nothing.
