# Proposal

## Why

On 2026-10-23 sluice 0.36.4 landed #132 and #134 on
`bolt/crews-small-leftovers-of-late-october` in the host's own refs,
the second at `c43609b`, which brought the policy naming the bolt's
remote. The next round read the bolt from that remote, where a refused
landing had left an older copy, `10e4791`. sluice logged `moved` from
`c43609b` to `10e4791`, formed #138 on it, tested it and landed it as
`8079db3`. Two landed merges were gone from the branch, and no branch
held `c43609b` any more (signal
`2026-10-23-madswan-operator-9ee49c2e/01-sluice-forms-merge-on-stale-base`;
the bolt's `log.jsonl` and `.finished/132, 134, 138.json` on deneb).

Clause 121 says a revision read that does not contain the last landing
sluice recorded is a branch whose reading and records disagree: nothing
is formed on it or landed there, and it is reported, until the landing
is back in the revision read or the operator accepts the revision read,
an act naming the revision that is recorded as the landing the branch
then stands on. A branch sluice makes stands on the revision it made it
at, and one removed and made again owes nothing to the landings of the
one removed.
sluice today treats any revision it did not move the branch to as a
hand's move it forms on (S8), whether it went forward or back.

## What Changes

- Each round, after reading the branch, checks that the revision read
  contains the branch's last landing. One that does is formed on as it
  stands, a hand's commit on top included (22, S8). One that does not
  holds the branch: no combination is formed, no run starts and
  nothing lands, and runs already under test are left to finish (121).
- The hold is written to the branch's records, once per revision read,
  naming the revision read and the landing it lacks. `sluice step`,
  `sluice run`, `sluice status <branch>`, `sluice overview` and a
  waiting `sluice submit` or `sluice wait` say so, with both revisions
  and the two ways out (121; C.4 pushed refs).
- The hold ends by itself on the first round whose revision read
  contains the landing again, as when the operator puts the branch back
  on the remote. The queue is then formed on that revision.
- `sluice branch accept <branch> <revision>` is the operator's
  acceptance of the revision read as the branch's. It is refused unless
  the branch is held at exactly that revision. Once recorded, the next
  round forms the queue on it (121, S57).
- The branch's records keep the revision it must contain: the last
  landing, the revision the operator accepted, or the revision
  `sluice branch create` made it at (121). Records written before this
  change take it from the last landing in their log, unless the
  revision the records last formed on lacks that landing, as for a
  branch removed and made again, where they take that revision.
- S57 has a conformance scenario over the fakes and an end-to-end test
  over real git with a remote.
- An ADR for the decisions below; rule core.6's Details and the site's
  rules page name 121; the commands and state references and the
  remote-branch guide describe the hold and `sluice branch accept`.

No clause changes: this makes 121, its C.4 pushed-refs row, the C.5
branch row's `sluice branch accept` and S57 true as they stand at
blueprints `d352658`.

Out of scope:

- The host's own ref of a branch on a remote as sluice's copy, and the
  hook refusing other writes to it (124, S58). That is its own change;
  this one reads the branch where it reads it today.
- Restoring a branch for the operator. sluice reports the hold and the
  two ways out; putting a landing back on a remote is the operator's.
- Repairing `bolt/crews-small-leftovers-of-late-october`, which crew's
  team already did with #143.

## Capabilities

### New Capabilities
- `branch-reading`: what a round requires of the revision it reads a
  branch at, the hold when that revision lacks the last landing, how it
  is reported, and how it ends.

### Modified Capabilities
- `remote-branch`: a round forms on the fetched revision only when it
  holds the branch's last landing.
- `branch-lifecycle`: a branch sluice makes is read from the revision it
  made it at; `sluice branch accept` is added beside create and delete.

## Impact

- **The core:** the round's decide step (`step.rs`) checks the read
  against the revision the branch must contain and holds; the landing
  confirmation sets that revision; `make_branch` sets it and
  `remove_branch` clears a hold; a new `accept_branch`; `Step::Held`;
  the branch document gains `landed` and `held`; the log gains `held`
  and `accepted` events; status and the overview carry the hold.
- **The CLI:** `sluice branch accept`; the hold in the text and JSON of
  `step`, `status <branch>` and `overview`, and in the text of a
  waiting `submit` and `wait`.
- **Tests:** the conformance suite (S57 and the hold's other cases), the
  CLI's tests over the fakes, and `tests/remote_branch.rs` for S57 over
  real git.
- **Docs:** a new ADR, `docs/architecture/core.md`, the site's rules
  page, `reference/commands.md`, `reference/state.md`, `guides/github.md`.

## Touches

```
crates/sluice-core/src/**
crates/sluice-conformance/**
crates/sluice-fakes/src/**
crates/sluice/src/lib.rs
crates/sluice/src/main.rs
crates/sluice/src/branch.rs
crates/sluice/src/overview.rs
crates/sluice/src/progress.rs
crates/sluice/src/profiles/local_refs.rs
crates/sluice/tests/**
docs/adr/*
docs/architecture/core.md
site/src/content/docs/architecture/rules.md
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/state.md
site/src/content/docs/guides/github.md
README.md
openspec/changes/archive/2026-10-23-a-branch-read-past-its-landings-is-refused/.openspec.yaml
openspec/changes/archive/2026-10-23-a-branch-read-past-its-landings-is-refused/design.md
openspec/changes/archive/2026-10-23-a-branch-read-past-its-landings-is-refused/proposal.md
openspec/changes/archive/2026-10-23-a-branch-read-past-its-landings-is-refused/specs/branch-lifecycle/spec.md
openspec/changes/archive/2026-10-23-a-branch-read-past-its-landings-is-refused/specs/branch-reading/spec.md
openspec/changes/archive/2026-10-23-a-branch-read-past-its-landings-is-refused/specs/remote-branch/spec.md
openspec/changes/archive/2026-10-23-a-branch-read-past-its-landings-is-refused/tasks.md
openspec/specs/branch-lifecycle/spec.md
openspec/specs/branch-reading/spec.md
openspec/specs/remote-branch/spec.md
```
