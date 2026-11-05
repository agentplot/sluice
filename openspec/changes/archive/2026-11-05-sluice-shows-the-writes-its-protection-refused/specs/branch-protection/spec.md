## MODIFIED Requirements

### Requirement: A refusal is recorded with the command and both revisions
A refused write whose cause does not stand SHALL be recorded in the
branch's log as `protected`, with the branch's ref, the value it held on
the host, the value the write would have set or none for a deletion,
the command that tried it as git ran it, and the host. A refused write
whose cause stands SHALL add no line and SHALL be counted with its cause
in the branch's state, as refused writes are kept (1, 124, 148).

#### Scenario: The log line of a refused move
- **WHEN** the host's `bolt/x` is at `10e4791` and `git branch -f bolt/x unit/a` is refused
- **THEN** the branch's log has a `protected` line with `ref` `refs/heads/bolt/x`, `from` `10e4791`, `to` `unit/a`'s revision, `by` naming `git branch -f bolt/x unit/a`, and `host` the host's name

#### Scenario: The same command refused again
- **WHEN** `git -C <repository> update-ref refs/heads/bolt/x <new> <old>` is refused, and then again with two other full revisions
- **THEN** the branch's log has one `protected` line, for the first, and `branch.json`'s `refused_writes` has one cause for it with `times` 2 (`a_timers_writes_are_one_cause` in `crates/sluice/tests/protect.rs`)
