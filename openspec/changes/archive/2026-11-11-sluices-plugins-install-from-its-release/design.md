# Design

## Context

**The brief.** A person installs sluice's plugins from the release they
install sluice from, as Flume's are installed. The Claude Code
marketplace is added as `agentplot/sluice#release`, a release branch the
release job moves to each release commit. The herdr plugin is installed
with `--ref v<version>`. The README, the site's guides, the plugins'
READMEs and the setup skill say so. A plugin then always matches the
sluice it calls, and its version is the release's. The herdr watcher's
replace-on-new-release then works for everyone. Main stays the
development tree and is never an install source.

**Sources.**
- Batch 50's report,
  `~/.local/state/crew/agentplot-main/reports/batch-agentplot-50.md`.
  With record 0111, a release's version lives only on its release
  commit, off main. So a marketplace or herdr plugin installed from main
  always says `0.0.0`. The report left open two items: "installing
  sluice, its marketplace or its herdr plugin from a release tag rather
  than main", and "a release branch for the marketplace as Flume has".
- Flume at agentplot/flume origin/main e2973d57, read 2026-11-11:
  - `README.md` and `site/src/content/docs/install.mdx`:
    `/plugin marketplace add agentplot/flume#release`. The install page
    says: "The marketplace is read at the `release` branch, which each
    release moves to the commit its tag names". It then gives
    `/plugin marketplace update flume` and `/plugin update flume@flume`.
  - `plugins/herdr/README.md`:
    `herdr plugin install agentplot/flume/plugins/herdr --ref v<version>`,
    and "Install it at the tag of the flume you run, so the plugin and the
    binary are one version."
  - `.github/workflows/ci.yml`'s `release` job:
    `git push --atomic origin "refs/tags/$TAG" "+HEAD:refs/heads/release"`.
    `origin/release` is 65911c26, `chore(release): v0.21.2`, whose parent
    is main's e2973d57.
  - Its install page takes the flake at `github:agentplot/flume/v<version>`.
- GitHub, read 2026-11-11 with `gh api`:
  - agentplot/sluice's rulesets are `main: only sluice` (refs/heads/main:
    update, non-fast-forward, deletion) and `bolt/*: only sluice`. None
    covers `release`, and no `release` branch exists yet.
  - The latest release, v0.89.1, names `chore(release): v0.89.1`, whose
    one parent is 49fcf7d3. It is the first release made under record
    0111.
- herdr 0.9.5's `herdr plugin install --help` gives `--ref <REF>`.
  herdr v0.9.2's notes say "`plugin install` accepts options before the
  repository", so `--ref` predates the plugin's floor of herdr 0.9.3.
- agentplot/timber origin/main also installs from main:
  `plugins/claude-code/README.md:9` and `plugins/herdr/README.md:8`.

**sluice today, read on this branch at 49fcf7d3.**
- The install commands that name main:
  - The marketplace: `README.md:35`, `plugins/sluice/README.md:7`,
    `plugins/sluice-board/README.md:11`,
    `site/src/content/docs/guides/claude-code.md:18`,
    `site/src/content/docs/index.mdx:33`. Also `/plugin install` lines
    that follow these (`claude-code.md:126` for the board).
  - The herdr plugin: `plugins/herdr/README.md:11`,
    `site/src/content/docs/guides/herdr.md:20`.
  - sluice by nix: `README.md:116` (`nix profile install
    github:agentplot/sluice`) and `README.md:141` (a flake input).
    `site/src/content/docs/index.mdx:120`,
    `site/src/content/docs/guides/walkthrough.md:28`,
    `docs/examples/README.md:20`, and the setup skill's table at
    `plugins/sluice/skills/setting-up-sluice/SKILL.md:24`.
  - sluice by cargo: `README.md:135`, `SKILL.md:26`, `index.mdx:124`,
    `walkthrough.md:32-33` and `docs/examples/README.md:25-26`. The last
    two are wrapped across lines in prose.
  - The release archive: `README.md:126`, `SKILL.md:31` and
    `index.mdx:128` use `releases/latest/download/`, which is a release
    already.
- The setup skill's step 1 says "This plugin needs **sluice 0.12 or
  later**" and proposes an install from main. The board's band says "the
  board needs sluice 0.12 or later" (plugin-board's spec), which this
  change leaves.
