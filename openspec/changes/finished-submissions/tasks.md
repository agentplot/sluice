# Tasks

Read first: this change's proposal and design;
clauses 61 and 130 to 132 of
`agentplot/blueprints:design/sluice/requirements.md`;
`docs/architecture/core.md`. Work inside `devenv shell`.

## 1. Finished submissions

- [x] 1.1 The records seam: `FINISHED`, `Changes::remove`, `read_finished`, `read_all_finished`; `read` leaves the finished records out; the local files profile carries removals in its pending write; the fakes the same.
- [x] 1.2 The core: `Line` loads `finished.jsonl`; a write moves each submission landed or withdrawn that no outbox item, landing or combination names; ordering, `after`, `attempt`, `find`, `state`, `amend` and `withdraw` read the index or the finished file.
- [x] 1.3 Readers of history: `Sluice::history` and `Sluice::latest_of`; `status <branch>` lists the queue, `--all` the history, as text and `--json`; `submit --status` and the pull request driver find a finished submission.
- [x] 1.4 Tests: the local files profile's move and crash; the CLI's move, `after`, `attempt` and refusals; conformance and real-tool tests reading history with `--all`.
- [x] 1.5 Docs: the state page, the walkthrough.

## 2. The log in parts

- [ ] 2.1 Close `log.jsonl` at 1,000 lines into `log/<n>.jsonl`, under the branch's lock; `read` returns the current part and the newest closed one.
- [ ] 2.2 A reader of the whole log (`read_log`) for the readers that search it for an entry's events: the pull request driver's removal time, a report's run counts, the time a combination was under test.
