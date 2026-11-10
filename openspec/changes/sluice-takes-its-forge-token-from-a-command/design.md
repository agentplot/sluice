## Context

See proposal.md for why. Record 0108, the statement's clause 134 and
C.8 as amended on 2026-11-09 (agentplot/blueprints 60c25ef), and rules
core.29 and core.51 say what holds. This design says how sluice builds
it on this change's base.

**The family.** The runner and sluice do one thing here, and git's
credential-helper protocol is the one design between them.
agentplot/boomdrive record 0007 as amended on 2026-11-09, and its rule
core.33, make `boomdrive forge-token` speak that protocol and give a
token only on its standard output. Record 0108 makes sluice run the
command the host names in that protocol. The runner owns the server
side. sluice owns a client that names no tool and no forge (rules core.1
and core.26), so any host's secrets manager can stand where the runner
does. sluice already speaks the protocol's other end to its own git
calls, through `sluice credential` (`identity::helper`). The client
lives beside it in `identity.rs`, so one module holds both of sluice's
ends of git's protocol. No other tool of the family takes a forge token
from a command: crew and the agents keep the person's credentials
(record 0007 as amended). Nothing else needs reconciling.

What the code has today:

- `Instance` (`crates/sluice/src/instance.rs`) reads `[forge]` through
  `ForgeForm { app_id, key, discovery }` into
  `Forge { app_id, key: PathBuf, discovery }`. `private_key` expands `~/`
  and refuses a key file others may read. `Instance::guarding_from`
  reads `[forge]` through `GuardingForgeForm { app_id, key }` for branch
  protection's hook (124; record 0059). `Instance::changed_tables`
  compares `forge` whole.
- `github::App` (`crates/sluice/src/profiles/github/app.rs`) holds the
  key as an `EncodingKey`. `App::open` reads it from `forge.key`.
  `App::jwt` signs a JWT. `App::installation` finds the installation by
  JWT, and `App::buy` buys a token for one repository by JWT. That token
  is kept in `tokens: RefCell<BTreeMap<Repo, Token>>` until five minutes
  before its end (`REFRESH`). `App::issue(repo, fresh)` hands one over,
  a fresh one once it is five seconds old (`SETTLE`), with `about`
  saying what it is (`scope`). `App::answer` runs every API call under
  the retry policy. Its `Bearer::Repo` call on a renewing try takes
  `issue(repo, true)`, and a 401 on the repository's token is
  `Judged::Credential`.
- `impl Identity for App` (`crates/sluice/src/profiles/github/mod.rs`)
  gives sluice's git calls `Credentials { username: "x-access-token",
  password, about }` from `issue`. `PushedRefs::call` gathers the
  `about` of each credential the remote refused into the report of a
  refusal that stands.
- `App::allowance` keys a pause by
  `Allowance { app, installation, resource }`. It reads the installation
  from the token held, so it is `None` before a token is bought. `Kept`
  (`kept.rs`) writes the pause at
  `pauses/<app>-<installation>-<resource>.json`.
- `Forge::login` (`forge.rs`) is the App's bot. `impl Forge for App`
  (`rest.rs`) reads it from `App::slug`, which is `GET /app` with the
  JWT. Its one caller is `pulls.rs`'s retitling of a request sluice
  opened (C.5 pull request, 120).
- `github::check::check` (`check.rs`) reports the token minting, the
  installation's four permissions from `App::installation`, and each
  ruleset's bypass actors. `profiles::forge_check` (`profiles/mod.rs`)
  opens the App and says `[forge] key <path> reads` first.
- The flake's module (`nix/modules/options.nix`) declares `forge.app_id`,
  `forge.key` and `forge.discovery`. It asserts that a key is a path
  outside the store (`outside`). `nix/tests/file.nix` holds the
  module's cases, and `nix/tests/settings.nix` holds their settings.

**What GitHub answers a token alone.** These were read on 2026-11-09
with an installation token of this host's own sluice App on
agentplot/sluice. The calls were reads, and each token was revoked
after:

- `GET /app`, and `GET /repos/<owner>/<name>/installation`: `401 A
  JSON web token could not be decoded`. Only the key reaches the App's
  record and the installation's id and grants.
- GraphQL `{ viewer { login } }`: `madswan-agentplot[bot]`, the App's
  bot.
- `GET /apps/madswan-agentplot`: the App's `id`, 5324750.
- `GET /repos/<owner>/<name>` and `GET /installation/repositories`: the
  repository, with every one of `permissions` false. GraphQL's
  `repository { viewerPermission }`: `null`. A token's grants are
  shown to nobody holding only the token.
- `GET https://github.com/<owner>/<name>.git/info/refs?service=git-receive-pack`
  with the token as git's password: 200 for a token minted with
  `contents: write`, and 403 for one minted with `contents: read`.

