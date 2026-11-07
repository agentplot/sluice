## MODIFIED Requirements

### Requirement: A pull request a submit opened comes back only on its submitter's act
A pull request a submit opened or updated is one whose description
names its agent. When its submission is withdrawn, blamed, timed out or
removed for a conflict, sluice SHALL mark it a draft. No round SHALL
submit it again until a submit marks it ready for review, or a person
does on GitHub, after the submission left. That holds on any head,
the one sent back included. A withdrawal for a hand-over of intake
holds none of this (120, 137).

#### Scenario: Refused, then its branch moves
- **WHEN** a pull request `wt submit` opened is removed for a conflict, and another submission lands a fix on its branch so its head moves
- **THEN** it is a draft, no round submits it, and nothing lands in its agent's name

#### Scenario: Blamed, then submitted again on the same head
- **WHEN** a pull request `wt submit` opened is blamed, and its agent runs `wt submit` again with no new commits
- **THEN** the next round submits the same head and its checks run again

#### Scenario: Timed out, then submitted again on the same head
- **WHEN** a pull request `wt submit` opened is blamed because the branch's time limit ended its run, and its agent runs `wt submit` again with no new commits
- **THEN** the next round submits the same head and its checks run again

#### Scenario: Marked ready by a person
- **WHEN** a pull request `wt submit` opened was sent back and marked a draft, and a person marks it ready for review on GitHub
- **THEN** the next round submits it once its conditions hold

#### Scenario: Withdrawn by the operator
- **WHEN** the operator runs `sluice withdraw` on the submission of a pull request `wt submit` opened
- **THEN** sluice marks the pull request a draft and no later round submits it until a submit or a person marks it ready

#### Scenario: Held when it cannot be marked a draft
- **WHEN** a pull request `wt submit` opened is sent back and GitHub refuses to mark it a draft
- **THEN** no round submits it, its `sluice` check says it waits for the next submit, and each later round tries the draft again

#### Scenario: Withdrawn for a hand-over
- **WHEN** the submission of a pull request `wt submit` opened is withdrawn because another host takes the branch's pull requests in
- **THEN** sluice marks it no draft, and the host that holds the mark takes it in again as its conditions hold
