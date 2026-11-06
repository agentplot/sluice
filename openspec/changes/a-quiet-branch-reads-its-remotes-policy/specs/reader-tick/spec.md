## MODIFIED Requirements

### Requirement: A branch's policy is read once per revision

A reader SHALL read a branch's policy at the branch's revision as the
host's own files hold it: the branch's own ref, or for a branch on a
remote its tracking ref as last fetched. While the host holds a revision
of the branch, the reader SHALL NOT read the policy at the revision the
branch's state last saw. It SHALL read the policy at a revision once,
and again only once the branch's revision has changed. Every round,
idle check, intake and overview of a pass that reads the policy at a
revision already read SHALL take what was read then, its refusal
included. A policy read at no revision a branch still names MAY be
forgotten (56, 124, 129).

#### Scenario: Policy read once while the revision stands

- **WHEN** `sluice run --rounds 6` passes over an idle `main` whose
  revision stands
- **THEN** `main`'s policy is read from git once, at its first pass

#### Scenario: Policy read again once the branch moves

- **WHEN** `main` moves to a revision whose policy sets `parallel = 4` while
  a watch is running
- **THEN** the watch's next line shows `main` testing up to 4 at once

#### Scenario: A refused policy stays refused at its revision

- **WHEN** `main`'s revision holds a policy sluice refuses, and `sluice
  run` passes over it three times
- **THEN** each pass says `main` passed by in the same words as today, the
  words said once as today, and the policy is read from git once

#### Scenario: A refusal ends at the tick after the host's revision moved

- **WHEN** `main` lands on `origin`, its state is on the host, nothing is
  queued, its state last saw a revision whose policy sluice refuses, a
  watch is reading, and this host's `origin/main` is fetched by hand to
  a revision whose policy reads
- **THEN** the watch's next line shows `main` with no `unread`, the
  policy at the new revision read from git once, and no round has run