## Goals / Non-Goals

**Goals:**

- A host's `[forge]` names a credential command in place of a key, and
  everything sluice does on GitHub works with the command's token as it
  does with a minted one.
- A token refused is erased and got afresh, never used again.
- A refusal that stands names a command's token as clause 134 asks.
- A spent allowance holds every process on the host, whichever way its
  token came.
- `sluice forge check` verifies what record 0108 names.

**Non-Goals:**

- The runner's `forge-token`, a Boomdrive host's settings, the App's
  other grants under a command, a cloud host's token, and several Apps.
  See the proposal's Out of scope.

## Decisions

### D1. `[forge]`'s source: a key or a command

`Forge` holds `source: Source` in place of `key`, where
`enum Source { Key(PathBuf), Credential(Command) }` and
`Command { program: PathBuf, args: Vec<String> }`. Both derive `Clone`,
`Debug`, `PartialEq` and `Eq`, so `changed_tables` still names
`[forge]` when the source changes. `Display` of a command is its
program and arguments, space-separated.

`ForgeForm` takes `key: Option<String>` and
`credential: Option<toml::Value>`. Reading the value whole lets sluice
refuse a string in its own words, not serde's. `forge()` refuses each of
these, naming the setting:

- both named: `[forge] names both key and credential: name one, the
  app's private key or the command that gives its token`;
- neither named: `[forge] names neither key, the app's private key, nor
  credential, a command that gives its token: name one`;
- a string: `[forge] credential is a string: name the command as a list
  of its program and then each argument, which sluice runs with no
  shell, credential = ["<program>", "<argument>"]`;
- an empty list, or an element that is no string: `[forge] credential
  names no program` or `… is no list of strings`;
- a program that holds a `/` and neither begins `/` nor `~/`: `[forge]
  credential's program <p> is a relative path: name it absolutely,
  under ~/, or by a name the PATH finds`.

`~/` in the program is expanded as a key's path is. The arguments are
taken as written. sluice does not judge whether an argument is a
token's text. That is the module's guard, where the file would land in
the store (D9). `GuardingForgeForm` takes the same two fields, through
the same `forge()`, so the hook reads a command as the rounds do.

Not taken: a shell string run by `sh -c`, which ADR 0108 and rule
core.51 rule out. Not taken either: reading `credential` straight into
`Vec<String>`, whose refusal of a string would be serde's words.

### D2. The client: `identity::CredentialCommand`

`crates/sluice/src/identity.rs` gains `CredentialCommand`, git's
credential-helper protocol from the asking side. It names no forge.

- `CredentialCommand::new(program, args)` has a limit of thirty seconds,
  `COMMAND_LIMIT`, the same as an API call's (`http_config`).
  `limited(self, Duration)` sets another limit, for tests.
