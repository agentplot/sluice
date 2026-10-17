## 1. Seam and profiles

- [x] 1.1 `Branch::make` and `Branch::remove` in the seam, the fakes and every profile
- [x] 1.2 Pushed refs leased on absent and on the revision read; local copies removed after a delete
- [x] 1.3 The router routes a make by the policy at the revision it is made at

## 2. Core and command

- [x] 2.1 `make_branch` and `remove_branch` under the branch's lock, with their refusals
- [x] 2.2 `sluice branch create` and `delete`, text and `--json`, exits 0, 1 and 2
- [x] 2.3 The missing-branch refusal names `sluice branch create`

## 3. Checks and docs

- [x] 3.1 Fakes tests, and end-to-end tests with a remote and with none
- [x] 3.2 `forge check` back to app-only; a bolt not made yet checked by name
- [x] 3.3 The reference, the guides and the plugin skills
