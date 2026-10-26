# Proposal

## Why

sluice's branch protection reads the host's settings and the policy in
full, refusing any field it does not know, so a host whose settings a
newer sluice wrote stops being guarded by the hook an older sluice
installed. On one host, settings gaining `[runs] cores` a minute before
the hooks were moved to the newer sluice made every git ref update in
every guarded repository fail for that minute. Today's hook lets such a
transaction through instead, but it then guards nothing until the hook
is re-installed. Clause 124 (agentplot/blueprints 1340786) now says the
protection reads only what guarding needs and passes over every other
field, known to it or not.

## What Changes

- The hook reads of the host's settings only `[forge]`, the identity
  sluice reaches a branch's remote as, and passes over every other
  table and field, `[runs]`, `[signing]`, `[peers]`, `[[caches]]` and
  any table no sluice yet writes among them.
- The hook reads of the policy only what guarding needs: the
  `[branches]` patterns, each one's `remote`, and whether `[signers]`
  names any signer, which together say which branches are guarded and
  whether their records are on the remote or in the host's own files
  (C.6). Every other table and field, known or not, is passed over.
- Host settings or a policy the hook cannot read at all, a file that is
  not TOML, a `[forge]` or a `remote` of the wrong kind, a pattern tie,
  are a reason of its own: the transaction passes with the one warning
  it already gives, never a refusal and never an aborted git write.
- Every other command keeps reading the settings and the policy in
  full, refusing a field it does not know, as now.
- The hook keeps running the sluice that installed it, by the path it
  was installed with, never a `sluice` found on the caller's `PATH`.
- Every command sluice runs in a repository, not only `submit`, `wait`,
  `step` and `run`, re-installs the hook where sluice's hook already
  stands for another sluice, so a host whose sluice was upgraded is
  guarded by the newer one from its next run there. A repository whose
  hook was taken out with `sluice protect --remove` gets none back from
  a command other than those four.

Out of scope: hooks already installed by a sluice before this change
keep their full reading until re-installed, which their next run of
`submit`, `wait`, `step` or `run` does; how a host's configuration
manager orders its writes of the settings and its upgrade of sluice is
that manager's, not sluice's. The plugin's own session guard
(`plugins/sluice/scripts/`) reads the policy on its own and is not
changed here.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `branch-protection`: the protection reads of the host's settings and
  the policy only what guarding needs and passes over every other field;
  settings or a policy it cannot read at all pass the transaction, said
  once; the hook runs the sluice that installed it, which sluice
  re-installs at its next run in the repository (124).

## Impact

- `sluice-core`'s policy gains a guarding reading beside its full one,
  and a sluice bound for guarding reads its policy that way.
- The binary's host settings gain a guarding reading of `[forge]` alone;
  the profiles binding the branch and the records seams read the policy
  the guarding way when bound for the hook (rule core.2: the hook's
  binding is a profile choice, the core gains no profile's name).
- `main.rs` and `lib.rs` bind the hook with the guarding readings, and re-install
  the hook for every command run in a repository.
- A new ADR records why the hook reads partially while every other
  command reads strictly.
- No change to the records, the CLI's flags, or any output but the hook
  warning's reason.

## Touches

```
crates/sluice-core/src/policy.rs
crates/sluice-core/src/lib.rs
crates/sluice/src/instance.rs
crates/sluice/src/lib.rs
crates/sluice/src/main.rs
crates/sluice/src/protect.rs
crates/sluice/src/profiles/mod.rs
crates/sluice/src/profiles/branches.rs
crates/sluice/src/profiles/records.rs
crates/sluice/tests/protect.rs
crates/sluice/tests/instance.rs
docs/adr/0059-*.md
openspec/specs/branch-protection/spec.md
```
