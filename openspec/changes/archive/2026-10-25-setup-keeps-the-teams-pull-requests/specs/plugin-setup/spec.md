## ADDED Requirements

### Requirement: The setup asks whether a branch lands through pull requests today

For every branch it gates whose policy names a remote on GitHub, the
setup skill SHALL ask, before writing `.config/sluice.toml`, whether the
team lands work on that branch through pull requests today (127). Where
the answer is yes, it SHALL write the branch's
`[branches.<pattern>.pull_requests]` table with the conditions the team
names, so a request waits for them from the first day (C.7). It SHALL NOT
write the table where the answer is no, nor ask for a branch whose policy
names no remote.

#### Scenario: A team that reviews every pull request into main

- **WHEN** a user sets up sluice for `main` on GitHub and says the team
  lands on `main` through pull requests with one approving review
- **THEN** the policy the skill proposes has `[branches.main]` with
  `remote = "origin"` and `[branches.main.pull_requests]` with
  `reviews = 1`

#### Scenario: A branch the team does not land by pull request

- **WHEN** the user says agents land on `bolt/*` straight from their
  worktrees
- **THEN** the proposed policy has no `[branches."bolt/*".pull_requests]`
  table

#### Scenario: A branch kept on this host only

- **WHEN** a gated branch's policy names no remote
- **THEN** the skill does not ask about pull requests for it

### Requirement: The setup offers what GitHub requires of the branch now

Where the team lands through pull requests, the setup skill SHALL read
the approving reviews and required status checks GitHub's rulesets and
branch protection name for the branch today, and offer them as the
branch's `reviews` and `checks`, leaving out sluice's own checks (113).

#### Scenario: A ruleset that requires a review and CI

- **WHEN** `main`'s ruleset on GitHub requires one approving review and
  the status check `ci/test`
- **THEN** the skill offers `reviews = 1` and `checks = ["ci/test"]`

#### Scenario: sluice's own check is required

- **WHEN** `main`'s ruleset requires the status check `sluice`
- **THEN** the skill leaves `sluice` out of `checks`

### Requirement: The setup names a requirement the policy cannot hold

The setup skill SHALL say plainly, before writing the policy, of any
requirement GitHub holds on the branch that the policy has no condition
for, such as a review from code owners, that sluice will not wait for
it.

#### Scenario: Code owners must review

- **WHEN** `main`'s ruleset requires a review from code owners
- **THEN** the skill says sluice's policy has no condition for a code
  owner's review and that sluice will not wait for one, before writing
  the policy

### Requirement: The setup moves the conditions out of GitHub, asking first

Once the policy holds a branch's conditions, the setup skill SHALL
propose removing the required reviews and status checks from GitHub's
settings for the branch and turning GitHub's merge queue off, and SHALL
change no GitHub setting without the user's word.

#### Scenario: The ruleset still requires a review

- **WHEN** the policy holds `reviews = 1` for `main` and `main`'s ruleset
  still requires one approving review
- **THEN** the skill proposes removing the requirement from the ruleset
  and asks before doing so

#### Scenario: GitHub's merge queue is on

- **WHEN** `main`'s ruleset turns on GitHub's merge queue
- **THEN** the skill says sluice is the queue and asks to turn GitHub's
  off before landing is handed to sluice

### Requirement: The setup says what stays the same and who lands

Where a branch keeps its pull requests, the setup skill SHALL tell the
user, before committing the policy, that requests are opened as always,
by a submit or by hand; that a draft waits until marked ready and a ready
one waits for the policy's reviews and checks; that sluice then tests it
with what is ahead and lands it with its own push; and that nobody merges
it on GitHub or by hand (127, S62).

#### Scenario: The team is told what changes

- **WHEN** the skill has written `[branches.main.pull_requests]`
- **THEN** before committing it tells the user that only who lands a
  request changes, from GitHub's merge button to sluice's push

### Requirement: The setup says requests opened by hand need sluice running

Where a branch keeps its pull requests, the setup skill SHALL say that
`sluice run` must be kept going for a request opened by hand to be
taken, since no submit waits on it.

#### Scenario: Keeping sluice running

- **WHEN** the skill has written `[branches.main.pull_requests]`
- **THEN** it tells the user to keep `sluice run` going, and that sluice
  starts no service itself

### Requirement: The setup offers no mode in which the host's button lands

The setup skill SHALL offer no policy, setting or GitHub configuration
in which GitHub's merge button, GitHub's merge queue or a merge by hand
lands on a branch sluice keeps while sluice only tests. Asked for one, it
SHALL say sluice has none, and why: what the button lands was never
tested with what landed just before it (127, I1).

#### Scenario: Asked to keep the merge button

- **WHEN** a user asks the skill to let sluice test pull requests while
  people keep merging them with GitHub's button
- **THEN** the skill says sluice has no such mode, gives the reason, and
  offers the pull request conditions instead

### Requirement: The read-back names a branch's pull request conditions

When a branch has a `pull_requests` table, the setup skill's read-back
SHALL name, in that branch's block, what a pull request into it waits
for, its draft state, the number of approving reviews and each other
system's check by the name GitHub shows, and that sluice lands it once
those hold and its combination passes.

#### Scenario: main with one review and a QA check

- **WHEN** the policy has `[branches.main.pull_requests]` with
  `reviews = 1` and `checks = ["qa/approved"]`
- **THEN** the read-back's block for `main` says a pull request into it
  waits until it is out of draft, has one approving review and the check
  `qa/approved` passes, and then sluice tests and lands it
