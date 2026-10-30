## ADDED Requirements

### Requirement: One query reads a branch's open pull requests whole
When the list of a branch's open pull requests came back changed, while
a pull request out of draft waits on its conditions, or when a pull
request has nothing kept, sluice SHALL ask GitHub's GraphQL API one query
for every open pull request into the branch, with its latest deciding
reviews, its head's checks and statuses, its ready marks and sluice's own
check runs, in place of a read of each per pull request (136; rule
core.32).

#### Scenario: Three pull requests, one query
- **WHEN** main has three open pull requests and its list comes back changed
- **THEN** one GraphQL query is made, and no REST read of any one's reviews, checks, timeline or check runs

#### Scenario: Reviews counted as before
- **WHEN** the query's reviews hold one reviewer's approval on an older head and another's on the head
- **THEN** the pull request counts one approval toward `reviews`

### Requirement: A refused query falls back to the reads per pull request
A query GitHub refuses other than for its allowance SHALL be followed,
for that pass, by the REST reads per pull request, said once (136).

#### Scenario: The query refused
- **WHEN** GitHub answers the query with an error that is not `RATE_LIMITED`
- **THEN** that pass reads each pull request by REST, and says so once
