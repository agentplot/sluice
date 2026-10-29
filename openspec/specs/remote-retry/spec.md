# remote-retry Specification

## Purpose
Every call sluice makes to a remote, by git or to the forge's API, runs
under one retry policy that tries again only what the remote never
judged, so a dropped connection, a server error or a token the forge
refuses in its first seconds never fails a submission, and a push the
remote answered is never made twice (134).

## Requirements

### Requirement: Every call to a remote runs under one retry policy
Every fetch, listing and push sluice makes to a remote with git, and
every call it makes to the forge's API, SHALL run under one retry
policy: exponential backoff with jitter, from a maintained crate, bounded
by what called it. No call to a remote SHALL be tried again outside it
(134; rule core.29).

#### Scenario: A round's fetch meets a dropped connection
- **WHEN** a round fetches the branch and the first fetch fails with `Could not resolve host`, and the next succeeds
- **THEN** the round reads the branch and goes on, and nothing is logged on the branch

#### Scenario: A pull request read answered 503
- **WHEN** a waiting submit reads its pull request and GitHub answers `503` three times, then `200`
- **THEN** the wait goes on to the outcome, and four reads were made, each after a longer pause than the one before

### Requirement: What the remote never judged is tried again
A call SHALL be tried again when the remote never judged it: the
connection was not made, the server could not answer (a 5xx) before a
push was sent or to a forge call that makes nothing, or the remote
refused the call's credential before it negotiated any ref, saying not
found, permission denied or unauthenticated (134). A `429`, and a `403`
that says an allowance is spent, SHALL be neither: each is the
allowance's pause, which the policy waits out or stops on and never
backs off (134 as amended 2026-10-28; rule core.29).

#### Scenario: A token refused on a fetch
- **WHEN** a round fetches the branch, GitHub answers `Repository not found.` on the token held, and serves the branch on a fresh one
- **THEN** the round reads the branch and goes on

#### Scenario: A 429 is no backoff
- **WHEN** GitHub answers a read `429` with `retry-after: 120`
- **THEN** the read is not made again for two minutes, whatever the backoff's pauses would be

### Requirement: What the remote answered is not tried again
A call the remote answered SHALL NOT be tried again: a ref rejected
under its lease, a branch protected, any other ref rejected, and any
4xx answer of the forge's API but a refused credential (401), a `429`
and a `403` that says an allowance is spent, a pull request not found
among them (24, 45, 134).

#### Scenario: A protected branch is answered once
- **WHEN** a push to `main` is answered `[remote rejected] (protected branch hook declined)`
- **THEN** one push was made, and the move is reported refused in the remote's words

#### Scenario: A pull request not found is answered once
- **WHEN** sluice reads a pull request and GitHub answers `404 Not Found` on a token it has accepted before
- **THEN** one read was made, and the pull request reads as absent

#### Scenario: A spent allowance is no answer
- **WHEN** a waiting submit's read is answered `403` with `x-ratelimit-remaining: 0` and `API rate limit exceeded`
- **THEN** the submit does not fail: it waits until `x-ratelimit-reset` and reads again

### Requirement: A read is tried again whatever happened
A fetch, a listing, a forge `GET` and a forge query that changes nothing
SHALL be tried again on any failure until the policy's time is spent,
since reading changes nothing; a fetch the remote answers with no such
ref is that answer and SHALL NOT be tried again (134).

#### Scenario: A fetch cut midway
- **WHEN** a fetch of the records fails with `the remote end hung up unexpectedly`, and the next succeeds
- **THEN** the records are read, and the round goes on

#### Scenario: No such ref
- **WHEN** a fetch asks for a branch the remote does not have
- **THEN** one fetch was made, and the branch reads as absent

### Requirement: A credential the remote refused is bought afresh for the next try
Where sluice reaches the remote as its own identity, each try after a
credential refusal SHALL carry a credential bought afresh and settled,
never the one refused. Where sluice reaches the remote on the host's own
credentials, a credential refusal SHALL NOT be followed by a push, since
sluice cannot buy those afresh; a read on them is tried again as any
read is (134).

