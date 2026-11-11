## MODIFIED Requirements

### Requirement: Nothing of a release reaches main, and no tag moves

The release job SHALL push the tag and the `release` branch in one
atomic push, so both move or neither does. The tag SHALL be pushed with
no force. `release` SHALL be forced to the release commit, the commit
the tag names, since each release commit's parent is main's pushed
commit and not the release commit before it. The job SHALL push no
other ref. It SHALL publish the GitHub release at that tag, naming no
other commit for it. Nothing SHALL be committed to main, and no tag
SHALL be moved.

#### Scenario: The tag and the release

- **WHEN** `root:release` reads `.github/workflows/ci.yml`
- **THEN** its `release` job's one push is
  `git push --atomic origin "refs/tags/$TAG" "+HEAD:refs/heads/release"`,
  with no `--force`, no `+` on the tag and no branch but `release`
- **AND** it runs `gh release create "$TAG" --verify-tag`, with no
  `--target`

#### Scenario: A push that moves the tag alone

- **WHEN** the `release` job's push is `git push origin "refs/tags/$TAG"`
- **THEN** `sh check-release.sh` fails with a line starting
  `release: release: `, saying the job's one push is the tag and
  `release`, atomically

#### Scenario: A push that forces the tag

- **WHEN** the `release` job's push names `"+refs/tags/$TAG"` or
  `--force`
- **THEN** `sh check-release.sh` fails, naming the push
