# Previews

**Choose it when** a person should try each landing running: the
repository's own stack on its test fakes, at addresses to open, held until
released.

## Keys

```toml
[tests]
release = "scripts/preview-down"   # also a preview's release command

[checkout]
ports = ["web", "api"]

[preview]
command = "scripts/preview-up"
addresses = { web = "https://{name.web}.localhost", api = "https://{name.api}.localhost/docs" }
timeout = "10m"                    # default: the branch's timeout
```

- `[preview]` is read from the revision being previewed; `[executor]` and the
  limits from the branch's current revision.
- The command brings the stack up, hands what stays running to the tools that
  keep it (a container engine, a process manager), and exits. Exit 0 is `up`.
  It runs once under the executor with the toolchain.
- It gets `SLUICE_PORT_<NAME>`, `SLUICE_NAME_<NAME>` (one DNS label per port,
  for a proxy), `SLUICE_BRANCH`, `SLUICE_REVISION` and `SLUICE_PLACE`.
- Only a preview gets names. An agent's worktree under `sluice exec`, and a
  candidate, get ports and `SLUICE_PLACE` alone, so a script they share with
  the preview defaults each name: `${SLUICE_NAME_WEB:-web-$SLUICE_PLACE}`.
- `{name.<port>}` and `{port.<port>}` fill the addresses; a port not in
  `[checkout] ports` is refused. sluice never contacts the addresses.

## Traps

- With no `[preview]` table at a revision, that revision cannot be previewed.
- The release command must remove what the command handed off, routes
  included, or released previews leave stacks behind.
- `[tests] release` is every place's release command: it runs after each
  candidate's checks and at an agent's worktree's release too, where no
  `SLUICE_NAME_<NAME>` is set. Default each name as the command does, or under
  `set -u` the release fails there.
- A preview is not a deployment: infrastructure, permissions and real
  integrations need a real environment from a deployment tool.

## Verify

```sh
sluice preview <branch> <landed-revision> --wait
sluice previews
sluice release <revision>
```

Reference: <https://agentplot.github.io/sluice/guides/previews/>.
