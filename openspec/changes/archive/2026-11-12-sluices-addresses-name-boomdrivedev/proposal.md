# sluice's addresses name boomdrivedev

## Why

sluice, blueprints, boomdrive, crew, flume, site-theme, strategy, timber
and weir moved from the agentplot owner on GitHub to boomdrivedev, and
their sites moved with them: sluice's site answers at
https://boomdrivedev.github.io/sluice and https://agentplot.github.io/sluice
answers 404 (read on 2026-11-12). Boomdrive's record 0025, as amended on
2026-11-09, has each moved repository's own change rewrite its addresses.

Four of sluice's specs still cite the statement at `agentplot/blueprints`
or Boomdrive's record at `agentplot/boomdrive`, give each module the key
`github:agentplot/sluice#<platform>Modules.sluice`, or offer
`herdr plugin install agentplot/sluice/plugins/herdr`. Each cited commit
is the same commit at boomdrivedev, so a citation changes its owner and
keeps its revision.

## What Changes

- Every requirement that cites the statement at `agentplot/blueprints`,
  or Boomdrive's record 0023 at `agentplot/boomdrive`, cites it at
  boomdrivedev at the same revision: host-caches, host-settings-module
  and place-scratch. No clause, scenario or behaviour changes.
- Each module's key is `github:boomdrivedev/sluice#<platform>Modules.sluice`
  (host-settings-module), as `flake.nix` and the module tests now say.
- "The module installs the package and nothing else" cites principle 10
  as `boomdrivedev/blueprints principle 10`, as the rules cite the
  principles, since the longer owner would take the requirement past
  strict validation's 500 characters.
- The setup offers `herdr plugin install boomdrivedev/sluice/plugins/herdr`
  (plugin-setup).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-caches`, `place-scratch`: the statement's citations name
  boomdrivedev/blueprints.
- `host-settings-module`: the citations name boomdrivedev/blueprints and
  boomdrivedev/boomdrive, and the modules' key names boomdrivedev/sluice.
- `plugin-setup`: the herdr plugin installs from boomdrivedev/sluice.

## Impact

- The names sluice writes and reads that say agentplot stay, as record
  0120 decides: the run kind `https://agentplot.github.io/sluice/run/v1`,
  the herdr plugin's id `agentplot.sluice`, its actions and its state
  directory, and the site theme's package name.
- plugin-board's scenarios give a repository sample remotes under
  `agentplot/sluice`. They are fixtures that address no tool, and they
  stay as written.
- release-version's Purpose cites `agentplot/blueprints` principle 14
  and `agentplot/boomdrive` record 0023. A delta reaches a requirement
  and no Purpose, so those lines stay until a change rewrites that
  capability's Purpose.
