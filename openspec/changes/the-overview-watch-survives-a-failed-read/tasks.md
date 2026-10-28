# Tasks

## 1. What the tests need

- [ ] 1.1 In `crates/sluice-fakes`, give the world a way to make reads
  of the queues fail with given words, from now until told otherwise,
  as `refuse_state_reads` is set and cleared: a read of any branch's
  records, through the records seam, then returns an error carrying the
  words. Verified by a unit test in the crate that the next read fails
  with the words and a read after clearing succeeds (`fakes:test`).
- [ ] 1.2 Give the world a flag saying the repository is gone, and the
  tests' `FakeHost` (`crates/sluice/tests/support/mod.rs`) the new
  `Host::repository_gone` answering from it. Verified by its use in
  group 2's tests (`cli:test`).

## 2. A watch keeps watching

- [ ] 2.1 Add `repository_gone(&self, repository: &str) -> bool` to
  `exec::Host`, the real host (`profiles/mod.rs`) answering whether the
  path is no longer a directory through a free function. Verified by a
  unit test of that function on a scratch directory before and after it
  is removed (`cli:test`).
- [ ] 2.2 In `overview::run`, catch a failed read of the overview: unless
  the host says the repository the reader was taken for is gone, the
  reader writes, and the watch prints, the error document a failing
  overview prints (`told::document(Kind::Internal, "{e:#}", &[])`), and
  the loop goes on to its pause. A gone repository returns the error, as
  today. Verified in `crates/sluice/tests/cli.rs` (`cli:test`) by the
  spec's scenarios "A watch's read fails and then succeeds", "A failed
  read under load", "A standing failure is written once" and "The
  repository is removed", with the fakes of group 1.
- [ ] 2.3 In `overview::run`, a failed write of the reader's document no
  longer ends the watch: the watch prints what it read and the write is
  made again at the next tick; a failed `lead_once_free` leaves the watch
  following and is tried again at the next tick. Verified by the
  scenario "The reader's document cannot be written", the readers'
  document made unwritable for one tick (`cli:test`), and by a unit test
  in `reader.rs` that a write that failed is made again even when the
  document is unchanged.
- [ ] 2.4 Verify a follower watch prints the reader's error document once
  and the overview after it, and a watch stopped while a failure stands
  exits 130: the scenarios "A follower prints the reader's failure" and
  "Stopped while a failure stands" (`cli:test`).

## 3. Waits beside a failed read

- [ ] 3.1 In `Reading::read` (`crates/sluice/src/lib.rs`), the reader
  wait's failed read writes the error document as the reader's document
  before reading its own branch as it does now. Verified by the scenario
  "A wait that is the reader cannot read the queues" (`cli:test`).
- [ ] 3.2 In `Reading::read`, a follower wait treats a document with no
  `branches` array as no whole document: it keeps its last view and reads
  no record. Verified by the scenarios "The reader cannot read while a
  wait follows" and "Out of time on a failure" (`cli:test`), the first
  asserting the records were not read for submission 7.

## 4. The site

- [ ] 4.1 In `site/src/content/docs/reference/commands.md`, the
  `sluice overview` row says `--json --watch` keeps watching through a
  read it could not make, printing the error as a line, and ends only
  when stopped or when its repository is gone. In
  `site/src/content/docs/reference/state.md`, the readers paragraph says
  the reader writes a read it could not make as its document, in the
  words it was refused with, and reads again a second later. Verified by
  `site:prose` with no warnings.

## 5. The gate

- [ ] 5.1 Run the gate's checks with `sluice exec main` once the work is
  committed, and see `cli:test`, `fakes:test`, `site:prose`,
  `root:format` and `root:lint` pass.
