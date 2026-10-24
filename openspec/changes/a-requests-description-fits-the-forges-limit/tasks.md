# Tasks

Read first: this change's proposal, spec delta and design; C.5's pull
request row of `design/sluice/requirements.md` in agentplot/blueprints,
with 125; ADR 0021; `docs/architecture/core.md`, rules core.1 and core.4.
Run suites through `sluice exec main -- <command>`; before submitting,
`sluice exec main`.

## 1. The description, built and measured

- [x] 1.1 In `crates/sluice/src/profiles/github/pulls.rs` add `DESCRIPTION_LIMIT: usize = 65_536` and a pure function that builds a request's description from the messages (oldest first), the agent, the group and the compare address, laid out and fitted as the design's "The description, laid out" and "How much fits" say, measured with `chars().count()` over the whole body; and a function giving the compare address `https://github.com/<owner>/<repo>/compare/<branch>...<head_ref>` from a `Repo`. Keep `with_marks` for the marks themselves. Verify with unit tests in `pulls.rs`'s test module: every message fits (the body is the messages then the marks, as today); three 30,000-character messages (two whole, `1 more commit, each by its subject; all of them in full: <address>`, the third's subject, the marks, at most 65,536); one 70,000-character message (no body of it, its subject, `1 more commit`); 70 subjects of 1,000 characters (no message whole, subjects as many as fit, `and <m> more, in the compare view`); a non-ASCII message measured in characters, not bytes; and `marked_agent` and `Group::of` reading every such body. Run `sluice exec main -- cargo test -p sluice --lib profiles::github::pulls`.

## 2. Opened and updated requests carry it

- [x] 2.1 In `crates/sluice/tests/profiles/forge/github.rs` make `POST /pulls` and `PATCH /pulls/<n>` answer a body over 65,536 characters with GitHub's 422 as the design gives it, storing nothing. Verify with the first test of 2.2 failing against the driver as it stands before 2.2's code change, then passing after.
- [ ] 2.2 In `open_group`, open a new request with the description of 1.1 for its head, and for an open request build the description for the head it submits and `forge.edit` it, before the push, whenever the body differs, in place of the marks-only edit. Verify in `crates/sluice/tests/profiles/pull_requests.rs`, with a helper committing a given message: three commits of 30,000-character messages opened by `wt submit main --no-wait` (exit 0, the request open, its body as the spec's scenario says, no longer than 65,536 characters); one 70,000-character message; a new commit `Add the second file` submitted again (both messages in the body); and a second submit at the same head sending no `PATCH /pulls/<n>` (read from the fake's `requests()`). Run `sluice exec main -- cargo test -p sluice --features tools --test pull_requests`.
- [ ] 2.3 Check that `submit_opens_then_updates_a_pull_request`, the group submit tests reading `group_mark`, the test of ops submitting an agent's sent-back request (its body naming `madswan-ops` and not the agent), and the rest of `pull_requests.rs` still hold, changing an assertion only where it read a body kept from before a submit that now rewrites it. Same test run as 2.2.

## 3. Docs

- [ ] 3.1 In `site/src/content/docs/guides/pull-requests.md`, "An agent submits": the description holds the commits' messages, each whole; a branch whose messages pass GitHub's 65,536 characters gets as many whole as fit, then a line counting the rest with a link to GitHub's comparison of the branch, and the rest's subjects; each submit writes it again for the commits it pushes, replacing text a person added to it. Verify `site:prose` (`sluice exec main -- moon run site:prose`).

## 4. Gate

- [ ] 4.1 Verify `openspec validate a-requests-description-fits-the-forges-limit --strict` (by hand: no gate task runs openspec) and `sluice exec main` with everything committed.
