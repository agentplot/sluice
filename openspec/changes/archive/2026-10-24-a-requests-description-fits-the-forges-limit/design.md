# Design

## Context

`open_group` in `crates/sluice/src/profiles/github/pulls.rs` is where a
submit opens or updates its pull requests, one worktree or a group
(125). For a new request it reads `messages(tip, head)`, the commit
messages of `tip..head` oldest first, joins them with blank lines,
appends the marks with `with_marks` (`<!-- sluice: agent=… -->`, and for
a group `<!-- sluice: group=… -->`), and posts the result as the body.
Nothing measures it, so a long branch gets GitHub's `422 Validation
Failed` (signal 2026-10-24-madswan-ops-ead6f322). For an open request it
edits the body only when the marks differ, keeping the rest of the body
as it was; group submit edits it before the push so a request is never
at a head its mark does not name (125).

GitHub's REST reference does not document the limit. Its error names it,
`body is too long (maximum is 65536 characters)`, and the community
reference of GitHub's limits (dead-claudia/github-limits, "PR body")
records it as 65,536 code points, verified against `PATCH
/repos/{owner}/{repo}/pulls/{number}`. The same reference records
comments' limit in bytes, 262,144, which is a different field.

The forge profile reads only github.com repositories (`Repo::from_url`),
so GitHub's compare view is `https://github.com/<owner>/<repo>/compare/<base>...<head>`.

## Goals / Non-Goals

**Goals:**

- One function builds a request's description from the messages, the
  marks and the compare address, measured against the limit, used for
  an opened request and an updated one alike.
- The GitHub fake refuses what GitHub refuses, so the tests prove the
  limit holds rather than assume it.

**Non-Goals:**

- Trimming the title, the report comment or the check runs' summaries.
- A limit read from configuration: GitHub's is fixed, and no other forge
  has a profile.

## Decisions

### The description, laid out

```
<message 1>

<message 2>

…

<n> more commits, each by its subject; all of them in full: https://github.com/<owner>/<repo>/compare/<branch>...<head branch>

- <subject>
- <subject>
and <m> more, in the compare view

<!-- sluice: agent=<agent> -->
<!-- sluice: group=… -->
```

The section after the messages appears only when some message is left
out; `1 more commit` in the singular. The `and <m> more` line appears
only when not every subject fits. The marks stay last, in their present
form, so `marked_agent` and `Group::of` read them unchanged.

A message is `git log --format=%B` trimmed, as `messages` reads it now;
a subject is its first line. The compare view names the head by its
branch, the request's `head_ref`, not by its revision, so it keeps
showing the request's commits after a later submit; the description is
rewritten at each submit anyway.

### How much fits

The limit, `DESCRIPTION_LIMIT = 65_536`, is a constant of the GitHub
profile, measured with `str::chars().count()` over the whole body
posted, marks and separators included.

1. If every message fits, the body is every message and the marks.
2. Otherwise take the largest `k` such that the first `k` messages, the
   count line for the other `n - k`, all their subjects and the marks
   fit. The whole messages take the room the design gives them, and the
   rest are each named.
3. When even `k = 0` cannot hold every subject, list the subjects in
   order, each whole, as many as fit with the `and <m> more` line, and
   no message whole.

Alternatives considered: counting UTF-8 bytes, which can never exceed
the limit however GitHub counts but leaves out messages GitHub would
hold whenever the text is not ASCII; the design asks for as many as the
limit holds, and the verified count is code points. Cutting the last
message to fill the room: the design forbids it. Putting the marks
first: they are HTML comments, invisible either way, and moving them
would change every request's body for nothing.

### An update writes the whole description

A submit that finds an open request from its branch builds the
description for the head it submits and edits the body when it differs,
before the push, where today it edits only the marks. So the order 125
needs holds (the mark names the head before the head arrives), and the
description holds the request's current commits as C.5 says, not the
ones it was opened with. A request a submit updates that a person opened
by hand gets the same description, its own text replaced: the design
says what the description carries, and that request is now the agent's,
as its agent mark already says. A request no submit touches is never
written.

The head's messages are readable before the push: the head is in the
repository's own objects (`self.dir`), or, for a branch sluice lands on,
fetched by `submitted_head` before `open_group` reads it.

### The fake refuses as GitHub does

`crates/sluice/tests/profiles/forge/github.rs` answers `POST /pulls` and
`PATCH /pulls/<n>` with a body over 65,536 code points with `422`,
`{"message": "Validation Failed", "errors": [{"resource": "PullRequest",
"field": "body", "code": "custom", "message": "body is too long (maximum
is 65536 characters)"}]}`, and stores nothing. The tests then fail the
way the bolt did if the description is not measured.

## Risks / Trade-offs

- [GitHub counts differently from the community reference, in UTF-16
  units say] → A body full of characters outside the basic plane could
  be refused. Commit messages are nearly all ASCII; the error, if it
  comes, is the 422 the submit reports today, and the constant and its
  measure sit in one function to change.
- [A person's text in a request a submit updates is replaced] → The
  pull-requests guide says so; a person who wants their own words keeps
  them in a comment, or opens the request by hand and lets the round
  take it without a submit.
- [The description rewritten at every submit with new commits] → One
  `PATCH` per submit that changes the head, none when it reads the same.

## Open Questions

- Whether GitHub counts 65,536 code points or UTF-16 units is not in its
  documentation; a live probe with a body of 65,536 astral characters
  would settle it. It changes only the measure in one function.
