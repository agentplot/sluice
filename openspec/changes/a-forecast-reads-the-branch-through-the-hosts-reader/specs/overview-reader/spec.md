# Spec Delta

## ADDED Requirements

### Requirement: A reader marks each read and names its tick

A reader SHALL mark in the host's state, beside its document, when each
read begins and when it ends, whether or not the document changed, and
without rewriting the document when it did not. The reader's document
SHALL name its tick, the pause between its reads, in whole seconds as
`tick` (64, 129).

#### Scenario: A read that changes nothing
- **WHEN** a watch is the reader and three of its reads find the queues as they were
- **THEN** the document's content is as it was after the first, and the mark of its last read's end is the time the third ended

#### Scenario: The tick is named
- **WHEN** `sluice overview --json --watch` is the reader
- **THEN** its document holds `"tick": 1`, and a waiting `sluice wait`'s document as reader holds `"tick": 3`

### Requirement: A reader's ask of a remote brings what a forecast forms from

Where a reader reads a branch's records from the branch's remote, the
same call SHALL bring the branch's revision, the work submitted to
branches on that remote, and the combinations formed on the branch, and
the reader SHALL make no other call to that remote for them (64, 129;
rule core.29).

#### Scenario: One call for the branch
- **WHEN** a watch reads `main`, which is on a remote with `#1` queued from another host
- **THEN** each of its reads makes one call to the remote for `main`, and after it the host holds `main`'s revision on the remote and `#1`'s work

#### Scenario: A branch the remote lacks
- **WHEN** the remote holds `main`'s records but no longer holds the branch `main`
- **THEN** the reader reads `main`'s records as it does today, and its document names no revision for `main`
