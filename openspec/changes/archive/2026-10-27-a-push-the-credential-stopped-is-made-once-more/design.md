## Context

Every git call sluice makes to a remote goes through
`PushedRefs::remote_git` (`crates/sluice/src/profiles/pushed_refs.rs`):
the pushed refs profile's fetches, `move_to`, `make`, `remove` and
`push_branch`, and every fetch, listing and push of the remote refs
profile (`remote_refs.rs`: `push_leased`, `delete`, `delete_if_present`,
`publish_head`, `beat`, `publish_candidate`, the logs' ref). With an
identity, `remote_git` gives git the identity's credentials. Since the
chore `a-fetch-with-sluices-token-is-never-silently-refused` (c5fca2f8),
a call whose stderr matches `REFUSED` is made once more on
`Identity::renew`, which `App::issue(repo, true)` answers with a token
bought now and handed over once it is five seconds old, but only when
`reads(args)` (a `fetch` or `ls-remote`). A refused push gets the
refused token's `about` appended to git's words and is not made again.

`leased` (landing, make, remove) reads `[rejected]` on porcelain stdout
as the lease refused and returns `Moved::Elsewhere` with a fresh fetch;
anything else is an error, which a landing's next step settles from its
intent (`perform_landing`, ADR 0017) and which `sluice branch` reports to
the asker.

Clause 134 and ADR 0062 (proposed) make a push refused on the
credential before any ref was negotiated a second try too, with the
first's lease; rule core.5's details already say so, and core.29
(proposed) draws the line.

## Goals / Non-Goals

**Goals:**
- One rule, in `remote_git`, for every read and push: refused on the
  credential before any ref, made once more on a fresh, settled token
  with the same arguments.
- A push refused at the ref, or with no answer, never made again.
- A leased push with no answer decided by what the remote holds.

**Non-Goals:**
- The core (`sluice-core`): no seam changes, no new outcome. A landing
  still settles from its intent the way it does now.
- Logging a refused read on the branch (133), another unit's.
- The token's lifetime, the settle time and `App::issue`: unchanged.
- The host's own credentials, with no identity: no renewal exists to
  make, so nothing is made again.

## Decisions

**The retry stays in `remote_git`, with the push gate replaced by a
"before any ref" test.** `reads(args)` goes. A refused call is made
again when git's stderr says the remote refused the credential
(`REFUSED`, as now) and git's porcelain stdout carries no ref's answer:
no line with a tab, the shape of `<flag>\t<from>:<to>\t<summary>`. A
fetch or a listing never prints such lines, so reads behave as they do
today. When the remote refuses the credential over HTTP, git exits
before it sends a command, so stdout is empty; when the remote answered
for a ref, stdout names it and the call is the move's answer, whatever
the stderr says. The same `args` slice is passed to the second
`git_as`, so the refspec and `--force-with-lease=<ref>:<expected>` are
the first's by construction.
*Alternatives:* a retry in each caller (`leased`, `push_branch`,
`publish_head`, …), which repeats one rule a dozen times and lets one
drift; a retry in the core around `move_to`, which teaches the core
about credentials (rule core.2) and misses the records' pushes.

**A refusal that stands is reported as the chore reports a read's.** The
existing `told` lines stay: `<remote> refused <first>, then <fresh>`,
or `<remote> refused <first>, and none could be made afresh: <why>`,
after git's words, whole. The branch "a push is never run again" and
its line `<remote> refused <first>` go, since a push now takes the same
path as a read. `about` never holds the secret (`Issued::about`).

**A leased push with no answer reads the branch.** In `leased`, an
outcome that is neither success nor `[rejected]` and that carries no
ref's answer (no porcelain ref line), and is not a credential refusal
that stood, is an answer git never read. `leased` then fetches the
branch: when it holds `to` (or is absent, for a removal), the move is
`Moved::Moved`, and a removal deletes its local branch and tracking ref
as an answered one does; otherwise the error stands, its message saying
the push went unanswered and naming what the remote holds (or that the
fetch failed, with its words). It never pushes again (core.29).
A landing gets the same answer `perform_landing` would reach next step
(122). A ref line with `[remote rejected]`, a protected branch, stays an
error as now: the remote answered.
*Alternative:* leave the unanswered case to the callers. A landing
already settles next step, but `sluice branch create` and `delete` have
no next step and would fail on a move that was made.

**The records' pushes need no change of their own.** `push_leased` on
the state ref already re-reads on its next write; `publish_head`,
`beat`, `publish_candidate`, the logs' push and the deletions write refs
of sluice's own, content-named or rewritten whole. Each gains the
second try through `remote_git`, and an unanswered one stays an error
the next round meets, as now.

**Testing against a fake that answers a push.** The fake HTTP server
(`tests/support/http.rs`) already refuses and serves upload-pack's
listing. For a push accepted on the second token, the fake answers
`info/refs?service=git-receive-pack` with a pkt-line advertisement of
the branch at the revision the test chooses, and `POST
/git-receive-pack` with a report-status of `unpack ok` and `ok <ref>`.
A push of a few objects is sent with a `Content-Length`, which the fake
reads; its body begins with the pkt-line command `<old> <new> <ref>`,
which the test reads (lossily) to show the second push carried the
first's old value and ref. A fake that advertises a different revision
on the second token shows git's lease refusing it locally. An
unanswered push is a fake that closes the connection, or answers 500,
on the POST; the read after it is the fake's upload-pack listing.

## Risks / Trade-offs

- [A remote's stderr words in `REFUSED` appearing in a ref-level
  refusal] → the "no ref's answer on stdout" test decides first; a
  `[remote rejected]` line is never made again whatever stderr says.
- [A 403 on the POST after the listing succeeded: was a ref
  negotiated?] → HTTP 401, 403 or 404 on receive-pack means the
  server never ran it, so no ref moved; git prints no ref line, and
  the second push carries the same lease, so if anything moved the
  branch in between, the remote refuses it at the ref (46).
- [An unanswered push applied after the read] → the read finds the
  branch at `expected`; the error stands and nothing pushes again. A
  landing's next step reads again and settles it (122); a `sluice
  branch` asker reruns and is refused as present or absent.
- [Five seconds added to a push refused on its token] → only on a
  refusal, which today fails outright.

## Migration Plan

None: no state, config or record changes shape. Rolling back restores
"a push is never run again".
