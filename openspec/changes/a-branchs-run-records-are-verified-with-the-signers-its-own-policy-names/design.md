## Context

See proposal.md for why. What the code does today:

- `Records::route(branch)` (`crates/sluice/src/profiles/records.rs`)
  routes a branch whose policy names signers and a remote to
  `Records::open(remote, branch)`. `open` is cached by remote: the first
  call reads the signers, with `Records::signers`, from the policy at
  that branch's revision on the remote and builds one `RemoteRefs` with
  them; every later branch on that remote gets the same `RemoteRefs`,
  and its signers, back.
- `RemoteRefs` (`remote_refs.rs`) holds `signers: BTreeMap<String,
  SignerKey>`. `RemoteRefs::results()` reads `refs/sluice/runs/<signer>`
  for each name in that map and verifies each statement with that
  name's key; `verified` caches the outcome by the statement's blob id.
- `State::results()` (`crates/sluice-seams/src/lib.rs`) takes no branch.
  `Records::results()` joins the host's own results file and every
  opened remote's `results()`. The core reads it in `Sluice::results`
  (`sluice-core/src/lib.rs`), used by a round's run and landing
  (`step.rs`), a session run (`session.rs`), the verdict view
  (`verdict_view.rs`) and `set_aside`.
- The gate already filters by signer *name* per branch:
  `verdict::read` counts a signed line only when
  `Rules::trusts_signed(signer, ..)` holds for the branch's own rules.
  What is shared across branches is which names are read at all and
  which *key* a name is verified with.

So, with `main` opened first: a signer only `release` names is never
read, and a run signed by the key `main` lists for `box` counts for
`release`'s `box`, whatever key `release` lists.

Rules this rests on: core.22 (a result counts only through a signer the
branch's policy names), core.15 (a gate is read from the branch's
current revision), core.27 (routing reads each revision only as far as
the remote and whether signers are named), core.31 (a branch's policy
is not read again while its revision stands), core.14 (nothing decided
is kept in a loop's memory). Clauses 27, 28, 56, 66, 78, 87 (as amended
2026-10-30, agentplot/blueprints c078b45), 124 and S18; ADRs 0032,
0037, 0069.

Once each branch verifies with its own list, a second gap shows. A round
claims a run on a signed branch when this host's signer name is one the
gate trusts (`Sluice::claimable`, `step.rs`), whatever key the host
signs with. Two hosts that both sign as `box`, `main` listing one's key
and `release` the other's, each claim both branches' runs; the result a
host signs for the branch that does not list its key is never read back,
and the verdict reads its checks as failed.

## Goals / Non-Goals

**Goals:**

- Each branch's runs verified with its own policy's signers and keys,
  read at its revision on its remote, never another branch's and never
  the host's copy.
- The list re-read when, and only when, the branch's revision moves.
- No new fetch: the revision the list is read at is the one the
  existing reads already bring.
- A host takes a shared branch's run only where the branch would count
  what it signs, and a result the branch does not count is missing,
  never failed (87).

**Non-Goals:**

- Changing which result lines a branch whose records are this host's own
  files counts (see proposal, out of scope); only the keys that verify
  the signed ones change.
- Changing which remotes' runs a branch reads (every opened remote's,
  as today).
- Changing how runs are signed or recorded (`record_results`), or the
  routing (core.27).

## Decisions

### D1. The results log is read for a branch

`State::results(&self, branch: &str)`. The branch is the one whose gate,
session run or verdict view reads it; every caller in the core already
has it in hand. `LocalFiles` and the fakes ignore it. `Records` uses it
to pick the signer list.

Alternatives considered:

- *Keep `results()` and verify each signer's runs with the union of the
  keys every routed branch lists, tagging each line with the key that
  verified it.* The gate would then need the keys' identities in
  `Rules`, which the core cannot read (a key may be a file at the
  revision, read by the profile). It also keeps a list made of other
  branches' keys in the read path, the shape of the defect.
- *One `RemoteRefs` per branch.* It would duplicate the per-remote state
  (the id counter, the beats, the tick's ask, the stale reads), which is
  the remote's and not a branch's.

`set_aside` has no branch: it looks for its result among the results of
each branch the records list (`State::branches`), joined and
de-duplicated, which is the set of results any gate could count. It is
an operator's command, run rarely; the cost of a read per branch is
acceptable.

### D2. `Records` keeps one signer list per branch, by revision

`Records` holds, per branch routed to a remote, the revision its list
was read at and the list (`Rc<BTreeMap<String, SignerKey>>`). On each
`results(branch)` for a routed branch:

1. The revision is the branch's tracking ref on its remote as last
   fetched (`Branch::held`, no fetch). The first time in a process, and
   only when the tick has not asked the remote, it is fetched as
   `Records::signers` does today (C.7), so a short command reads the
   branch as the remote holds it.
2. When that revision equals the one the list was read at, the list is
   used as is (core.31). Otherwise the policy is read whole at the new
   revision, as `Records::signers` reads it today, and replaces the
   list.
3. When the remote holds no revision and the host no tracking ref, the
   list is empty. The fallback to the routing's revision, which may be
   the host's own ref, is removed (124: the host reads nothing from its
   copy).
4. A policy refused at that revision fails the read, naming the branch,
   the remote and the revision, as `open` fails today.

