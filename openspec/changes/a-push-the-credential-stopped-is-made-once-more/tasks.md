## 1. A fake remote that answers a push

- [ ] 1.1 In `crates/sluice/tests/pushed_refs.rs` (or `tests/support/http.rs` where it is shared), add answers for git's smart HTTP push: `info/refs?service=git-receive-pack` advertising a ref at a revision the test names (or no refs), and `POST /git-receive-pack` answering report-status `unpack ok` and `ok <ref>`, with a helper reading the pkt-line command `<old> <new> <ref>` from a request's body; verify with a test that `push_branch` through a fake that accepts the first token succeeds and the fake saw one push naming the branch and revision.

## 2. A push refused on the token is made once more

- [ ] 2.1 In `PushedRefs::remote_git`, replace the `reads(args)` gate: make the call once more on `Identity::renew` when stderr matches `REFUSED` and porcelain stdout holds no ref's answer (no line with a tab), for a push as for a read, with the same `args`; drop `reads` and the push-only `told` branch, and update the doc comments of `remote_git`, `move_to` and `leased` to say a credential refusal is made once more with the same lease (134, core.5, core.29). Verify with 2.2 to 2.5.
- [ ] 2.2 Test: a leased push (`make`, and `move_to` on an advertised branch) refused `Repository not found.` on the first token and accepted on the fresh one returns `Moved::Moved`; the fake saw two receive-pack pushes, the first token then `token-1`, both carrying the same old value and ref; `renewed` is 1. Run `sluice exec main -- cargo test -p sluice --test pushed_refs`.
- [ ] 2.3 Test: on the second token the fake advertises the branch at another revision; the second push is refused by the lease and `move_to` returns `Moved::Elsewhere` with no third push. Same command.
- [ ] 2.4 Test: `push_branch` refused on the first token and accepted on the fresh one succeeds; rewrite `a_refused_push_is_not_run_again` as a push refused on both tokens, whose error carries `Repository not found.` and ends `origin refused the fake's token number 0, then the fake's token number 1`, carries no token, and saw two pushes. Same command.
- [ ] 2.5 Test: a push answered at the ref, porcelain `!\t<ref>\t[remote rejected] (protected branch hook declined)` on stdout with a 403-like line on stderr, is made once and renews nothing; and a lease refused `[rejected] (stale info)` is made once. Same command.

## 3. A leased push with no answer reads the branch

- [ ] 3.1 In `leased`, when the push failed with no ref's answer and no credential refusal (a cut connection, a server error), fetch the branch: return `Moved::Moved` when it holds the pushed revision, or is absent for a removal (deleting the local branch and tracking ref as `remove` does), and otherwise fail saying the push went unanswered and naming what the remote holds, or the fetch's failure; never push again. Verify with 3.2.
- [ ] 3.2 Tests: a `make` whose POST the fake answers 500 (or cuts) while its upload-pack listing then shows the branch at the pushed revision returns `Moved::Moved` after one push; the same with the listing showing no branch fails naming the unanswered push and the branch's absence, after one push; a `remove` whose answer is lost and whose listing shows no branch returns `Moved::Moved` and leaves no local branch or tracking ref. Run `sluice exec main -- cargo test -p sluice --test pushed_refs`.

## 4. The guide

- [ ] 4.1 In `site/src/content/docs/guides/github-app.md`, after the paragraph on how sluice uses the token, say that GitHub sometimes refuses a token in its first seconds, that sluice then makes the fetch or push once more on a fresh token, and that a refusal that stands is reported with GitHub's words and what each token was (its app, repository, installation, scope and age), never the token; verify with the `site:prose` moon task clean.

## 5. The gate

- [ ] 5.1 Commit, then run the gate's checks with `sluice exec main`; every task passes.
