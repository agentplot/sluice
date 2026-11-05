---
number: 80
title: Flywheel is renamed Boomdrive, and tailrace timber
status: accepted
date: 2026-11-04
decision-makers:
- Chuck Swanberg
---

# Flywheel is renamed Boomdrive, and tailrace timber

Status: accepted by the user on 2026-11-04, in their words: "the ecosystem, the org, the branding, the cloud and the opensource runner tool are all boomdrive. There is also a "boom" open source tool that does the claim reconciliation." The rename is made in every repository at once; this record is sluice's part of it, and moves no clause and no decision.

Deciders: the user, whose words above were relayed with the mapping below in the brief of the chore `flywheel-is-renamed-boomdrive`.
Sources: rules core.1, core.26, core.31; ADRs 0003, 0004, 0005, 0007, 0010, 0045, 0049, 0058.

## Context and problem statement

sluice names Flywheel where it says who drives a queue (crew or Flywheel, a Flywheel effect), in the strings rule core.1 keeps out of the core, in the family's public name that rule core.26 lets the site carry, in a repository an incident of rule core.31 happened in, and on the site's start page. It names tailrace, the tool that fills the task cache, in ADR 0049. Every one of those names changes. Which mentions take the new names, and which keep their text?

## Considered options

1. Every live mention takes the new name, in records, rules, plans, examples and the site, a record's or a rule's names only and never its decision; archived OpenSpec changes, and a record's history of a name in its own words, keep their text.
2. Only what a reader is shown takes the new name, the site and the rules, and each record keeps the words it was written in.

## Decision outcome

Option 1, on the user's word. The names:

| was | is |
|---|---|
| Flywheel, the product, brand and ecosystem, and the family of open tools on their sites | Boomdrive, one word; `boomdrive` in code, commands, packages, paths and repository names |
| Flywheel Cloud, agentplot/flywheel-cloud | Boomdrive Cloud, agentplot/boomdrive-cloud |
| the runner, agentplot/flywheel-runner, its command `flywheel` and crates `flywheel` and `flywheel-hosted` | Boomdrive, agentplot/boomdrive, its command `boomdrive` and crates `boomdrive` and `boomdrive-hosted` |
| agentplot/flywheel-web | agentplot/boomdrive-web |
| tailrace, agentplot/tailrace, its crate, command, site and records | timber, agentplot/timber |
| a customer's unit of work, a Flywheel; crew's word for a partition's state branch | a drive, drives; `Drive` in code |
| a launch's mark `flywheel:<partition>/<n>` | `boomdrive:<partition>/<n>` |
| agentplot/flywheel, the original | archived, named as archived where history mentions it |
| boom, and anything third-party that says flywheel | unchanged |
| enrol, enrolment | enroll, enrolls, enrolled, enrolling, enrollment |

In sluice that makes Boomdrive the driver and its effects in ADRs 0003, 0005, 0010 and 0045 and in `docs/examples/README.md`, the product whose recutils records ADR 0007 set aside, the family's name on the site's start page and in rule core.26, and `boomdrive` the string rule core.1 keeps out of the core; boomdrive-web the repository of ADR 0004's principle and of rule core.31's incident; and timber the tool of ADR 0049.

One choice is this record's own. agentplot/flywheel-next, the repository whose crates and storage design ADRs 0002, 0003, 0008, 0009 and 0028 and the phase 1 plans cite, and its statement at `design/flywheel-next/requirements.md` in agentplot/blueprints, are renamed by no line of the mapping and are reached at those names today, so their citations keep them, as does ADR 0069's account of that repository's checkout on deneb. Renaming them would cite a repository and a path that do not exist. When either is renamed, the citations follow.

Option 2 leaves a reader of a record told of a product, a repository and a tool by names nothing answers to.

### Consequences

- Rule core.1 rules out the string `boomdrive` where it ruled out `flywheel`; rule core.26 lets the site carry Boomdrive as the family's public name; rule core.31's incident names boomdrive-web. Each cites this record.
- The site's family diagram, bar and footer come from `@agentplot/site-theme` at the version `site/package.json` names, v0.1.3, the theme's renamed release, so they say Boomdrive and draw timber in the loop.
- No mention in sluice names a drive, a launch's mark or the original repository, and no word here is spelled enrol.