- `get(&self, asked: &Asked) -> Result<Given>` runs the program with its
  arguments and then `get`, with no shell, in its own process group. It
  writes `protocol=<p>\nhost=<h>\npath=<path>\n\n` on its standard input
  and closes it. `Asked { protocol, host, path }` names the request.
  Standard output and standard error are each read on a thread of their
  own, up to 64 KiB each. Output lines are read as `key=value`, up to
  the end or a blank line. `password` is the token. `password_expiry_utc`
  is its end, in seconds since the epoch. An end that does not read is
  none, logged once at warn. Every other key is passed over, as git
  passes them over. `Given { password, ends: Option<DateTime<Utc>> }`.
- The command fails `get` when it exits non-zero, when it prints no
  `password`, or when it gives nothing within the limit. The failure
  says `[forge] credential <command>`, then what went wrong: `exited
  <code>`, `gave no password`, or `gave nothing in 30s and was stopped`.
  It then gives the command's standard error, trimmed. It never holds
  standard output, since that may hold a token. At the limit, sluice
  kills the command's process group, so a child holding its pipes open
  ends with it.
- `erase(&self, asked: &Asked, username: &str, password: &str)` runs the
  program with `erase`, writing `protocol`, `host`, `path`, `username`
  and `password` as git does. Its exit and output are not read, as git
  reads none, but a failure to start it is logged at warn.

Not taken: running the command through a shell, and reading only
`password`. Without its end, sluice would ask for every call, or hold
a token past its life.

### D3. The App's two ways to a token

`App` holds `mint: Mint` in place of `key`, where
`enum Mint { Key(EncodingKey), Command(CredentialCommand) }`.
`App::new(id, pem, api, now, policy)` stays as it is, for a key.
`App::commanded(id, command, api, now, policy)` makes an App that gets
its tokens from the command. `App::open(forge, policy)` matches
`forge.source`. It never runs the command, so nothing is asked of the
command before a token is needed. `App::command() -> Option<&CredentialCommand>`
tells the two apart for `check` and `forge_check`. Under a command,
`App::jwt` fails with `[forge] names a credential command, not a key`.

`Token` keeps `secret` and `bought`, now the time sluice got it either
way. Its `ends` becomes `Option<DateTime<Utc>>`. `installation:
Option<u64>` is `None` for a command's token. It also gains
`about: String`, built when the token is got: from `scope` for a minted
one, and as D5 gives it for a command's. `App::held(repo)` keeps a token
while its end is more than five minutes off. It keeps one with no end
until it is refused, which is git's own reading of a credential with no
expiry. `App::buy(repo)` becomes `App::get(repo)`. For a key, it buys
as today. For a command, it calls `CredentialCommand::get` with
`protocol=https`, `host=github.com` and `path=<owner>/<name>`, then
reads the token's reach (D5). A token whose end is under five minutes
off serves the call it was got for, and the next call asks again.

Not taken: asking the command for every call when it gives no end,
which would run it once per API call and per git call of a round. Not
taken either: assuming an hour, GitHub's life for an installation
token, which a command handing over an older token would outlive.

### D4. A refused token is erased, then got afresh

`App::issue(repo, true)` under a command first erases the token held
for the repository, if any: `erase` with `username=x-access-token` and
the token as `password`. It drops the token from `tokens`, then gets
one. A token got equal to the one just erased is no fresh token.
`issue` fails with `[forge] credential <command> gave again the token
GitHub refused for <repo>`, and the caller's report says no token could
be got afresh. A fresh token is handed over once it is `SETTLE` old, as
a bought one is. GitHub may refuse a token in its first seconds, and
sluice cannot tell how new a command's token is. A command that fails
fails `issue` with its words. In `App::answer` that is
`Judged::Answered`, as a key that cannot sign is, so no call is made
again. Its words for this case say `got` for a command where they say
`bought` for a key.

`PushedRefs::call` and `App::answer` already reach renewal through
`Identity::renew` and `issue(repo, true)`, so neither changes.

### D5. What the forge says of a command's token

When the command gives a token for a repository, `App::get` asks GitHub
with it, once, `GET /installation/repositories?per_page=100`. This is
`App::reach`. The answer is what clause 134 calls what the forge says
of the token, as the bought token's answer gives `scope` for a minted
one:

