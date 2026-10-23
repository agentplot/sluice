---
number: 49
title: A pull request a submit opened is a draft until its submitter acts
status: accepted
date: 2026-10-23
decision-makers:
- Chuck Swanberg
---

# A pull request a submit opened is a draft until its submitter acts

Deciders: the user, after a bolt's pull request landed on `main` without their word, 2026-10-23.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 120, 54, 67, C.5 pull request, S56; ADRs 0019, 0021; rules core.2, core.11, core.14.

## Context and problem statement

On 2026-10-23 a bolt's pull request into crew's `main`, opened by `sluice submit`, was refused for a conflict. A fix then landed on its branch and moved its head. Intake, which submits any pull request on a new head once its conditions hold (ADR 0021), took the new head in the name of the agent that had opened it, and landed it (afterthought/crew pull/92, submission 128). Nobody had asked for that landing. The same night, a pull request whose run timed out was told to submit the same head again, and that submit was answered with the earlier verdict, "nothing was queued or run again" (pull/93), because ADR 0021 submits a pull request sent back on the same head only after a draft and a ready mark on GitHub.

Clause 120 says sluice submits in a submitter's name only on that submitter's own act: a submit, or the operator's. A person's ready mark on a pull request they opened is a standing offer; an agent's submit stands for the head it named and no other. How does intake tell the two apart, hold the agent's pull request until its next act, and see that act when it comes?

## Considered options

1. A pull request whose body carries a submit's agent marker is the submit's. Once its latest submission leaves the queue without landing, intake holds it, on any head, until GitHub's timeline holds a ready mark at or after the second it left; while it is held, sluice marks it a draft. A submit takes intake's lock, withdraws the submission of an older head, and marks the pull request ready.
2. The draft alone is the hold: sluice marks the pull request a draft when its submission leaves, and intake takes it again once it is out of draft.
3. A new event in the core's log for a submit's act, which intake reads.
4. Tell the submit's pull request by its author, sluice's app.

## Decision outcome

Option 1.

**Whose pull request it is.** `sluice submit` writes `<!-- sluice: agent=… -->` into the body of the pull request it opens or updates (ADR 0021). That marker makes the pull request the submit's: a person's pull request that an agent later submits from the same branch takes the marker, because that submit is the act that now stands for it, and a person who deletes the marker makes it theirs again. A pull request with no marker is one a person opened by hand, and keeps ADR 0021's rules.

**The hold.** Intake reads, each pass, the latest submission from the pull request's place, finished or not, and the log line where it left the queue: withdrawn, blamed or conflicting. If it left without landing and no `ready_for_review` on GitHub's timeline is at or after that second, the pull request is held, on any head, the one sent back included. The log and GitHub both keep whole seconds, so the comparison is to the second, and a ready mark in the second the submission was made counts for nothing, since it is the act that made it. While it is held, intake submits nothing for it, posts its `sluice` check as `waiting: the next submit`, forecasts it as any waiting pull request, and marks it a draft if it is not one. Nothing new is stored: the hold is read from the log and the timeline (rules core.11, core.14), and the core is unchanged (rule core.2).

**The submit.** `sluice submit` and `wt submit` take the branch's intake lock, the file intake's passes take, waiting for it, so no pass reads the pull request halfway. Under it they push and open or update the pull request; withdraw, as its agent, the submission of an older head still in the queue and not landing (67); and, once a submission has left the queue or the pull request is a draft, mark it ready for review, a draft first if it is not one, since GitHub records a ready mark only on a draft. They read the timeline back, and mark it again the next second while the newest mark reads before the leaving, three times at most, for GitHub's clock may be behind the host's. A submit of the same head is then taken and tested again like any other, so the advice after a time-out is true, and the "nothing was queued or run again" answer goes. A submit of a new head on a pull request still queued is no leaving of the submitter's, so it is not held.

**GitHub's calls.** GitHub's REST API cannot change a pull request's draft state. The app posts one GraphQL mutation, `convertPullRequestToDraft` or `markPullRequestReadyForReview`, keyed by the node id REST gives each pull request, to `{api}/graphql` with its installation token, and reads an `errors` array as an error though GraphQL answers it with 200. The app's Pull requests: read and write covers both.

Option 2 makes a failed call the hold's undoing: one draft GitHub refuses after a withdrawal, and the next pass takes the new head in the agent's name, which is the incident again. With option 1 the draft only shows the hold, and a refused one holds it all the same while the next pass tries again. Option 3 puts a pull request's notion into the core's records, against rule core.2, when GitHub's timeline already holds the act. Option 4 reads wrong for a submit made on the operator's behalf, and for a hand-opened pull request an agent takes over.

### Consequences

- ADR 0021's rule for a pull request sent back, a new head or a draft and a ready mark, now holds only for one a person opened by hand, and holds as well for one the operator or a planner withdrew. Clause 120, as amended in blueprints a7d9646, makes that person's ready mark a standing offer that the operator's withdrawal rests: such a pull request rests until a new head, or a ready mark after a draft, as one sent back does, and its `sluice` check, `waiting: withdrawn by the operator`, names who withdrew it and how to bring it back (C.5 pull request). Intake reads who withdrew it from the submission's record, so nothing new is stored. A withdrawal intake makes itself, as the pull request's agent, when the person moves it to draft or closes it, is no one else's, and keeps ADR 0021's path.
- Marking a draft and then ready sends GitHub's ready-for-review notifications again, but only on a submit after the pull request left the queue, when its reviewers' attention is wanted anyway.
- A pull request sent back before this landed is held at intake's first pass after it, and marked a draft then. A rollback would take such pull requests again once they are out of draft; left drafts, they stay held through it.
- Each pass reads the timeline of every submit's pull request whose latest submission is out, one call each.
- A waiting submit whose pull request is held while it waits keeps waiting, saying `waiting: the next submit`, as one on a draft says `waiting: a draft`.
