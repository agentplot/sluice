# Tasks

Each group's tests run in the gate's moon tasks, `cli:test` and
`cli:tools` (`crates/sluice/moon.yml`); while iterating, run one
with `sluice exec main -- cargo test -p sluice --features tools --test
<file> <name>`, and before submitting the gate whole with `sluice exec
main`.

## 1. The forge carries a title and names its own login

- [x] 1.1 `Forge::edit` takes `title: Option<&str>` beside the body; `App::edit` (`rest.rs`) sends one `PATCH /repos/{owner}/{repo}/pulls/{number}` with `title` only when given, and `FileForge::edit` (`fake.rs`) sets it; update every caller to pass `None` for now. Verify: a unit test in `rest.rs` beside `opening_a_draft_or_ready` that the request body holds `title` only when given, and `crates/sluice/tests/profiles/forge.rs`'s edit check extended to read back a changed title, both green in `cli:test` and `cli:tools`.
- [x] 1.2 `Forge::login`: `App` reads `GET /app` with its JWT once per process through its one call path (core.29) and answers `<slug>[bot]`; `FileForge` answers `sluice`, the author `open` gives every request it opens. Verify: in `crates/sluice/tests/profiles/forge.rs`, the fake server answers `GET /app` with a `slug`, the test asserts the login and that a second `login` makes no second call, green in `cli:tools`.

## 2. `--subject` on the command line

- [x] 2.1 `Command::Submit` takes `--subject <text>`, conflicting with `--check` and `--status`; beside the several-worktrees check in `run`, an empty subject, one holding a line break, or one given with more than one `--worktree` prints why and returns `USAGE`, before anything is read or pushed. Add `$ sluice submit main --no-wait --subject "Batch review cards"` to `SUBMIT_AFTER_HELP`. Verify: tests in `crates/sluice/tests/cli.rs` for each refusal (exit 64, nothing pushed or opened, the reason named), green in `cli:test`.
- [ ] 2.2 `PullRequests::open` takes the subject, `Option<&str>`, and `run` passes it on the one-worktree pull request path; `open_group` passes none. The `()` impl and the test double in `crates/sluice/tests/cli.rs` (`Pull7`) take it. Verify: `cargo check -p sluice --all-targets` clean and `cli:test` green.
- [ ] 2.3 On a branch with no pull requests, a submit with a subject adds the warning `<branch> opens no pull request; the subject titles nothing` to its accepted submission's warnings. Verify: a test in `crates/sluice/tests/cli.rs` that the submission is queued, the exit is 0, the `warning:` line is printed, and under `--no-wait --json` the document's `warnings` holds the sentence, green in `cli:test`.
- [ ] 2.4 `wt submit` forwards `--subject` untouched (`worktrunk::Submit::parse` forwards every argument but `--no-remove`). Verify: a unit test beside `Submit::parse`'s own in `crates/sluice/src/profiles/worktrunk/mod.rs` that `main --no-wait --subject "Two words"` hands sluice `--subject` and `Two words` as one argument and removes nothing, green in `cli:test`.

## 3. The title of a request a submit opens or updates

- [ ] 3.1 In `open_group`'s shared body (`pulls.rs`), a request not yet open is opened titled by the subject or, where none is given, `head_ref`; the first message no longer titles anything. Verify: in `crates/sluice/tests/profiles/pull_requests.rs`, `submit_opens_then_updates_a_pull_request` asserts the title `unit/review` and the line `pushed unit/review · opened #<n>: unit/review`, and new tests hold the spec's "A bolt submitted with no subject", "A session submitted with a subject" and "The description still carries the commits", green in `cli:tools`.
- [ ] 3.2 For an open request: when the submit gives a subject that differs from the title and the request's author is `forge.login()`, the one edit before the push carries the subject as its title (and the body, when it differs); the `Pull` the act goes on with, and so the `updated #<n>: <title>` line, carries the new title. No subject, or a request a person opened, leaves the title and makes no login call. Verify: tests in `crates/sluice/tests/profiles/pull_requests.rs` for "A new head with a subject", "A new head with no subject", "A request opened before titles changed" (a request `open_by("sluice", …)` titled by a commit) and "A subject on a request opened by hand" (`open_by("person", …)`), and `submitted_again_at_the_same_head_edits_nothing` still green, all in `cli:tools`.
- [ ] 3.3 The pull requests guide (`site/src/content/docs/guides/pull-requests.md`, "An agent submits"): the example opens with a subject, and a paragraph says the title is the subject or the branch's name, a later subject retitles a request sluice opened, and a request a person opened keeps its title. Verify: `site:prose` (Vale) clean in the gate.
- [ ] 3.4 The using-sluice skill (`plugins/sluice/skills/using-sluice/SKILL.md`), where it shows a pull request branch's submit, and the worktrunk reference's list of what `wt submit` takes (`plugins/sluice/skills/setting-up-sluice/references/profiles/worktrunk.md`) name `--subject`, in words any operator reads (core.26). Verify: `plugin-sluice`'s checks green in the gate (`sluice exec main`).
