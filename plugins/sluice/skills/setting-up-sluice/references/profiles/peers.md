# Many hosts: signers, systems and pools

**Choose it when** sessions on more than one host submit to the same branch,
when an agent should be able to submit and switch its host off, or when a
check must pass on more than one system (Linux and macOS, x86 and ARM).
Every host runs the same sluice; the queue lives on the git remote, and
there is no server.

## What turns it on

A branch with a `remote`, in a policy that has a `[signers]` table, keeps its
queue on that remote under `refs/sluice/`. Without `[signers]`, every branch
keeps its queue on the host as before, and `trust` keeps listing executor
kinds. Add signers in one reviewed landing that also rewrites `trust`.

## Keys

```toml
[systems]
linux-x64   = { os = "linux", arch = "x86_64" }
macos-arm64 = { os = "macos", arch = "aarch64" }

[signers.box]                    # a host the operator keeps running
ssh = "exec@build-1 ssh-ed25519 AAAA..."
confined = true                  # its checkouts are containers no session reaches
systems = ["linux-x64"]

[signers.forge]                  # GitHub Actions jobs, signed keylessly
github = { workflow = ".github/workflows/sluice.yml", ref = "refs/heads/main" }
confined = true
systems = ["linux-x64", "macos-arm64"]

[signers.sessions]               # the hosts agents work on
ssh = ".config/sluice.sessions"  # one allowed-signers line per host
confined = false

[pools.forge]                    # tried in the order written
github = { workflow = ".github/workflows/sluice.yml" }
signer = "forge"
runners = { linux-x64 = "ubuntu-24.04", macos-arm64 = "macos-15" }

[branches."bolt/*"]
remote = "origin"
required = ["#check-fast"]
trust = ["sessions", "box", "forge"]   # no systems: a pass from any one

[branches.main]
remote = "origin"
required = ["#check-fast", "cli:build"]
trust = ["box", "forge"]
confined = true
systems = ["linux-x64"]

[branches.main.systems_for]
"cli:build" = ["linux-x64", "macos-arm64"]
```

- `trust` lists signer names. A result counts only when a signer the branch
  trusts signed it. A branch with a remote and no `trust` trusts no signer.
- `confined = true` counts only results whose signer has `confined = true`.
  Say it of a signer only when no session can reach its checkouts: a box
  that runs no sessions, or a pool's fresh machines.
- `systems` is what every required check must pass on; `systems_for` names
  other systems for one check. A check with neither takes one pass from any
  system. Each system is one more run per combination.
- A replay from the task runner's cache counts only when a trusted signer
  recorded a pass for that key on that system. `cache` in `trust` is
  ignored, with a warning.
- A standing host claims runs before any pool is started; a pool gets a run
  only when no host that may take it has room.

## The host's own settings

In `~/.config/sluice/config.toml` on each host:

```toml
[signing]
signer = "box"                     # the policy's name for this host's key
key = "~/.config/sluice/signing"   # an SSH private key, mode 0600

[runs]
systems = ["linux-x64"]            # default: the one system the host is

[peers]
lease = "5m"   # how long a quiet lead keeps a branch before another takes it
alive = "2m"   # how often this host shows it is alive; a claim lapses after it
```

`sluice run` on a host with `[signing]` claims the runs its signer counts
for. A host with no `[signing]` records nothing on a branch kept on the
remote, and says so.

## GitHub Actions

`sluice init actions` writes the workflows, adds `[signers.forge]` and
`[pools.forge]`, and prints what to set on GitHub: the app's permissions and
the `sluice` environment that holds its key. `sluice forge check main` says
whether the workflow, the environment and the signer agree. The guide is
<https://agentplot.github.io/sluice/guides/github-actions/>.
