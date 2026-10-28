//! `sluice branch create` and `sluice branch delete` (branches-1 104 to
//! 106): sluice makes and removes the branches it lands on, on their
//! remote when their policy names one and in local refs otherwise.
//! `sluice branch accept` (121): the operator accepts the revision a held
//! branch is read at.

use std::collections::BTreeSet;
use std::io::Write;

use anyhow::Result;
use sluice_core::branches::{AcceptRefusal, MakeRefusal, RemoveRefusal};
use sluice_core::Sluice;

use crate::told::{self, Kind};
use crate::{BranchCommand, CONFLICT};

pub fn run(s: &Sluice<'_>, command: BranchCommand, out: &mut dyn Write) -> Result<i32> {
    match command {
        BranchCommand::Create { branch, from, json } => create(s, &branch, &from, json, out),
        BranchCommand::Delete { branch, into, json } => delete(s, &branch, &into, json, out),
        BranchCommand::Accept {
            branch,
            revision,
            json,
        } => accept(s, &branch, &revision, json, out),
    }
}

fn create(
    s: &Sluice<'_>,
    branch: &str,
    from: &str,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let refusal = match s.make_branch(branch, from)? {
        Ok(at) => {
            if json {
                let doc = serde_json::json!({ "branch": branch, "created": at, "from": from });
                writeln!(out, "{}", serde_json::to_string_pretty(&doc)?)?;
            } else {
                writeln!(out, "created {branch} at {at}, {from}'s revision")?;
            }
            return Ok(0);
        }
        Err(r) => r,
    };
    let (kind, text) = match refusal {
        MakeRefusal::Busy => (
            Kind::Refused,
            format!("refused: a step holds {branch}; ask again"),
        ),
        MakeRefusal::NoFrom => (Kind::NotFound, format!("no branch {from}")),
        MakeRefusal::FromNotManaged => (
            Kind::Refused,
            format!("refused: no [branches] table in .config/sluice.toml on {from} names {from}"),
        ),
        MakeRefusal::NotManaged => (
            Kind::Refused,
            format!("refused: no [branches] table in .config/sluice.toml on {from} names {branch}"),
        ),
        MakeRefusal::Queued(ids) => (
            Kind::Refused,
            format!(
                "refused: {branch}'s records still queue {}; withdraw them first",
                crate::hashes(&ids, None, &mut BTreeSet::new())
            ),
        ),
        MakeRefusal::Present(at) => (
            Kind::Refused,
            match at {
                Some(at) => format!("refused: {branch} is already there, at {at}"),
                None => format!("refused: {branch} is already there"),
            },
        ),
    };
    told::tell(out, json, kind, &text, &[format!("sluice status {branch}")])?;
    Ok(exit(kind))
}

fn delete(
    s: &Sluice<'_>,
    branch: &str,
    into: &str,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let refusal = match s.remove_branch(branch, into)? {
        Ok(at) => {
            if json {
                let doc = serde_json::json!({ "branch": branch, "deleted": at, "into": into });
                writeln!(out, "{}", serde_json::to_string_pretty(&doc)?)?;
            } else {
                writeln!(
                    out,
                    "deleted {branch}, which was at {at}, landed into {into}"
                )?;
            }
            return Ok(0);
        }
        Err(r) => r,
    };
    let (kind, text) = match refusal {
        RemoveRefusal::Busy => (
            Kind::Refused,
            format!("refused: a step holds {branch}; ask again"),
        ),
        RemoveRefusal::Absent => (Kind::NotFound, format!("no branch {branch}")),
        RemoveRefusal::NoInto => (Kind::NotFound, format!("no branch {into}")),
        RemoveRefusal::Queued(ids) => (
            Kind::Refused,
            format!(
                "refused: {} still queued on {branch}; wait for them to land, or withdraw them",
                crate::hashes(&ids, None, &mut BTreeSet::new())
            ),
        ),
        RemoveRefusal::LandingUnfinished => (
            Kind::Refused,
            format!("refused: a landing on {branch} is unfinished; the next round settles it"),
        ),
        RemoveRefusal::NotLanded(at) => (
            Kind::Refused,
            format!("refused: {branch} at {at} has not landed into {into}; submit it there first"),
        ),
        RemoveRefusal::Elsewhere(at) => (
            Kind::Refused,
            match at {
                Some(at) => format!("refused: {branch} moved to {at} while it was being deleted"),
                None => format!("refused: {branch} went while it was being deleted"),
            },
        ),
    };
    told::tell(out, json, kind, &text, &[format!("sluice status {branch}")])?;
    Ok(exit(kind))
}

fn accept(
    s: &Sluice<'_>,
    branch: &str,
    revision: &str,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let refusal = match s.accept_branch(branch, revision)? {
        Ok(a) => {
            if json {
                let doc = serde_json::json!({
                    "branch": branch, "accepted": a.revision, "lacked": a.lacked,
                });
                writeln!(out, "{}", serde_json::to_string_pretty(&doc)?)?;
            } else {
                writeln!(
                    out,
                    "accepted {branch} at {}, which lacks the landing {}; \
                     the next round forms the queue on it",
                    a.revision, a.lacked
                )?;
            }
            return Ok(0);
        }
        Err(r) => r,
    };
    let (kind, text) = match refusal {
        AcceptRefusal::Busy => (
            Kind::Refused,
            format!("refused: a step holds {branch}; ask again"),
        ),
        AcceptRefusal::Absent => (Kind::NotFound, format!("no branch {branch}")),
        AcceptRefusal::NotHeld(read) => (
            Kind::Refused,
            format!("refused: {branch} is not held; it reads {read}, which holds its last landing"),
        ),
        AcceptRefusal::NotHeldAt { held, read } if held == read => (
            Kind::Refused,
            format!("refused: {branch} is held at {held}, not at {revision}"),
        ),
        AcceptRefusal::NotHeldAt { held, read } => (
            Kind::Refused,
            format!(
                "refused: {branch} is held at {held} and now reads {read}; \
                 the next round holds it there or forms on it"
            ),
        ),
    };
    told::tell(out, json, kind, &text, &[format!("sluice status {branch}")])?;
    Ok(exit(kind))
}

fn exit(kind: Kind) -> i32 {
    match kind {
        Kind::NotFound => 1,
        _ => CONFLICT,
    }
}
