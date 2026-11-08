## RENAMED Requirements

- FROM: `### Requirement: sluice's check names the host that posts it and the pull request's submission there`
- TO: `### Requirement: sluice's check names the round job and the pull request's submission there`

## MODIFIED Requirements

### Requirement: sluice's check names the round job and the pull request's submission there
Every `sluice` check sluice posts on a pull request's head SHALL begin
its summary with `Submission <n> on <instance>.` where the branch's
records hold a submission of the pull request at that head, and
otherwise with `Waiting to be taken in on <instance>.`, `<instance>`
being the instance that keeps the branch, `pool/<pool>/round` for the
forge's round job. Its title SHALL stay as the state it shows (137,
C.5).

#### Scenario: Queued behind another
- **WHEN** the round job holds `#12` as submission 1406, queued behind `#10`'s submission 1405
- **THEN** `#12`'s `sluice` check is titled `queued behind #10`, and its summary begins `Submission 1406 on pool/forge/round.`

#### Scenario: Waiting for a review
- **WHEN** `#13` is out of draft and needs one review, and the round job asks for `main`'s open pull requests
- **THEN** `#13`'s `sluice` check is titled `waiting: needs 1 review`, and its summary begins `Waiting to be taken in on pool/forge/round.`

### Requirement: sluice's check carries the pull request's queue as fields
Every `sluice` check SHALL end its summary with one hidden block,
`<!-- sluice:queue {…} -->`, whose JSON has `host`, the instance that
keeps the branch; `taken_in`; `submission` and `state`, the
submission's number and its record's state, or `null` while not taken
in; `ahead`, each submission ahead of it in queue order as
`{"submission", "pull_request"}`; and `waits_for`, what it lacks before
it is taken in, or taken in again. A changed field SHALL be posted, and
an unchanged block posts nothing (137, C.5; rule core.32).

#### Scenario: A queued submission's block
- **WHEN** the round job holds `#12` as submission 1406, queued behind `#10`'s submission 1405
- **THEN** the summary of `#12`'s `sluice` check ends with `<!-- sluice:queue {"host":"pool/forge/round","taken_in":true,"submission":1406,"state":"queued","ahead":[{"submission":1405,"pull_request":10}],"waits_for":[]} -->`

#### Scenario: A queue reordered by priority and after
- **WHEN** the round job holds `#10`'s submission 1405 under test, then `#12`'s 1406 and `#14`'s 1407 queued, and a planner gives 1407 priority 5
- **THEN** `#12`'s check is titled `queued behind #10, #14` and its block's `ahead` names 1405 and 1407 in that order
- **AND** once 1407 is amended to land after 1406, `#12`'s check is titled `queued behind #10` and `#14`'s `queued behind #10, #12`

#### Scenario: A waiting pull request's block
- **WHEN** `#13` needs one review and `qa/approved`, and the round job asks for `main`'s open pull requests
- **THEN** its block reads `"taken_in":false`, `"submission":null`, `"state":null`, `"ahead":[]` and `"waits_for":["1 review","qa/approved"]`

#### Scenario: The queue ahead moves
- **WHEN** `#10`'s submission 1405 lands and `#12`'s submission 1406 is next on main
- **THEN** the round job posts `#12`'s check again, titled `queued: next on main`, its block's `ahead` empty

#### Scenario: Nothing in it changed
- **WHEN** two rounds of the round job run and neither `#12`'s state nor any field of its block changed between them
- **THEN** the second round posts nothing on `#12`
