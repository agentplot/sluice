---
title: Previews
description: A running copy of what a branch landed, at addresses a person opens, held until the caller releases it. For a crew that puts each landing in front of a person to try.
sidebar:
  order: 3
---

A preview is what lets a person try the work a branch has landed. sluice
checks out a revision the branch landed, gives it ports and a name for
each, runs your repository's command to bring the stack up, and hands
back the addresses to open. The preview stays up until something asks
for its release. Nobody keeps a worktree of the branch for it.

```console
$ sluice preview bolt/checkout-flow 2eb676c
preview/2eb676c8e45d of bolt/checkout-flow at 2eb676c: starting
  api  https://api-2eb676c--bolt-checkout-flow--shop.localhost/docs
  web  https://web-2eb676c--bolt-checkout-flow--shop.localhost
  ports: SLUICE_PORT_API=20001 SLUICE_PORT_WEB=20000
```

## What a preview is for

A preview is a cheap environment. It runs your repository's own stack,
on the fakes and stand-ins your tests already bring up: an emulator for
the cloud services, stand-ins for the outside systems, your servers. It
answers "does the work behave?" for a person who wants to click through
it. It starts in seconds to a minute, costs nothing but the host, and
many run at once. Its stack is the one the tests ran, so what the person
sees is what was tested.

A preview never shows work that did not pass. sluice previews only a
revision in the branch's history, which sluice moved there after the
combination passed. A submission still queued, or a combination under
test, is refused.

## What a preview is not

A preview does not answer "does it deploy and run in our cloud?" The
fakes do not model your infrastructure code, permissions, your identity
provider's real settings, real outside integrations or performance. That
takes a deployment into a real environment, by a deployment tool holding
cloud credentials, which sluice never holds. sluice is the integration
half of CI/CD: it decides what lands and shows what landed, running. The
delivery half, putting a revision into a real environment, belongs to a
deployment tool.

| the work touches | ask for |
|---|---|
| behavior, pages, flows, data the fakes model | a preview, on every landing |
| infrastructure code, permissions, identity provider settings, outside integrations, performance | a real environment from your deployment tool, once the branch is ready for `main` |

A caller with both can hold the same idea for each: an environment for
one revision, linked to its landing, replaced on the next landing and
ended when the branch lands. Only who provides it differs.

## Setting it up

The policy's `[preview]` table says how to bring the stack up and what a
person opens. It is read from the revision being previewed, so the
command always matches the code it brings up.

```toml
[tests]
runner = "moon"
toolchain = ["devenv", "print-dev-env"]
release = "scripts/preview-down"

[checkout]
method = "git-worktree"
ports = ["web", "api", "emulator"]

[preview]
command = "scripts/preview-up"
addresses = { web = "https://{name.web}.localhost", api = "https://{name.api}.localhost/docs" }
```

The command runs once in the preview's checkout, under the executor and
the branch's time limit, with `[tests] toolchain`. It gets the ports and
names in its environment:

| variable | example |
|---|---|
| `SLUICE_PORT_WEB` | `20000` |
| `SLUICE_NAME_WEB` | `web-2eb676c--bolt-checkout-flow--shop` |
| `SLUICE_BRANCH` | `bolt/checkout-flow` |
| `SLUICE_REVISION` | the full revision |
| `SLUICE_PLACE` | a short name of the checkout's own, for labels |

The command must bring the stack up, hand what stays running to the
tools that keep it, and exit. Anything left in its own process group
ends when it exits, as every run's does. A container engine keeps its
containers, and a process manager started detached keeps its processes.
Exit 0 means the preview is up.

### Names under a `.localhost` proxy

Each name is one DNS label, `<port>-<revision>--<branch>--<repository>`,
at most 63 characters, cut with a hash when longer. One label holds under
a wildcard certificate and a `*.localhost` redirect, which cover one
label each. sluice makes the name unique among the repository's
previews. Routing the name to the port is your command's job. With
[portless](https://github.com/vercel-labs/portless), for example:

```sh
#!/bin/sh
# scripts/preview-up: the stack, handed to the container engine, and a route per name
set -eu
docker compose -p "$SLUICE_PLACE" up -d --wait
portless alias "$SLUICE_NAME_WEB" "$SLUICE_PORT_WEB"
portless alias "$SLUICE_NAME_API" "$SLUICE_PORT_API"
```

```sh
#!/bin/sh
# scripts/preview-down: safe to run again, and on a stack that never came up
docker compose -p "$SLUICE_PLACE" down -v || true
portless alias --remove "$SLUICE_NAME_WEB" || true
portless alias --remove "$SLUICE_NAME_API" || true
```

`[tests] release` is the preview's release command. It runs when the
preview is released, with the same environment, and removes what the
command handed off.

## The life of a preview

A preview is `waiting` while the host is at its bound, `starting` while
its command runs, `up` once the command exited 0, or `failed`. Its
command counts against the host's bound only while it runs. A preview
held after that counts against nothing.

sluice never ends a preview that is up on its own: it cannot tell when a
person has finished looking. The caller ends it with
`sluice release <preview>`. sluice releases a preview on its own only
when nobody can use it: its command failed or ran out of time, or someone
removed its checkout by hand. A failed preview keeps what its command
printed and stays listed until it is released, so the caller can see why
it never came up.

```console
$ sluice previews
preview/2eb676c8e45d of bolt/checkout-flow at 2eb676c: up
  api  https://api-2eb676c--bolt-checkout-flow--shop.localhost/docs
  web  https://web-2eb676c--bolt-checkout-flow--shop.localhost
  ports: SLUICE_PORT_API=20001 SLUICE_PORT_WEB=20000
$ sluice release preview/2eb676c8e45d
released preview/2eb676c8e45d: its command's run ended, the release command run, its ports and names free
```

## Driving previews from a crew

sluice knows no bolt, no landing card and no proxy. A caller such as
crew or flywheel decides when a preview is made and when it goes:

```
 unit lands on the bolt at R1  -->  sluice preview <bolt> R1 --json
                                    addresses onto R1's landing card
 next unit lands at R2         -->  sluice preview <bolt> R2 --json
                                    addresses onto R2's card
                                    sluice release <R1's preview>
 the bolt lands on main        -->  sluice release <the bolt's last preview>
 after a crash                 -->  sluice previews --json
                                    release any whose revision is on no live card
```

- **One preview per landing**, of the revision the landing card names.
  The card's link always shows exactly that revision, and a new landing
  gets a new preview with new names.
- **The caller owns the lifetime.** A caller that drops a preview leaks a
  stack until its reconciliation finds it.
- **Asking again is safe.** A revision has one preview on the host; asking
  for it again gives it back unchanged and starts nothing. A caller that
  cannot tell whether it asked asks again.
- **`--wait`** returns once the command has ended, and exits 1 if it
  failed; without it, `sluice preview` returns as soon as the command
  starts, with the addresses already filled in.

A preview's names are `.localhost` names, so they reach a browser on the
host that runs it. A host whose dev servers already reach your
workstation through a tunnel, or a mirror of its proxy's routes, carries
a preview's routes the same way.

See [commands](/sluice/reference/commands/#sluice-preview) and
[configuration](/sluice/reference/configuration/#preview-a-running-copy-of-a-landed-revision).
