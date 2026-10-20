//! A run's results as an in-toto statement (v1): one subject per check
//! and system, named `<check>@<system>` with the task runner's key as its
//! digest, and a predicate holding each outcome and the claims the
//! signer makes of the run (79; C.6).

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sluice_seams::{Outcome, Rev};

use super::dsse::{Envelope, IN_TOTO};

pub const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";

/// What a run's predicate is.
pub const RUN_PREDICATE: &str = "https://agentplot.github.io/sluice/run/v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statement {
    #[serde(rename = "_type")]
    pub type_: String,
    pub subject: Vec<Subject>,
    pub predicate_type: String,
    pub predicate: Run,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    pub name: String,
    pub digest: BTreeMap<String, String>,
}

/// The run's predicate: what it observed and what its signer claims of
/// it (16, 79, 85).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub results: Vec<RunResult>,
    pub claims: Claims,
    pub branch: String,
    pub combination: Rev,
    /// The claim it ran under (75); `None` for a run on a pull request's
    /// head (113) or a session's (89).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<String>,
    /// When it finished, RFC 3339, as the signer's host read its clock.
    pub at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunResult {
    pub check: String,
    pub key: String,
    pub outcome: Outcome,
}

/// 79: what the signer claims of the run; each counts only when the
/// policy lets the signer make it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// The executor's kind (C.2).
    pub executor: String,
    pub confined: bool,
    pub system: String,
}

impl Statement {
    /// The statement of one run's results on `claims.system`.
    pub fn of(run: Run) -> Statement {
        let subject = run
            .results
            .iter()
            .map(|r| Subject {
                name: format!("{}@{}", r.check, run.claims.system),
                digest: BTreeMap::from([("sha256".to_string(), r.key.clone())]),
            })
            .collect();
        Statement {
            type_: STATEMENT_TYPE.into(),
            subject,
            predicate_type: RUN_PREDICATE.into(),
            predicate: run,
        }
    }

    /// An unsigned envelope of the statement.
    pub fn envelope(&self) -> Result<Envelope> {
        Ok(Envelope::new(IN_TOTO, &serde_json::to_vec(self)?))
    }

    /// The statement an envelope holds, checked for being a run's: its
    /// subjects exactly its results on its system, so neither can say what
    /// the other does not.
    pub fn from_envelope(e: &Envelope) -> Result<Statement> {
        let body = e.payload_of(IN_TOTO)?;
        let s: Statement =
            serde_json::from_slice(&body).context("the envelope holds no in-toto statement")?;
        if s.type_ != STATEMENT_TYPE {
            bail!("the statement is {}, not {STATEMENT_TYPE}", s.type_);
        }
        if s.predicate_type != RUN_PREDICATE {
            bail!(
                "the statement's predicate is {}, not a run",
                s.predicate_type
            );
        }
        if Statement::of(s.predicate.clone()).subject != s.subject {
            bail!("the statement's subjects are not its results");
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn run() -> Run {
        Run {
            results: vec![
                RunResult {
                    check: "cli:build".into(),
                    key: "6de6b709".into(),
                    outcome: Outcome::Pass,
                },
                RunResult {
                    check: "cli:test".into(),
                    key: "4863a9af".into(),
                    outcome: Outcome::Fail,
                },
            ],
            claims: Claims {
                executor: "container".into(),
                confined: true,
                system: "linux-x64".into(),
            },
            branch: "main".into(),
            combination: Rev::new("c0ffee"),
            claim: Some("main/c3/linux-x64".into()),
            at: "2026-10-19T12:00:00Z".into(),
        }
    }

    #[test]
    fn a_runs_statement_names_each_check_on_its_system() {
        let s = Statement::of(run());
        assert_eq!(s.subject[0].name, "cli:build@linux-x64");
        assert_eq!(s.subject[0].digest["sha256"], "6de6b709");
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["_type"], STATEMENT_TYPE);
        assert_eq!(json["predicateType"], RUN_PREDICATE);
        assert_eq!(json["predicate"]["claims"]["system"], "linux-x64");
        let back = Statement::from_envelope(&s.envelope().unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn subjects_that_say_other_than_the_results_are_refused() {
        let mut s = Statement::of(run());
        s.subject[0].digest.insert("sha256".into(), "beef".into());
        let e = Envelope::new(IN_TOTO, &serde_json::to_vec(&s).unwrap());
        assert!(Statement::from_envelope(&e).is_err());
        let mut s = Statement::of(run());
        s.predicate_type = "https://slsa.dev/provenance/v1".into();
        let e = Envelope::new(IN_TOTO, &serde_json::to_vec(&s).unwrap());
        assert!(Statement::from_envelope(&e).is_err());
    }
}
