# Design

## Context

The rule: clause 26 as amended on agentplot/blueprints's branch
`design/sluice-a-number-two-branches-hold-names-neither` (94fd86b),
record 0114 and rule core.11. A number that submissions on more than one
branch hold names none of them alone. A lookup by it is refused, naming
each branch that holds it and each that could not be read. The act that
asked takes the branch to name one.

Why two branches hold one number: `Records::allocate_id` in
`crates/sluice/src/profiles/records.rs` numbers from the remote's count
where a remote is opened, and from the host's own otherwise. A branch on
its remote and a branch in the host's own files each count from their
own. Record 0114 keeps the two counts (its option 2) and names the
submission by its branch instead.

Each reader of a submission by its number at `045cb7d0`:

| Reader | Where | Knows the branch |
|---|---|---|
| `sluice wait <n>` | `lib.rs` `Command::Wait`, `s.lookup` | no |
| the wait's follow, each read | `lib.rs` `record`, `landed` | yes, `sub.branch` |
| `sluice status <n>` | `lib.rs` `Command::Status`, `s.lookup` | no |
| `sluice amend <n>` | `ops.rs` `amend`, `find` | no |
| amend's `--json` and a refusal's owner | `lib.rs` `s.state(id)`, `owner`, `owner_next` | no |
| `sluice withdraw <n>` | `ops.rs` `withdraw`, `find` | no |
| a waiting submit's own submission | `lib.rs` `s.state(id)` after submit, `--status`'s `taken` | yes, its target |
| the pull request driver's withdrawals | `pulls.rs` `s.withdraw(sub.id, ..)`, three places | yes, `sub.branch` |
| a landing's commit for a wait | `ops.rs` `landed_as` | yes, in the wait |
| the board's blamed row | `register.tsx` `blamedTests` | yes, `b.branch` |
| the herdr plugin's `jump` and `learn` | `jump.sh`, `document.sh` | yes, the event's branch |

### Looking across

The other tools of the family that name a sluice submission by number:

- **Boomdrive** (`crates/boomdrive/src/sluice.rs` `Sluice::entry`) runs
  `sluice status <id> --json` in a place's clone. A number two branches
  hold is refused there, exit `2`, once this lands. Boomdrive knows the
  place's base, so its change is to pass `--branch`. That is
  agentplot/boomdrive's change.
