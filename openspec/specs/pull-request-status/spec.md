# pull-request-status Specification

## Purpose

Shows on each pull request what sluice is doing with it, from waiting
to landed, so its author and reviewers read the queue where they work.

## Requirements

### Requirement: sluice posts one check on the pull request's head
sluice SHALL post a check run named `sluice` on the head of every open
pull request it considers, as its app, and update it when the state
changes: waiting and queued (queued, naming what is missing), testing
(in progress), failed and conflict (failure), landed (success). Once a
result touches the pull request's submission, the check's text SHALL
carry the submission's report. sluice SHALL NOT update a check that
already reads the same, text included.

#### Scenario: What is missing
- **WHEN** a pull request is out of draft and needs one review and `qa/approved`
- **THEN** its `sluice` check is queued and its summary names "1 review" and `qa/approved`

#### Scenario: Landed
- **WHEN** a pull request's submission lands
- **THEN** its `sluice` check is success and reads landed, and its text lists every required check of the combination it landed on

#### Scenario: No repeated posts
- **WHEN** two rounds run and nothing about a pull request changed between them
- **THEN** the second round posts nothing on it

### Requirement: A waiting pull request shows its forecast
Each round SHALL forecast every waiting pull request against the branch
and everything queued, running no check, and include the result in its
`sluice` check: that it would merge without conflict behind the queued
submissions, or which pull requests and files it conflicts with.

#### Scenario: A conflict seen early
- **WHEN** a waiting pull request changes a file that a queued pull request changes in a way git cannot merge
- **THEN** its `sluice` check names the queued pull request and the file, and no check of the branch's gate runs for it

### Requirement: A pull request sent back gets a comment
When a pull request's submission is blamed for a failure or removed for
a conflict, sluice SHALL comment on the pull request once. The comment
SHALL name the failed checks, or the contested files and pull requests,
and say how to submit again. A blamed submission's comment SHALL link
to the pull request's report.

#### Scenario: Blamed
- **WHEN** a hand-opened pull request's submission is blamed
- **THEN** one comment names the failed checks, links to the report, and says to push a fix or mark it draft and ready again

#### Scenario: Blamed, opened by a submit
- **WHEN** a pull request `wt submit` opened is blamed
- **THEN** one comment names the failed checks, links to the report, and says sluice marked it a draft and that the next submit from its branch, or marking it ready for review, submits it again

### Requirement: A landing merges the pull request
A pull request's submission SHALL land by the push of its branch, never
by GitHub's merge, and GitHub SHALL then show the pull request merged
because its head is in the branch.

#### Scenario: Merged by landing
- **WHEN** a pull request's submission lands
- **THEN** GitHub shows the pull request merged, and the branch's new commit is sluice's landing commit with the pull request's head as its second parent

### Requirement: The pull request's conversation carries its report
sluice SHALL post the submission's report as one comment in the pull
request's conversation, as its app. It SHALL do so at the first result
that touches the submission, and edit that same comment as the report
changes, never posting a second. A pull request with no result yet gets
no report comment.

#### Scenario: Edited, not repeated
- **WHEN** a pull request's combination is tested, then lands
- **THEN** the pull request has one report comment, which first lists the checks as they end and then reads landed

#### Scenario: Queued only
- **WHEN** a pull request is queued behind others and none of its combinations has started
- **THEN** it has no report comment, and its `sluice` check says what is ahead

### Requirement: The report heads with the combination
The report SHALL open with:
- the branch, where the submission stands, and the blamed pull request when there is one;
- the combination, linked to the branch revision and the pull requests tested together;
- the run's total time and estimated savings, when they were recorded.

#### Scenario: A blamed pull request
- **WHEN** #12 is blamed for `app:test` failing on the combination of `main`, #10 and #12
- **THEN** its report's head line names #12 as blamed, and links `main`'s revision, #10 and #12

### Requirement: The report lays out each check as a row
The report SHALL give one row per required check. A row SHALL hold the
check's result, name and time, and whether it ran, with its executor
and host, or was replayed. It SHALL note attempts, a flaky reading and
why a check could not run. Failed, flaky and could-not-run rows SHALL
come first. Past 20 rows, the rest SHALL be in a collapsed section.

