## MODIFIED Requirements

### Requirement: The segment is the server's

The segment SHALL print the queue held in the plugin's state on the host
it runs on, for the repository of the directory it runs in, whichever
herdr server runs it, and SHALL need nothing of its environment but that
host's home directory and `PATH`. herdr runs it on the server the panes
run on, so a client attached from another host shows the same line as
one on the server's host.

#### Scenario: Only the server's environment

- **WHEN** the segment is run through `/bin/sh -lc` in the focused
  pane's directory, with an environment holding only the server's
  `HOME` and `PATH`, and the host's watcher holds a queue for the
  repository
- **THEN** it prints that queue's line, the same as when run with the
  plugin's whole environment

#### Scenario: Any server on the host

- **WHEN** two herdr servers on the host have panes in the repository,
  and the segment runs for each with its own `HERDR_SOCKET_PATH`
- **THEN** both print the same line
