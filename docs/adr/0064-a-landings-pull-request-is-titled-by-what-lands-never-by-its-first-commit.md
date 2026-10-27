---
number: 64
title: A landing's pull request is titled by what lands, never by its first commit
status: proposed
date: 2026-10-27
decision-makers:
- Chuck Swanberg
---

# A landing's pull request is titled by what lands, never by its first commit

Status: proposed to the user on 2026-10-27, as `briefs.15` has it; accepted only on their yes.

Deciders: the user, whose word of 2026-10-27 swancloud-design relayed (afterthought/crew-state, swancloud/main, signal 2026-10-27-swancloud-design-60cb0ec3/01): "do both: a landing's pull request is titled by what lands, the bolt branch's name or the submission's own subject, never its first commit"; the shape is madswan-design's, and the user's yes is asked on the record's card, since the word reached this record through another agent.
Sources: statement clauses 118, 120 and the C.5 pull request row; records 0021, 0025, 0052; `crates/sluice/src/profiles/github/pulls.rs` on main at 2026-10-27, where a request sluice opens takes the first line of the first commit between the branch's tip and the head, and the head's ref where there is none; WilldanGroup/switchboard-kit pull requests 17, 31 and 38.

## Context and problem statement

When a branch's policy turns pull requests on, `submit` pushes the session's branch and opens a request, which sluice lands once its conditions hold. The request's title is the first line of the first commit the branch carries beyond its base. Under crew every bolt branch begins with the construct's commit, `docs(openspec): propose <unit>`, so a landing that carries 1,577 lines of TypeScript reads in the forge's list as a documentation change (switchboard-kit 38, 31, 17). The title names what the branch began with, not what lands.

## Considered options

1. The first line of the first commit beyond the base, as built.
2. The submitted branch's name.
3. A subject the submit gives, in the submitter's words.
4. Both: the subject the submit gives, and the branch's name where it gives none; never a commit's message.

## Decision outcome

Option 4, the user's "do both".

**A title names what lands.** A request is the forge's row for a landing, read in a list beside every other; its title is the one line a reader has before opening it. What lands is the branch whole, which the branch's name says under crew (`bolt/<bolt>`, `fix/<bolt>/<name>`) and a person's session may say better in a subject of its own. A commit's first line says what one commit did, and the first commit of a branch least of all says what the branch became.

**The submit gives it, or the branch does.** A subject is the submitter's word, given with the submit as the entry's other facts are (120), and carried to the request as written, in no other voice (core.26 for the words sluice adds). Where the submit gives none, the branch's name is the title, which is always there and always names the thing. The body keeps the commits the request carries, where a reader looks for them.

**A title is set when the request is opened, and changed with a new subject.** A request sluice opened and updates on a new head keeps its title unless the new submit gives a subject; a request a person opened by hand keeps the title the person gave it, which sluice never rewrites (120).

**Why not the others.** Option 1 is the row that reads as a docs change. Option 2 alone leaves a person's session with a branch name as its only word, where a subject would say more. Option 3 alone leaves a request with no title when a submit gives none, which every landing under crew would.

### Consequences

- The statement's C.5 pull request row and clause 118 say a request sluice opens is titled by the subject the submit gives or the submitted branch's name, never a commit's message.
- `submit` takes a subject, and a request it opens or updates carries it as written; a request opened with none is titled by its branch.
- Requests already open keep their titles; the next head with a subject changes one sluice opened.
