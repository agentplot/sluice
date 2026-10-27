## MODIFIED Requirements

### Requirement: A push refused at the ref or left unanswered is not made again
A push the remote refused at the ref, its lease stale, the branch
protected or any ref rejected, SHALL be the move refused and SHALL NOT
be made again, whatever the remote said besides. A push whose answer
git did not read, a connection cut or a server error after the push was
sent, SHALL NOT be made again: what the remote holds decides it (24, 45,
122). A server error the remote gave before the push was sent, while git
asked it for its refs, is no answer to the push, and the push is tried
again under the retry policy (134).

#### Scenario: A protected branch is not pushed twice
- **WHEN** a push to `main` is answered `[remote rejected] (protected branch hook declined)`
- **THEN** one push was made, and the move is reported refused in the remote's words

#### Scenario: A stale lease is not pushed twice
- **WHEN** a landing's push is refused because the remote's branch moved
- **THEN** one push was made, the refusal names the revision the remote holds, and no token is bought afresh

#### Scenario: A server error is not pushed again
- **WHEN** the remote answers a push with `RPC failed; HTTP 500` after the push was sent
- **THEN** one push was made, and what the remote holds of the branch decides whether the move was made

#### Scenario: A server error before the push is tried again
- **WHEN** the remote answers git's request for its refs with `The requested URL returned error: 503`, and accepts the next try
- **THEN** the move is made

### Requirement: A refusal that stands names each refused token
A call the remote refused on sluice's token on every try until the
retry policy's time is spent SHALL fail with the remote's own words,
whole, followed by what each refused token was, in the order tried: the
app, the repository, the installation it was bought from, the
repositories and permissions GitHub gave it, and its age, and never the
token itself.

#### Scenario: Refused twice
- **WHEN** GitHub answers a push with `Repository not found.` on the token held and on a fresh one, and the policy's time is spent before another try
- **THEN** two pushes were made, and the failure carries `Repository not found.`, then says the remote refused the first token and then the fresh one, naming each one's app, repository, installation, scope and age

#### Scenario: No fresh token can be bought
- **WHEN** a push is refused on its token and GitHub refuses to issue another
- **THEN** one push was made, and the failure carries the remote's words, what the refused token was, and why none could be bought afresh

## REMOVED Requirements

### Requirement: A call refused on the app's token is made once more on a fresh one
**Reason**: Clause 134 and ADR 0062 put every call to a remote under one
retry policy, which tries a call refused on the app's token again on a
fresh token until the policy's time is spent, not once.
**Migration**: The behaviour is the `remote-retry` capability's: its
requirements that a credential the remote refused is bought afresh for
the next try, and that a push is tried again only with the first's
lease.
