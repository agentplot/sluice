# Design

## Context

The statement's clauses 72 to 82, 85 to 88 and 107 to 112, its
amended 27, 28, 29 and 69, its rewritten Part C and its scenarios
S17 to S20, S41 and S42 are what this change builds; the
`actions-pool` change builds the pool's jobs, 113 and S40. They are cited
by number. See proposal.md for why.

What exists today, and what this design builds on:

- **Records.** `State` in `crates/sluice-seams/src/lib.rs` is the
  records seam (ADR 0008). Its one live profile, `local_files`, keeps a
  branch's documents in the host's state directory, writes them all or
  nothing from the version read (62, 63), and keeps the repository's
  results log, `results.jsonl` (69). `Guard` is a host-local file lock
  (core.14).
- **Gate.** `Rules.trust` in `crates/sluice-core/src/policy.rs` lists
  executor kinds, `confined` and `cache`; `verdict::read` counts a ran
  result by its recorded `Enforced` and a replay when `trust` lists
  `cache` (ADRs 0004, 0015). A result is matched by task and hash.
- **Remote branches.** The `pushed_refs` branch profile fetches every
  round and lands by a leased push (ADR 0017); with a `[forge]` table it
  pushes as the GitHub App (ADR 0019). The state stays on one host, so
  two hosts on one remote branch today run two queues.
- **Executors.** `open`, `sandbox` and `container`, behind the check
  seam (`Tests`), bounded by the host's `[runs] max` (71, ADR 0012).
- **Pull requests.** The GitHub driver posts the `sluice` check run and
  reads `pull_requests.checks` (ADR 0021, pull-requests).
- **Site.** A "Many hosts (designed, not built)" section under
  `site/src/content/docs/many-hosts/` describes the earlier draft,
  many-hosts-2, with a central server.

## Goals / Non-Goals

**Goals:**

- One binary that is either today's one-host sluice or a peer, chosen
  by the policy, with no flag and no second deployment.
- Every decision a peer makes is recomputable from the refs on the
  remote and the policy on the branch's revision (core.11, core.14).
- A pass any trusted signer recorded is reusable by every combination
  and branch that shares its key and system (69).
- The guides that make standing executors and systems usable land
  with it.

**Non-Goals:**

- A server, a listener, or any process sluice starts besides `sluice run`
  (core.10, ADR 0010).
- Protecting `refs/sluice/` on the forge: GitHub's rules cover branches
  and tags only. The design survives a host that writes there anything
  it likes (D8).
- Progress (ADR 0028) across hosts: a run's tasks stay on the host or
  job that runs it.

## Decisions

### D1. The records seam gains a `remote refs` profile; signers choose it

A branch's state goes on the remote when the branch has a `remote` and
the policy has a `[signers]` table. Otherwise it stays in `local_files`,
`trust` keeps executor kinds, and nothing changes (78, third sentence).

*Why signers and not `remote` alone:* sluice's own `main` and every bolt
already name `remote = "origin"` with kind-based `trust`. If `remote`
alone chose the remote profile, the release that ships this change would
refuse the policy on `main`'s revision, and no landing could fix it,
since the gate is read from that revision (core.15). Keyed on
`[signers]`, the upgrade changes nothing, and a policy moves to peers by
one reviewed landing that adds signers and rewrites `trust`.
*Alternative:* a `records = "remote"` key per branch. It is one more
setting meaning what `[signers]` already implies: no gate on a shared
remote can count a result without a signer (78).

### D2. The layout under `refs/sluice/`

As C.6 has it:

| ref | written by | how |
|---|---|---|
| `state/<branch>` | any instance | a root commit whose tree is the branch's documents in `local_files`' layout; pushed `--force-with-lease=<read>` (62, 63) |
| `runs/<signer>` | that signer's hosts or jobs | a chain of commits, each adding files; never rewritten (109) |
| `heads/<oid>` | the submitting host | pushed once, removed when the entry leaves sluice (82) |
| `combinations/<branch>/<id>` | the lead | pushed when a combination opens, removed once its result is read |
| `logs/<branch>/<run>` | the host or job that ran it | the failed tasks' output; the last 20 failed per branch kept (`KEPT_LOGS`) |
| `hosts/<host>` | that host alone | rewritten each sign-of-life tick: signer, systems, bound, runs held |

`state/<branch>` is a root commit each time, so its version is its id
and nothing grows; the documents' history is the finished records (61).
The live documents are the same serde forms `local_files` writes, so the
core reads both profiles alike.

`runs/<signer>` is shared by every host of one signer, the `sessions`
signer's many hosts or the forge's parallel jobs. An add that the
remote refuses as not fast-forward is fetched, put on top of the new
tip and pushed again. Adds are files under distinct paths, so the
union is order-free, and re-pushing loses nothing. This is not the
blind retry core.5 forbids: that rule is about moving a branch, and an
add to a set decides nothing. 109's "one signer's write never refuses
another's" holds between signers.

A run's file path is `<key>/<system>/<claim>.json`, so reading a key on
a system is a tree lookup, not a scan.