#### Scenario: A landing's push refused on two tokens, accepted on the third
- **WHEN** a submission lands on a branch on GitHub, and GitHub answers the push `Repository not found.` on the token held and on the first fresh token, and accepts the second fresh token
- **THEN** three pushes were made, each on a different token, the submission reads landed, and nothing reports a failure

#### Scenario: The forge's API refuses a token
- **WHEN** GitHub answers an API call `401 Bad credentials` on the token held, and answers the same call on a fresh token
- **THEN** the call succeeds, and the fresh token is the one sluice holds after

#### Scenario: The host's own credentials are not renewed
- **WHEN** the host has no `[forge]` table and the remote refuses a push on the host's own credentials
- **THEN** the push is made once only and fails with the remote's words

### Requirement: A push is tried again only with the first's lease
A push SHALL be tried again only when the remote never judged it, with
the refspec and the lease of the first try: a landing, a branch made or
removed, a session's branch or head pushed for a submit, and the
records' refs alike (45, 46, 104, 105, C.5, C.6).

#### Scenario: The branch moved between the tries
- **WHEN** a landing's first push is refused on its token, and the branch moves on the remote before the next try
- **THEN** the next try is refused as the branch having moved, the submission is not landed, and the next round forms the chain again on the revision found

#### Scenario: A server error before the push was sent
- **WHEN** a landing's push meets `The requested URL returned error: 503` while git asks the remote for its refs, and the next try is accepted
- **THEN** the submission reads landed

### Requirement: A push whose answer was not read is settled by reading its ref
A push whose answer git did not read, the connection cut or a server
error after the push was sent, SHALL NOT be pushed again. sluice SHALL
read the ref it pushed: when the remote holds what the push set, or no
ref for a removal, the push is done; otherwise it fails, saying the push
went unanswered and what the remote holds (122; ADR 0017).

#### Scenario: A landing whose answer was lost, made
- **WHEN** a landing's push meets `RPC failed; HTTP 502` after the push was sent, and the remote's branch then holds the landing
- **THEN** one push was made, and the submission reads landed

#### Scenario: A landing whose answer was lost, not made
- **WHEN** a landing's push meets `RPC failed; HTTP 502` after the push was sent, and the remote's branch still holds the revision the landing expected
- **THEN** one push was made, the round fails saying the push went unanswered and where the branch is, and the next round settles the landing (122)

#### Scenario: A record pushed with its answer lost
- **WHEN** a host's sign of life is pushed, the connection is cut after the push was sent, and the remote's ref holds the pushed commit
- **THEN** one push was made, and the sign of life stands

### Requirement: A forge write that sets a state is tried again
A forge call that sets a state, a pull request's draft mark, or a check
run's, a comment's or a pull request's text edited, SHALL be tried
again on any failure but the forge's answer, as a read is, since the
state read decides and setting it twice leaves it as once (134).

#### Scenario: A draft mark answered 502
- **WHEN** sluice marks a pull request a draft and GitHub answers `502`, then `200`
- **THEN** two calls were made, and the pull request reads as a draft

#### Scenario: A check run's update cut after it was sent
- **WHEN** sluice updates its check run on a pull request and the connection is cut while it waits for GitHub's answer, and the next try is answered
- **THEN** two calls were made, and the check run shows what sluice posted

### Requirement: A forge write that makes something is tried again blind only while unsent
A forge call that makes or starts something, a check run, a comment, a
pull request or a workflow's run, SHALL be tried again blind only while
the connection was never made, or on a token GitHub refused before
taking the call (134).

#### Scenario: A connection never made
- **WHEN** sluice posts a comment and the first try cannot connect, and the next is answered
- **THEN** the comment is posted, once

### Requirement: A thing made whose answer was not read is read back before it is made again
A check run, a comment or a pull request whose making's answer was not
read, the connection cut after the call was sent or a 5xx or a 429
among them, SHALL NOT be made again until the forge is read: one the
forge already holds SHALL be carried on and not made again, and one it
lacks SHALL be made then (134).

