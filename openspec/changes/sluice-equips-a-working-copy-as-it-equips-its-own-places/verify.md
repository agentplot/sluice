# Verify

The work on `change/sluice-equips-a-working-copy-as-it-equips-its-own-places`
at 9cbc5e21, cut from main at 6b29c5ce, checked on 2026-11-05 against
this change's proposal, design, equip spec and tasks.

**The work holds.** Every requirement and scenario of the equip spec
is met by the code and by the test the scenario names, each of which
passed. Every task is ticked and its condition holds, and the checks
the tasks name pass.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 9cbc5e21, a clean worktree | 27 of 27 tasks passed, each run; 27 results recorded under their keys; cli:test 559 tests and cli:tools 516, every test the table below names among them |
| `sluice exec main` on 9cbc5e21, run first | 26 passed and `plugin-board:test` failed, nothing recorded: see below |
| `devenv shell -- moon run cli:sandbox --force`, outside any sandbox, since macOS refuses one inside another and no gate task runs it | passed: 8 sandbox tests, each through `Checks::start` and its equipping |
| `openspec validate sluice-equips-a-working-copy-as-it-equips-its-own-places --strict` | valid |
| `rg -n '"proto"\|"setup"\|"--version"' crates/sluice/src crates/sluice-core/src` (7.1) | `moon.rs:378` (`proto install`), `moon.rs:386` (`moon setup`), `moon.rs:445` (`moon --version`), and worktrunk's own `--version` (`worktrunk/mod.rs:567`), which equips nothing |
| `rg -n 'Text\("command"' crates/sluice-core/src` (7.1) | nothing: no request is written to the command runner |
| `sluice equip main` and `--json`, the branch's debug build, in a folder below the top of a scratch repository whose policy names `runner = "command"` and no toolchain | `nothing to equip: main's policy names no toolchain, and its runner nothing to set up`, exit 0; `{"branch":"main","revision":…,"steps":[]}`, exit 0 |

The first gate run failed one test of `plugin-board:test`, `sluice
0.44.0 on the host: the board says so once, and shows and delivers
nothing` (`plugins/sluice-board/tests/register.test.tsx:2615`), on the
engine's 5000 ms limit. It recorded nothing, because this report was
written into the worktree while that run went. The task's inputs are
the board plugin's own files and `devenv.nix`, none of which this change
touches, so its key, 2477d075, is the one main gives. That key passed in
this change's own 7.2 gate and in the second run, on a host running
twenty agents at once. The work did not cause the failure. A fail and a
pass under one key would read as a flake, and it is reported to the
orchestrator with this verify.

moon warned 51 times across the second run that it could not archive a
task's outputs under `.moon/cache/outputs/` or store a blob in the
shared cache. The tasks still passed. These are the sandbox's known
refusals of those writes, which the change `a-checks-key-holds-its-executor`
owns, and this change did not cause them.

The last row checks the one claim of a scenario that no test asserts:
"The command runner" says that with no toolchain `sluice equip` says
there is nothing to equip and exits 0. `equip_under_the_command_runner_runs_the_toolchain_alone`
holds the empty sequence in the profile, and the fakes' tests hold the
exit with no steps, but none reads the person's line. The binary prints
it, from `write_text` in `crates/sluice/src/equip.rs`.

## Requirements and scenarios

| Requirement | Scenario | Where it is met | Test, and the task it passes in |
|---|---|---|---|
| A working copy is equipped by one sequence | A moon repository whose pins nothing has installed | `Equipping::through` in `checks.rs`; `Moon::steps` in `moon.rs` | `equip_runs_the_sequence_in_order`, cli:test |
| | Nothing pinned | `Moon::steps` gives `proto install` only where `.prototools` exists | `equip_installs_only_what_is_pinned`, cli:test |
| | The command runner | `Equipping::new` gives the command runner no steps; `write_text`'s "nothing to equip" | `equip_under_the_command_runner_runs_the_toolchain_alone`, cli:test; the binary, above |
| The sequence stops at the first step that fails | The setup fails | `Equipping::through` returns at a failed step; `failed`, `why`, `printed` | `equip_stops_at_the_step_that_failed`, cli:test; `a_failed_step_exits_1`, cli:test |
| | An old moon | `Moon::check_version` against `FLOOR` | `an_old_moon_is_refused`, cli:tools; `a_moon_below_the_floor_fails_its_version_check`, cli:test |
| | The toolchain fails | `toolchain` answers a failed `EquipStep`, its error stream alone | `a_toolchain_that_fails_runs_nothing_after_it`, cli:test |
| sluice equip equips the working copy it runs in | At a revision named | `Sluice::equip` with `at`, `policy_at(rev)` | `equip_reads_the_policy_at_the_revision_named`, cli:test |
| | At a revision whose policy manages no branch | `Sluice::equip` with `at` reads no `[branches]` | `equip_at_a_revision_equips_whatever_its_branches_name`, cli:test |
| | At the branch's revision | `Sluice::equip` without `at`, the branch's tip | `equip_reads_the_branchs_policy`, cli:test |
| | From a folder inside the worktree | `repository_dir` in `main.rs`; `Bound::working_copy` by `worktree_top` | `equip_runs_at_the_top_of_the_worktree`, cli:tools |
| sluice equip answers its steps in one document | Every step passed | `document` in `crates/sluice/src/equip.rs`; exit `LANDED` | `every_step_passed_exits_0`, cli:test |
| | Printed for a person | `write_text` | `a_failed_step_is_printed_with_its_words`, cli:test |
| A refusal gives no steps | No such branch | `EquipRefusal::NoBranch`, `told::tell` as `Kind::Refused`, exit `CONFLICT` | `equip_refuses_a_branch_it_cannot_read`, cli:test |
| | A branch the policy does not manage | `EquipRefusal::NotManaged` | `equip_refuses_a_branch_its_policy_does_not_manage`, cli:test |
| | A revision with no policy | `EquipRefusal::NoPolicy` | `equip_refuses_a_revision_with_no_policy`, cli:test |
| | A revision that is no commit | `Host::revision` answers none before the core is asked | `equip_refuses_a_revision_it_cannot_read`, cli:tools |
| | A folder in no worktree | `Host::working_copy` answers `None` | `equip_refuses_a_folder_in_no_worktree`, cli:tools |
| sluice equip adopts and keeps nothing | Nothing to release after it | no `lock_sessions`, `begin_session`, `adopt` or admission on the path | `equip_adopts_nothing`, cli:tools |
| | At the host's bound | `Checks::equip` admits nothing | `equip_takes_no_part_of_the_bound`, cli:test |
| | Its script is gone | `Checks::equip`'s `Scratch` under `.runs/equip-<pid>/`, removed on drop | `equip_keeps_nothing`, cli:test |
| Each step runs outside the executor with a run's environment | The caller's variables do not reach the steps | `environment` for a place named `equip`; `env_clear` in `toolchain` and `Moon::spawn` | `equip_steps_get_a_runs_environment`, cli:test |
| Every working copy a check or a command runs in is equipped through the setup | A session's command on a fresh clone | `session_request` sets `command`, keeps `[tests]`; `Checks::start` equips through `Running` | `exec_equips_before_its_command`, cli:tools; `a_run_with_a_command_is_equipped_by_its_branchs_runner`, cli:test |
| | A preview on a fresh clone | `preview_request` sets `command`, keeps `[tests]` | `a_preview_is_equipped_before_its_command`, cli:tools |
| | A worktree sluice did not make | `sluice equip` through `Running` | `equip_installs_the_pins_before_moon_runs`, cli:tools |
| | A combination that runs a check | `Checks::start`: `Reading`, `Moon::plan`, then `Running` where `argv` is `Some` | `a_run_that_runs_a_check_is_set_up`, cli:test |
| A working copy sluice only reads in is equipped up to the version check | A combination that affects nothing | `Checks::start` skips `Running` where the plan runs nothing | `a_run_that_runs_nothing_is_never_set_up`, cli:test |
| | A submitter's worktree | `Checks::kept_out` through `Reading` | `the_submitters_read_is_never_set_up`, cli:test |
| A run whose working copy could not be equipped could not start | A combination's setup fails | `Equipping::ready` fails `start` with `<step>: <words>`; the core's `could_not_start` as before | `a_run_whose_working_copy_could_not_be_equipped_could_not_start`, cli:test; the toolchain's words, `could not start: toolchain: exited 1: …`, at `checks.rs:1126`, cli:test |

The affected reading's tree (140) is equipped through `Reading` in
`Checks::reads`, as the spec's requirement on working copies only read
in says; no scenario names it, and its tests in cli:tools pass.

## Tasks

29 of 29 are ticked, and each one's condition holds:

- 1.1, 1.2: record 0081 is accepted on the user's approval of
  2026-11-05; rule core.46 stands under "Places" with its details; the
  details of core.37 and core.38 name equipping in "Not ruled out"; the
  site's rules page carries core.46. `root:adrs`, `root:changes` and
  `site:rules` pass in the gate.
- 2.1 to 2.6: `Through`, `Equip`, `EquipStep`, `Equipped` and
  `Tests::equip` in the seams; `Moon::steps`, `install`,
  `check_version`, `setup` and `Moon::step` in `moon.rs`, with the six
  unit tests of a step's words; `Equipping` and `Checks::equip` in
  `checks.rs`; the eleven tests in `crates/sluice/tests/checks.rs`;
  `an_old_moon_is_refused` reads `moon --version: `.
- 3.1 to 3.5: `RunRequest::command`, set by `session_request` and
  `preview_request`, `None` elsewhere; `Checks::start` plans a command
  as the command runner does, `Meta::runner` `"command"`; the fakes and
  the tests read `request.command`.
- 4.1 to 4.7: `Sluice::equip` and `EquipRefusal` in
  `crates/sluice-core/src/equip.rs`; `Host::working_copy`;
  `Command::Equip` and `crates/sluice/src/equip.rs`, naming no step or
  tool of the runner's (rule core.1); `FakeTests::equip`; the ten tests
  of `crates/sluice/tests/equip.rs`; `exec.md`'s section, which
  `every_command_is_named_in_the_skills` reads.
- 5.1 to 5.3: the seven tests of `crates/sluice/tests/equip_tools.rs`;
  `--test equip_tools` in the `tools` task; the timing in 9ccc8d4d's
  message: `sluice exec main -- true` took about 13s more on this branch
  than on main, all of it before the command, on a busy host.
- 6.1 to 6.3: `## sluice equip` in the commands page, and the exec and
  preview paragraphs; the configuration page's `toolchain` row,
  "Equipping a checkout", `[checkout]` and `[preview] command`; the
  README's row; the moon and previews setup references. No host, tool
  or team of ours is named (rule core.26). `site:prose` and
  `plugin-sluice:validate` pass in the gate.
