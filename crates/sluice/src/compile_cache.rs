//! The compile-cache step of the check seam's profile (35; ADR 0049 and
//! its amendment): `sluice compile-cache`, run inside a check run's
//! process group (and sandbox), between the toolchain and the command.
//! It names the one tool sluice serves, sccache, which is the check
//! profile's concern (rule core.1); the place profile reads the
//! declaration and refuses what this step cannot serve.

/// The one compile cache sluice serves.
pub const TOOL: &str = "sccache";

/// The tool's client's own variables start so.
pub const PREFIX: &str = "SCCACHE_";

/// What the step sets for a run's compiles itself, which a policy may
/// not name among the client's variables it takes from the host.
pub const SET: [&str; 5] = [
    "RUSTC_WRAPPER",
    "SCCACHE_NO_DAEMON",
    "SCCACHE_CLIENT_SIDE",
    "SCCACHE_START_SERVER",
    "SCCACHE_IDLE_TIMEOUT",
];
