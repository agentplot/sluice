//! What the sandbox refused a run, read from the kernel's own reports of
//! the run's mark, which the run's watch kept in `watch.out` as the run
//! went, since the persisted log keeps none of them (139; record 0072 as
//! amended 2026-11-11; design D6 of the change
//! `a-check-its-executor-refused-has-no-outcome`).
//!
//! The watch is `log stream --style ndjson`, filtered to the Seatbelt
//! reports that carry the run's mark. A line that is no JSON object, the
//! stream's first among them, is passed over, and so is one whose message
//! does not end with the run's mark. From each other it takes
//! `Sandbox: <process>(<pid>) deny(<n>) <operation> <target>`, after any
//! `<n> duplicate report(s) for `, and keeps each refusal once, in order.
//! Nothing a check printed, its exit or how it failed is read (38; rule
//! core.35).

/// The resolver's socket: a refusal of it is a name lookup refused.
const RESOLVER: &str = "/private/var/run/mDNSResponder";

/// How many refusals the words name before counting the rest.
const NAMED: usize = 3;

/// One thing the sandbox refused a run: what it refused, of what, and the
/// program that tried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// The program that tried, as the kernel names it.
    pub process: String,
    /// The operation refused, as the profile names it: `file-write-create`.
    pub operation: String,
    /// What it was refused of: a path, or an address.
    pub target: String,
}

/// The refusals of `mark` among `kept`, the lines a run's watch kept, each
/// once, in the order they were first reported.
pub fn read(kept: &str, mark: &str) -> Vec<Refusal> {
    let mut refusals: Vec<Refusal> = Vec::new();
    for line in kept.lines() {
        let Some(refusal) = refusal(line, mark) else {
            continue;
        };
        if !refusals.contains(&refusal) {
            refusals.push(refusal);
        }
    }
    refusals
}

/// The refusal one kept line reports, where it is a report of `mark`.
fn refusal(line: &str, mark: &str) -> Option<Refusal> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    let message = value.as_object()?.get("eventMessage")?.as_str()?;
    let report = message.trim_end().strip_suffix(mark)?;
    let report = report.strip_suffix('\n')?.trim_end();
    let report = after_duplicates(report);
    let report = report.strip_prefix("Sandbox: ")?;
    let (who, rest) = report.split_once(" deny(")?;
    let process = who.rsplit_once('(').map_or(who, |(p, _)| p);
    let (_, rest) = rest.split_once(") ")?;
    let (operation, target) = rest.split_once(' ').unwrap_or((rest, ""));
    Some(Refusal {
        process: process.to_string(),
        operation: operation.to_string(),
        target: target.to_string(),
    })
}

/// `report` after any `<n> duplicate report(s) for `.
fn after_duplicates(report: &str) -> &str {
    let Some((count, rest)) = report.split_once(' ') else {
        return report;
    };
    if count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit()) {
        return report;
    }
    ["duplicate report for ", "duplicate reports for "]
        .iter()
        .find_map(|p| rest.strip_prefix(p))
        .unwrap_or(report)
}

