# Spec Delta

## ADDED Requirements

### Requirement: A forecast takes the branch from a live reader and fetches nothing

Where the host's reader of the repository is live and keeping its tick,
and its last read holds the branch read whole, `sluice submit <branch>
--check` SHALL form on the branch's revision and the submissions queued
on it as that read found them, and SHALL ask the branch's remote
nothing. What it prints and how it exits SHALL be what forming on that
revision and queue gives (64, 129).

#### Scenario: A remote out of reach
- **WHEN** a watch on the host reads the repository, `main` is on a remote and `#1` is queued on it, and the remote then cannot be reached
- **THEN** within the watch's tick of its last read, `sluice submit main --check` prints the forecast against `#1` and exits as it says, and nothing it runs asks the remote anything

#### Scenario: Formed from what the reader read
- **WHEN** a live reader's last read, within its tick, says `main` is at revision `R` with `#4` queued, and the host's records say otherwise
- **THEN** `sluice submit main --check --json` gives `"onto": "R"` and `"ahead": [4]`

### Requirement: A branch forecast with no worktree is taken from the reader too

`sluice submit <branch> --check --head <from>` SHALL take `<from>`'s
revision from the same read where the reader's last read holds
`<from>`, and SHALL then fetch nothing for it either. Where the read
does not hold `<from>`, its revision SHALL be read as it is today (64).

#### Scenario: A bolt against main, with nothing fetched
- **WHEN** a live reader keeping its tick last read `main` and `bolt/y`, both on a remote, and the remote then cannot be reached
- **THEN** `sluice submit main --check --head bolt/y` prints the forecast of `bolt/y`'s revision as the reader read it, and exits as it says

### Requirement: A reader keeps its tick while it reads on time

A forecast SHALL count the reader as keeping its tick only while the
reader's process holds the reader's lock and either its last read ended
no more than its tick ago, or it is reading now and its last read ended
no more than a tick before this read began. Any other reader SHALL be
read as none (64, 129).

#### Scenario: A reader that has stalled
- **WHEN** the reader's lock is held but its last read ended five ticks ago and it began no read since
- **THEN** `sluice submit main --check` reads `main` and its records as a round would

#### Scenario: A reader in a slow read
- **WHEN** the reader's last read ended a second before its next began, and that next read has run for twenty seconds
- **THEN** a forecast takes the branch from the last read and fetches nothing

### Requirement: A forecast with no current reader reads as a round does

Where no reader keeps its tick, where the reader's last read does not
hold the branch, says it could not read it, or could not ask its remote
for it, or where a revision the forecast forms from is not on the host,
a forecast SHALL read the branch and its records as a round does (124)
and form on what it read. Where a submission's work or a combination it
forms from is still not on the host, it SHALL fetch them as a round does.

#### Scenario: No reader on the host
- **WHEN** no watch or wait reads the repository on the host and `main` is on a remote
- **THEN** `sluice submit main --check` fetches `main` and its records, and forms on what it fetched

#### Scenario: Another host's submission
- **WHEN** `#9` was submitted to `main` from another host, its work is not on this host, and no reader keeps its tick
- **THEN** `sluice submit main --check` fetches the work submitted to `main` and forecasts against `#9` rather than failing

#### Scenario: The reader could not reach the remote
- **WHEN** the reader's last read, within its tick, could not ask `main`'s remote and read `main`'s records as last fetched
- **THEN** a forecast of `main` reads as a round does
