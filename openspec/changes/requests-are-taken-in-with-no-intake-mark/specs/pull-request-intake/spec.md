## MODIFIED Requirements

### Requirement: A branch turns pull requests on in its policy
A branch's policy SHALL accept a `[branches."<pattern>".pull_requests]`
table with `reviews`, a count of approving reviews, default 0, and
`checks`, a list of check names, default empty. The table SHALL be
refused on a branch with no `remote`, and SHALL refuse its branch alone,
naming `[signers]`, in a policy that names no `[signers]`, whose
branches' records each host keeps. The conditions SHALL be read from
the branch's revision (56, 137, S64).

#### Scenario: Pull requests need a remote
- **WHEN** a branch's policy has a `pull_requests` table and no `remote`
- **THEN** the policy is refused with a message naming the branch and `remote`

#### Scenario: Pull requests need records on the remote
- **WHEN** main's policy has a `pull_requests` table and `remote = "origin"`, names no `[signers]`, and gives `bolt/*` a table too
- **THEN** a `sluice submit main` exits 2 naming main and `[signers]`, pushes and opens nothing, and a submission to `bolt/x` is tested and lands

#### Scenario: Status shows the refusal
- **WHEN** main is so refused
- **THEN** `sluice status main` prints the refusal, and `sluice overview` names main with it

#### Scenario: A pull request cannot loosen its own conditions
- **WHEN** a pull request changes `.config/sluice.toml` to lower `reviews`
- **THEN** its conditions are still the ones on the branch's revision

### Requirement: A pull request sluice cannot read holds no other
When a round cannot fetch an open pull request's head, or cannot read
its reviews, checks or timeline, draft or ready, it SHALL NOT submit it
that round, and SHALL go on to the branch's other pull requests,
submitting, forecasting and posting each as it would have been. It
SHALL read that pull request again each round until it can be read or
it is closed, and SHALL say what failed, naming `pull/<n>`, as a
standing intake failure is said (128).

#### Scenario: GitHub serves no head for one of two pull requests
- **WHEN** two pull requests are open into `main`, out of draft, every condition holding, and GitHub serves no head for the first, whose branch is gone, and `sluice step main` runs
- **THEN** its warning names the first and the failed fetch, and the second is submitted, its combination formed and its state posted

#### Scenario: Read again until GitHub serves its head
- **WHEN** a pull request's head could not be fetched for ten passes, and later its branch is pushed again so GitHub serves its head
- **THEN** each `sluice step` until then names it in a warning, main's log holds one `read-refused` line marked `intake` for it, and the first round after submits it

#### Scenario: Closed, it is no longer read
- **WHEN** a pull request whose head could not be fetched is closed
- **THEN** no later round names it

#### Scenario: A draft whose head cannot be fetched
- **WHEN** a draft's head cannot be fetched and another pull request is ready with every condition holding, and `sluice step main` runs
- **THEN** its warning names the draft, and the other pull request is submitted

#### Scenario: Reviews that cannot be read
- **WHEN** the policy names `reviews = 1`, GitHub answers an error for one pull request's reviews, another pull request has its review, and `sluice step main` runs
- **THEN** its warning names the first and the failed read, the first is not submitted, and the second is submitted

### Requirement: A failure that is no one pull request's still ends intake
A failure that is no single pull request's, such as listing the
branch's pull requests or reading its records, SHALL end that branch's
intake for the round, said as a standing intake failure is said, and
the round SHALL still land what is queued.

#### Scenario: GitHub's list of pull requests fails
- **WHEN** GitHub answers an error for the list of `main`'s open pull requests, and `sluice step main` runs
- **THEN** its warning says intake of `main` failed, no pull request is submitted or withdrawn, and a submission already queued is still tested and landed

## ADDED Requirements

### Requirement: A standing intake failure is said once for each cause and host
A cause of an intake failure, what intake asked and the words it was
refused with, SHALL be said by the host process once while it stands on
that host, at the pass that logs it on the branch, and never again
until it has ended, whether the process restarted or not, which is read
from what the records seam keeps and from no process's memory. A
`sluice step` SHALL print every warning its round's intake met, each
time (128; rules core.11, core.14, core.28).

#### Scenario: Forty passes refused alike
- **WHEN** GitHub refuses main's list of pull requests with the same words for forty passes of the host process, which restarts once among them
- **THEN** the host process's log says it once, and main's log holds one `read-refused` line marked `intake` for it

#### Scenario: A step beside them
- **WHEN** that refusal stands and `sluice step main` runs twice
- **THEN** each step prints the warning on stderr, and main's log gains no line for it

#### Scenario: A submit's own intake
- **WHEN** that refusal stands and `sluice submit main --no-wait` runs on the host that keeps main
- **THEN** the submit prints the warning on its stderr, and main's log gains no line for it