- **crew**'s conductor amends, waits on and reads its units' submissions
  by number (the statement's C table, crew row). Each unit's branch is
  the bolt it lands on, so passing `--branch` is crew's change.
- **flume**'s merge queue profile reads `sluice status <base> --all
  --json` by branch, and needs nothing.

Each reads a number one branch holds as now, so nothing breaks until two
branches hold one.

## Goals / Non-Goals

**Goals:**

- No lookup by number acts on, or answers for, one of two submissions.
- Every command that looks one up can be told which branch.
- Every reader in this repository that knows the branch names it.

**Non-Goals:**

- One count for every branch (record 0114, option 2).
- The notices' hints, `sluice inbox` and `sluice confirm`'s keys.
- The herdr plugin's own cache of worktrees, keyed by number.

## Decisions

### D1. One lookup reads every branch

`Sluice::find(id, on)` is the one lookup. With no branch, it reads
every branch the records list, in their order, and stops at none. It
collects each branch that holds `id`, live or finished, and each branch
that could not be read:

- one branch holds it: `Found::On(branch)`, as now;
- none holds it: `Found::Unfound(unread)`, as now;
- more than one holds it: `Found::Several(Several { branches,
  unread })`, the branches in the records' order.

`Several` holds the branches that could not be read too, so the
refusal names them beside the ones that hold the number (record 0114's
third point). Several holders are refused even where some branch could
not be read, since a third holder changes nothing.

Path not taken: stop at the second holder. Reading on costs one read of
each remaining branch's live records, which the unfound case already
pays, and lets the refusal name every holder.

### D2. A branch named reads that branch alone

With `on = Some(branch)`, `find` reads that branch alone. It holds `id`:
`On(branch)`. It does not: `Unfound(vec![])`. It cannot be read:
`Unfound` naming it. No other branch is read, so a branch elsewhere that
cannot be read is never named.

The core's public reads take the branch as a variant, so the number-only
calls and their tests stand:

| Number alone | With the branch |
|---|---|
| `lookup(id)` | `lookup_on(branch, id)` |
| `state(id)` | `state_on(branch, id)` |
| `landed_as(id)` | `landed_as_on(branch, id)` |
| `amend(id, by, after, priority)` | `amend_on(branch, id, by, after, priority)` |
| `withdraw(id, by)` | `withdraw_on(branch, id, by)` |

Each pair calls one private body with `on: Option<&str>`.
`Lookup::Several`, `AmendRefusal::Several` and `WithdrawRefusal::Several`
carry the `Several`. `state` and `landed_as` answer `None` where the
number is on several branches, as they do where it is on none. Every
caller that knows the branch moves to the `_on` form (D5).

Path not taken: add `on` to the five calls. That touches some seventy
test call sites to pass `None`.

### D3. A moved submission is read the same way

D10's read, the submission an old number of the host's became when its
branch's records moved to the remote (`moved`), runs only where no
branch holds the number itself, as now. A `was` that names the number is
no holding of it. It reads every branch too: where more than one
submission was the number, it is refused with `Several` the same way.
With a branch named, it reads that branch alone. Record 0116's coming
home renumbers the other way, and is its own change.

### D4. The commands take `--branch`

`sluice wait`, `status`, `amend` and `withdraw` take `--branch
<branch>`: "The branch the submission's number is on, where more than
one branch holds it." With a number, the command reads that branch
alone (D2). A number one branch holds needs none.

`wait` and `status` take a branch as their target too. `--branch` with a
target that is a branch, or with `status` and no target, is a command
line sluice cannot read. It exits `64`, saying `--branch names the
branch of a submission's number; <target> is a branch`.

With `--branch` naming a branch that reads and holds no such number, the
command says `no submission <n> on <branch>`, with the exit it gives
`no submission <n>` now. `next` is `sluice status <branch>`. With
`--branch` naming a branch that cannot be read, it says the number is
on none of the branches sluice could read and names that branch, exit
`3`, as now.

Path not taken: `<branch>#<n>` or a second positional. A flag reads the
same on all four commands, and leaves `wait <branch>` and `status
<branch>` as they are.

### D5. The refusal's words and exits

Text, one line, then a line for each branch that could not be read, as
the unfound lines give it:

```
refused: submission 26 is on more than one branch: bolt/a, bolt/b; name one with --branch, as sluice withdraw 26 --branch bolt/a
bolt/c could not be read: <words>
```

Under `--json`, the error document: kind `refused`, its message those
lines joined by `; `, and `next` the command with `--branch` once for
each branch that holds the number, in the records' order (`sluice
withdraw 26 --branch bolt/a`, `sluice withdraw 26 --branch bolt/b`).
`next` names the command, the number and the branch. The asker adds
the flags they gave.

Exits: `status`, `amend` and `withdraw` exit `2`, as every refusal of
theirs does. `wait` exits `64`. A wait's `2` is a conflict, which
`wt submit` and an agent's background wait read as the work's outcome.
`64` is the exit kept apart from every outcome, and the command line is
what the asker changes. `sluice wait --help`'s exit codes say so.

Path not taken: `64` for all four. `2` is what a script calling
`withdraw` or `amend` already reads as refused.

### D6. Whatever knows the branch names it

- **The wait's follow.** `record(s, id, branch)` reads with
  `lookup_on(branch, id)`. `landed(s, id)` takes the branch and reads
  with `landed_as_on` and `state_on`.
- **A waiting submit.** Its read of the submission just made, and
  `--status`'s `taken`, read with `state_on` on the submit's target.
- **amend and withdraw's later reads.** Amend's `--json`, `owner` and
  `owner_next` read with `state_on` where the command was given the
  branch, and with `state` where it was not. The act found the number on
  one branch, so `state` finds it there too.
- **Hints.** Where the command was given `--branch`, every command it
  suggests for that submission carries `--branch <branch>`:
  `sluice status <n>`, `sluice withdraw <n>` and `sluice amend <n> --as
  …`. Otherwise hints are as now.
- **The pull request driver.** Its three `s.withdraw(sub.id, ..)` calls
  in `pulls.rs` call `withdraw_on(&sub.branch, ..)`.
- **The board.** `blamedTests($, id, branch)` runs `sluice status <id>
  --branch <branch> --json`. The row has `b.branch`.
- **The herdr plugin.** `jump.sh` reads the culprit's `branch` from the
  event it picked, and `learn` in `document.sh` is given the event's
  branch, `$f3`. Each runs `sluice status <id> --branch <branch>
  --json`.

The plugins and the CLI ship together at one version (rule core.55), so
the flag is always there for them. An older sluice on the path refuses
the flag with `64`, which both plugins already read as nothing learned.

### D7. The fakes number like two counts

`World::numbers_from(n)` in `crates/sluice-fakes` sets the number the
records give next. A test submits to one branch, sets the count back,
and submits to another, so both hold one number, as a remote's count
and the host's give.

## Risks / Trade-offs

- [A lookup reads every branch where it stopped at the first] → One
  read of each branch's live records, as the unfound case pays already.
  A finished record is read only where its index names the number.
- [A caller outside this repository breaks on a number two branches
  hold] → It got the wrong submission before, silently. It now gets a
  refusal naming the commands that work. Boomdrive and crew are named
  above.
- [Amend's `--json` reads `state` after the act, by number alone] → A
  second branch would have to take the number between the amend and the
  read. The read then answers none, and the document says only the
  number.

## Migration Plan

Nothing is migrated. Records and numbers stay as they are.
