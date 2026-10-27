## Context

See proposal.md for why. Today, in `open_group`
(`crates/sluice/src/profiles/github/pulls.rs`), a request that is not
yet open is opened with the first line of the first message of
`tip..head`, or the head's ref where there is none; a request that is
open has only its description rewritten (`Forge::edit` patches the body
alone). Nothing tells a request sluice opened from one a person opened
except the agent mark in the body, and a submit writes that mark into a
person's request too when it updates one, so the mark cannot say who
opened it.

A landing commit's first line is the policy's `[landing] subject`,
`chore(land): #{id} {from}` by default, with `{from}` filled from the
submission's place (`sluice-core/src/text.rs`, `landing_message`); the
`Sluice-From` trailer names the agent and the same place. A submission
taken from a pull request has the place `pull/<n>`, so this repository's
own landings read `chore(land): #212 pull/121`. A submission keeps no
word of the submitter's.

The rules that bind: 118 and the C.5 pull request row (the title), 23
(the landing commit names the entry's work by its subject or its
branch), 125 (a subject per head, or none), 120 (a person's request is
their own act, which sluice never rewrites), core.26 (the subject is
the submitter's words, carried as written; the lines sluice adds speak
to any operator), core.29 (every call to GitHub runs under the one
retry policy), core.4 (nothing here writes a branch sluice lands on;
the subject changes no push), core.11 and core.14 (the subject a later
round's landing reads is in the records).

## Goals / Non-Goals

**Goals:**

- The title a request is opened with is the subject or the branch's
  name, and nothing else.
- A request sluice opened is retitled by a submit's subject, in the
  same write as its description.
- A request a person opened is never retitled.
- A group submit gives each head its own subject, or none.
- A landing commit names its entry's work by the subject its submit
  gave, or its branch, on every branch, with or without pull requests.

**Non-Goals:**

- Showing the subject in `sluice status`, the overview or the home
  screen.
- A subject on a submission queued before this change: it lands named
  by its branch.

## Decisions

### `--subject` on `sluice submit`, repeated for a group

`Command::Submit` gains `subject: Vec<String>`, `--subject`, with
clap's `conflicts_with_all = ["check", "status"]`. Beside the existing
several-worktrees check in `run`, before anything is read from a
remote: each subject is one non-empty line, and their count is zero or
the count of worktrees (one where no `--worktree` is given); anything
else prints why and returns `USAGE` (64). The *n*th subject is the
*n*th worktree's (125). `wt submit` forwards every argument but
`--no-remove` (`worktrunk::Submit::parse`), so it takes `--subject`
with no change; a test holds that.

Alternative: one subject for a whole group, applied to each head. Not
taken: 125 refuses it, since a title names what each head lands.

### The subject on the submission

`Submission` gains `subject: Option<String>`, with
`#[serde(default, skip_serializing_if = "Option::is_none")]`, so a
record written before reads as none and one with none writes as
before. `SubmitRequest` gains `subject: Option<String>` and
`GroupRequest` gains `subjects: Vec<String>`, empty or one per
worktree; `submit_group` refuses any other count as a caller's error
(`bail!`, the CLI having refused it first) and writes each subject on
its submission in the one write of the act. A submission adopted from
another host's records (`adopt_left_behind`) keeps it through `..old`.

The subject is in the branch's records because the landing that reads
it is a later round's, perhaps another process's or another host's
(core.11, core.14).

### The landing commit names the work by `{from}`

`landing_message` fills `{from}` with the submission's subject where it
has one, and its place, the branch, where it has none. The
`Sluice-From: <agent> (<place>)` trailer is unchanged, so the branch
stays named. The `[landing] subject` row in the configuration reference
says `{from}` is what lands: the submit's subject, or the agent's
branch.

Alternative: a new `{subject}` placeholder, with the default changed to
use it. Not taken: every policy that sets `subject = "chore(land):
#{id} {from}"` itself, as the walkthrough shows, would drop the subject
the submit gave, which 23 says is neither refused nor lost.

### A submission taken from a pull request keeps the request's title

Intake takes a request as a submission with the place `pull/<n>` (its
single path and both group paths, `SubmitRequest` and `GroupRequest` in
`pulls.rs`); each now carries the request's title as its subject. A
request sluice opened is titled by the subject its submit gave or its
branch, so the landing commit names it as 23 asks; a request a person
opened carries the title they gave with their own act (120).

### The title on opening

`title = subject.unwrap_or(head_ref)`. The first-message lookup goes.
`head_ref` is `session.from.worktree`, the branch pushed or, for a
branch sluice lands on, the branch as its remote holds it (124), which
is the submitted branch's name in both cases. `open` passes its one
subject to the shared body, `opening`; `open_group` passes one per
worktree, or none, and each request is opened or updated with its own.

### Telling a request sluice opened: its author

A request sluice opened is one whose author is sluice's own login on
GitHub. sluice opens every request with its app's installation token,
so GitHub records the app's bot user as its author, which is
`<slug>[bot]`. `Forge` gains `fn login(&self) -> Result<String>`: `App`
reads its `slug` once per process from `GET /app`, which takes the
app's JWT (GitHub REST, "Get the authenticated app": "You must use a
JWT to access this endpoint"; the response carries `slug`), through the
same `send`/`call` path, and so the same retry policy (core.29), as
every other call; `FileForge` answers `sluice`, the author it gives
every request it opens. The login is read only when a submit with a
subject updates an open request, so no other submit makes the call.

Alternatives:

- A mark in the body written on opening (`<!-- sluice: opened -->`).
  Not taken: a request sluice opened before this change carries none, so
  record 0064's "the next head with a subject changes one sluice
  opened" would not hold for it; and the body is rewritten on every
  submit, so the mark would have to survive every rewrite.
- The agent mark already in the body. Not taken: a submit writes it
  into a person's request when it updates one, so it does not say who
  opened the request.
- `user.type == "Bot"`. Not taken: another app's request is a bot's
  too, and is no request sluice opened.

### Retitling in the description's write

`Forge::edit` becomes `edit(repo, number, title: Option<&str>, body:
&str)`: one `PATCH /repos/{owner}/{repo}/pulls/{number}` carrying
`title` when given and `body` (GitHub REST, "Update a pull request":
`title` and `body` are both optional fields of the one request). In
`opening`, for an open request: the new title is the head's subject
when the submit gives one, the request's author is sluice's login and
the subject differs from its title; the edit is made when the body or
the title differs, before the push, as the description's edit is
today. The `Pull` the act goes on with carries the new title, so the
line `updated #<n>: <title>` names it.

## Risks / Trade-offs

- [The bot login is not `<slug>[bot]` on some GitHub] → The forge test
  that answers `GET /app` checks the login sluice builds, and a request
  sluice opened whose author does not match is only left with its
  title, never retitled wrongly. One read on GitHub itself settles it:
  the `user.login` of a request sluice's app opened, beside the app's
  `slug`.
- [`GET /app` fails] → The submit fails as any refused forge call does,
  with GitHub's words, after it has written nothing: the login is read
  before the edit.
- [A long subject] → GitHub refuses a title it will not hold with its
  own words, and the submit reports them; sluice does not cut a
  subject, since a cut subject is not as written. On a branch with no
  pull requests the landing commit's first line carries it whole.
- [A policy's landing subject names no `{from}`] → Its landing commits
  name no subject, as they name no branch today; the subject stays on
  the submission's record.

## Migration Plan

None. Records written before read with no subject. Requests already
open keep their titles until a submit with a subject updates one sluice
opened. Rolling back restores the first-commit title on requests opened
afterwards and the place in landing commits; a record holding a
subject is still read, the field ignored.

## Open Questions

- Whether a policy whose `[landing] subject` names no `{from}` should
  have the subject put somewhere else in the landing commit, so that it
  is not lost there (23). Until then such a policy's landings name
  neither the subject nor the branch in their first line.
