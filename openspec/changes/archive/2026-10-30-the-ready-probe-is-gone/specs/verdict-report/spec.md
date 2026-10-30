## MODIFIED Requirements

### Requirement: The report lists each required check

For each required check of the combination, on each system the branch
names, the report SHALL give the check's name, system and key, and one
of these results:
- passed;
- failed;
- flaky, with the result it contradicts (19);
- could not run, with why and in how many runs in a row (114, 115);
- missing;
- set aside, with its reason (116).

#### Scenario: Two systems

- **WHEN** `main` needs `cli:build` on `linux-x64` and `macos-arm64`, and a combination passed it on `linux-x64` and failed it on `macos-arm64`
- **THEN** the report lists `cli:build` twice, passed on `linux-x64` and failed on `macos-arm64`

#### Scenario: Could not run

- **WHEN** `site:check` could not run in three runs in a row because it exited with a code the policy's `not_run` lists
- **THEN** the report lists `site:check` as could not run, naming the exit and the three runs, not as missing

#### Scenario: Flaky

- **WHEN** `app:e2e` failed on the combination and a pass for the same key is recorded, and the policy reads a flaky key as not passed
- **THEN** the report lists `app:e2e` as flaky, naming the pass and when it was recorded