/// What the sandbox refused, in the profile's words: the first three
/// refusals and how many more, a refusal of the resolver's socket called
/// a name lookup, since that is what it refused (design D6).
pub fn words(refusals: &[Refusal]) -> String {
    let named: Vec<String> = refusals
        .iter()
        .take(NAMED)
        .map(|r| {
            let by = match (r.operation.as_str(), r.target.as_str()) {
                ("network-outbound", RESOLVER) => format!("a name lookup, by {}", r.process),
                _ => format!("by {}", r.process),
            };
            match r.target.is_empty() {
                true => format!("{} ({by})", r.operation),
                false => format!("{} {} ({by})", r.operation, r.target),
            }
        })
        .collect();
    let more = refusals.len().saturating_sub(NAMED);
    let list = match (named.split_last(), more) {
        (None, _) => String::new(),
        (Some((last, [])), 0) => last.clone(),
        (Some((last, rest)), 0) => format!("{} and {last}", rest.join(", ")),
        (Some(_), n) => format!("{} and {n} more", named.join(", ")),
    };
    format!("sluice's sandbox refused the run {list}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARK: &str = "sluice-run:0123456789abcdef";

    /// A kept line of the stream, as `log stream --style ndjson` writes a
    /// sandbox's report (design, "What macOS gives").
    fn kept(message: &str) -> String {
        serde_json::json!({
            "messageType": "Error",
            "eventType": "logEvent",
            "processImagePath": "/kernel",
            "senderImagePath": "/System/Library/Extensions/Sandbox.kext/Contents/MacOS/Sandbox",
            "timestamp": "2026-11-11 19:03:58.138566-0500",
            "eventMessage": message,
            "processID": 0,
        })
        .to_string()
    }

    /// 139, design D6 (the run-outcome spec's "The reports read"): of the
    /// stream's first line, a report of another mark, a duplicate report
    /// and five refusals of the run's mark, the run's refusals are the
    /// five, each once, named as the first three and 2 more, a refusal of
    /// the resolver's socket named as a name lookup.
    #[test]
    fn the_runs_marked_reports_are_its_refusals() {
        let ours = |r: &str| kept(&format!("Sandbox: {r}\n{MARK}"));
        let lines = [
            format!(
                "Filtering the log data using \"sender == \"Sandbox\" AND composedMessage \
                 CONTAINS \"{MARK}\"\""
            ),
            ours("bun(32182) deny(1) network-outbound /private/var/run/mDNSResponder"),
            kept("Sandbox: touch(1) deny(1) file-write-create /elsewhere\nsluice-run:fedcba9876543210"),
            ours("python3(32174) deny(1) file-write-create /Users/me/.cache/x"),
            kept(&format!(
                "3 duplicate reports for Sandbox: bun(32182) deny(1) network-outbound \
                 /private/var/run/mDNSResponder\n{MARK}"
            )),
            ours("python3(32174) deny(1) file-write-create /Users/me/.cache/x"),
            ours("git(40001) deny(1) file-write-unlink /Users/me/.gitconfig.lock"),
            kept(&format!(
                "1 duplicate report for Sandbox: Google Chrome Helper(5) deny(1) \
                 network-outbound remote:*:443\n{MARK}"
            )),
            ours("host(32182) deny(1) network-outbound remote:*:53"),
            "not json".to_string(),
            kept("Sandbox: touch(9) deny(1) file-write-create /no/mark"),
        ]
        .join("\n");
        let read = read(&lines, MARK);
        let seen: Vec<(&str, &str, &str)> = read
            .iter()
            .map(|r| (r.process.as_str(), r.operation.as_str(), r.target.as_str()))
            .collect();
        assert_eq!(
            seen,
            [
                ("bun", "network-outbound", "/private/var/run/mDNSResponder"),
                ("python3", "file-write-create", "/Users/me/.cache/x"),
                ("git", "file-write-unlink", "/Users/me/.gitconfig.lock"),
                ("Google Chrome Helper", "network-outbound", "remote:*:443"),
                ("host", "network-outbound", "remote:*:53"),
            ]
        );
        assert_eq!(
            words(&read),
            "sluice's sandbox refused the run network-outbound \
             /private/var/run/mDNSResponder (a name lookup, by bun), file-write-create \
             /Users/me/.cache/x (by python3), file-write-unlink /Users/me/.gitconfig.lock \
             (by git) and 2 more"
        );
        assert_eq!(
            words(&read[1..2]),
            "sluice's sandbox refused the run file-write-create /Users/me/.cache/x (by python3)"
        );
        assert_eq!(
            words(&read[..2]),
            "sluice's sandbox refused the run network-outbound \
             /private/var/run/mDNSResponder (a name lookup, by bun) and file-write-create \
             /Users/me/.cache/x (by python3)"
        );
        assert!(super::read(&lines, "sluice-run:ffffffffffffffff").is_empty());
    }
}
