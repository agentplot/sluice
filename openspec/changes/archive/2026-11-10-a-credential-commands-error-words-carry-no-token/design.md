## Context

See proposal.md for why. Record 0108 (accepted 2026-11-09, as amended
that day) says what holds: nothing of the token is kept, and a command
that gives no token fails the call, naming the command and its words.
This design says how sluice keeps a token out of those words on this
change's base.

**The family.** Three readings of a GitHub token's shape stand today,
and all three name the same six prefixes:

- flume refuses a program's setting shaped as a credential (flume
  design 7, `crates/flume-core/src/own.rs`, `PREFIXES`). It reads a
  token as a known prefix opening a run, followed by sixteen or more
  letters, digits, `_` or `-`.
- sluice's flake module refuses `forge.credential` where an element
  begins with `ghs_`, `ghp_`, `gho_`, `ghu_`, `ghr_` or `github_pat_`
  (`nix/modules/options.nix`, `prefixes`; rule core.51). A value in the
  Nix store is read by every user, so any element that begins with a
  prefix is refused.
- sluice's scrub (`crates/sluice/src/scrub.rs`) redacts what reads as a
  secret before text leaves the host. That covers a telemetry body
  (record 0022) and a failed check's output posted on a pull request
  (record 0044). Its GitHub shapes are `gh[pousr]_` and `github_pat_`,
  each followed by twenty or more characters.

No other tool of the family passes a credential helper's words on.
This change makes sluice's scrub the one reading of a token in text that
sluice passes on. It builds that reading from the module's list, and
counts a token's run as flume does. Giving the family's credential
shapes a single owner across the tools is a question for the statement's
principles. Nothing here waits on it, and nothing here moves away from
it.

What the code has today:

- `CredentialCommand::get` (`crates/sluice/src/identity.rs`) runs the
  command. `drain` reads its standard output and its standard error on
  threads of their own, keeping the first 64 KiB (`OUTPUT_LIMIT`) of
  each and passing over the rest. `get` takes the standard error as
  `words`, lossily decoded and trimmed, and every failure it makes ends
  `: <words>`. The failure never gives the standard output.
- `App::get_token` and `App::renewed`
  (`crates/sluice/src/profiles/github/app.rs`) pass `get`'s failure up
  unchanged. `check::commanded` (`crates/sluice/src/profiles/github/check.rs`)
  puts it in `sluice forge check`'s finding. A round's call, a wait and
  a status carry it as any failure of the forge's identity.
- `Scrub::new(env)` scrubs the values of `env`'s secret-named
  variables, then every shape in `SHAPES`. The telemetry exporter builds
  one from sluice's environment, and the pull request's report builds
  one from `std::env::vars()`.

## Goals / Non-Goals

**Goals:**

- A credential command's standard error is carried in its failure, and
  so wherever that failure goes, with every string shaped as a GitHub
  token redacted.
- The prefixes sluice redacts are the module's. A test fails when the
  two lists differ.
- A token cut at the 64 KiB limit leaves none of itself in the words.

**Non-Goals:**

- Reading the command's standard output into a failure, or redacting
  the `password` a failed command printed. sluice reads no standard
  output of a command that failed.
- Changing the scrub's other shapes, or what it reads from variables.
- A family-wide list of credential shapes (proposal, Out of scope).

## Decisions

### D1. The words go through the scrub at the one place they are read

`get` builds its `words` from the command's standard error, then
replaces them with `Scrub::shapes().scrub(&words)` before any failure
is made. `Scrub::shapes()` is a new constructor, `Scrub::new` of no
variables: every shape the scrub knows, and no variable's value. Every
reader of the failure gets words that are already redacted. That covers
`forge check`'s finding, the App's failure, a round's report, a log
line or kept state made from it, and a status. None of them needs a
scrub of its own. The telemetry exporter scrubs again, which changes
nothing.

The client still names no forge (rules core.1, core.26). `identity.rs`
calls the scrub, the core's one module of what reads as a secret, and
holds no shape itself. The module doc of `identity.rs`, and `get`'s doc,
say the words are scrubbed, citing record 0108.

Rule core.28 rules out a remote's words shortened or rewritten. The
command's words are the command's, not a remote's, so redacting them
rewrites no remote's words. Rule core.28's details say so.

Not taken:

- **Leaving the standard error out of the failure.** Record 0108 says
  the failure names the command and its words, and an operator reads
  why the command refused from them.
- **A list of GitHub prefixes in `identity.rs`.** That would be a second
  home for the prefixes, and it would make the client name a forge.
- **The values of sluice's secret-named variables,** as the pull
  request's report scrubs. The command inherits sluice's environment,
  but a variable named for `KEY` or `AUTH` often holds a path or a
  socket, and redacting those makes the command's words harder to read.
  A token taken from a variable still has its shape.
- **Redacting at each reader,** the log, the kept state, a status. Each
  reader would be a separate home, and the first one missed lets a token
  through.

### D2. One list of GitHub prefixes, the module's

