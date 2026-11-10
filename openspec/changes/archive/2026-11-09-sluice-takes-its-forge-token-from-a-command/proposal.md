## Why

sluice lands as a GitHub App. Today the host's `[forge]` names the App's
id and the path of its private key, and sluice signs a JWT with the key
to buy an installation token for each repository (record 0019). sluice
has no way to take a token it did not mint.

On a Boomdrive host, the user chose on 2026-11-09 one App for the
family, whose key the runner alone holds, or the cloud on a cloud host
(agentplot/boomdrive record 0007 as amended that day, its rule core.33).
Every tool there asks the runner for its token by a credential command.
Record 0108, accepted the same day, says how sluice takes one: `[forge]`
may name a credential command in place of a key, run as git runs a
credential helper. The statement's clause 134 and C.8's row of who holds
the branch's credential were amended with it (agentplot/blueprints
60c25ef), and rule core.29's and core.51's details read with it. This
change builds it.

## What Changes

- **`[forge] credential` names a command.** The table takes `app_id`
  with either `key` or `credential`, never both and never neither.
  `credential` is a list, the program and then each argument, run with
  no shell. A string, an empty list, or a program written as a relative
  path is refused, naming the setting. Branch protection's reading of
  the host's settings takes `credential` as it takes `key`.
- **sluice speaks git's credential-helper protocol to it.** For a token
  it runs the command with `get`, writing `protocol=https`,
  `host=github.com` and `path=<owner>/<name>` on its standard input, and
  reads `password` and `password_expiry_utc` from its standard output.
  It holds the token in memory for that repository alone, as it holds
  one it minted, and asks again five minutes before the end the command
  gave. It holds a token given with no end until GitHub refuses it.
- **A token GitHub refuses is got afresh from the command.** Wherever
  sluice would buy a fresh token, it runs the command with `erase` for
  the refused token, then with `get`. It never tries again with the
  refused token, and a `get` that gives the refused token back is no
  fresh token (134; rule core.29).
- **A command that gives no token fails the call** as a key that cannot
  sign does. That covers a command that exits non-zero, prints no
  `password`, or gives nothing in thirty seconds. The failure names the
  command and its own words, and never a token.
- **What a command's token is, for a refusal that stands.** When sluice
  gets a token from the command, it asks GitHub with it, once, which
  repositories it reaches. A refused token is then named by its app, its
  repository, the command, what GitHub says it reaches, its age and its
  end (134 as amended). A token that GitHub says does not reach the
  repository is refused, as a minted one is.
- **The App's bot is read from the token.** Under a command, sluice
  reads the login of the App's bot from GraphQL's `viewer` with the
  repository's token, once in a process, since `GET /app` takes the JWT
  only a key can sign. That login tells a pull request sluice opened from
  one a person opened (C.5 pull request, 120).
- **A spent allowance is kept for the installation's account.** The
  pause is written as `pauses/<app>-<owner>-<resource>.json`, the owner
  of the repository standing for the App's one installation on that
  account, whichever way the token came. A command's token carries no
  installation id, and GitHub gives that id to the JWT alone (134; rule
  core.29).
- **`sluice forge check` verifies a command** as record 0108 says. The
  command gives a token for the repository. The token is the named App's,
  read from `viewer` and `GET /apps/<slug>`. The token may push, read
  from git's receive-pack advertisement. Every ruleset lets only that App
  through. With a key, it checks what it checks today.
- **The flake's module declares `forge.credential`** as a list of
  strings. Evaluation fails where an argument holds a GitHub token's
  text, since the file is in the Nix store that every user reads (rule
  core.51).
- The configuration reference, the guide to sluice's GitHub identity,
  the commands and state references, and the setup skill say how to name
  a command. Record 0108 gets an amendment recording the choices this
  design makes within it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `forge-identity`: `[forge]` names a key or a credential command. The
  command's token is held, refused and got afresh as a minted one is,
  and described for a refusal that stands. `sluice forge check` verifies
  a command.
- `forge-allowance`: the pause is kept for the App's installation, named
  by the account it is on, whichever way the token came.
- `host-settings-module`: the module declares `forge.credential` and
  refuses a token's text in it.

## Out of scope

- **`boomdrive forge-token` itself.** It is the runner's
  (agentplot/boomdrive record 0007 as amended, rule core.33), built in
  that repository. sluice names no tool (rules core.1, core.26). Its
  tests run a command of their own, and its pages show a placeholder.