- The herdr plugin's README and guide say "Needs herdr 0.9.3 or later,
  `sluice` 0.12 or later and `jq`".
- The `release` job (`.github/workflows/ci.yml:205-234`) runs
  `git push origin "refs/tags/$TAG"`. `check-release.sh` holds that it
  is the job's one push, and that the push is exactly that line.
- Record 0111's decision says the release job "pushes the tag alone,
  with no branch and no force". Rule core.55's details rule out "a tag
  pushed with a branch or by force, moved or deleted".
- The herdr watcher keys its lease by the plugin's version
  (`plugins/herdr/bin/watch.sh`, record 0094). It needs no change: a
  plugin at a release's tag says that release's version.
- The statement has no clause on how sluice or its plugins are
  installed. principles.md has none either.

## Goals / Non-Goals

**Goals:**
- A person who follows sluice's docs gets plugins of a release, and a
  sluice of a release. The setup skill installs the sluice of its own
  plugin's release.
- The marketplace has a ref that moves at each release, so
  `/plugin marketplace update sluice` takes the next release.
- One form across the family, Flume's. No doc of sluice's names main as
  an install source, and a check holds that.

**Non-Goals:**
- Changing the plugins' code, the herdr watcher's lease, or the board's
  supported range.
- The Boomdrive set, which installs from its pinned release already.
- timber's install commands. They are timber's own change, told to the
  orchestrator.

## Decisions

### D1. The release job moves `release` with its tag, in one atomic push

The `release` job's push becomes Flume's:

```sh
git push --atomic origin "refs/tags/$TAG" "+HEAD:refs/heads/release"
```

- `HEAD` is the release commit the job just made and tagged.
- `release` is forced. Each release commit's one parent is main's pushed
  commit, so the release commit before it is not its ancestor.
- The tag is never forced. A tag that exists refuses the push.
- `--atomic` means the tag and `release` move together or not at all.
  So `release` never names a commit no tag names, and a tag is never
  pushed while `release` lags it.
- No ruleset of GitHub's covers `release`, and the job's `GITHUB_TOKEN`
  has `contents: write`. sluice lands on no `release` branch in its own
  policy (`.config/sluice.toml` names `main` and `bolt/*`). So no rule of
  sluice's about the branches it lands on reads `release` (core.4,
  core.20, core.45).
- Nothing is committed to main and no tag moves. core.55's line holds as
  written, and its details take `release` (D6).

The job's comment and the workflow's header say the release branch, which
the marketplace is read at, moves to the release commit with its tag.

Not taken:
- **The marketplace at a tag, `agentplot/sluice#v<version>`.** Claude
  Code would read that ref for good. A person would have to remove and
  add the marketplace again at each release, as no `update` moves it.
- **`release` moved without force, each release commit merging the one
  before it.** The tag's commit would then have two parents, and
  `release`'s history would carry trees no reader wants. Nothing reads
  `release`'s history, only its tip.
- **Two pushes, the tag and then `release`.** A failure between them
  would leave a release that `release` does not name.

### D2. Every install command names a release

Each form, everywhere sluice's docs give one:

| what | form |
|---|---|
| the Claude Code marketplace | `/plugin marketplace add agentplot/sluice#release` |
| the herdr plugin | `herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>` |
| sluice by nix, a profile or a flake input | `github:agentplot/sluice/v<version>` |
| sluice by cargo | `cargo install --locked --git https://github.com/agentplot/sluice --tag v<version> sluice` |
| a release's archive | `releases/latest/download/<archive>`, or `releases/download/v<version>/<archive>` |

- `<version>` is written so, as Flume's docs write it. Where a page
  gives the herdr plugin or a nix install, it says `<version>` is a
  release's: the version `sluice --version` prints for the plugin, and a
  version from the releases page for sluice itself.
- The marketplace is the one install read at a branch, because Claude
  Code needs a ref that moves for its `update`. Every other install names
  a tag, so it reads exactly one release.
- Main is the development tree. A page may say a build of main reports
  `0.0.0`. No page tells a person to install from it.
- The places that give the marketplace say that it is read at
  `release`, which each release moves to the commit its tag names. They
  give the commands to take a new release:
  `/plugin marketplace update sluice`, then
  `/plugin update <plugin>@sluice`.
- The herdr plugin's README and guide say to install it at the tag of
  the sluice the person runs, so the plugin and the binary are one
  release. "sluice 0.12 or later" goes from both. herdr 0.9.3 and `jq`
  stay.

