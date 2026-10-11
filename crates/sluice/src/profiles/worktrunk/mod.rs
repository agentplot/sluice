//! The worktrunk driver profile's front door: worktrunk runs any
//! `wt-<name>` on the path as `wt <name>`, so `wt submit` runs
//! `wt-submit`, which forwards to `sluice submit` (C.5; ADR 0009).

use std::ffi::OsString;

/// The `sluice` command line a `wt submit` stands for.
pub fn forward(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut out = vec![OsString::from("submit")];
    out.extend(args);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wt_submit_is_sluice_submit() {
        let args = forward(["bolt/x".into(), "--no-wait".into()]);
        assert_eq!(args, vec!["submit", "bolt/x", "--no-wait"]);
    }
}