- It reaches the repository: `about` is `app <id>'s token for <repo>
  from [forge] credential <command>, reaching <names or "every
  repository of <owner>">, got <age>s ago, ending <time>`. Where the
  command gave no end, the tail is `given with no end`.
- The listing is whole (`total_count` no more than it lists), and the
  repository is not in it: the token is refused as `scope` refuses a
  minted one, with `[forge] credential <command> gave a token for <repo>
  that GitHub says reaches <names>, not <repo>`. Nothing is called with
  it.
- GitHub refused the read: its words take the place of the reach, as
  `GitHub refused to say what it reaches: <status> <words>`. The token
  is still handed over. A 401 here is GitHub's answer to this read and
  never renews the token. The call it was got for meets the refusal and
  renews it under the policy (D4).
- The installation's REST allowance is paused: the read is not made,
  and the reach reads `not asked what it reaches: GitHub's allowance is
  spent until <time>`.

The read goes as `Bearer::Given(repo, secret)`, a new bearer that is
the repository's for its allowance (D6) and the given secret's for its
`Authorization`. `App::answer` treats it as `Bearer::Repo` for the
pause and as `Bearer::Token` for a 401. `about` never holds the secret.

Not taken: reading the reach only when a refusal stands, which would
change `Identity` so that `PushedRefs::call` could ask about each
refused credential afterwards. Not taken either: naming only the
command, which leaves "what the forge says of the token" as the
refusal's own words, already given.

### D6. A pause is kept for the installation's account

`Allowance` becomes `{ app: u64, owner: String, resource: String }`, and
its file `pauses/<app>-<owner>-<resource>.json`. An app has at most one
installation on an account, so the owner names the installation on
every repository of it. That is the one name a command's token can
give: GitHub answers the installation's id to the JWT alone. One keying
holds both ways, so a process on a key and one on a command honour each
other's pause. `App::allowance` reads the owner from the repository, so
a pause is known before any token is got, and the reach read (D5) and
the command's first call honour it too. `Kept`'s module doc and the
state reference name the new file.

Not taken: the owner for a command's token and the installation id for
a minted one, two keys for one allowance. Not taken either: no pause for
a command's token, which rule core.29 rules out.

### D7. The bot is read from the token under a command

`Forge::login(&self, repo: &Repo)` takes the repository its token is
for. `pulls.rs` passes the request's repository, and `FileForge`
ignores it. `impl Forge for App` keeps `GET /app` with the JWT under a
key. Under a command it posts `{ viewer { login } }` to `/graphql` as
`Bearer::Repo(repo)`, a `Call::Read` under GraphQL's allowance, and
reads `data.viewer.login`. Either way the login is kept once read for
the process, in the `slug` cell's place, a `login` cell.

Not taken: `viewer` under a key too, one call path for both. That would
spend a GraphQL point where the JWT spends none, and change a path that
works.

### D8. `sluice forge check` under a command

`check(app, repo, branch, operator)` keeps its key path. Under a
command, the token minting and the permissions are replaced by three
findings, each naming the command as `[forge] credential <command>`:

1. **A token comes.** `app.token(repo)`: `<command> gives a token for
   <repo>`, or a failure in its words, after which nothing more is
   checked, as today.
2. **It is the app's.** `App::app_of_token(repo)`: `viewer`'s login,
   its `[bot]` taken off, is the slug. `GET /apps/<slug>` as `Bearer::Repo(repo)` gives its `id`.
   Equal to `app.id()`: `the token is app <id>'s`. Otherwise: `the
   token <command> gives is app <other>'s (<slug>), not app <id>'s:
   name in [forge] app_id the app whose token it gives`. A ruleset's
   bypass is checked against `app_id` (record 0108's reason for keeping
   it), so a token of another App is the one setup that would pass the
   ruleset check and fail every push.
