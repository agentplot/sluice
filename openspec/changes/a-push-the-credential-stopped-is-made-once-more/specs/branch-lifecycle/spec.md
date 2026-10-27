## ADDED Requirements

### Requirement: A branch made or removed with no answer is decided by what the remote holds
When `sluice branch create` or `sluice branch delete` pushes and git
reads no answer, sluice SHALL NOT push again. It SHALL read the branch
on the remote: when the remote holds what the push set, the branch made
at its revision or the branch gone, the command SHALL succeed as if the
push had been answered; otherwise it SHALL fail naming what the remote
holds (122, 134).

#### Scenario: Made, though the answer was lost
- **WHEN** `sluice branch create bolt/x --from main` pushes, the connection is cut before git reads the answer, and the remote holds `bolt/x` at `main`'s revision
- **THEN** one push was made, the command prints that revision and exits 0, and `bolt/x` reads from that revision

#### Scenario: Not made, and the answer was lost
- **WHEN** `sluice branch create bolt/x --from main` pushes, git reads no answer, and the remote has no `bolt/x`
- **THEN** one push was made, and the command fails saying the push went unanswered and `bolt/x` is absent on the remote

#### Scenario: Removed, though the answer was lost
- **WHEN** `sluice branch delete bolt/x` pushes the deletion, git reads no answer, and the remote no longer holds `bolt/x`
- **THEN** one push was made, the command exits 0, and the local branch and remote-tracking ref of `bolt/x` are gone
