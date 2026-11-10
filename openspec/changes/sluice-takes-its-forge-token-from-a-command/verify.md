# Verify: sluice-takes-its-forge-token-from-a-command

Verified on 2026-11-09, on `change/sluice-takes-its-forge-token-from-a-command`
at `dddb7d92`, cut from `bolt/sluice-takes-its-forge-token-from-a-command`
(`b51d6ad`), with sluice 0.73.1.

**Outcome: not archived.** The whole checks pass, every task is done, and
the code meets every scenario of the three delta specs. But the specs the
archive would write fail `root:specs`, a `#check-fast` task of the bolt's
gate. Seven requirements this change modifies are over OpenSpec's 500
characters, which `--strict` fails ("What falls short" below). The change
goes back to have them split, one behaviour each, before it is archived.

## What falls short

`openspec archive sluice-takes-its-forge-token-from-a-command --yes`,
applied and then undone in the working tree, gives specs that fail
`openspec validate --specs --strict`, the command `root:specs` runs: 71
passed, 3 failed. At `dddb7d92`, before the archive, 74 passed. `openspec
validate --changes --strict` (`root:changes`) passes the delta itself,
since it doesn't measure a requirement's length. Each failure is "Requirement
text is very long (>500 characters)". Each requirement below is one this
change's delta MODIFIES, made longer by it:

| Spec | Requirement | Characters |
|---|---|---|
| forge-allowance | Every process on the host honours the pause | 698 |
| forge-identity | The host names sluice's app | 723 |
| forge-identity | sluice's git calls carry the app's token | 694 |
| forge-identity | forge check verifies the setup | 713 |
| forge-identity | A refusal that stands names each refused token | 602 |
| host-settings-module | The settings are the host's own file under its own names | 622 |
| host-settings-module | sluice reads the module's file as written | 510 |

The fix is in the delta specs, under
`openspec/changes/sluice-takes-its-forge-token-from-a-command/specs/`. Split
each of these into requirements of one behaviour each, under 500
characters, with each scenario under the requirement it tests. For
example, the command's own refusals of `[forge] credential` could leave
"The host names sluice's app", and `forge check`'s findings under a
command could leave "forge check verifies the setup". The code and its
tests need no change for it. Then `openspec archive` gives specs that pass
`root:specs`.

## The whole checks

`sluice exec bolt/sluice-takes-its-forge-token-from-a-command`, the bolt's
gate, `#check-fast` (`.config/sluice.toml`), run once:

- exit 0, `passed: 10 tasks`, `recorded 10 results under their keys for
  dddb7d9`;
- tasks: `root:format`, `root:lint`, `root:changes`, `root:adrs`,
  `nix:lint`, `nix:typecheck`, `nix:test` ("every case holds"),
  `site:prose` (0 errors, 0 warnings, 37 suggestions in 19 files),
  `plugin-sluice:validate` and `cli:test` (648 tests passed, 0 failed);
- `openspec validate sluice-takes-its-forge-token-from-a-command --strict`:
  valid.

`forge_check_runs_the_hosts_command` is under the `tools` feature, which
`#check-fast` doesn't run (`#check-tools` is main's). Task 5.3's own
command, `sluice exec bolt/sluice-takes-its-forge-token-from-a-command --
cargo test -p sluice --features tools --test forge
forge_check_runs_the_hosts_command`, passed: 1 passed, 0 failed.

## Tasks

Every task in `tasks.md` is ticked, and each one's "true when" holds:

| Task | Evidence |
|---|---|
| 1.1 | `CredentialCommand`, `Asked`, `Given`, `COMMAND_LIMIT`, `new`, `limited`, `get` in `crates/sluice/src/identity.rs`; no shell, own process group, 64 KiB per pipe on its own thread. `a_credential_command_is_asked_as_git_asks_a_helper` ok. |
| 1.2 | `get`'s failures start `[forge] credential <command>`, carry stderr trimmed, never stdout; the group is killed at the limit. `a_credential_command_that_gives_no_token_says_why` ok (three cases). |
| 1.3 | `erase` writes the five keys and reads nothing back. `erase_tells_the_command_the_refused_token` ok. |
| 2.1 | `Allowance { app, owner, resource }`, `pauses/<app>-<owner>-<resource>.json`; `App::allowance` reads the owner from `Bearer::Repo` and `Bearer::Given`. `a_pause_is_kept_under_the_owner` ok. |
| 2.2 | `reference/state.md` names `forge/pauses/<app>-<owner>-<allowance>.json` and the owner standing for the one installation either way; `site:prose` with no warning. |
| 3.1 | `Mint`, `App::commanded`, `App::command`; `Token`'s `ends` and `installation` optional; `jwt` fails under a command. `a_commands_token_is_got_once_before_each_end` ok. |
| 3.2 | `a_commands_token_with_no_end_is_held_until_refused` ok. |
| 3.3 | `Bearer::Given`, `App::reach`: counted against the repository's allowance, skipped while paused, a refusal no renewal. `a_commands_credentials_say_what_their_token_is` ok. |
| 3.4 | `a_commands_token_for_another_repository_is_refused` ok. |
| 3.5 | `App::renewed` erases, drops and gets; a token equal to the erased one fails; `SETTLE` before handing over. `a_call_refused_a_commands_token_is_made_on_one_got_afresh` ok. |
| 3.6 | `a_command_that_gives_the_refused_token_back_gives_none_afresh` ok. |
| 3.7 | `a_command_that_gives_no_token_fails_the_call` ok. |
| 3.8 | The fake GitHub answers `GET /installation/repositories`. `a_commands_token_honours_the_pause` ok. |
| 3.9 | `Forge::login(&Repo)` in `forge.rs`, `rest.rs`, `fake.rs`; `pulls.rs` passes the request's repository; `App::bot` reads `/app` under a key and `viewer` under a command, kept once read. `the_apps_login_is_read_once` and `the_bot_under_a_command_is_the_viewers` ok. |
| 4.1 | `Source`, `Command`; `forge()` refuses in design D1's words, `~/` expanded; `App::open` matches the source; `[forge] key <path> reads` only under a key in `profiles/mod.rs`. `the_forge_table_names_a_key_or_a_command` ok. |
| 4.2 | `GuardingForgeForm` through the same `forge()`. `guarding_reads_a_forge_credential` ok. |
| 4.3 | `reference/configuration.md`'s `[forge]` section, table row, example comment and "Who writes it"; `site:prose` with no warning. |
| 4.4 | `guides/github-app.md` names `credential` in place of `key`, and what a command's refused token names; `site:prose` with no warning. |
| 4.5 | `references/profiles/github-app.md` and `SKILL.md`'s `[forge]` row; `plugin-sluice:validate` ok. |
| 5.1 | `App::may_push`, `App::app_of_token`, `App::serving_git`; `check.rs`'s `commanded` gives D8's three findings, then the rulesets. `forge_check_with_a_command_all_set` ok. |
| 5.2 | `forge_check_with_a_command_names_each_failure` ok. |
| 5.3 | `OnGitHub::give_forge_command`; `forge_check_runs_the_hosts_command` ok, run by its own command above. |
| 5.4 | `reference/commands.md` under `sluice forge check`, with a placeholder example; `site:prose` with no warning. |
| 6.1 | `nix/modules/options.nix` declares `forge.credential` with the token-prefix assertion; `nix/tests/settings.nix`, `nix/tests/files/credential.toml`, and `nix/tests/file.nix`'s three cases; `nix:test` ok. |
| 6.2 | `the_modules_credential_file_is_read_as_written` ok. |
| 6.3 | `reference/configuration.md`'s "Who writes it" names the module's `forge.credential`; `site:prose` with no warning. |
| 6.4 | `SKILL.md` gives `forge.credential` with placeholders; `plugin-sluice:validate` ok. |
| 6.5 | Record 0108's amendment of 2026-11-09 records D1 to D9, each with the path not taken; `root:adrs` ok. |

## Requirements and scenarios

### forge-identity

