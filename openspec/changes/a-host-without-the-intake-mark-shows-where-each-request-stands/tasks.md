# Tasks

The groups land in order, as one landing, from this change's place on
`change/a-host-without-the-intake-mark-shows-where-each-request-stands`,
into `bolt/requests-show-where-they-stand-from-any-host`. Run suites
through sluice (`sluice exec main -- cargo test ...`). A quick `cargo
check` or one unit test may run directly. Tests use scratch
repositories, a bare clone standing for GitHub's git, other hosts'
scratch clones (`OnGitHub::other`) and the GitHub double only, never a
real account or the host's own sluice state. Commit by pathspec,
Conventional Commits scoped to the part of the tree, citing clauses by
number and rules by id. Each group lands the site's and the plugin's
words for what it changes.

## 1. The decision recorded

- [x] 1.1 Write the record of this decision in `docs/adr/`, MADR as record 0088 is, at the next number free on main and every branch when it is written. It holds 137 and C.5's pull request row as amended at agentplot/blueprints 4791adf, the design agent's, and D1 to D6 as this change's own, each with the path not taken. It names record 0093's 137 as the statement at 7c2fbf0 proposes it, and what of this change goes with the mark. Rule core.33's details gain what it rules out: a status, overview or wait on a host that does not take the requests in reading another host's records; a request shown with no host; a check's fields read from its words where it carries them; a followed request read by each process on its own bound. Write the site's rules page with `node site/scripts/rules.mjs`. True when `sluice exec main -- moon run root:adrs root:changes site:rules` passes.

## 2. sluice's check names its host and carries the queue

- [x] 2.1 `queued`, `queue_shown`, `waiting`, `waiting_in_group` and `outcome_shown` in `crates/sluice/src/profiles/github/pulls.rs` are given the posting host and the request's queue on that host. Each opens its summary with `Submission <n> on <host>.` or `Waiting to be taken in on <host>.` and ends it with one `<!-- sluice:queue {…} -->` block of `host`, `taken_in`, `submission`, `state`, `ahead` and `waits_for`. A reader of a summary takes its last block and reads none where there is none (D1). True when `cargo test -p sluice --lib -- what_a_waiting_pull_request_shows what_a_queued_pull_request_carries_its_queue a_queue_block_reads_back_and_an_older_check_has_none` passes.
- [x] 2.2 The suite: host `b` takes main's requests in, and `#12`'s check on its head names `b` and submission 1406 behind 1405 and `#10`, in its words and its block. `#13`'s check, needing a review, says it waits to be taken in on `b`. A round where neither moved posts nothing, and one after `#10` lands posts `#12`'s block with nothing ahead. The pull requests guide says what the check's summary names. True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- the_check_names_its_host_and_queue the_queue_block_moves_with_the_queue_ahead no_repeated_posts` and `sluice exec main -- moon run site:prose` pass.

## 3. A host keeps the requests it follows

- [x] 3.1 `Kept::follows` and `Kept::change_follows` in `crates/sluice/src/profiles/github/kept.rs`: `forge/follows/<owner>/<repo>/<base>.json`, changed under the `.lock` beside it and written whole. Each request is kept by number with when it was read, its head, open, merged, draft, title, the branch it is opened from, and sluice's check on that head. The module's doc names the file (D3). The state reference lists `forge/follows/` and its lock. True when `cargo test -p sluice --lib -- a_followed_request_is_kept_and_leaves` and `sluice exec main -- moon run site:prose` pass.
- [x] 3.2 `opening` adds each request it opens, updates or marks ready, beside the `.moved` touch. `hand_over` adds each request it withdraws, and `follow_across` adds what it follows. A read that finds a request merged or closed takes it out once a wait or status has said so, and this host's intake taking it in at its head takes it out (D3). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- submitted_here_taken_in_there handed_over_is_followed a_merged_request_leaves_the_follows` passes, each failing with `pulls.rs` as main held it.
- [x] 3.3 `follow_across` reads a followed request and its check through the kept file, from GitHub only once a quarter of `discovery` has passed since any process of the host read them, each read conditional (D4). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- two_waits_and_a_status_read_at_one_pace a_wait_across_hosts_reads_at_its_pace` passes, the first failing with `pulls.rs` as main held it.

## 4. The check's fields decide over its words

- [x] 4.1 `follow_across` reads a request's state from its check's block where there is one: taken in and `failed` or `conflict` is sent back, and `landed` is landed. A check with no block is read by its words, as now (D6). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- a_run_github_keeps_completed_reads_by_its_block an_older_sluices_check_reads_by_its_words landed_on_another_host blamed_on_another_host` passes. The first sets `#12`'s run on the double completed as a failure with a block reading `testing`, and fails with `pulls.rs` as main held it.

## 5. A wait names where its request stands

- [ ] 5.1 `Followed::Open` in `crates/sluice/src/lib.rs` carries `taken_in_by`, from the intake mark as the host's last fetch brought it, and `queue`, from the check's block on the request's current head (D2). The wait's `--status` document and its timeout document give both. Its words give the line naming the request, the submission there and its host, or that it waits to be taken in by the mark's host, again only when it changes (D5). The commands reference's `sluice wait` and the `using-sluice` skill's lines on a submit's `{"pull_request","branch","state"}` name both fields. True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- out_of_time_queued_on_the_other_host the_status_a_submit_reads_across_hosts a_waits_words_across_hosts not_yet_taken_in_across_hosts` and `sluice exec main -- moon run site:prose plugin-sluice:validate` pass.

## 6. status shows each followed request as the taking host's

- [ ] 6.1 A `PullRequests` method gives a branch's followed requests whose submissions this host's queue does not hold at their heads, each read as D4 allows, with `taken_in_by` and `queue`. The trait's default answers none (core.1, core.2). `sluice status <branch>` prints them after its queue, naming the host and its holder, or that no host holds the mark. Each row names its request's number and title (26), and the submission there or what it waits for. A row found merged or closed is shown once. Under `--json`, `taken_in_by` and `pull_requests` hold the same, each row `{"pull_request", "title", "from", "head", "draft", "state", "queue", "read_at"}` (D2, D5). The commands reference's `sluice status <branch>` row says so. True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- queued_on_the_other_host waiting_for_a_review_there no_check_on_its_head_yet the_mark_and_the_check_name_different_hosts` and `sluice exec main -- moon run site:prose` pass.

## 7. The overview lists them

- [ ] 7.1 `overview::document` gives each branch with such requests `taken_in_by` and `pull_requests`, as status gives them, and the overview's words list the same rows under the branch. A one-shot overview reads them as status does. A reader's document carries the kept file whenever its modified time moved, and its tick reads GitHub for none of it (D5; core.25, core.31). The commands reference's `sluice overview` section names the fields. True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- a_one_shot_overview_lists_requests_taken_in_elsewhere a_watch_reads_no_followed_request_itself` and `sluice exec main -- moon run site:prose` pass.

## 8. The gate

- [ ] 8.1 The whole gate on the committed branch, the change valid. True when `sluice exec main` passes.

## Workflow follow-up

- Verify the change against its specs and archive it with `openspec archive`, so `pull-request-status`, `request-intake-host`, `submission-wait` and `overview` take its requirements.
- Tell flume that `sluice status <base> --all --json` on a host that does not take the base's requests in lists the place's request under `pull_requests`, by `from`, for its read-submission. Tell crew that a wait's `taken_in_by` and `queue` say where a landing queued on another host stands.
