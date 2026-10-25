## ADDED Requirements

### Requirement: A pull request sluice cannot read holds no other
When a round cannot fetch an open pull request's head, or cannot read
its reviews, checks or timeline, draft or ready, it SHALL print a
warning naming the pull request (`pull/<n>`) and what failed, SHALL NOT
submit it that round, and SHALL go on to the branch's other pull
requests, submitting, forecasting and posting each as it would have
been. It SHALL read that pull request again each round until it can be
read or it is closed.

#### Scenario: GitHub serves no head for one of two pull requests
- **WHEN** two pull requests are open into `main`, out of draft, every condition holding, and GitHub serves no head for the first, whose branch is gone
- **THEN** the round's warning names the first and the failed fetch, and the second is submitted, its combination formed and its state posted

#### Scenario: Read again until GitHub serves its head
- **WHEN** a pull request's head could not be fetched, and later its branch is pushed again so GitHub serves its head
- **THEN** each round until then names it in a warning, and the first round after submits it

#### Scenario: Closed, it is no longer read
- **WHEN** a pull request whose head could not be fetched is closed
- **THEN** no later round names it

#### Scenario: A draft whose head cannot be fetched
- **WHEN** a draft's head cannot be fetched and another pull request is ready with every condition holding
- **THEN** the round's warning names the draft, and the other pull request is submitted

#### Scenario: Reviews that cannot be read
- **WHEN** the policy names `reviews = 1`, GitHub answers an error for one pull request's reviews, and another pull request has its review
- **THEN** the round's warning names the first and the failed read, the first is not submitted, and the second is submitted

### Requirement: A failure that is no one pull request's still ends intake
A failure that is no single pull request's, such as listing the
branch's pull requests or reading its records, SHALL end that branch's
intake for the round with a warning, and the round SHALL still land
what is queued.

#### Scenario: GitHub's list of pull requests fails
- **WHEN** GitHub answers an error for the list of `main`'s open pull requests
- **THEN** the round's warning says intake of `main` failed, no pull request is submitted or withdrawn, and a submission already queued is still tested and landed
