# Tasks

Each test named here is in `crates/sluice/tests/cli.rs` unless it says
otherwise, runs in the gate's `cli:test` task (check-fast), and can be
run alone while iterating with `sluice exec main -- cargo test -p sluice
--test cli <name>`.

## 1. The reader's lock and document

- [x] 1.1 Add `crates/sluice/src/reader.rs`: the lock `readers/<repository>.lock` taken with `File::try_lock`, the repository's common git directory named by `runs::flat_name`; the lead's `write`, to a temporary file renamed over `readers/<repository>.json`; the follower's `read`, which returns no document for a file absent or not whole, and `try_lead` (design: the lease, the document). Verify with unit tests in the module: a second take of a held lock follows, the lock taken again once the first holder is dropped, a document written is read back whole, and a half-written temporary file is never read.
- [x] 1.2 Add `readers(&self) -> Result<PathBuf>` to `exec::Host`: `state_dir()?.join("readers")` for the real host in `crates/sluice/src/profiles/mod.rs`, a directory of the test's own for the fake host in `crates/sluice/tests/support/`. Verify `cargo check -p sluice --tests` builds and `cli:test` passes unchanged.

## 2. A watch leads or follows

- [x] 2.1 In `overview::run`, a watch takes the reader: as the lead it writes each document it prints; as a follower it reads no seam, prints the file's document at its start and on each change but its time, and tries the lock each tick, reading for itself from the tick it gets it. The one-shot overview never opens the lock. Verify with `watch_with_no_reader_writes_each_document`, `watch_follows_the_readers_document` (the file holds `#7` testing, which the records do not; the watch prints it), `watch_follows_each_change_once` (two changes and a rewrite with only `at` moved print two lines), `watch_leads_once_the_reader_stops` (the test drops the lock it held; within one pause the watch writes a document read from the records), `watch_of_another_repository_leads` and `one_shot_overview_reads_the_records_beside_a_reader` (prints the records' overview without `#7`, and the file is unchanged); `overview_watch_prints_a_line_at_start_and_on_each_change` passes unchanged.

## 3. A wait leads or follows

- [x] 3.1 Give `Waiting` the host's readers directory and repository, from `run`'s `host`, at each of its builders (`sluice wait`, the waiting submit and the pull-request wait). Verify `cli:test` passes unchanged.
- [x] 3.2 In `wait_on`, take the reader beside the loop and keep the round each tick. As the lead, read `overview::document`, write it when it changed, and read the submission, its combination and its branch's hold from it. As a follower, read them from the file's document; once the document no longer holds the submission among its branch's `submissions`, read `s.state(id)` once and report as now. Verify with `wait_with_no_reader_writes_the_whole_overview` (two branches in the document), `wait_follows_the_readers_progress` (3 of 5 tasks done, from the file), `wait_out_of_time_prints_the_readers_record` (exit 5, `state` `testing` as the file holds it), `followed_wait_reports_its_landing` (exit 0, `passed · landed as <rev>`), `followed_wait_reports_its_withdrawal` (exit 4) and `watch_leads_once_the_readers_wait_ends`; every existing `wait_*` test and the `submission-wait` scenarios pass unchanged.

## 4. The site

- [ ] 4.1 In `site/src/content/docs/reference/state.md`, add `readers/<repository>.lock` and `readers/<repository>.json` to the host's state, with a paragraph: the first watch or wait of a repository holds the lock and keeps its overview in the file, every other one on the host follows it, and the next takes over within a tick when it stops. In `site/src/content/docs/reference/commands.md`, say in the `sluice overview` and `sluice wait` rows that one watch or wait per repository on the host reads the queue and the others follow it, and that `sluice overview` alone always reads it itself. Verify the `site:prose` task passes with no warning.

## 5. The whole

- [ ] 5.1 Run the gate's checks with `sluice exec main` once the work is committed. Verify every check-fast and check-tools task passes.
