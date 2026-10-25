# Spec Delta

## ADDED Requirements

### Requirement: A pull request's description holds its commits' messages whole
The description of a pull request `sluice submit` or `wt submit` opens
or updates SHALL hold the message of each commit between the branch it
lands on and the head submitted, whole and oldest first, a blank line
between them, followed by sluice's marks, whenever that fits GitHub's
limit of 65,536 characters (C.5 pull request).

#### Scenario: Every message fits
- **WHEN** an agent's branch holds three commits whose messages total a few hundred characters, and the agent runs `wt submit main --no-wait`
- **THEN** the pull request's description is the three messages, whole and oldest first, then the line naming the agent, and holds no line counting more commits

### Requirement: A description too long for GitHub ends with the rest's subjects
When the messages do not all fit, the description SHALL hold the first
messages, each whole, as many as fit; then a line giving the count of
the rest and the address of GitHub's compare view of the request's
branch against the branch it lands on; then each of the rest's subjects
in order; then sluice's marks. No message SHALL be cut part way, and
GitHub SHALL NOT refuse the description for its length (C.5).

#### Scenario: A bolt too long for one description
- **WHEN** an agent's branch holds three commits whose messages are 30,000 characters each, and the agent runs `wt submit main --no-wait`
- **THEN** GitHub accepts the pull request; its description holds the first two messages whole, then the line `1 more commit, each by its subject; all of them in full: https://github.com/<owner>/<repo>/compare/main...unit/<name>`, then `- ` and the third commit's subject, then the line naming the agent; and it is no longer than 65,536 characters

#### Scenario: One message longer than the limit
- **WHEN** an agent's branch holds one commit whose message is 70,000 characters, and the agent runs `wt submit main --no-wait`
- **THEN** GitHub accepts the pull request; its description holds no part of the message's body, the line `1 more commit, …` with the compare view's address, the commit's subject, and the line naming the agent

#### Scenario: More subjects than the limit holds
- **WHEN** an agent's branch holds 70 commits whose subjects are 1,000 characters each, and the agent runs `wt submit main --no-wait`
- **THEN** GitHub accepts the pull request; its description holds no message whole, the line `70 more commits, …` with the compare view's address, the first subjects in order as many as fit, each whole, then the line `and <m> more, in the compare view`, then the line naming the agent

### Requirement: A submit again writes the description for its new head
A submit that updates an open pull request SHALL write its description
as these requirements say for the head it submits, in place of the
description the request had, before that head is pushed, and SHALL leave
the description as it is when it already reads so.

#### Scenario: A new commit, submitted again
- **WHEN** an agent's pull request was opened at one commit, the agent commits again with the message `Add the second file`, and runs `wt submit main --no-wait`
- **THEN** the pull request's description holds both messages, oldest first, then the line naming the agent

#### Scenario: Submitted again at the same head
- **WHEN** an agent runs `wt submit main --no-wait` twice with no commit between
- **THEN** the second submit does not edit the pull request's description
