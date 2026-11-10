//! What reads as a secret, replaced before text leaves the host: in a
//! telemetry body (ADR 0022), in a failed check's output posted on a pull
//! request (ADR 0044), and in a credential command's words, which its
//! failure carries to a report, the branch's log, the host's kept state
//! and a status (record 0108). It lowers the chance a secret leaves and
//! does not rule it out.

use regex::Regex;

/// What begins a GitHub token: the flake module's `prefixes`, in
/// `nix/modules/options.nix`, which it refuses in `forge.credential` (rule
/// core.51), in its order; a test holds the two lists to each other
/// (record 0108).
pub const GITHUB_TOKEN_PREFIXES: [&str; 6] =
    ["ghs_", "ghp_", "gho_", "ghu_", "ghr_", "github_pat_"];

/// A GitHub token: one of [`GITHUB_TOKEN_PREFIXES`] followed by sixteen or
/// more of a token's characters, as flume's design 7 counts a prefixed
/// token, with no word boundary before it, since a false match here costs
/// only a word (record 0108).
fn github_token() -> String {
    let prefixes: Vec<String> = GITHUB_TOKEN_PREFIXES
        .iter()
        .map(|p| regex::escape(p))
        .collect();
    format!("(?:{})[A-Za-z0-9_-]{{16,}}", prefixes.join("|"))
}

/// What reads as a secret whatever variable it came from, beside a GitHub
/// token.
const SHAPES: &str = concat!(
    r"A[KS]IA[0-9A-Z]{16}",
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
    /// Scrubbing every secret shape and no variable's value: a credential
    /// command's words (record 0108).
    pub fn shapes() -> Scrub {
        Scrub::new(std::iter::empty())
    }

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
            shapes: Regex::new(&format!("{}|{SHAPES}", github_token())).expect("the secret shapes"),
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

    /// The forge-identity spec's "Each prefix's token redacted" and "A
    /// prefix with no token after it" (record 0108).
    #[test]
    fn each_github_token_prefix_is_redacted() {
        let s = Scrub::shapes();
        for prefix in GITHUB_TOKEN_PREFIXES {
            assert_eq!(
                s.scrub(&format!("before {prefix}0123456789abcdef after")),
                "before [redacted] after",
                "{prefix}"
            );
            let short = format!("{prefix}0123456789abcde");
            assert_eq!(s.scrub(&short), short, "{prefix}");
        }
        assert_eq!(s.scrub("expected a ghs_ token"), "expected a ghs_ token");
        assert_eq!(s.scrub("ghp_0123456789abcdef_ghij-klm"), "[redacted]");
    }

    /// The forge-identity spec's "The module's list and sluice's differ":
    /// the module's `prefixes`, the quoted strings between `prefixes = [`
    /// and the next `];`, are [`GITHUB_TOKEN_PREFIXES`] (record 0108).
    #[test]
    fn the_github_token_prefixes_are_the_modules() {
        use std::collections::BTreeSet;
        let module = include_str!("../../../nix/modules/options.nix");
        let start = module
            .find("prefixes = [")
            .expect("the module's prefixes = [")
            + "prefixes = [".len();
        let list = &module[start..];
        let list = &list[..list.find("];").expect("the module's prefixes' ];")];
        let modules: BTreeSet<&str> = list.split('"').skip(1).step_by(2).collect();
        let sluices: BTreeSet<&str> = GITHUB_TOKEN_PREFIXES.into_iter().collect();
        assert_eq!(
            modules, sluices,
            "the module's prefixes in nix/modules/options.nix, {modules:?}, \
             and sluice's GITHUB_TOKEN_PREFIXES, {sluices:?}, differ"
        );
    }

    #[test]
    fn the_last_lines() {
        assert_eq!(last_lines("a\nb\nc\n", 2), "b\nc\n");
        assert_eq!(last_lines("a\nb\nc", 2), "b\nc");
        assert_eq!(last_lines("a\n", 5), "a\n");
        assert_eq!(end("héllo", 4), "llo");
    }
}
