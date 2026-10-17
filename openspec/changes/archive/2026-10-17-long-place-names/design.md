# Design

## Context

Two functions in `crates/sluice/src/runs.rs` turn a checkout's name
into one file name: `flat_name` (`%` and `/` written `%25` and `%2F`,
a leading `.` as `%2E`), used for records, `.runs/`, `sessions/` and
`.logs/`; and `dir_name` (`+`, `%`, `/` written `+2B`, `+25`, `+2F`,
a leading `.` as `+2E`), used for a checkout's directory and its
scratch, where a `%` breaks clang's temporary files. A worktree's
checkout is named `session/` plus its whole path, so every `/` of the
path costs three bytes. The longest suffix sluice adds to one of these
names is `.json.tmp`, 9 bytes, written while a record is replaced.

Nothing parses a file name back into a name. `.places/` is listed by
reading each record's `path`, `.previews/` by each record's revision,
`sessions/` by each file's `worktree`; `.runs/` and `.logs/` are found
from the name, never listed for it.

## Goals / Non-Goals

**Goals:**
- Every name sluice makes fits in 255 bytes, its suffixes included.
- A name that fits today keeps exactly its file name.
- Two names never share a file name.

**Non-Goals:**
- Moving or renaming files an older sluice wrote.
- A whole path longer than the system's path limit.

## Decisions

**Keep a name of at most 246 bytes as it is.** 246 is 255 less the 9
bytes of `.json.tmp`, so every name that works today in every place
sluice writes it is at most 246 bytes, and is kept. A name between 247
and 255 bytes could make a run directory today but not its record, so
no place with such a name ever held a record to lose. Alternative: a
round limit such as 200, which would move the records of places that
work today and lose their ports.

**Shorten a longer one to a readable start, a mark and a hash.** The
first 160 bytes of the encoded name, cut where a character ends, then
`%~` (for `flat_name`) or `+~` (for `dir_name`), then the first 16 hex
digits of the SHA-1 of the whole name before it was encoded: at most
178 bytes. The start lets a person listing `.places/` see whose it is;
the hash tells apart two paths that share the start.

**The mark keeps shortened names apart from kept ones.** In a kept
`flat_name`, every `%` is followed by `2`; in a kept `dir_name`, every
`+` is. So `%~` or `+~` is in no kept name, and a shortened name can
never be one a shorter name is kept as, whatever its length.
Alternative: telling them apart by length alone, which would force a
shortened name longer than 246 bytes and leave it no room for a suffix.

**SHA-1 through `gix`.** `gix::hash::hasher(Kind::Sha1)` is already
linked, with collision detection; 64 bits of it leave a chance of two
paths on one host sharing a name far below anything else that can go
wrong. Alternatives: `std`'s `DefaultHasher`, whose output may change
between Rust releases and would move records; a new crate, for
nothing `gix` lacks.

**Shorten inside the two functions.** Every caller already goes through
`flat_name` or `dir_name`, so every file name gets the rule at once,
and a later caller cannot forget it.

## Risks / Trade-offs

- [A sluice older than this change, run on a host with a shortened
  name] → It fails as it does today with `File name too long`; it never
  finds a wrong record, since it never makes the shortened name.
- [Two paths whose SHA-1 share 64 bits] → Accepted: far less likely
  than a disk fault, and the path in the record would show it.
- [A record found by its path is listed with the path from its JSON,
  not its file name] → Already so; a test lists a shortened worktree's
  place by its path.

## Migration Plan

None: kept names are unchanged, and only names that never worked are
shortened. Rolling back leaves shortened files that an older sluice
neither reads nor needs, since it could never use those worktrees.
