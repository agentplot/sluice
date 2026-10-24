---
number: 54
title: A group submit to pull requests takes its requests itself, or leaves them drafts
status: accepted
date: 2026-10-24
decision-makers:
- Chuck Swanberg
---

# A group submit to pull requests takes its requests itself, or leaves them drafts

Deciders: the user, amending clause 125 in blueprints 763d21b, and building the OpenSpec change `a-group-submit-meets-pull-requests`.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 6, 62, 67, 120, 125 and C.5 pull request; ADRs 0048, 0052; rules core.2, core.4, core.6, core.14.

## Context and problem statement

Clause 125, as amended, says what a group submit does on a branch that takes pull requests: the act marks each head's request ready at that head as one submit does; a head whose request cannot be taken, from a fork or short of its conditions, refuses the whole act, naming it; and once taken, a new head on one request withdraws that entry alone and holds that request as a draft.

Under ADR 0048 the act opened every request ready, marked one of the group, and answered with what each waited for; intake took the group whole whenever all of it was ready. An act short of a review so stood in its submitter's name for as long as the review took, which 120 rules out: a submit stands for the heads it named when it was made, and a round that takes them later, on a reviewer's word, takes them on no act of the submitter's. ADR 0048 chose that because the forge seam had no call to mark a request ready; ADR 0052 gave it one, and one to mark a request a draft.

When does the act read the requests' conditions, who queues the group, what does a refused act leave behind, and how is a new request opened?

## Considered options

For when the act reads the conditions:

1. After it pushes every branch and opens or updates every request, under intake's lock, reading each request on the head it pushed as out of draft.
2. Before anything is pushed, so a refused act pushes nothing, as a refused worktree does.

For who queues the group:

3. The act, in one write, in the order given, while it holds intake's lock.
4. Intake's next pass, which takes the group whole by its marks once the act has marked each ready.

For a refused act:

5. Every request of the act left a draft at its head, its group mark kept.
6. Every request left ready, bound by the group's marks, as under ADR 0048.

For a request the act opens:

7. Opened a draft.
8. Opened ready, and marked a draft when the act is refused.

## Decision outcome

Options 1, 3, 5 and 7.

**The read.** The act reads each request's conditions with the same reading intake uses: reviews approving its head, and each check the policy names passing on it. Out of draft is the act's to give, not a condition, so the act reads a request it just opened as a draft as it would read it out of draft. Option 2 refuses what nobody could yet review: GitHub keeps a review on the commit it was given on the request, and a head not yet on its request has none, so on a branch with `reviews = 1` every act of new heads would be refused before any request existed, and the submitter would have to open drafts by hand to get them reviewed. Reading after the push lets the refused act leave the drafts a reviewer needs.

**The taking.** When every request meets its conditions, the act marks each ready as one submit does (ADR 0052), a request sluice held as a draft included, and then, still holding the branch's intake lock, calls one `Sluice::submit_group` over the checkouts `pull/<n>` in the order given. What the act read is what it takes: no intake pass runs between the read and the write, and no round can form a combination on part of the act (62; rules core.6, core.14). The act answers with each submission beside its pull request. Under option 4 a check that changed between the act's read and intake's pass would leave the act answered as taken and its requests waiting, and the act would be neither the taking nor the refusal 125 says it is.

**The refusal.** When any request falls short, the act marks none ready and queues nothing; a request it opened is a draft already, and one it updated that is ready is marked a draft. So no round takes any of them, by its conditions or by its group's marks, in the submitter's name (120). The act names each request short of its conditions with its worktree and what it lacks, says every request of it is a draft, and exits 2. The group mark stays at the head pushed: the same act run again once the reviews and checks are in finds them met and takes all, and a person who marks every request ready offers them, which intake takes whole by ADR 0048's group binding. Option 6 is the act standing until the review comes, which 120 and 125 rule out.

**The draft.** A request the act opens for more than one worktree is opened a draft, GitHub's `draft` on the call that opens it, and a single submit opens its request ready as before. A taken act marks its drafts ready, and GitHub's timeline records the mark, which ADR 0052's hold reads. Option 8 sends reviewers the notifications of a request ready for review for an act that may be refused, and marks every refused request twice.

The refusal is the pull-request seam's: what a request lacks is the driver's to read and the CLI's to say, and no type of the core names a request or its conditions (rule core.2). The act pushes only its sessions' branches, which sluice lands on nowhere (rule core.4). A fork's request is never the act's: the act opens or updates only a request from the branch's own repository, so a fork's request on a branch of the same name is neither edited nor marked nor queued.

### Consequences

- Once taken, a request of the act is a submit's request as any other: a new head on one withdraws its submission alone, and ADR 0052's hold marks it a draft until its worktree submits again or a person marks it ready; the others stay queued and land without it (120, 125).
- A refused act still pushes its branches and moves its requests' heads. A queued submission on an older head is withdrawn, as intake would withdraw it once the head moved (67), and the act says so for each.
- The act run again over requests already queued at those heads queues them again in one write, the earlier submissions withdrawn (67), as the act does on a branch without pull requests.
- GitHub lets a reviewer review a draft. Whether an approval given on a draft counts as one on a ready request is a reading to take on a live repository; if it does not, the refused act's advice becomes marking the requests ready after review, which intake's group binding takes whole.
- C.5's worktrunk row still says a group under pull requests answers with every request and what it waits for; this record follows 125, and the statement reconciles the row.

## More information

Narrows ADR 0048 under pull requests: its option 5, intake taking a marked group whole, stands for requests a person marks ready or opens by hand, and its reason against option 6, a call the forge seam lacked, no longer holds since ADR 0052.