#### Scenario: The failed row first
- **WHEN** #12 is blamed for `app:test`, and `app:lint` passed
- **THEN** the first row is `app:test` failed, with its time and executor, and `app:lint` follows

#### Scenario: Many checks
- **WHEN** a combination has 45 required checks, all passed
- **THEN** the report shows 20 rows, and the other 25 in a collapsed section

### Requirement: sluice posts a check per required check per system
Beside its `sluice` check, sluice SHALL post on a pull request's head
one check run per required check per system the branch names, named
`sluice / <check> (<system>)`, or `sluice / <check>` when the policy
names no systems. Each SHALL carry the result's outcome, the signer
that signed it or the host that ran it, and that check's row of the
report. sluice SHALL update it when a result is recorded for that key,
and never post one that already reads the same.

#### Scenario: A build on three systems
- **WHEN** `main` needs `cli:build` on three systems and a pull request's combination passed it on two and is running the third
- **THEN** the pull request's head shows `sluice / cli:build (linux-x64)` and `(linux-arm64)` as success naming their signers, and `(macos-arm64)` in progress

#### Scenario: A result read by key
- **WHEN** a pull request's combination reads a pass recorded for the same key from an earlier run
- **THEN** its check run reads success, says it was replayed, and names the signer of that pass

#### Scenario: No systems
- **WHEN** `main`'s policy names no systems and requires `app:test`
- **THEN** the pull request's head shows `sluice / app:test`

### Requirement: A pull request tested with a blamed one shows no failure
A pull request whose submission was tested in a combination another
pull request is blamed for SHALL show its `sluice` check and its
checks per system as queued, and
its report SHALL say its combination is formed again. The report's
earlier rows SHALL be removed, and the blamed pull request and the
failed checks SHALL NOT be named (20).

#### Scenario: Behind a blamed pull request
- **WHEN** #10 is blamed for `app:test` failing on the combination of #10 and #12
- **THEN** #12's `sluice` check and its `sluice / app:test` check are queued, its report comment says its combination is formed again, and none of them names #10 or `app:test`

### Requirement: A failed check's output is posted only as the policy asks
The end of a failed check's output SHALL be posted in the report only
when the branch's policy sets `pull_requests.log_lines`. The policy is
read from the branch's revision (56). At most that many lines SHALL be
posted, with secrets and token-shaped strings scrubbed as telemetry
scrubs them. Otherwise the report SHALL say where the output is kept.

#### Scenario: Not asked
- **WHEN** `main`'s policy sets no `log_lines` and `app:test` failed
- **THEN** the report names where `app:test`'s output is kept, and posts none of it

#### Scenario: Asked, with a secret in it
- **WHEN** `main`'s policy sets `log_lines = 40` and `app:test`'s output ends with a line holding `GITHUB_TOKEN=ghp_abc123...`
- **THEN** the report posts at most the last 40 lines, and that line reads `[redacted]` in place of the token

### Requirement: A pull request held for its submitter says so
The `sluice` check on a pull request a submit opened, held after its
submission left the queue, SHALL be queued and say it waits for the
next submit from its branch or for a person to mark it ready for
review.

#### Scenario: A draft sluice made
- **WHEN** sluice marked a pull request `wt submit` opened a draft after its head moved
- **THEN** its `sluice` check reads "waiting: the next submit" and its summary names the next `sluice submit` from its branch and marking it ready for review

#### Scenario: A draft a person made
- **WHEN** a person marks a hand-opened pull request a draft
- **THEN** its `sluice` check reads "waiting: a draft", as it does today

### Requirement: A pull request withdrawn by the operator says so
The `sluice` check on a pull request opened by hand, whose submission
the operator or a planner withdrew and that has had no new head or
ready mark since, SHALL be queued, name who withdrew it, and say that
new commits, or marking it a draft and then ready for review, submit it
again.

#### Scenario: Withdrawn by the operator
- **WHEN** the operator withdraws the submission of a pull request a person opened by hand
- **THEN** its `sluice` check reads "waiting: withdrawn by the operator", its summary says the operator withdrew it and that new commits or marking it draft and then ready submits it again, and no comment is left