Not taken:
- **The plugins alone, as the brief's list of lines has it.** sluice's
  own installs would stay on main. A plugin of a release would then call
  a sluice built from main, which says `0.0.0` and may differ from the
  release either way. The brief's own rule, that main is never an install
  source, decides it.
- **`github:agentplot/sluice/release` for nix and cargo.** One moving
  name would make `nix profile upgrade` take each release. But a tag
  names the release exactly, as Flume's flake install does. The branch
  stays the marketplace's alone.

### D3. The setup skill installs its own plugin's release

Step 1 of `plugins/sluice/skills/setting-up-sluice/SKILL.md` becomes:

1. Read the plugin's version:
   ```sh
   sed -n 's/^ *"version": *"\([^"]*\)".*/\1/p' "${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json"
   ```
   Then run `command -v sluice && sluice --version`.
2. Where the plugin's version is a release's (not `0.0.0`), compare it
   with sluice's:
   - sluice missing or older: install the plugin's release, `v<version>`.
   - sluice newer: the plugin is behind. Propose
     `/plugin marketplace update sluice` and `/plugin update sluice@sluice`,
     and `sluice-board@sluice` where it is installed. Install no sluice.
   - equal: go on to step 2.
3. Where the plugin says `0.0.0`, it was loaded from a working copy of
   sluice's tree and is no release. Say so. Where sluice is missing or
   older than 0.12, take the newest release, `releases/latest`.

The table keeps its rows, each at the release:

