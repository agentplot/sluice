---
number: 120
title: The addresses of the moved tools name boomdrivedev, and the names sluice writes stay
status: accepted
date: 2026-11-12
decision-makers:
- the fix main--sluices-addresses-name-boomdrivedev-on-main
---

# The addresses of the moved tools name boomdrivedev, and the names sluice writes stay

Status: accepted on 2026-11-12. The brief of the fix `main--sluices-addresses-name-boomdrivedev-on-main` decides the addresses in its own words: "Every agentplot/ address in sluice's tree of a tool that has moved names boomdrivedev/ instead: its flake inputs, its site's address, its README, policies and skills." Which strings are names rather than addresses, and which files keep what they say, are this fix's own choices below. Each is cheap to change.

Sources: the brief; boomdrivedev/boomdrive record 0025 as amended on 2026-11-09 ("Every address moves once") and on 2026-11-12; `gh repo list boomdrivedev` and `gh repo list agentplot`, and the sites' answers, read on 2026-11-12; boomdrivedev/timber record 0032, the same move in timber's tree; rules core.12 and core.40; record 0117.

## Context and problem statement

On 2026-11-12 GitHub's boomdrivedev owner holds sluice, blueprints, boomdrive, boomdrive-cloud, boom, crew, flume, site-theme, strategy, timber and weir, among others. Each old address redirects to the new one. agentplot keeps boomdrive-web, flywheel-next and the archived flywheel. https://boomdrivedev.github.io/sluice/ and https://boomdrivedev.github.io/blueprints/sluice/ answer 200, and https://agentplot.github.io/sluice/ answers 404. A commit cited in agentplot/blueprints is the same commit in boomdrivedev/blueprints, and the same holds for the other repositories, which moved whole.

The tree said agentplot in five kinds of place:

- addresses of the moved tools;
- addresses of the tools that stayed;
- names that use agentplot as a namespace;
- what a record or a reading reports as it was printed or found on a host;
- fixtures.

The fix decides four questions:

1. Do the names move?
2. Which records move their addresses?
3. Do the fixtures move?
4. What happens to a spec's Purpose, which no delta reaches?

## Considered options

The names are the run kind `https://agentplot.github.io/sluice/run/v1` and the set-aside kind beside it, the herdr plugin's id `agentplot.sluice` with its actions, its state directory and its metadata source, the site theme's package name `@agentplot/site-theme`, the marketplace's owner and the license's holder. For these:

1. Keep them.
2. Move them to boomdrivedev.

For decision records:

1. Move every address of a moved tool in `docs/adr/`, the rules and the open OpenSpec changes, and keep quoted words, paths on a host, a rename's table and the archived changes as written.
2. Move every string, quoted words and paths included.
3. Move none.

For the fixtures, the sample remotes, pull requests and commits that the board's tests and the plugin-board spec give a repository:

1. Keep them.
2. Move them with the addresses.

## Decision outcome

**The names stay** (option 1). A name is not an address. The run kind is a predicate type written into signed statements that a signer's ref only adds to (109), and rule core.40 names it. It has never served a page. The herdr plugin's id keys the person's bindings, its state and its config directory. Moving it would install the plugin a second time, beside the first, and leave every binding pointing at the old id. The site theme's own `package.json` still names `@agentplot/site-theme`, so its import path stays while its source moves to `github:boomdrivedev/site-theme#v0.1.3`.

**Decision records move their addresses** (option 1). A record is read for what it cites, and the reader follows the address. A repository that moved whole is the same repository under its new address, so even an address of a past reading, such as deneb's records of boomdrivedev/sluice's `main`, still names it truly. These stay as written:

- what a command printed, such as `GET /repos/agentplot/sluice`;
- a path on a host, such as `~/Code/github_agentplot/` or a runner's `repositories/agentplot/` folder;
- record 0080's table, which records a rename as it was decided;
- the archived changes, which are the changes as they were archived, as timber's record 0032 also keeps them.

**The fixtures stay** (option 1). They are sample input that `githubBase` and the board's links parse. They address nothing a reader follows.

**A spec's Purpose stays as written.** OpenSpec's deltas reach requirements and not a Purpose. release-version's Purpose cites `agentplot/blueprints` principle 14 and `agentplot/boomdrive` record 0023 until a change rewrites that capability's Purpose. The change `sluices-addresses-name-boomdrivedev` carries the requirements of host-caches, host-settings-module, place-scratch and plugin-setup.

**This fix is not frozen.** Record 0117 halts fixes of sluice's main, meaning its defects. This fix moves addresses as record 0025 of boomdrive asks each moved repository to do. It changes no queue's behaviour.

### Consequences

These now name boomdrivedev/sluice, its site at https://boomdrivedev.github.io/sluice, and the other moved tools:

- the README;
- CLAUDE.md;
- `openspec/config.yaml`;
- the rules and the site's rules page;
- the records;
- the crates' comments;
- `cliff.toml`;
- the flake's header, homepage and the modules' keys;
- the pages workflow;
- the site's configuration and pages;
- the plugins' manifests, READMEs and skills;
- the vale styles' links;
- the social preview and the site's card.

The weir crates' git source in `Cargo.toml` and `Cargo.lock` names boomdrivedev/weir at the same revision.

- Each module's key is `github:boomdrivedev/sluice#<platform>Modules.sluice` (record 0098). A configuration that imports the module from both sluice's flake and Boomdrive's re-export still takes it once, since both come from this flake.
- A host adds the marketplace and installs the herdr plugin from boomdrivedev/sluice. It finds them under the names it had: `sluice` for the marketplace and `agentplot.sluice` for the herdr plugin.
- The repository's social preview in GitHub's settings is uploaded by hand from `docs/brand/social-preview.png`, as that file's header says.
- No rule changes. This record moves addresses once and binds no later diff.
