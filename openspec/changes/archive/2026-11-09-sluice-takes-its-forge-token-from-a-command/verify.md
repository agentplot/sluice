# Verify: sluice-takes-its-forge-token-from-a-command

Verified on 2026-11-09, on `change/sluice-takes-its-forge-token-from-a-command`
at `aa430c5e`, cut from `bolt/sluice-takes-its-forge-token-from-a-command`
(`b51d6ad`), with sluice 0.73.1.

**Outcome: archived.** The whole checks pass, every task is done, the
code meets every requirement and scenario of the three delta specs, and
the specs the archive writes pass `root:specs`.

## The whole checks

`sluice exec bolt/sluice-takes-its-forge-token-from-a-command`, the bolt's
gate, `#check-fast` (`.config/sluice.toml`), run once:

- exit 0, `passed: 10 tasks`, `recorded 10 results under their keys for
  aa430c5`;
- tasks: `root:format` (375 files, 0 changed), `root:lint`,
  `root:changes` (10 passed, 0 failed), `root:adrs` (106 records),
  `nix:lint`, `nix:typecheck` and `nix:test` ("every case holds"),
  `site:prose` (0 errors, 0 warnings, 37 suggestions in 19 files),
  `plugin-sluice:validate` and `cli:test` (648 tests passed, 0 failed);
- `openspec validate sluice-takes-its-forge-token-from-a-command --strict`:
  valid.

`root:specs` is not among the ten, since nothing under `openspec/specs/`
changed before the archive. In a copy of `openspec/` in a scratch
directory, `openspec archive sluice-takes-its-forge-token-from-a-command
--yes` applied 15 added and 5 modified requirements, and then `openspec
validate --specs --strict`, the command `root:specs` runs, passed 74 of 74.
Each requirement the archive modifies keeps every scenario the spec held.

The code is unchanged since `dddb7d92`, whose run of the same gate passed
with the same ten tasks and 648 tests. `forge_check_runs_the_hosts_command`
is under the `tools` feature, which `#check-fast` doesn't run
(`#check-tools` is main's); task 5.3's own command, `sluice exec
bolt/sluice-takes-its-forge-token-from-a-command -- cargo test -p sluice
--features tools --test forge forge_check_runs_the_hosts_command`, passed
at `dddb7d92`: 1 passed, 0 failed.

## Tasks

Every task in `tasks.md` is ticked, 29 of 29, and each one's "true when"
holds:

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
| 6.6 | Every requirement the delta modifies or adds is under 500 characters, one behaviour each, each scenario under the requirement it tests; `openspec validate --strict` passes the change, and the archive's specs pass 74 of 74, as above. |

## Requirements and scenarios

### forge-identity

Modified:

- **The host names sluice's app.** `app_id` with `key` or `credential`.
  "A key others can read" and "No forge table" are unchanged and their
  tests pass.
- **sluice's git calls carry the app's token.** A minted token or the
  command's, held in memory in `App::tokens` alone. "A landing pushed as
  the app" and "Nothing left behind" are unchanged and their tests pass.
  `Given`'s and `Token`'s descriptions name no token's text.
- **A refusal that stands names each refused token.** "Refused twice" and
  "No fresh token can be bought" are unchanged and their tests pass.
- **forge check verifies the setup.** The rulesets for either way, the
  key's findings under a key. "A ruleset that lets the operator
  through", "All set" and "No permission to post checks" are unchanged
  and their tests pass.

Added:

- **A forge credential is a command written as a list.** "A command in
  place of a key" is `the_forge_table_names_a_key_or_a_command` and, for
  branch protection, `guarding_reads_a_forge_credential`. "A command
  written as a shell line" is the string case of the first. The empty
  list, the relative program and a list with a non-string are refused in
  the same test.
- **A forge table names a key or a command, never both.** "Both a key and
  a command" is in `the_forge_table_names_a_key_or_a_command` and
  `guarding_reads_a_forge_credential`. A table naming neither is refused
  there too, naming both.
- **The credential command is asked as git asks a helper.** "The command
  asked as a credential helper" is
  `a_credential_command_is_asked_as_git_asks_a_helper`.
- **A command's token is named by its command.** "A command's token
  described" is `a_commands_credentials_say_what_their_token_is`. The
  description names app, repository, command, reach, age and end, and
  never the token's text.
- **forge check verifies a credential command.** "A command, all set" is
  `forge_check_with_a_command_all_set` and, from the command line,
  `forge_check_runs_the_hosts_command`. "A command that gives another
  app's token, one that may not push" is
  `forge_check_with_a_command_names_each_failure`. Push is read from the
  receive-pack advertisement (`App::may_push`).
- **A command's token is held until five minutes before its end.** "A
  token got once an hour" is `a_commands_token_is_got_once_before_each_end`:
  `REFRESH` is five minutes, so renewal falls at 55 and 115 minutes. "A
  token with no end" is `a_commands_token_with_no_end_is_held_until_refused`.
- **A command's token refused is got afresh from the command.** "Refused,
  erased and got afresh" and "The command gives the refused token back"
  are `a_call_refused_a_commands_token_is_made_on_one_got_afresh` and
  `a_command_that_gives_the_refused_token_back_gives_none_afresh`.
- **A command that gives no token fails the call.** "The command
  refuses", "The command prints no password" and "The command hangs" are
  `a_credential_command_that_gives_no_token_says_why`, and at the call
  `a_command_that_gives_no_token_fails_the_call`. `erase` reads nothing
  back (`erase_tells_the_command_the_refused_token`).
- **A command's token is asked what it reaches.** "A token for another
  repository" is `a_commands_token_for_another_repository_is_refused`.
- **The app's bot is read from the token under a command.** "The bot under
  a command" is `the_bot_under_a_command_is_the_viewers`.

### forge-allowance

Added:

- **The pause names the installation by its account.** "A command's token
  honours the pause" is `a_commands_token_honours_the_pause`, with
  `a_pause_is_kept_under_the_owner` for the file's name. "Every process
  on the host honours the pause" is left as the spec holds it, and its
  scenarios' tests pass on the owner-keyed pause.

### host-settings-module

Modified:

- **The settings are the host's own file under its own names.** "Every
  key the module declares" renders as `nix/tests/files/every.toml`, with
  `forge.credential` set apart; "A key the module does not declare"
  holds (`nix:test`).

Added:

- **The module renders the forge's credential command.** "A forge
  credential command" renders as `nix/tests/files/credential.toml`
  (`nix:test`).
- **sluice reads the module's credential command as written.** "Reading
  the module's credential command" is
  `the_modules_credential_file_is_read_as_written`.
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

The gate's run printed twenty moon warnings: ten `Failed to store blobs`
under the shared store, `sluice.git/moon-home/cache/shared/blobs/temp`,
and ten `Failed to package task outputs` under `.moon/cache/outputs`,
which links to `sluice.git/moon-outputs`. So the installed sluice 0.73.1's
sandbox looks to refuse a run's writes to the store the repository's
`devenv.nix` names as `MOON_HOME`, the case rule core.35 rules out (record
0105). Nothing in this change touches it, and no check failed by it. It is
the host's sluice to look at, not this change's.