| the host has | install | upgrade |
|---|---|---|
| sluice from a flake input or Home Manager | tell the user to set the input to `github:agentplot/sluice/v<version>` and rebuild; don't edit their config unasked | same |
| nix | `nix profile install github:agentplot/sluice/v<version>` | `nix profile remove sluice` (or the entry's name from `nix profile list`), then the install at the new tag |
| neither, on macOS on Apple silicon or Linux x86-64/ARM | the release archive at `releases/download/v<version>/` | the same at the new tag |
| Rust 1.98+ | `cargo install --locked --git https://github.com/agentplot/sluice --tag v<version> sluice` | the same with `--force` |

`nix profile upgrade` does not move an entry installed from a tag, so the
upgrade removes the entry and installs the new tag. The skill still shows
every command before it runs it and asks first.

The skill's version comparison is the skill's own reading, as its step
already reads `sluice --version`. No script is added for it, so the
plugin's tests (`plugins/sluice:hooks`, `plugins/sluice:lay-out`) are
unchanged, and `root:installs` holds the skill's commands (D5).

The site's Claude Code guide, which tells what the skill does, says it
installs the sluice of the plugin's own release. Where sluice is newer,
it says the skill offers to update the plugin.

### D4. The pages' words

Each place gives the commands in D2's forms:

- **`README.md`**:
  - "Get started", with Claude Code: the marketplace at `#release`, and
    one sentence that each release moves `release`, with the update
    commands.
  - "With herdr" (its paragraph, where it names the herdr plugin).
  - "Install": `nix profile install github:agentplot/sluice/v<version>`,
    with one sentence naming the releases page for `<version>`. Under the
    details, cargo with `--tag v<version>`, and the flake input at
    `github:agentplot/sluice/v<version>`. The sentence that a build of
    main reports `0.0.0` stays and says main is the development tree.
- **`plugins/sluice/README.md`** and **`plugins/sluice-board/README.md`**:
  the marketplace at `#release`, and one line on taking a new release.
- **`plugins/herdr/README.md`**: the install with `--ref v<version>`, and
  Flume's sentence: "Install it at the tag of the sluice you run
  (`sluice --version`), so the plugin and the binary are one release."
- **The site**:
  - `index.mdx`: the marketplace, the nix install and the cargo
    mention.
  - `guides/claude-code.md`: Install, the skill's binary step (D3),
    and the board's install. The board's `/plugin install` needs no
    change, since the marketplace is the one added at `#release`.
  - `guides/herdr.md`: Install, as the herdr README.
  - `guides/walkthrough.md`: the nix install and its cargo aside.
- **`docs/examples/README.md`**: the nix install and its cargo aside.

The words name no host, team or tool of ours (core.26). `release` is the
branch's name in sluice's own repository, which the reader sees in the
command.

### D5. `root:installs` holds every install command to a release

`check-installs.sh`, at the root, is the root project's `installs` task
(check-fast). It spans the README, the plugins and the site, as
`root:marketplace` does (core.21's details). It is POSIX sh, `tr`,
`grep` and `awk`, with nothing installed.

- `sh check-installs.sh [<root>]` reads the tree at `<root>`, by default
  the folder the script is in. With a root given, the verify stage can
  run each failing scenario on a scratch copy.
- It reads `README.md`, `docs/examples/README.md`,
  `plugins/*/README.md`, every `*.md` under `plugins/*/skills/`, and
  every `*.md` and `*.mdx` under `site/src/content/docs/`.
- Each file is read with its whitespace collapsed to single spaces
  (`tr -s '[:space:]' ' '`). A command a page wraps across lines is then
  one command.
- It finds each install command by its fixed start and holds it to D2's
  form. A failure is one line,
  `installs: <file>: <command as found>: <what it must be>`. The forms
  it holds:
  - `marketplace add agentplot/sluice` must be followed at once by
    `#release`.
  - `plugin install agentplot/sluice/plugins/herdr` must be followed by
    ` --ref v` and then `<`, a digit or `$`.
  - `github:agentplot/sluice` must be followed by `/v` and then `<`, a
    digit or `$`. `#darwinModules.` and `#nixosModules.` are passed, as
    a module's key names no install source.
  - `--git https://github.com/agentplot/sluice`, with or without `.git`,
    must be followed by ` --tag v` and then `<`, a digit or `$`.
  - A URL under `github.com/agentplot/sluice/releases/` that holds
    `/download/` must be `releases/latest/download/`, or
    `releases/download/v` and then `<`, a digit or `$`. A link to the
    releases page, `releases` or `releases/latest` with no `/download/`,
    is passed: the README's badge links there.
- It exits 0 and prints
  `installs: every install command names a release` where nothing
  failed.

`moon.yml`'s `installs` task runs `sh check-installs.sh`, tagged
`check-fast`, with `runFromWorkspaceRoot: true`. Its inputs are
`/check-installs.sh`, `/README.md`, `/docs/examples/README.md`,
`/plugins/*/README.md`, `/plugins/*/skills/**/*` and
`/site/src/content/docs/**/*`. A comment above it says what it holds and
cites records 0111 and 0112 and core.56. It sits beside `release`.

Not taken:
- **A check per place, in each plugin's project and the site's.** The
  rule spans the parts. One check reads them all with one list of forms,
  as `root:marketplace` spans the plugins.
- **Reading the docs by hand at verify alone.** core.21 rules out a task
  whose only check is a command run by hand where the gate can run one.
  A page edited later would then name main again with nothing to stop
  it.

### D6. `root:release` holds the new push

`check-release.sh` changes in its push alone:
- The `ordered` step for the push reads
  `git push --atomic origin "refs/tags/$TAG" "+HEAD:refs/heads/release"`,
  "the tag and release, atomically".
- The job's pushes are still counted. Exactly one is allowed, and it
  must be that line exactly. Where it is not, the failure is
  `release: release: pushes other than the tag and release, atomically: <push>`.
  Where the count is not one, it is
  `release: release: pushes <n> times: its one push is the tag and release, atomically`.
- Its opening comment's `release` line says the job pushes the tag and
  `release` in one atomic push, the tag unforced and `release` forced to
  the release commit. It cites records 0111 and 0112. Its last line says
  `release: the workflow releases as records 0111 and 0112 have it`.

The `release` task's comment in `moon.yml` says the release commit, its
tag and `release` are pushed atomically.

### D7. The decision is recorded

- **Record 0112**,
  `docs/adr/0112-sluice-and-its-plugins-are-installed-from-a-release-the-marketplace-at-a-release-branch.md`,
  in MADR, `number: 112`, laid out as record 0111 is. It decides D1 to
  D5, each with the path not taken.
  - **Status:** accepted. Its form is the family's, Flume's. The planner's
    brief of 2026-11-11 asked for it, on batch 50's report. The choices
    within that form are the construct's own, each cheap to change.
  - **Deciders:** the planner's brief of 2026-11-11, and the construct of
    the change `sluices-plugins-install-from-its-release`.
  - **Sources:** this design's.
  - **Considered options:**
    1. Plugins and sluice from a release, the marketplace at a `release`
       branch the release job moves (Flume's).
    2. The marketplace at a release's tag.
    3. The plugins alone from a release, sluice from main.
    4. Installs from main, as today.

    It takes 1.
  - **Consequences:** the herdr watcher is replaced at each release, as
    record 0094's lease intends. `release` does not exist until the first
    release after this lands. timber's installs differ until timber's
    own change.
  - Where another record takes 112 first, it takes the next number free
    on main and on this bolt.
- **Record 0111** gets an amendment, as record 0107 carries its own. A
  line after its Deciders and Sources:
  "Amended 2026-11-11 by record 0112, below: the release job pushes the
  `release` branch with the tag." Then a section at its end,
  `## Amendment, 2026-11-11: the release job moves the release branch`.
  The section has its status (accepted, with record 0112) and says the
  release job's push is now the tag and `release` in one atomic push,
  `release` forced to the release commit and the tag never forced. Its
  decision text above stays as it was written.
- **Rule core.55**'s line stands. Its details change:
  - "a tag pushed with a branch or by force, moved or deleted" becomes
    "a tag pushed by force, moved or deleted; a branch other than
    `release` pushed by the release job, or `release` pushed apart from
    its tag or to a commit its tag does not name".
  - Its Not ruled out gains "`release` forced to each release commit in
    one atomic push with the tag, since a release commit's parent is
    main's pushed commit (record 0112)".
  - Its Source gains record 0112.
- **Rule core.56**, after core.55 under `### Releases`:

  > **[core.56]** sluice and its plugins are installed from a release:
  > every install command the README, a plugin's README or skill, or the
  > site gives names a release's tag or archive, or, for the Claude Code
  > marketplace, the `release` branch the release job moves with each
  > release's tag; none names main.

  Its details, after core.55's:
  - **Rules out:**
    - `/plugin marketplace add agentplot/sluice` with no `#release`;
    - `herdr plugin install agentplot/sluice/plugins/herdr` with no
      `--ref v<version>`;
    - `github:agentplot/sluice` with no release's tag, as a profile
      install or a flake input;
    - `cargo install --git` of sluice with no `--tag`;
    - a setup skill that proposes a sluice of another release than its
      plugin's, where its plugin is a release's.
  - **Not ruled out:**
    - `releases/latest/download/`, the newest release's archive;
    - a page saying a build of main reports `0.0.0`;
    - a module's key, `github:agentplot/sluice#<platform>Modules.sluice`;
    - the records, the plans and the OpenSpec changes, which keep the
      commands they were written with.
  - **Source:** record 0112; Flume's install; batch 50's report.

  The rule's line names no host or tool of ours (core.26).
  `site/src/content/docs/architecture/rules.md` is rewritten by
  `node site/scripts/rules.mjs`.
- **The statement** needs no clause. How a tool is installed is in no
  clause of `design/sluice/requirements.md` (core.12). The form is
  Flume's, so the family has one way. Flume owns it, and sluice cites it.

### Rules relied on

- core.12, core.13.
- core.21: one new check, its own root task in check-fast, spanning the
  parts as `root:marketplace` does. The new `.sh` is under treefmt's
  shellcheck and shfmt.
- core.24: `sh`, `tr`, `grep` and `awk`, as the host has them; nothing
  pinned is added.
- core.26: every word for an operator; no host, team or tool of ours.
- core.55: its line holds; its details take `release` (D7).

This change departs from no rule. It amends core.55's details and adds
core.56, both through record 0112.

## Risks / Trade-offs

- **`release` before the first release after this lands.** The docs on
  main say `#release` once this lands, and the branch does not exist
  until the release job pushes it. The landing is itself released, since
  this change's commits include a `feat` and `docs` commits, which
  git-cliff bumps. So the gap is the release job's minutes. If that job
  fails, `/plugin marketplace add agentplot/sluice#release` fails until
  the next release.
- **A ruleset over every branch.** A ruleset added later over all
  branches, or over `release`, would refuse the forced push. The atomic
  push then moves nothing, the tag included, and the job fails. That
  ruleset needs `release` left out, or the job's identity let through.
- **A person's marketplace added before this.** It stays at main, and
  its plugins say `0.0.0`. The README's and the guide's words show the
  add at `#release`. A person moves by removing the marketplace and
  adding it again. sluice's docs carry no note of this transition
  (principle 12).
- **Concurrent work on the setup skill.**
  `bolt/sluices-setup-skill-adds-its-herdr-lines` and its change touch
  the same `SKILL.md`. Whichever lands second takes the other's lines in.
- **The skill's comparison is read by the agent.** No test runs the
  skill's step 1. `root:installs` holds its commands, and its scenarios
  are read at verify.

## Migration Plan

Nothing moves on hosts. The first release after this lands pushes
`release` at its release commit. A person who added the marketplace from
main adds it again at `#release`.

## After deploy

Ops looks at the first release this change's landing makes:
- `git ls-remote https://github.com/agentplot/sluice refs/heads/release
  refs/tags/v<x.y.z>^{}` names one commit for both.
- `/plugin marketplace add agentplot/sluice#release` in a fresh Claude
  Code config, then `/plugin install sluice@sluice`, shows the plugin at
  `<x.y.z>`.
- `herdr plugin install agentplot/sluice/plugins/herdr --ref v<x.y.z>`
  installs the plugin at `<x.y.z>`.

## Task notes

**1.1** Record 0112 and record 0111's amendment, as D7 gives them.
Record 0112 cites:
- Flume's README, install page, herdr README and release job, at
  e2973d57, and `origin/release` at 65911c26;
- batch 50's report;
- GitHub's rulesets, read 2026-11-11;
- v0.89.1, the first release under record 0111;
- herdr 0.9.2's notes on `plugin install`'s options;
- record 0094's lease.

**1.2** core.55's details and core.56, as D7 gives them. core.56 goes
after core.55 in `### Releases`, and its details after core.55's. Then
run `node site/scripts/rules.mjs` and commit the page it writes.

**2.1** In `.github/workflows/ci.yml`:
- The `release` step's push becomes D1's line, in place of
  `git push origin "refs/tags/$TAG"`.
- The job's comment above `release:` says the release commit is tagged,
  the tag and `release`, which the marketplace is read at, are pushed in
  one atomic push, and nothing is committed to main and no tag moves. It
  cites records 0111 and 0112 and core.55.
- The workflow's header, after "main's tree names 0.0.0", adds that the
  `release` branch, which the Claude Code marketplace is read at, moves
  to each release commit with its tag (record 0112).

In `check-release.sh`, make D6's changes. In `moon.yml`, the `release`
task's comment says the release commit and its tag are pushed with
`release`, atomically. actionlint (`root:format`) still reads the
workflow; this task's check is `root:release`.

**3.1** `check-installs.sh` and `moon.yml`'s `installs` task, as D5
gives them.
- Its opening comment says what it holds and why: a plugin installed
  from main says `0.0.0`, its herdr watcher is never replaced, and it may
  call commands a released sluice lacks. It cites records 0111 and 0112
  and core.56.
- A failure line starts `installs: `.
- To find the commands: for each form, `grep -oE` its fixed start and
  the text up to the next space, backtick, quote or `)` in the collapsed
  file. Then hold each match to its form with a `case` in sh.
- The check is written before the docs move, so at this task it fails on
  the tree. Its lines then name exactly the ten files Context lists:
  - `README.md`
  - `docs/examples/README.md`
  - `plugins/herdr/README.md`
  - `plugins/sluice-board/README.md`
  - `plugins/sluice/README.md`
  - `plugins/sluice/skills/setting-up-sluice/SKILL.md`
  - `site/src/content/docs/guides/claude-code.md`
  - `site/src/content/docs/guides/herdr.md`
  - `site/src/content/docs/guides/walkthrough.md`
  - `site/src/content/docs/index.mdx`

  That list is this task's check. Each task of group 4 then clears its
  own files, and 4.4 leaves `root:installs` passing.

**4.1** `plugins/sluice/skills/setting-up-sluice/SKILL.md`, step 1, as
D3 gives it. It keeps its order, its "Show every command before running
it, and ask before anything that installs software", and its archive's
targets line. Commit this as `feat(plugins)`: the setup skill installs
its own plugin's release. That commit bumps the next release's minor
version, so the landing is released and pushes `release`.

**4.2** The plugins' READMEs, as D4 gives them.

**4.3** The site's pages, as D2 to D4 give them. `site:prose` (Vale)
must be clear of warnings. `<version>` inside a code fence is literal in
`.mdx`. Outside a fence, write it in inline code, as `` `v<version>` ``.

**4.4** `README.md` and `docs/examples/README.md`, as D4 gives them.