### D3. The lead and the lease

The state holds `lead.json`: the instance that holds the branch. A step
on a remote branch proceeds only when it can write the state from the
version read with `lead.json` naming itself; the first write of a round
that finds another lead stops the round (72). The host-local `Guard`
still keeps two processes of one host from stepping one branch at once
(core.14).

A host with work on a branch it does not lead keeps, in its own state
directory, the version it saw and when it first saw it, by its own
monotonic clock. Once that version has stood for the lease, it writes
`lead.json` naming itself (73). The observation decides nothing but
when to try; the write decides, so a restart that loses the observation
only delays a takeover. *Alternative:* a timestamp in `lead.json`
compared with the reader's clock, which 73 rules out.

`[peers] lease` (default 5m) and `alive` (default 2m) are in the host's
config (74). Hosts with different values still agree on safety: the
value only sets how soon each one tries.

A waiting `sluice submit` and `wt submit` run rounds as a lead does
(ADR 0011); a `--no-wait` submit writes its entry and wakes (D7).

### D4. Claims and signs of life

Claims are entries in the branch's state, written like every other
document, so two hosts reaching for one run contend on one version and
one wins (75). The claim names the host, or, for a pool, the job
dispatched. A host reads a run as claimable when it is open, unclaimed,
the earliest open run of its chain (76), of a system it runs, on a
branch that trusts its signer for the claims the gate needs (79, 87).

A host rewrites `hosts/<host>` at its `alive` cadence. A reader marks a
claim lapsed when it has itself seen the holder's `hosts/` ref
unchanged for `alive` (77); a pool's claim lapses when the forge reports
the job ended with no report, cancelled, or lost. A lapsed claim is
removed by the next write of the lead and its run is open again; a
report under it is refused by the same check that refuses any write
from an old version (60).

Before it claims, a host releases every checkout it made whose
combination has a result, is obsolete, or whose claim it no longer
holds (81, core.16). That reuses the round's release of candidate places.

### D5. Signed results: DSSE, in-toto, SSH keys and keyless jobs

A run's results are one in-toto statement (v1) in a DSSE envelope: one
subject per check and system, `name` the check and system, `digest`
`{"sha256": <the task runner's hash>}`; the predicate
(`https://agentplot.github.io/sluice/run/v1`) holds each outcome, the claims of 79
(executor kind, confined, system), the branch, the combination, the
claim and the time.

- **SSH signers** sign the DSSE pre-authentication encoding with
  `SSHSIG`, namespace `sluice-run`, with the `ssh-key` crate, and are
  verified against the policy's allowed-signers lines. No `ssh-keygen`
  process; one key file named by the host's `[signing] key`, refused
  unless mode `0600` (as `[forge] key`).
- **GitHub signers** sign keylessly through `actions/attest`. Their
  verification is the `actions-pool` change's; until it lands, a
  `github` signer's records count for nothing.
- *Alternatives:* sluice signing keylessly itself with sigstore-rs. It
  speaks only the public-good instance and would publish a private
  repository's workflow names. Public repositories alone, or forge jobs
  signing with an SSH key from the `sluice` environment: each leaves
  private repositories without keyless signing. `gh attestation verify`:
  every verifying host would need `gh`. GPG signatures: a second key
  system for operators who already have SSH keys.

DSSE and in-toto are a few serde types and the pre-authentication
encoding, written in `crates/sluice`, not a dependency. Verification
belongs to the records seam's `remote refs` profile. The core passes it
each signer's identity as opaque settings, as it passes `[tests]`, and
receives results carrying `signer` and the claims. So the core names no
forge (core.1) and checks only trust, `confined` and systems against
`Rules` (78, 79).

### D6. Systems and keys

`[systems]` maps names to `os` and `arch`. A host's system is the one
whose `os` and `arch` match it, or those its `[runs] systems` names,
each checked against the host. A result is matched by task, hash and
system (69 amended). A required check's systems come from `systems_for`,
else the branch's `systems`, else "any one" (86). The lead opens one run
per system needed, each run holding the checks that need that system.
Checks needing "any one" ride in the first of those runs, or in a run of
system `any` that every executor may take (D4).

`results.jsonl` lines on a one-host branch gain `system`. A line with no
`system`, written before this change, is read as the host's own system,
so a one-host repository's earlier passes keep counting.

### D7. The pool is the check seam's `launched` executor; GitHub is its one profile

The check seam (`Tests`) gains a launched side: list the policy's pools
in order, `launch` a run on a pool and get its claim, ask whether a
claim's job is still alive, and `wake` a round. The GitHub profile
implements it with `workflow_dispatch` (inputs: branch, combination ref,
system, claim id) and the Actions jobs API. It is the only profile, so
core.3 wants an ADR saying why one is enough (ADR 0034). The lead
launches only after D4 finds no live host with room for the run (107).

The workflows that run rounds, runs and signing as jobs, `sluice job`
and `sluice init actions` are the `actions-pool` change's.

### D8. A landing re-reads its verdict from signed results

