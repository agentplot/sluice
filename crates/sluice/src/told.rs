//! What a command could not do, said so its reader can act on it. With
//! `--json`, one `{"error": ...}` document goes to stdout in place of the
//! one the command would have printed, so a reader that asked for JSON
//! always gets JSON; without it, the text is what it always was.

use std::io::Write;

use anyhow::Result;

/// Why a command did not do what it was asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// It was asked something sluice will not do: a refusal (exit 2).
    Refused,
    /// What it was asked about does not exist (exit 1).
    NotFound,
    /// sluice itself failed (exit 3).
    Internal,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Kind::Refused => "refused",
            Kind::NotFound => "not_found",
            Kind::Internal => "internal",
        }
    }
}

/// The error document: `{"error":{"kind","message","next"}}`, where
/// `next` holds the commands that would move the reader on.
pub fn document(kind: Kind, text: &str, next: &[String]) -> serde_json::Value {
    let message = text.strip_prefix("refused: ").unwrap_or(text);
    serde_json::json!({
        "error": { "kind": kind.word(), "message": message, "next": next }
    })
}

/// Say `text`, or with `json` its document, on `out`.
pub fn tell(
    out: &mut dyn Write,
    json: bool,
    kind: Kind,
    text: &str,
    next: &[String],
) -> Result<()> {
    if json {
        writeln!(out, "{}", document(kind, text, next))?;
    } else {
        writeln!(out, "{text}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_loses_its_prefix_and_keeps_its_next_steps() {
        let d = document(
            Kind::Refused,
            "refused: no branch nosuch",
            &["sluice overview".to_string()],
        );
        assert_eq!(
            d,
            serde_json::json!({"error": {
                "kind": "refused",
                "message": "no branch nosuch",
                "next": ["sluice overview"],
            }})
        );
    }

    #[test]
    fn without_json_the_text_is_unchanged() {
        let mut out = Vec::new();
        tell(&mut out, false, Kind::NotFound, "no submission 9", &[]).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "no submission 9\n");
    }
}
