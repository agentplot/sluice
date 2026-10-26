# Design

## Context

The hook, `sluice hook reference-transaction prepared`, is bound in
`crates/sluice/src/main.rs` (`fn hook`) by the same `Bound::open` every
command uses. That reads the host's settings with `Instance::read`,
whose forms in `crates/sluice/src/instance.rs` all carry
`#[serde(deny_unknown_fields)]`, opens the `[forge]` app and the
`[signing]` key, and builds the check and place seams. The judging in
`crates/sluice/src/protect.rs` (`judge`) then reaches the policy three
ways, each through `Policy::parse` in `crates/sluice-core/src/policy.rs`,
whose forms are also strict:

- `Sluice::protects` → `rules_at` → `policy_at` (`sluice-core/src/lib.rs`);
- the branch seam's routing, `Branches::remote` / `named` and
  `policy_of` (`profiles/branches.rs`), for the host's copy of a branch
  on a remote;
- the records seam's routing, `Records::route` (`profiles/records.rs`),
  which puts a branch's records on its remote when the policy names
  signers (C.6), and opens the signers' keys and the host's `[signing]`
  key to do it.

Since 0.41.0 a failure anywhere there passes the transaction with one
warning (`protect::unread`), which is the right answer to an unreadable
file but the wrong one to an unknown field: the hook stops guarding.
Clause 124 now says what guarding needs and that the rest is passed
over. The hook's shim (`protect::shim`) already runs the sluice it was
installed for by absolute path; `install` is called before binding for
`submit`, `wait`, `step` and `run` only.

## Goals / Non-Goals

**Goals:**

- One guarding reading of the host's settings and one of the policy,
  used by the hook alone, that pass over every field guarding does not
  need.
- The strict reading stays what every other command uses, unchanged.
- The hook re-installed for the running sluice at any command's run in
  the repository, where sluice's hook stands.

**Non-Goals:**

- Reading records written by a newer sluice: the records' documents are
  already read without refusing unknown fields.
- Making the strict readings lenient, or warning of unknown fields in
  them: a command that runs checks or lands needs every field it is
  given to mean what it says.

## Decisions

### D1. The hook is bound for guarding, not for work

`Bound::guarding(dir)` in `profiles/mod.rs` binds the repository with
`Instance::guarding()` and with the branch and records seams told to
read the policy the guarding way. `main.rs`'s `fn hook` uses it in place
of `Bound::open`; every other command keeps `Bound::open`. The check and
place seams are still built, from an `Instance` with the defaults, since
`run` takes a whole binding; the hook calls neither.

*Alternative:* a hook path that bypasses `run` and the seams and reads
the refs and the policy by hand. Rejected: the judging, the refusal's
record and the branch's lock would be written twice, once for the hook
and once for the core, and drift.

### D2. The guarding reading of the host's settings is `[forge]` alone

`Instance::guarding()` (with `Instance::guarding_from(path)` for tests)
reads the settings file as a TOML document and takes `[forge]` only,
`app_id` and `key`, through a form without `deny_unknown_fields`; other
fields of `[forge]` are passed over too. Everything else is the default:
no `[signing]`, default `[peers]`, no caches, a bound of one. `[forge]`
is what guarding needs because it is how sluice reaches a branch's
remote, where a remote branch's records are and where the hook's
`protected` line goes. A file that is not TOML, or a `[forge]` whose
`app_id` or `key` is missing or of the wrong kind, is an error; the hook
says it once and passes (D4).

`[signing]` is not read: the hook signs no run, and the records seam
uses the host's key only to sign runs (`remote_refs.rs`, its one
`self.signing` use).

### D3. The guarding reading of the policy is the branches and where their records are

`Policy::guarding(text)` in `sluice-core/src/policy.rs` reads the text as
a TOML document and builds a `Policy` with only:

- `branches`: each `[branches]` pattern, in the order written as
  `parse` keeps it, with `Rules::default()` and its `remote`, a string;
- `signers`: each `[signers]` name, its identity and claims passed
  over, so that `signers.is_empty()` says what it says in `parse`.

Every other table and field is passed over, and `Policy`'s other fields
take their defaults. A value guarding reads of the wrong kind, a
`[branches]` entry that is not a table, a `remote` that is not a string,
`[signers]` not a table, is an error naming it.

`Sluice` gains a reading, set with `Sluice::guarding(self)` beside
`as_peer` and `serving`; `policy_at` parses with `Policy::guarding`
when it is set. `run` in `crates/sluice/src/lib.rs` sets it for `Command::Hook`.
`Branches` and `Records` gain the same switch (`Branches::guarding`,
`Records::guarding`), set by `Bound::guarding`, and route every policy
read of theirs through one function that picks the reading. In guarding,
`Records::open` gives the remote refs profile no signer keys and no
signing, since the hook verifies and signs no run; the route itself
still turns on whether `[signers]` names one.

*Alternative:* drop `deny_unknown_fields` from the policy and settings
forms everywhere. Rejected (non-goal above), and it would pass over a
mistyped field in a gate.

*Alternative:* a separate guarding type rather than a thin `Policy`.
Rejected: `rules_for`'s pattern matching, ties included, and every
seam's routing take a `Policy`; one type keeps one matcher.

### D4. Unreadable is said once and passes, as today

No new path: an error from `Bound::guarding`, from `run`, or from
`judge` reaches the existing `protect::unread` warning and exit 0. The
change is only that fewer things are errors. The warning's reason names
the file, as the error chain already does (`reading <path>`,
`<branch>'s policy at <rev>`).

### D5. Every command run in a repository re-installs the hook

`protect::reinstall(dir, program) -> Result<bool>` writes the hook for
`program` only where the hook in place is sluice's (its marker line) and
names another program, keeping the chained hook as `install` does. It
writes nothing where there is no hook, where the hook is someone else's,
or where the hooks directory is one sluice will not write. `main.rs`
calls it, before binding, for every command but `hook`, `supervise`,
`credential` and `protect` (which installs or removes on its own), and
keeps calling `install` for `submit`, `wait`, `step` and `run`. A
failure is said once on stderr as `sluice: warning: branch protection
not re-installed: …` and changes nothing else the command does, as the
install's failure does now; under `--json` stdout is untouched.

`program` is the canonical path of the running executable, as for
`install`. The shim keeps naming it by absolute path, so the caller's
`PATH` never chooses the sluice that judges.

### D6. An ADR records the split

`docs/adr/0059-the-protection-reads-only-what-guarding-needs.md` (MADR):
the hook reads leniently and every other command strictly, why `[forge]`
and the branches and signers are what guarding needs, and why a reading
command re-installs only an existing hook. Cites 124, rules core.2 and
core.4.

## Risks / Trade-offs

- [The guarding and strict readings of the policy disagree on which
  branches are guarded] → both build `branches` from the same
  `[branches]` keys in the same order and match through the same
  `rules_for`; a unit test parses every example policy both ways and
  checks each pattern and `remote` agree.
- [A hook installed before this change still reads strictly] → it is
  re-installed by the next sluice command run in the repository once the
  host's sluice is this one or later; until then it passes such a
  transaction with its warning, as 0.41.0 does.
- [A reading command now writes a file in the git directory] → only
  where sluice's own hook already stands, only to name the sluice
  running, and atomically, as `install` writes it.

## Migration Plan

None beyond the release: the first command of a new sluice in each
repository re-installs the hook for it. Rollback is the previous
release, whose next `submit`, `wait`, `step` or `run` re-installs its
own hook.
