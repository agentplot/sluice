## ADDED Requirements

### Requirement: The board reads the queue only through its watch

Once the board follows sluice, it SHALL read the overview only through
`sluice overview --json --watch`, one at a time, for the session's life,
and SHALL start the watch again whenever it ends or can't be started. It
SHALL never run `sluice overview --json` on a timer. While no watch is
open, the band, the pane and the status line SHALL keep the last
overview read, and an outcome decided meanwhile SHALL be told once a
watch prints an overview that holds it.

#### Scenario: The watch ends

- **WHEN** the session's watch prints `main` with #13 queued and ends,
  and the next watch prints `main` with #13 landed
- **THEN** the board starts a second `sluice overview --json --watch`,
  the session is told #13 landed on `main`, and no `sluice overview
  --json` runs after the session's first read

#### Scenario: The watch can't be started

- **WHEN** the session's first read lists `main` with #3 queued and
  every `sluice overview --json --watch` fails to start
- **THEN** the board keeps trying the watch, runs no `sluice overview
  --json` in between, and the band still shows `main` with #3 queued

#### Scenario: One watch at a time

- **WHEN** the board's watch is open and the user runs `/sluice` twice
- **THEN** one `sluice overview --json --watch` is running, and no other
  is started until it ends

### Requirement: The wait before the next watch grows

The board SHALL wait before starting its watch again: 1 second the first
time, then twice the last wait each time, never more than 60 seconds. A
watch that was open for at least 60 seconds before it ended SHALL be
followed by a 1-second wait again.

#### Scenario: Watches that keep failing

- **WHEN** every `sluice overview --json --watch` fails to start
- **THEN** the board starts the next one 1, 2, 4, 8, 16 and 32 seconds
  after each failure, and every 60 seconds after that

#### Scenario: A watch killed mid-session

- **WHEN** the session's watch has been open for 10 minutes and ends
- **THEN** the board starts the next watch 1 second after it ended

#### Scenario: A watch that ran a while after waits had grown

- **WHEN** the board's watches have failed until it waits 60 seconds,
  and the next watch stays open for 10 minutes and then ends
- **THEN** the board starts the next watch 1 second after that one ended
