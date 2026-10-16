# Tasks

Written after the fact: every task below was done in commit `6b97938`,
landed on main as submission #68 on 2026-10-16, and its tests were run
again when this change was written.

## 1. The places profile

- [x] 1.1 Replace the two copies of the method check in `materialize` and `preview` with `one_method`: no `method` or `"git-worktree"` accepted, another string refused with the reason naming ADR 0013, a non-string refused; records made with the constant `METHOD`; done in `6b97938`; verified by `a_policy_need_not_name_the_method` (no method and `"git-worktree"` each make and release a checkout) and `an_unknown_method_is_refused` (`"worktrunk"`, `"archive"`, `"rsync"` refused, no directory made) in `crates/sluice/tests/profiles/places.rs`.

## 2. Fixtures and the conformance suite

- [x] 2.1 Drop `method` from the conformance policy, the real-profiles rig, `previews_tools.rs`, `stack.rs`, `exec.rs`, `instance.rs`, `protect.rs`, `remote_branch.rs`, the pull request and worktrunk profile tests and the core's policy test; done in `6b97938`; verified by the gate that landed #68, which ran them with no method named.
- [x] 2.2 Keep a method in `the_core_gives_the_profiles_a_policy_names_no_meaning_of_its_own`, added to the empty `[checkout]`, so it still shows the core gives the key no meaning; done in `6b97938`; verified by that test in `crates/sluice-conformance/tests/records.rs`.
- [x] 2.3 Drop `method` from `e2e/e2e.sh`'s walkthrough policy and append `[checkout] ports` for S27 and S30 as a table of its own; done in `6b97938`; verified by `moon run ':#check-e2e'` in the gate that landed #68.

## 3. Documents

- [x] 3.1 Drop the key from `.config/sluice.toml`, `README.md`, `docs/examples/`, `docs/plans/phase-1-integrations.md`, and the site's `index.mdx`, `guides/walkthrough.md`, `guides/previews.md` and `reference/configuration.md`, which says why no key names the method and that an older one is still read; rewrite ADR 0013's consequence to say the same; done in `6b97938`; verified by `site:prose` in the fast tier of the gate that landed #68, and by `grep -rn 'method = "git-worktree"'` outside `openspec/` finding the key only in sentences saying an older policy's is still read.

## 4. The spec

- [x] 4.1 Write this change's `checkout` capability; verified by `openspec validate no-checkout-method --strict` and the tests above passing on `ff4dd50`.
