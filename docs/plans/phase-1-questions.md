# Phase 1: questions the build met

Behaviours the statement, the walkthrough and the rulings do not settle,
met while building the core. Each names the clause it sits beside, the
reading the build took (the one the walkthrough implies, where it implies
one), and where in the code that reading lives. The operator rules; a
ruling that changes a reading is a change to the statement first (rule
core.12).

## Submission

**Q1. Beside clause 1 and rule core.4: does a waiting `wt submit` reach a
landing?** Walkthrough section 6 has a waiting `wt submit` run rounds of
sluice's work, and a round lands. Rule core.4 rules out a ref write
"anywhere a session's submit command reaches". Reading taken: the submit
operation (`Sluice::submit`) reaches no ref write, and the CLI's wait loop
calls the step separately, as `sluice step` would. If that is right,
core.4 might read "reachable from the submit operation". Code:
`crates/sluice/src/lib.rs`, `submit`.

**Q2. Beside clause 2: who refuses a place holding more than its
commits?** The statement puts the refusal in the driver. The driver seam's
`session` reports what the place holds beyond its commits, and the core's
`submit` refuses on that report, naming the paths. The outcome is the same;
the refusing code is the core's. Code: `sluice-seams` `Driver::session`,
`sluice-core/src/ops.rs`.

**Q3. Beside clause 27: a submission to a branch no `[branches]` table
names.** Clause 27 has sluice admit every submission a driver makes. With
no table there is no gate to judge it by, so it could never land. Reading
taken: refused at submit ("no [branches] table ... names it"), as a
statement that the branch is not sluice's, not a gate decision.

**Q4. Beside clause 67 and the answer to question 9: does a resubmission
keep the withdrawn submission's `after` and priority?** The answer says a
resubmission "joins at the back, keeping its `after` list". Reading taken:
the new submission carries the `after` and priority it is submitted with;
nothing is inherited from the one it withdraws. A driver that wants the
old list passes it again.

**Q5. Submission ids are numbered across the repository, not per branch,**
so `sluice status 12` and `sluice amend 14` need no branch (walkthrough
sections 5 and 9). The order within a branch is still the order of
submission (6). The records seam allocates them.

## Order and lanes

**Q6. Beside clauses 6 and 9: lanes joined by a submission whose parts
are unknown.** When an unknown submission joins two lanes that each have
a candidate under test, at most one of those candidates can stay a prefix
of the joined chain; the other lacks the first lane's submissions and is
formed again (10). Reading taken: the joined chain keeps every pinned
list's own order and merges the lists by order of submission, so no
submission under test goes behind one submitted after it; the candidates
that are no longer prefixes are dropped and formed again. Is re-forming
an entry under test, because of a submission made after it, a
"displacement" clause 6 forbids? Code: `sluice-core/src/order.rs`,
`pinned_prefix`.

**Q7. Beside clause 8: a landing in one lane moves the line under every
other lane.** Every candidate of the other lanes was formed on the old
revision, so it is formed again on the new one and tested again (24);
with a content-keyed cache those runs mostly replay. Reading taken: a
candidate of another lane still under test when the line moves is ended
and its place released. Letting it finish would warm the cache but run a
check whose verdict nothing reads. Code: `step.rs`, `arrange`.

**Q8. Beside clause 7: an `after` naming a submission that was blamed,
conflicted or withdrawn.** It will never land, so the submission waits
until amended or withdrawn. No bound is applied (5).

## Window and deepest first

**Q9. Beside clauses 13 and 14: what the window counts under deepest
first.** Reading taken: the window is how deep the one tested candidate
reaches (the deepest within the first `window` candidates), and it counts
as one toward the line's window; after it fails, every candidate it
skipped is started at once, within the window. The window halves on a
blame and grows by one on a pass, between one and `parallel`.

## Policy

**Q10. Beside clause 27 and ADR 0007: defaults for keys a table leaves
out.** Reading taken: ADR 0007's bolt defaults, `parallel = 4`, `batch =
false`, `flaky = "pass"`; and no time limit when `timeout` is absent (66
says "the time limit its line's policy states"). The walkthrough's
`bolt/*` table sets `flaky = "fail"` explicitly and its `main` table
leaves `flaky` and `timeout` out, so under these defaults `main` reads a
flaky key as passed and has no limit. ADR 0007's shared-line defaults
(deepest first, confined only) cannot be told apart by the policy, which
has no notion of which branch is the shared line.

**Q11. Beside clause 27: which table governs a branch several patterns
match.** Reading taken: an exact name before any pattern, then the pattern
with the most literal characters; a tie is an error. `*` matches within
one path segment, so `bolt/*` matches `bolt/x` and not `bolt/x/y`.

## Verdicts

**Q12. Beside clauses 15 and 70: when is the line's revision "covered by
a verdict"?** Reading taken: when sluice landed it. A new line's first
candidate and the first after a hand move run every required check;
after a landing, the run is asked only for the checks the candidate
affects since the line's revision. (With moon, unaffected checks would
replay anyway.)

**Q13. Beside clause 66: what a timed-out candidate's verdict names.**
The run is ended; the checks that finished are recorded; the result is
"timed out" and the submission blamed has `result: "timed out"` and no
failed checks of its own unless one finished failing. The CLI says
"timed out".

**Q14. Beside clauses 16 and 69: where a run is recorded.** Reading
taken: twice, in the branch's `log.jsonl` (`ran`, `replayed`, as the
walkthrough shows) and in the repository's `results.jsonl`, the record a
verdict is read from, keyed by task and hash for any branch. The
repository log is written before the branch write that carries the
verdict, and a repeat after a crash is not appended again.

## Landing

**Q15. Beside clause 24: a landing that moved the line, after which the
line was moved again by hand before the move was recorded.** The next
round finds the line at neither the landing's `from` nor its `to`, and
records the landing as refused although it happened. Telling the two
apart needs "is `to` contained in the line", which the line seam does not
offer. Add it to the seam, or accept the misreport for this race?

**Q16. Beside clauses 14, 22 and the answer to question 12: landing under
deepest first while a skipped candidate is still under test.** When a
skipped candidate passes while one ahead of it is still running, the
prefix up to it lands on its verdict (the deeper candidate's verdict
covers the shorter one's tree), and the running one is ended.

## Records

**Q17. Beside clause 53 and the brief's "a schema version in every
document".** Every JSON document names its schema (`sluice/submission@1`,
`sluice/branch@1`). The lines of `log.jsonl` and `results.jsonl` keep the
walkthrough's shape with no schema field; `branch.json` names the log's
schema. Should every log line carry one?

**Q18. A file the walkthrough's section 7 does not list:
`branch.json`** holds the queue's own state, the revision candidates are
formed on, the window, the candidates and their places and runs, a
landing intent, places to release and outcomes not yet told. Without it
the step would hold decisions outside the records (rule core.14).

## The CLI

**Q19. Walkthrough section 5's `testing ... · 2 tasks to run, 31
replayed` line.** The counts are known when the run finishes, so the CLI
prints the line with the outcome, not when testing starts. With moon, a
profile could know them at start, after hashing.