- **Moving a Boomdrive host's `[forge]` to the command.** That is the
  host's configuration, set where the host's configuration is managed,
  once the runner gives tokens. Boomdrive's set of modules sets
  `services.sluice` for its hosts (agentplot/boomdrive record 0023).
- **The App's other grants under a command.** GitHub shows a token's
  grant of workflows, checks and pull requests to nobody holding only
  the token: `GET /repos/<owner>/<name>` and GraphQL's `viewerPermission`
  answer nothing for an installation token. `sluice forge check` with a
  command checks what record 0108 names: the token, its App, its push,
  and the rulesets. The App's other grants are checked where its key is
  kept.
- **A cloud host's token.** The runner's protocol carries no forge
  token through its version 2 (record 0007 as amended). sluice reads
  whatever the command gives and needs nothing more.
- **Several Apps, or a command per repository.** One `[forge]` names one
  App for the host, as today.

## Impact

- `crates/sluice/src/identity.rs`: `CredentialCommand`, a client of
  git's credential-helper protocol that names no forge, beside the
  helper sluice's git calls run.
- `crates/sluice/src/instance.rs`: `Forge` holds a `Source`, the key's
  path or the command, in place of `key`. `ForgeForm` and
  `GuardingForgeForm` read `credential`.
- `crates/sluice/src/profiles/github/app.rs`: the App mints from a key
  or gets from a command. Its token is held, refused and renewed for
  either. The command's token's reach is read once. A pause is keyed by
  the repository's owner. `login` reads `viewer` under a command, and
  a token's push is read from git's receive-pack advertisement.
- `crates/sluice/src/profiles/github/kept.rs`: `Allowance` names the
  owner in place of the installation id.
- `crates/sluice/src/profiles/github/forge.rs`, `rest.rs`, `fake.rs`,
  `pulls.rs`: `Forge::login` takes the repository.
- `crates/sluice/src/profiles/github/check.rs` and
  `crates/sluice/src/profiles/mod.rs`: `sluice forge check` under a
  command.
- `nix/modules/options.nix`, `nix/tests/file.nix`,
  `nix/tests/settings.nix`, `nix/tests/files/credential.toml`: the
  module's `forge.credential`.
- Tests: `crates/sluice/src/identity.rs`, `crates/sluice/src/instance.rs`
  and `crates/sluice/src/profiles/github/kept.rs` (their own),
  `crates/sluice/tests/profiles/forge.rs`.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `site/src/content/docs/guides/github-app.md`,
  `site/src/content/docs/reference/commands.md`,
  `site/src/content/docs/reference/state.md`,
  `plugins/sluice/skills/setting-up-sluice/SKILL.md`,
  `plugins/sluice/skills/setting-up-sluice/references/profiles/github-app.md`,
  `docs/adr/0108-sluices-forge-token-may-come-from-a-credential-command-the-host-names.md`.
- Sources: clause 134 and C.8 (as amended 2026-11-09, agentplot/blueprints
  60c25ef), clauses 120, 124, 133, 136 and C.5's pull request row;
  record 0108 (accepted 2026-11-09), records 0019, 0059, 0071 and 0098;
  rules core.1, core.26, core.28, core.29, core.32 and core.51;
  agentplot/boomdrive record 0007 as amended 2026-11-09 and its rule
  core.33; git's `gitcredentials(7)` and `git-credential(1)`.

## Touches

```
crates/sluice/src/identity.rs
crates/sluice/src/instance.rs
crates/sluice/src/profiles/mod.rs
crates/sluice/src/profiles/github/mod.rs
crates/sluice/src/profiles/github/app.rs
crates/sluice/src/profiles/github/kept.rs
crates/sluice/src/profiles/github/check.rs
crates/sluice/src/profiles/github/forge.rs
crates/sluice/src/profiles/github/rest.rs
crates/sluice/src/profiles/github/fake.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/tests/profiles/forge.rs
nix/modules/options.nix
nix/tests/file.nix
nix/tests/settings.nix
nix/tests/files/credential.toml
site/src/content/docs/reference/configuration.md
site/src/content/docs/guides/github-app.md
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/state.md
plugins/sluice/skills/setting-up-sluice/SKILL.md
plugins/sluice/skills/setting-up-sluice/references/profiles/github-app.md
docs/adr/0108-sluices-forge-token-may-come-from-a-credential-command-the-host-names.md
openspec/changes/sluice-takes-its-forge-token-from-a-command/**
```
