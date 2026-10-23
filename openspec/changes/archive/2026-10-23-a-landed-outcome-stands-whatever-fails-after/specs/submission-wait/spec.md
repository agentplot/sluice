# Spec Delta

## ADDED Requirements

### Requirement: A landing stands whatever fails after it

A waiting `sluice submit` and `sluice wait` whose round fails SHALL, when
the submission's branch has moved to a revision holding its work, report
the submission as landed and exit 0, as when nothing failed (122). They
SHALL say what failed on stderr, on its own line beginning
`sluice: after the landing:`, and never as the outcome. A round that
fails while the submission's work is not on its branch SHALL exit 3.

#### Scenario: The release fails after the landing

- **WHEN** a submission's combination passes, the branch moves to it, and
  releasing the combination's checkout then fails in the round a waiting
  `sluice submit` ran
- **THEN** stdout ends `passed · landed as <rev>`, stderr has a line
  beginning `sluice: after the landing:` naming the failed release, and
  the command exits 0

#### Scenario: Telling the outcome fails after the landing

- **WHEN** a submission lands, and telling its session the outcome fails
  in the round `sluice wait <n>` ran
- **THEN** it reports the submission landed and exits 0, and stderr says
  the telling failed after the landing

#### Scenario: Under JSON

- **WHEN** a submission lands and releasing its combination's checkout
  fails in the round `sluice wait <n> --json` ran
- **THEN** stdout holds one document, the submission's record with
  `state` `landed` and its `landed_as`, stderr says what failed, and it
  exits 0

#### Scenario: The branch moved and the round stopped before recording it

- **WHEN** the round a waiting `sluice submit` ran moves the branch to a
  submission's landing and fails writing that it landed
- **THEN** the command reports the submission landed as the landing's
  commit and exits 0, under `--json` its record with `state` `landed` and
  that `landed_as`, and the next round records it landed

#### Scenario: A round that fails before the landing

- **WHEN** a waiting `sluice submit`'s round fails before its
  submission's branch has moved
- **THEN** it exits 3 with the cause on stderr, and the submission is not
  reported as landed

#### Scenario: A pull request's submission

- **WHEN** a waiting `sluice submit` follows a pull request whose
  submission lands, and releasing the combination's checkout then fails
  in the round it ran
- **THEN** it reports the submission landed, says what failed on stderr,
  and exits 0
