## ADDED Requirements

### Requirement: sluice's check names the host that posts it and the pull request's submission there
Every `sluice` check sluice posts on a pull request's head SHALL begin
its summary with `Submission <n> on <host>.` where that host's records
hold a submission of the pull request at that head, and otherwise with
`Waiting to be taken in on <host>.`, `<host>` being the host that posts
it. Its title SHALL stay as the state it shows (137, C.5).

#### Scenario: Queued behind another
- **WHEN** host `b` takes main's pull requests in and holds `#12` as submission 1406, queued behind `#10`'s submission 1405
- **THEN** `#12`'s `sluice` check is titled `queued behind #10`, and its summary begins `Submission 1406 on b.`

#### Scenario: Waiting for a review
- **WHEN** host `b` takes main's pull requests in, and `#13` is out of draft and needs one review
- **THEN** `#13`'s `sluice` check is titled `waiting: needs 1 review`, and its summary begins `Waiting to be taken in on b.`

### Requirement: sluice's check carries the pull request's queue as fields
Every `sluice` check SHALL end its summary with one hidden block,
`<!-- sluice:queue {…} -->`, whose JSON has `host`; `taken_in`;
`submission` and `state`, the submission's number and its state as its
record names it, or `null` while not taken in; `ahead`, each submission
ahead of it as `{"submission", "pull_request"}`; and `waits_for`, what
it lacks while not taken in. A changed field SHALL be posted, and an
unchanged block posts nothing (137, C.5; rule core.32).

#### Scenario: A queued submission's block
- **WHEN** host `b` holds `#12` as submission 1406, queued behind `#10`'s submission 1405
- **THEN** the summary of `#12`'s `sluice` check ends with `<!-- sluice:queue {"host":"b","taken_in":true,"submission":1406,"state":"queued","ahead":[{"submission":1405,"pull_request":10}],"waits_for":[]} -->`

#### Scenario: A waiting pull request's block
- **WHEN** host `b` takes main's pull requests in, and `#13` needs one review and `qa/approved`
- **THEN** its block reads `"taken_in":false`, `"submission":null`, `"state":null`, `"ahead":[]` and `"waits_for":["1 review","qa/approved"]`

#### Scenario: The queue ahead moves
- **WHEN** `#10`'s submission 1405 lands and `#12`'s submission 1406 is next on main
- **THEN** `b` posts `#12`'s check again, titled `queued: next on main`, its block's `ahead` empty

#### Scenario: Nothing in it changed
- **WHEN** two rounds on `b` run and neither `#12`'s state nor any field of its block changed between them
- **THEN** the second round posts nothing on `#12`
