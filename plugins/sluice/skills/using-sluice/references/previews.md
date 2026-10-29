# Previews

```console
$ sluice preview feature/x 2eb676c
preview/2eb676c8e45d of feature/x at 2eb676c: starting
  api  https://api-2eb676c--feature-x--shop.localhost/docs
  web  https://web-2eb676c--feature-x--shop.localhost
  ports: SLUICE_PORT_API=20001 SLUICE_PORT_WEB=20000
```

- Only a revision in the branch's history: queued heads and combinations
  under test are refused.
- Addresses are the policy's templates filled in: `{name.web}` becomes the
  preview's DNS label for port `web`, `{port.web}` its number.
- The policy's `[preview]` table, read from the previewed revision, says how
  to bring the stack up and which addresses to print. With no table, the
  revision is refused.
- States: `waiting` for room, `starting` while its command runs, `up` once
  it exited 0, `failed`. A failed preview is released at once with its output
  kept.
- Asking again for the same revision returns the same preview and starts
  nothing.
- `--wait` returns once the command has ended (exit `1` if it failed);
  `--json` prints one document with `name`, `state`, `addresses`, `ports`,
  `names`, and `output` when failed.

```sh
sluice previews [--json]          # every preview on the host; releases failed or orphaned ones first
sluice release preview/2eb676c8e45d   # or: sluice release 2eb676c
```

No round or landing ever releases a preview that is up: release yours when
the person is done. A preview is never a test result and no submission waits
on it.

A preview answers "does the work behave?" on the repository's own fakes. It
is not a deployment: infrastructure, permissions and real integrations need
a real environment from a deployment tool.
