# Servers in your worktree

A dev server or a stack you keep up while you work is started through
sluice, never by `wt step tether` or a worktrunk hook:

```sh
sluice exec <branch> -- scripts/dev-up   # brings the stack up on your ports, hands it off, exits
sluice release                           # runs the policy's [tests] release and frees the ports
```

- The script gets the worktree's ports, `SLUICE_PORT_<NAME>`, the same on
  every run, and `SLUICE_PLACE`, a short name unique to the worktree.
- It hands what stays up to a tool that keeps it, a container engine
  (`docker compose -p "$SLUICE_PLACE" up -d --wait`) or a process manager
  started detached (`devenv up -d`), and exits. What is left in its own
  process group ends when it exits, so `npm run dev &` does not stay up.
- Never run a server in the foreground under `sluice exec`: it holds one of
  the host's runs for as long as it serves, and the branch's time limit
  ends it.
- The policy's `[tests] release` stops what the script handed off.
  `wt submit` releases before worktrunk removes the worktree. On plain git,
  run `sluice release` before `git worktree remove`. A worktree removed
  without it is released by the next `sluice exec` or round on the host.

## Names

Only a preview is given names, `SLUICE_NAME_<NAME>`. Your worktree gets its
ports and `SLUICE_PLACE`, and no names. A script that routes a name to a
port, such as the preview's command, defaults each name from `SLUICE_PLACE`
so the same script serves your worktree:

```sh
web="${SLUICE_NAME_WEB:-web-$SLUICE_PLACE}"
portless alias "$web" "$SLUICE_PORT_WEB"
```

- Write the same default in the release command, which runs with the same
  environment, so it removes the route the script made.
- `SLUICE_PLACE` is one DNS label, unique on the host and the same on every
  run. A fixed default, such as `web-shop`, is one name for every worktree:
  the second worktree's route replaces the first's.
- The release command also runs after every candidate's checks, where no
  name is set either. The default keeps it from failing there.

## Leaving `wt step tether`

tether ends its command's process group once the worktree is gone. Drop it,
and any `post-start` or `pre-remove` hook that starts or stops a server, for
the script above:

- A container the server started is outside the group and outlives tether.
  The release command removes it.
- tether's command takes whatever port it picks, and worktrunk's
  `hash_port` collides by chance. sluice holds a block of ports for the
  worktree that nothing else takes.
- tether's command counts against no bound. A `sluice exec` waits for room.
