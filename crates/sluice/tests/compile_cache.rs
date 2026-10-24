//! The compile-cache step, `sluice compile-cache`, as a check run runs
//! it: what needs processes of its own to try, apart from the library's
//! tests, which start none.

mod support;

use std::path::Path;
use std::process::Command;

use support::Scratch;

/// A stand-in `sccache` in `dir/bin` running `body`; the directory.
fn fake_sccache(dir: &Path, body: &str) -> std::path::PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("sccache"), format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(
        bin.join("sccache"),
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .unwrap();
    bin
}

/// 35, design D4: a server of the run's own that never answers is ended
/// once sccache's own start limit passes, the command runs without the
/// cache and ends as it would have, and the directory is free again.
#[test]
fn a_server_that_never_listens_is_ended_and_falls_back() {
    let scratch = Scratch::new("cc-silent");
    let pid = scratch.path().join("pid");
    let bin = fake_sccache(
        scratch.path(),
        &format!("echo $$ > {}\nexec sleep 60", pid.display()),
    );
    // A socket's path is at most 104 bytes on macOS, and a sandboxed
    // gate's scratch directory is longer: the step runs in the scratch
    // directory and is given the cache's directory by a path from there.
    let out = Command::new(env!("CARGO_BIN_EXE_sluice"))
        .current_dir(scratch.path())
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("SLUICE_TMP", scratch.path())
        // As a run's environment has none from sluice.
        .env_remove("RUSTC_WRAPPER")
        .args(["compile-cache", "--own", "cc", "--", "/bin/sh", "-c"])
        .arg("echo \"${RUSTC_WRAPPER:-unset}\"; exit 4")
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(4), "{said}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "unset\n");
    assert!(
        said.contains("the compile server did not answer within 10s")
            && said.contains("compiling without the cache"),
        "{said}"
    );
    let pid: i32 = std::fs::read_to_string(&pid)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None),
        Err(nix::errno::Errno::ESRCH),
        "the server was left running"
    );
    let lock = std::fs::File::open(scratch.path().join("cc/lock")).unwrap();
    lock.try_lock().expect("the directory is free again");
}