3. **It may push.** `App::may_push(repo)`: `GET
   <git>/<owner>/<name>.git/info/refs?service=git-receive-pack`, with
   `Authorization: Basic` of `x-access-token` and the token, under the
   retry policy as a read. This is the advertisement git's push asks
   for first. 200 gives `the token may push to <repo>`. 401, 403 or 404
   gives `the token may not push to <repo>: GitHub answered git's push
   <status>: give the app Contents read and write`. `<git>` is
   `https://github.com`. `App::open` takes it from `SLUICE_FORGE_API`
   where that is set, since only a test's fake sets it and the fake
   answers both. `App::serving_git(base)` sets it for tests, as
   `resolving` sets the resolver.

The rulesets are then checked as today. `profiles::forge_check` says
`[forge] key <path> reads` only under a key. Under a command, the first
finding does that work. The workflows, checks and pull requests grants
are not reported under a command. GitHub shows them to no holder of the
token alone, and the configuration reference says so.

Not taken: `git push --dry-run` through the pushed refs profile, which
needs a revision of the branch on the host and a refspec chosen to push
nothing. Not taken either: reporting the three other grants as unread
findings, which would be lines that say nothing about the host.

### D9. The module

`options.nix` declares `forge.credential = unset (types.listOf types.str)
"…"`, so a string fails the option's type, naming it. A new assertion
fails evaluation where an element begins `ghs_`, `ghp_`, `gho_`, `ghu_`,
`ghr_` or `github_pat_`, naming `services.sluice.settings.forge.credential`.
It says the file is in the Nix store, which every user reads, and to
name a command that prints the token. A program in the store passes,
since a program is no secret. Both `key` and `credential` set pass to
the file for sluice to refuse at its read (D1). Rule core.51 has
sluice refuse a value it refuses, not the module.
`nix/tests/files/credential.toml` is the file the "A forge credential
command" case renders, and `the_modules_credential_file_is_read_as_written`
reads it.

### D10. Pages and the record

The configuration reference's `[forge]` names `credential` beside
`key`, with git's protocol, the `get` and `erase` sluice sends, and what
it reads. A token's end and a token with none are held as D3 gives
them. A command that fails, fails as D2 gives it. What `forge check`
reads under a command is D8's. Its example uses a placeholder program,
never the runner's (rules core.1 and core.26). The guide to sluice's
GitHub identity offers the command in place of steps 3 and 4. The
commands reference shows `forge check`'s lines under a command, the
state reference the pause's new file, and the setup skill the command
in its `[forge]` row and its module line. Record 0108 gets an amendment,
dated the day it is written, recording D1 to D9, each with the path not
taken.

## Risks / Trade-offs

- **A pause kept under the installation id before this change is not
  read after it.** A process of the new sluice meeting a spent
  allowance in the pause's remaining minutes makes one refused call,
  then keeps the pause under the owner. A pause lasts at most until
  GitHub's reset, within the hour.
- **One more read per token under a command.** D5's reach read is
  counted against the installation's REST allowance, once per token got,
  about once an hour per repository and process. It replaces the two
  JWT calls a minted token costs, which no installation allowance
  counts.
- **A token with no end is held until refused.** The first call after
  it ends is refused once and made again on a fresh one, under the
  policy.
- **The App's other grants are unchecked under a command.** A missing
  Checks or Pull requests grant shows only as a refused post. The
  reference says to check them where the App's key is kept.
- **A command that hangs holds a round for up to thirty seconds**, as an
  API call that hangs does.
- **An agent may run the command,** as it may read a key today. The
  ruleset holds because agents' git never asks sluice for the token
  (record 0108; agentplot/boomdrive record 0007 as amended).

## Migration Plan

Nothing on a host changes until its `[forge]` names `credential`. A
host on a key keeps working, its pauses keyed afresh (D6). A host moves
by replacing `key` with `credential`, by hand or where its configuration
is managed. A `sluice run` kept running takes the change up at its next
pass (rule core.54), and `sluice forge check` verifies it.

## Open Questions

None.
