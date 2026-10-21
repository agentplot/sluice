//! What reads as a secret, replaced before text leaves the host: in a
//! telemetry body (ADR 0022) and in a failed check's output posted on a
//! pull request (ADR 0039). It lowers the chance a secret leaves and does
//! not rule it out.

use regex::Regex;

/// What reads as a secret whatever variable it came from.
const SHAPES: &str = concat!(
    r"gh[pousr]_[A-Za-z0-9]{20,}",
    r"|github_pat_[A-Za-z0-9_]{20,}",
    r"|A[KS]IA[0-9A-Z]{16}",
    r"|xox[abprs]-[A-Za-z0-9-]{10,}",
    r"|sk-ant-[A-Za-z0-9_-]{10,}",
    r"|eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]+",
    r"|(?i:bearer)\s+[A-Za-z0-9._~+/=-]{8,}",
    r"|-----BEGIN [A-Z ]*PRIVATE KEY-----(?s:.)*?(?:-----END [A-Z ]*PRIVATE KEY-----|\z)",
);

/// A variable whose value is never sent.
fn secret_name(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "KEY",
        "PASS",
        "AUTH",
        "CREDENTIAL",
        "HEADERS",
    ]
    .iter()
    .any(|w| n.contains(w))
}

#[derive(Debug)]
pub struct Scrub {
    /// The values of variables whose names say they are secret.
    secrets: Vec<String>,
    shapes: Regex,
}

impl Scrub {
    /// Scrubbing the values of `env`'s secret variables and every secret
    /// shape.
    pub fn new(env: impl Iterator<Item = (String, String)>) -> Scrub {
        let mut secrets: Vec<String> = env
            .filter(|(k, v)| secret_name(k) && v.len() >= 8)
            .map(|(_, v)| v)
            .collect();
        // The longest first, so none is left half replaced.
        secrets.sort_by_key(|v| std::cmp::Reverse(v.len()));
        Scrub {
            secrets,
            shapes: Regex::new(SHAPES).expect("the secret shapes"),
        }
    }

    pub fn scrub(&self, text: &str) -> String {
        let mut text = text.to_string();
        for s in &self.secrets {
            text = text.replace(s.as_str(), "[redacted]");
        }
        self.shapes.replace_all(&text, "[redacted]").into_owned()
    }
}

/// The last `limit` bytes of `text`, from a character's start.
pub fn end(text: &str, limit: usize) -> &str {
    let start = text.len().saturating_sub(limit);
    let start = (start..=text.len())
        .find(|i| text.is_char_boundary(*i))
        .unwrap_or(text.len());
    &text[start..]
}

/// The last `n` lines of `text`.
pub fn last_lines(text: &str, n: usize) -> &str {
    let mut start = text.len();
    let body = text.trim_end_matches('\n');
    let mut seen = 0;
    for (i, b) in body.bytes().enumerate().rev() {
        if b == b'\n' {
            seen += 1;
            if seen == n {
                start = i + 1;
                break;
            }
        }
    }
    if seen < n {
        start = 0;
    }
    &text[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_by_name_and_by_shape_are_redacted() {
        let s = Scrub::new([("API_TOKEN".to_string(), "s3cr3t-value".to_string())].into_iter());
        let text = "token s3cr3t-value then GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz0123\n";
        let out = s.scrub(text);
        assert!(
            !out.contains("s3cr3t-value") && !out.contains("ghp_"),
            "{out}"
        );
        assert_eq!(out.matches("[redacted]").count(), 2);
    }

    #[test]
    fn the_last_lines() {
        assert_eq!(last_lines("a\nb\nc\n", 2), "b\nc\n");
        assert_eq!(last_lines("a\nb\nc", 2), "b\nc");
        assert_eq!(last_lines("a\n", 5), "a\n");
        assert_eq!(end("héllo", 4), "llo");
    }
}
