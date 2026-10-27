## ADDED Requirements

### Requirement: A call refused on the app's token is made once more on a fresh one
A fetch, a listing or a push of sluice's that the remote refuses on the
app's token, as not found, permission denied or unauthenticated, before
it answers for any ref, SHALL be made once more on a token bought afresh
and five seconds old. A push SHALL be made again with the same refspec
and the same lease. This SHALL hold for every push: a landing, a branch
made or removed, a session's branch, a submitted head and the records'
refs (134).

#### Scenario: A landing's push refused on a token just bought
- **WHEN** a submission lands on a branch on GitHub and GitHub answers the push with 403 on a token it gave a moment before, then accepts the next
- **THEN** the push is made once more on a fresh token, the submission reads landed, and nothing reports a failure

#### Scenario: The second push carries the first's lease
- **WHEN** the first push of a landing is refused on its token and the branch moves on the remote before the second
- **THEN** the second push is refused as the branch having moved, the submission is not landed, and the next round forms the chain again on the revision found

#### Scenario: A bolt made on a refused token
- **WHEN** `sluice branch create bolt/x --from main` pushes, GitHub answers `Repository not found.` on the first token and accepts a fresh one
- **THEN** `bolt/x` exists on the remote at `main`'s revision and the command exits 0

#### Scenario: A pull request's branch pushed on a refused token
- **WHEN** a submit pushes the session's branch for its pull request and the first token is refused
- **THEN** the branch is pushed on a fresh token and the submit goes on

#### Scenario: A fetch refused on a token just bought
- **WHEN** a round fetches the branch and the remote answers not found on the first token and serves the branch on a fresh one
- **THEN** the round reads the branch and goes on

#### Scenario: The host's own credentials are not renewed
- **WHEN** the host has no `[forge]` table and the remote refuses a push on the host's own credentials
- **THEN** the push is made once only and fails with the remote's words

### Requirement: A push refused at the ref or left unanswered is not made again
A push the remote refused at the ref, its lease stale, the branch
protected or any ref rejected, SHALL be the move refused and SHALL NOT
be made again, whatever the remote said besides. A push whose answer
git did not read, a connection cut or a server error, SHALL NOT be made
again: what the remote holds decides it (24, 45, 122).

#### Scenario: A protected branch is not pushed twice
- **WHEN** a push to `main` is answered `[remote rejected] (protected branch hook declined)`
- **THEN** one push was made, and the move is reported refused in the remote's words

#### Scenario: A stale lease is not pushed twice
- **WHEN** a landing's push is refused because the remote's branch moved
- **THEN** one push was made, the refusal names the revision the remote holds, and no token is bought afresh

#### Scenario: A server error is not pushed again
- **WHEN** the remote answers a push with a server error
- **THEN** one push was made and the failure is reported in the remote's words

### Requirement: A refusal that stands names each refused token
A call refused on a fresh token too SHALL fail with the remote's own
words, whole, followed by what each refused token was: the app, the
repository, the installation it was bought from, the repositories and
permissions GitHub gave it, and its age, and never the token itself. It
SHALL NOT be made a third time.

#### Scenario: Refused twice
- **WHEN** GitHub answers a push with `Repository not found.` on the first token and on a fresh one
- **THEN** two pushes were made, and the failure carries `Repository not found.`, then says the remote refused the first token and then the fresh one, naming each one's app, repository, installation, scope and age

#### Scenario: No fresh token can be bought
- **WHEN** a push is refused on its token and GitHub refuses to issue another
- **THEN** one push was made, and the failure carries the remote's words, what the refused token was, and why none could be bought afresh