- **The host names sluice's app.** "A command in place of a key", "Both a
  key and a command" and "A command written as a shell line" are
  `the_forge_table_names_a_key_or_a_command` and
  `guarding_reads_a_forge_credential`. The empty list and the relative
  program are refused in the same test. "A key others can read" and "No
  forge table" are unchanged and their tests pass.
- **sluice's git calls carry the app's token.** "The command asked as a
  credential helper" is `a_credential_command_is_asked_as_git_asks_a_helper`.
  The token is held in `App::tokens` in memory, as a minted one is.
- **A refusal that stands names each refused token.** "A command's token
  described" is `a_commands_credentials_say_what_their_token_is`. The
  `about` names app, repository, command, reach, age and end, and never the
  token's text.
- **forge check verifies the setup.** "A command, all set" is
  `forge_check_with_a_command_all_set` and, from the command line,
  `forge_check_runs_the_hosts_command`. "A command that gives another
  app's token, one that may not push" is
  `forge_check_with_a_command_names_each_failure`. Push is read from the
  receive-pack advertisement (`App::may_push`).
- **A command's token is held until five minutes before its end.** "A
  token got once an hour" is `a_commands_token_is_got_once_before_each_end`.
  `REFRESH` is five minutes and `held` renews once `ends - now` is no more
  than that, so renewal falls at 55 and 115 minutes. "A token with no end"
  is `a_commands_token_with_no_end_is_held_until_refused`.
- **A command's token refused is got afresh from the command.** "Refused,
  erased and got afresh" and "The command gives the refused token back"
  are `a_call_refused_a_commands_token_is_made_on_one_got_afresh` and
  `a_command_that_gives_the_refused_token_back_gives_none_afresh`.
- **A command that gives no token fails the call.** "The command refuses",
  "The command prints no password" and "The command hangs" are
  `a_credential_command_that_gives_no_token_says_why`, and at the call
  `a_command_that_gives_no_token_fails_the_call`. `erase` reads nothing
  back.
- **A command's token is asked what it reaches.** "A token for another
  repository" is `a_commands_token_for_another_repository_is_refused`.
- **The app's bot is read from the token under a command.** "The bot under
  a command" is `the_bot_under_a_command_is_the_viewers`.

### forge-allowance

- **Every process on the host honours the pause.** "A command's token
  honours the pause" is `a_commands_token_honours_the_pause`, with
  `a_pause_is_kept_under_the_owner` for the file's name. The existing
  scenarios' tests pass on the owner-keyed pause.

### host-settings-module

- **The settings are the host's own file under its own names.** "A forge
  credential command" renders as `nix/tests/files/credential.toml`
  (`nix:test`); "Every key the module declares" and "A key the module does
  not declare" hold.
- **sluice reads the module's file as written.** "Reading the module's
  credential command" is `the_modules_credential_file_is_read_as_written`.
- **A forge credential is a list of strings.** "A command written as a
  shell line" is "A credential given as a string" in `nix/tests/file.nix`.
- **A forge credential holds no token's text.** "A token's text in the
  command" is "A token's text in the credential" in `nix/tests/file.nix`.

## Rules

- core.1: the code this change adds outside `profiles/` names no forge or
  tool. `identity.rs`'s client takes what it is asked from the GitHub
  profile, and `github.com` appears there only in its tests' fixtures.
- core.26: the pages, the skill and its reference name
  `/opt/forge/bin/token-helper` or `<program>` as placeholders, and no host
  or tool of ours.
- core.29: a refused token is erased and got afresh, never tried again,
  and a token given back is no fresh one.
- core.51: the module declares `forge.credential` as a list of strings and
  refuses a token's text in it.

## Outside this change

The gate's run printed twenty moon warnings: `Failed to store blobs` under
the shared store, `sluice.git/moon-home/cache/shared/blobs/temp`, and
`Failed to package task outputs` under `.moon/cache/outputs`, which links
to `sluice.git/moon-outputs`. Both directories exist and are the user's.
So the installed sluice 0.73.1's sandbox looks to refuse a run's writes to
the store that the repository's `devenv.nix` names as `MOON_HOME`, the case
rule core.35 rules out (record 0105). Nothing in this change touches it,
and no check failed by it. It is the host's sluice to look at, not this
change's.