Every host that can submit can push `refs/sluice/state/<branch>`: the
remote submit writes entries there, and the forge cannot protect it.
So the state is trusted for order, not for verdicts. Before writing a
landing intent, the lead reads the combination's checks again from the
`runs/` refs of the signers the branch trusts, verified, and lands only
on a pass (run-signing spec). The landing commit's `Sluice-*` trailers
carry that reading (ADR 0006). A host that rewrites or deletes the state
can delay or drop queued work, which the queue's entries' heads under
`heads/` let an operator resubmit, but it cannot land anything.

### D9. A signer signs only keys it checked against the combination

A result names its key, and a pass on a key satisfies every gate that
trusts its signer (69). The checks run the combination's code, so a
check could report a pass under another combination's key. The signer
therefore checks every reported key against the combination's tree
before it signs, in a step that runs none of its tasks: the sign job on
the pool, the host's sluice outside the container on a box (C.6).

moon makes that check possible without running anything (moon 2.6, the
test `a_key_is_checked_against_the_tree_without_running`). A task's key
is the sha256 of its hash manifest, which moon keeps in its blob store
under the key (`.moon/cache/blobs/<2>/<62>`). The manifest names the
task and every input with the sha256 of the file's content, and each
dependency with its own key, or `ignored` when the dependency has no
outputs and enters no key. The signer takes the manifest the run left
for each reported key and signs that key only when:

- the manifest hashes to the key;
- it names the reported task;
- every input it names holds that content in the combination's tree;
- every dependency key it names passes the same check.

A report whose key fails is dropped and logged. The run hands its
manifests to the signer with its report: on a box, they are read from
the place's cache; on the pool, the run job uploads them beside the
report.

What the check leaves to the combination's own configuration: the
manifest's command, arguments, environment and toolchains. They come
from the task's configuration on the combination's revision, which no
other combination's key can stand in for unless its inputs are the same.
Within its own keys a check can still lie about its outcome, as a test
that exits 0 can; that is the gate's reviewers' to catch, as today.

### D10. Remote submit and moving a branch's state to the remote

`sluice submit` on a peer branch pushes the head to `heads/<oid>` with
the session's own git credentials, writes the entry, and wakes (111).
Submission ids come from a counter in the remote state, not the host.

When the remote has no `state/<branch>`, the first host to lead it
pushes its own live entries as the initial state, leased on the ref's
absence. A host whose push is refused re-submits its own live entries
under new ids and records the old id against the new in its finished
records, so `sluice status <old id>` still answers. `results.jsonl`
passes, unsigned, do not move: each check runs once more under a signer.

### D11. What the user sees

- `sluice status` and `sluice overview` show each result's system and
  signer, the branch's lead and how long since its last write, and each
  host's room by its sign of life. A host with no `[signing]` on a peer
  branch is told so on every run.
- `trust = ["cache"]` is read without `cache`, with a warning: refusing
  it would refuse the policy on `bolt/*`'s current revision (core.15),
  as in D1.

### D12. Rules and ADRs

Once the operator rules: core.11 loses its server-profile exception; a
rule under "Verdicts and attribution" says a result counts only through
a signer the policy names, never a record's executor field alone (78); a
rule under "Places" says an instance claims nothing while it has
checkouts to release (81). ADRs: 0031 the remote refs records and the
lease; 0032 signed results (DSSE, in-toto, SSH and keyless); 0033 a
landing reads its verdict from signed results; 0034 the pool is a
launched executor of the check seam, with GitHub its only profile
(core.3); and amendments to 0004 (signers, not the cache's write
credential), 0008 (one deployment: peers) and 0015 (a branch with no
remote trusts its host's unsigned runs).

## Risks / Trade-offs

- [Every write is a push] Rounds that write often, and many hosts
  polling, meet GitHub's rate of pushes and `ls-remote` calls. → One
  write per round step as today; hosts poll at `alive`, not faster; a
  round coalesces its writes; the conformance suite counts pushes per
  scenario.
- [The state is writable by every submitter] A session can rewrite or
  delete `state/<branch>`. → D8: no landing without signed passes; heads
  stay under `heads/` to resubmit; `sluice status` names the host that
  last wrote the state.
- [Upgrading] An older sluice on one host and a newer one on another
  share a peer branch. → The state's documents carry a schema version;
  an older binary that reads a newer one refuses to write and says to
  upgrade.

## Migration Plan

1. Ship the binary. Nothing changes for a policy without `[signers]`
   (D1); `cache` in `trust` reads as a warning.
2. Per repository: give each standing host a `[signing]` key and add its
   public half to the policy; land one change that adds `[signers]`
   and `[systems]` and rewrites `trust` to signer names. The next round moves the
   branch's state to the remote (D10).
3. Rollback: land a policy without `[signers]`. Branches go back to the
   host's own files; the remote state stays, unread, until the operator
   deletes `refs/sluice/`.

## Open Questions

- The default `lease` and `alive` (5m, 2m) are first guesses, to be
  tuned once peers run on sluice itself.
- Whether `sluice overview` should list hosts of other repositories that
  share a box. The `hosts/` refs are per repository, so the view needs
  one read per repository.