`Records::open(remote)` no longer reads any signer and no longer takes a
branch. `guarding` reads no list, as before.

A branch whose records are this host's own files gets its list from its
own policy at its own revision (`Branches::policy_of`, read whole), so
the rule is the same for every branch. A policy there that names no
signers gives an empty list, and no remote line is read for it.

Alternative considered: re-read the list on every call. It reads a
policy through git on every pass of a driver, which core.31 rules out.

### D3. `RemoteRefs` verifies with the list it is handed

`RemoteRefs` keeps no signer list. `RemoteRefs::results_with(&signers)`
reads `refs/sluice/runs/<name>` for each name in the list it is given
and verifies with that name's key. Its `State::results(branch)` impl,
used only where a `RemoteRefs` stands alone in tests, verifies with an
empty list. `RemoteRefs::new` loses its `signers` argument.

The `verified` cache is keyed by the statement's blob id together with
the signer's name and a digest of the key's allowed-signers lines, so a
statement verified for one branch's key is never reused for another's.
`SignerKey::Ssh` carries that digest, computed when the list is read.

### D4. A host takes a shared branch's run only where its signature counts

The records seam answers, and the core asks: `State::verifies_own(branch,
at)` says whether `branch`'s own policy at `at`, the revision the round
read, lists the key this host signs with for the signer it signs as. Its
default answers yes, for a profile that verifies no signed run. `Records`
answers from the branch's list at that revision (`signers_at`, D2), with
no fetch, and reads the public half of its own signing key once a
process; a host that signs nothing, or a binding for guarding, answers
no. `SignerKey::lists(key)` says whether a key is among a signer's
allowed ones. The core never reads a key (core.22).

A round asks once, where it reads the gate, and carries the answer as
`listed`:

1. `Sluice::counts_here(run, home, rules, listed)` holds on a branch
   that is not signed, and otherwise only when `listed` holds and the
   signer may claim what the gate needs (confined, the run's system), as
   before. `claimable` requires it, so the claims a round makes, the
   starts it chooses and the pools it launches on (`choose_starts`,
   `launch_on_pools`, the lead's pick) take no run whose result would
   not count.
2. The time limit (66) reads an open run as not passed once it is over
   when it is of a system this host is not, as before, or when it does
   not count here. A run no listed host is live to take therefore stays
   open and the candidate is not passed at its limit.
3. `verdict::read` puts a check the run ran, on a signed branch, with no
   counted result for its key into a new `Verdict::missing`, and a run
   with any missing check has not passed. A check whose key the tree did not
   bear out still fails, as before (15).
4. When a finished run leaves checks missing with none failed and no
   time out, the round opens the run again (no run, no host, its place
   released) and logs `Event::Uncounted { combination, tasks, host }`. A
   host whose result counts then takes it.

Alternatives considered:

- *Read the key in the core.* The core would have to read keys and
  allowed-signers files, which are the profile's (core.2, core.22).
- *Keep claiming by name and read an uncounted result as a fail.* It
  blames a submission for a host's key, which 28 and S18 forbid.
- *Ask per run.* The answer depends only on the branch and the revision
  the round read, so once a round is enough, and reads no policy while
  the revision stands (core.31).

### D5. An ADR and the rule's lines

A new ADR, the next free number under `docs/adr/`, records D1 to D4: a
branch's signed runs are verified with its own policy's signers at its
revision on its remote, the records seam reads results for a branch, and
a host takes a shared branch's run only where the branch would count
what it signs, a result it does not count missing, never failed. Its
sources include clause 87 as amended 2026-10-30 and S18.

core.22's details gain a "Rules out" line for a branch's runs verified
with another branch's signers or keys, or with a list read at a revision
the branch has left, citing batch 44's reading and this ADR. The clause-87
lines main already holds for core.22 (sluice main 639c9424: a run claimed
on the signer's name alone, an unverifiable result read as a fail, the
core reading a key; and, not ruled out, the run staying open where no
host the branch would count is live) are carried word for word, and the
line's sources name clause 87 as amended 2026-10-30 and this ADR, so the
bolt's details agree with main's when it lands.

## Risks / Trade-offs

- [A run opened again because its result was not counted can be taken
  again by the same host, where its key is listed but its signed write is
  not read back] → The candidate's time limit (66) ends it; no test
  shows it happening, and the log's uncounted lines name the host.
- [A host that signs as a trusted signer with an unlisted key now takes
  no run on that branch] → That is clause 87; a branch no listed host
  serves reads its candidates not passed at the time limit, naming the
  checks missing.
- [A branch made on the remote by `sluice branch create` or a first
  landing reads no list until origin holds it] → No gate is read for a
  branch its remote does not hold (56, 124), so nothing that would have
  counted is lost; `remote_branch.rs`'s tests are run to show creation
  still lands.
- [A local branch whose policy, at its own revision, names no signers no
  longer reads another branch's signed runs] → That branch counted them
  by a list it never named; reading none is the rule 78 states. It may
  re-run checks it reused before. Named in the final report.
- [The seam's signature changes] → Every implementor is in this
  repository and is changed in the same group; no external profile
  exists (core.2).
- [`set_aside` reads every branch's results] → An operator's command;
  its cost is a read per branch with records.

## Migration Plan

None. Nothing is stored differently: the lists and the cache live in a
process's memory, and the next process reads them afresh.