- 7.1, 7.2: as the checks above.
- 7.3: what the orchestrator does once the change lands is below. No
  task waits on the landing.

## After the landing

The orchestrator's, once the submission lands on main, since the code,
verify and submission stages all come before it:

1. Confirm the release CI makes from the landing's commit is a new minor
   version: `gh release view <tag> --repo agentplot/sluice` names that
   commit. The change's `feat` commits (c8748bc0, a13cfb16, 5fc33c30,
   452af6ce, 8232db61, f46abaae, 6177537d) make it a minor.
2. From that release's archive for the host, `sluice equip --help`
   prints the command.
3. Give the tag to flume's chore that pins sluice, so flume's change
   `a-places-readiness-is-one-design-across-the-tools`, groups 2 to 4,
   can ask `sluice equip` in place of its own copied steps.

## For the statement

The design's "Where the code shows the design wrong" gives four points
on decision 7's wording of 147, 33 and 84 for the blueprints chore that
writes those clauses: 147 placed in B.1 where the sequence is the check
profile's; 147's stop for a working copy only read, which covers the
affected reading's tree and a combination that runs nothing as well as
a submitter's worktree; 84's amendment calling the toolchain the
runner's own; and four variants of the sequence where the design counted
three. The build follows the readings the design gives. They are the
orchestrator's to carry to that chore.
