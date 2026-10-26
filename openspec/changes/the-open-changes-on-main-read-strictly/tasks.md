# Tasks

Read first: this change's proposal and design; the `actions-pool` and
`finished-submissions` changes as they are on `main`;
`openspec/specs/forge-identity/spec.md`. Work inside `devenv shell`, so
`openspec` is the version `devenv.nix` pins. Edit nothing outside the
three paths under Touches.

The gate does not yet validate open changes (the chore
`open-changes-are-checked-as-strictly-as-archived-ones`, which comes
after this change, makes it), so each verification here is the
`openspec validate` command the task names, run by hand.

## 1. actions-pool

- [x] 1.1 Replace `openspec/changes/actions-pool/specs/forge-identity/spec.md` with the text under "The text to write" in this change's design, word for word: the requirement names contents, workflows, checks and pull requests, and the block holds five scenarios, "No permission to post checks" third. Verify `openspec validate actions-pool --strict` passes.

## 2. finished-submissions

- [x] 2.1 Add `openspec/changes/finished-submissions/specs/finished-records/spec.md` with the text in this change's design, word for word: six ADDED requirements, the last two the log in parts. Verify `openspec validate finished-submissions --strict` passes and `openspec show finished-submissions --json --deltas-only` lists six ADDED requirements.
- [x] 2.2 Add the Capabilities section in this change's design to `openspec/changes/finished-submissions/proposal.md`, after "What Changes" and before "What it needs from the statement"; leave its tasks' boxes as they are. Verify `openspec validate finished-submissions --strict` still passes.

## 3. Every open change

- [ ] 3.1 Run `openspec validate --changes --strict` and `moon run root:specs`; verify every open change on the branch passes and the archived specs still pass.

## Workflow follow-up

- Archive with `/opsx:archive the-open-changes-on-main-read-strictly` once merged; it syncs no spec.
- The chore `open-changes-are-checked-as-strictly-as-archived-ones` goes on once this has merged into the bolt.
