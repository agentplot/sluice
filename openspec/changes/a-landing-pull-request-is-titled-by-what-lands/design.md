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

The rules that bind: 118 and the C.5 pull request row (the title), 120
(a person's request is their own act, which sluice never rewrites),
core.26 (the subject is the submitter's words, carried as written; the
lines sluice adds speak to any operator), core.29 (every call to GitHub
runs under the one retry policy), core.4 (nothing here writes a branch
sluice lands on; the subject changes no push).

## Goals / Non-Goals

**Goals:**

- The title a request is opened with is the subject or the branch's
  name, and nothing else.
- A request sluice opened is retitled by a submit's subject, in the
  same write as its description.
- A request a person opened is never retitled.

**Non-Goals:**

- Recording the subject on the submission or in the branch's records:
  only the request carries it (record 0064, Consequences).
- Using the subject on a branch with no pull requests, such as in a
  landing commit's message.
- A subject per request of a group submit.

## Decisions

### `--subject` on `sluice submit`, forwarded by `wt submit`

`Command::Submit` gains `subject: Option<String>`, `--subject`, with
clap's `conflicts_with_all = ["check", "status"]`. The check that it is
one non-empty line, and that it comes with one `--worktree`, sits beside
the existing several-worktrees check in `run`, printing and returning
`USAGE` (64) before anything is read from a remote. `wt submit`
forwards every argument but `--no-remove` (`worktrunk::Submit::parse`),
so it takes `--subject` with no change; a test holds that.

The subject reaches the driver as a new argument of
`PullRequests::open`. `open_group` takes no subject; `open` passes it
through to the one-worktree path of the shared body (an
`Option<&str>` parameter of a private helper both call, so the group
path passes `None`).

Alternative: one `--subject` repeated per worktree for a group. Not
taken: the statement gives no rule for it, and 125's group is one act
whose requests a reader reads together; the design should say what a
group's titles are before sluice invents them.

### The title on opening

`title = subject.unwrap_or(head_ref)`. The first-message lookup goes.
`head_ref` is `session.from.worktree`, the branch pushed or, for a
branch sluice lands on, the branch as its remote holds it (124), which
is the submitted branch's name in both cases.

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
`open_group`, for an open request: the new title is the subject when
the submit gives one, the request's author is sluice's login and the
subject differs from its title; the edit is made when the body or the
title differs, before the push, as the description's edit is today.
The `Pull` the act goes on with carries the new title, so the line
`updated #<n>: <title>` names it.

### A subject on a branch with no pull requests

The local `submit` path takes the subject only to add one warning to
the accepted submission's warnings: `<branch> opens no pull request;
the subject titles nothing`. It prints as every warning does, a
`warning:` line, or in `warnings` under `--json`. Refusing it would
make a driver that always passes a subject know which branches take
pull requests, which the submit already knows.

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
  subject, since a cut subject is not as written.

## Migration Plan

None. Requests already open keep their titles until a submit with a
subject updates one sluice opened. Rolling back restores the
first-commit title on requests opened afterwards and leaves every
title as it stands.

## Open Questions

- What a group submit (125) titles each of its requests with, beyond
  its branch's name: one subject for all, one per worktree, or none.
  Until then `--subject` is refused with several worktrees.
- Whether a subject on a branch with no pull requests should go
  somewhere, such as the landing commit's message, or be refused.
  Until then it is accepted with a warning.