#### Scenario: A comment posted, its answer lost
- **WHEN** sluice comments on a pull request, GitHub answers `502`, and the pull request then holds the comment
- **THEN** one post was made, and sluice goes on as if GitHub had answered it

#### Scenario: A comment not taken
- **WHEN** sluice comments on a pull request, GitHub answers `502`, and the pull request then holds no such comment
- **THEN** the comment is posted again, once, and the pull request holds it once

#### Scenario: A check run made, its answer lost
- **WHEN** sluice creates its check run on a revision and the connection is cut after the call was sent, and the revision then holds sluice's check run of that name
- **THEN** one call created a check run, and the revision holds one sluice check run of that name

### Requirement: A workflow's run is never dispatched twice
A workflow dispatch whose answer was not read SHALL NOT be sent again,
since nothing reads a dispatch back. A pool's run whose dispatch went
unanswered SHALL be claimed as dispatched, and the event in the
branch's log that claims it SHALL carry the forge's words saying its
answer was not read; the next round reads whether the job runs, and a
claim with no job lapses as any pool's claim does, its run open again
(77, 88, 133, 134). A dispatch whose connection was never made SHALL be
sent again.

#### Scenario: A run's dispatch answered 502
- **WHEN** a round dispatches a pool's job for a run and GitHub answers `502`
- **THEN** one dispatch was made, the run reads as claimed by the pool's job, and the branch's log holds that claim with GitHub's words saying its answer was not read

#### Scenario: The dispatch was not taken
- **WHEN** a run's dispatch went unanswered, and on the next round no job of its claim is queued or running
- **THEN** the claim lapses and the run is dispatched again under a new claim, never under the one whose answer was lost

#### Scenario: A dispatch that never connected
- **WHEN** a round dispatches a pool's job and the first try cannot connect, and the next is answered
- **THEN** the run is claimed, and the branch's log holds its claim with nothing unanswered

### Requirement: The policy's time is set by what called it
A waiting submit and `sluice wait` SHALL give each call five minutes of
tries, ending sooner at the wait's `--timeout` or at Ctrl-C; every other
round, command and submit SHALL give each call thirty seconds. Past
that time the call fails as its last try did (134).

#### Scenario: Five minutes of 5xx in a wait
- **WHEN** `sluice wait 77` reads and GitHub answers every try with `503` for five minutes
- **THEN** the wait exits `3`, saying on stderr `sluice: could not read #77 for` how long, then that whether it landed is not known, then GitHub's words, and on stdout `could not read #77: sluice wait 77`

#### Scenario: Ctrl-C between tries
- **WHEN** a wait's read is between tries and the operator presses Ctrl-C
- **THEN** the wait stops at once, as a wait stopped by Ctrl-C does, and the submission stays queued

#### Scenario: A round outside a wait
- **WHEN** `sluice run` fetches a branch whose remote answers every try with `503`
- **THEN** the fetch fails within thirty seconds of pauses, and the refusal is logged on the branch once (133)

### Requirement: A refusal that stands is reported once, in the remote's words
When the policy's time is spent, the call SHALL fail with the remote's
own words of its last try, whole, followed, where credentials were
refused, by what each refused credential was: the app, the repository,
the installation, its repositories and permissions, and its age, never
the credential itself. A round's refusal SHALL be its one event in the
branch's log, once per cause and host; no try SHALL be logged on the
branch (133, 134).

#### Scenario: Every token refused
- **WHEN** GitHub answers every push of a branch's creation `Repository not found.` until the policy's time is spent
- **THEN** `sluice branch create` fails with `Repository not found.`, then names each refused token's app, repository, installation, scope and age in the order tried, and no token appears in the output

#### Scenario: Many tries, one line
- **WHEN** a round's fetch is refused on every try until the policy's time is spent
- **THEN** the branch's log holds one event for the refusal, with the remote's words, and none for the tries