`scrub.rs` gets `pub const GITHUB_TOKEN_PREFIXES: [&str; 6]`: `ghs_`,
`ghp_`, `gho_`, `ghu_`, `ghr_` and `github_pat_`, in the module's
order. Its doc says it is the module's `prefixes`, in
`nix/modules/options.nix`, and a test holds them to each other. The
GitHub part of the scrub's shapes is built from the list:
`(?:<each prefix, escaped, joined by |>)[A-Za-z0-9_-]{16,}`. It
replaces `gh[pousr]_[A-Za-z0-9]{20,}` and
`github_pat_[A-Za-z0-9_]{20,}`. The other shapes stay as they are.
`Scrub::new` compiles the joined pattern once, as it compiles `SHAPES`
today.

- **Sixteen or more of a token's characters,** letters, digits, `_` and
  `-`, as flume's design 7 counts a prefixed token. GitHub's tokens run
  thirty-six or more characters past their prefix, so every real one
  matches. A bare prefix, or a short word such as `ghs_token`, stays as
  written.
- **No word boundary before the prefix.** flume asks for one because a
  false match there refuses a person's setting. A false match here
  costs only a word of the output, so the scrub redacts more rather
  than less.
- **The test.** `the_github_token_prefixes_are_the_modules` reads
  `nix/modules/options.nix` with `include_str!`. It takes the quoted
  strings between `prefixes = [` and the next `];`, and compares them
  with `GITHUB_TOKEN_PREFIXES` as sets. The module has kept its list
  there since `forge.credential` was added. `cli:test`'s inputs in
  `crates/sluice/moon.yml` gain `/nix/modules/options.nix`, so a change
  to the module's list alone runs the test again rather than replaying
  its pass. The inputs already name `/nix/tests/files/**/*` for the same
  reason.

The scrub already knows GitHub's shapes beside AWS's, Slack's and
Anthropic's. It names the issuers of tokens so it can read their
secrets, and reaches no forge through them. It is no seam, so rule
core.1 reads the list as it reads the shapes today. A list kept in the
GitHub profile would make the core import a profile to scrub, which
core.1 rules out.

The change reaches the scrub's other uses. A telemetry body, or a pull
request's posted output, now also loses a GitHub-prefixed run of
sixteen to nineteen characters, and a run continued through `_` or `-`.
That only adds redaction.

Not taken:

- **The module's own rule,** any element that begins with a prefix. In
  text, that would redact a bare `ghs_` and any short identifier that
  starts with a prefix. In the store, a refusal costs the operator an
  evaluation. Here it costs the reader the words.
- **Keeping twenty characters.** That would be a third reading of a
  token in the family, where flume's design 7 already gives one.
- **Generating the Nix list from Rust, or Rust's from Nix.** The module
  and the crate are built apart, and a test is the guard the gate
  already uses for one setting kept in two homes (rule core.24's two
  pins).

### D3. Words cut at their limit end at a whole line

`drain` returns what it kept and whether it passed over anything past
`OUTPUT_LIMIT`. Where `get`'s standard error was cut, `words` is the
kept part up to its last line break, then trimmed and scrubbed. Where
the kept part holds no line break, there are no words, and the failure
ends after what went wrong. Where the standard error was not cut,
`words` is all of it, as today.

A token straddling the 64 KiB limit may be kept as its prefix and
fewer than sixteen of its characters. That is too short for its shape,
yet still part of the token. Dropping the cut line keeps all of it out.
The standard output's cut is unchanged, since no failure gives it.

Not taken:

- **A shape that matches a prefix at the end of the text whatever
  follows,** as the private key's shape runs to the text's end. The
  scrub serves telemetry's tail too, whose text starts at a cut rather
  than ending at one. The cut is `get`'s concern, so `get` handles it.
- **A marker saying the words were cut.** A failure whose words run to
  64 KiB is already long. The command's own log is where its whole error
  output is read.

### D4. The page and the record

- `site/src/content/docs/reference/configuration.md`, in `[forge]`'s
  `credential` paragraphs: the failure gives what the command wrote on
  its standard error, with anything shaped as a GitHub token replaced
  by `[redacted]`. Where the command wrote more than 64 KiB, the failure
  gives its whole lines within that. Name no host or tool of ours (rules
  core.1, core.26).
- Record 0108 gets a second amendment, dated the day it is written,
  recording D1 to D3, each with the path not taken.
- Rule core.28's details: also rules out a credential command's standard
  error carried into a failure, the branch's log, the host's kept state
  or a status with a GitHub token's text in it. Not ruled out: those
  words with each string shaped as a token replaced by `[redacted]`,
  since they are the command's and not the remote's. Both cite record
  0108 as amended.

## Risks / Trade-offs

- **A token of another shape passes.** A command that gives the token
  of a forge other than GitHub, or prints an opaque secret, is caught
  only by the scrub's other shapes. Record 0044 says the same of the
  scrub: it lowers the chance a secret leaves and doesn't rule it out.
  sluice's forge is GitHub, and the app's token has GitHub's shape.
- **A word that only looks like a token is redacted.** A prefix followed
  by sixteen token characters that is no token loses its text in the
  words. Only how the words read suffers.
- **The module's list changes without the scrub's.** The test in D2
  fails the gate's `cli:test` until both say the same.

## Migration Plan

None. Nothing is stored in a new form. A failure already written to the
branch's log or the host's kept state before this change stays as it
was written (rule core.41: no line changed once written).

## Open Questions

None.
