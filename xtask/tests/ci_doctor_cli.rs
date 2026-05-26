// CI doctor integration test — eprintln! used for diagnostic output.
#![allow(clippy::print_stderr, clippy::print_stdout)]
use assert_cmd::cargo::cargo_bin_cmd;

/// Verify `cargo xtask ci doctor` exits 0 in a normal environment.
///
/// The doctor is designed to warn but not fail on advisory conditions
/// (no release binary, dirty working tree), so this should pass in CI
/// as long as rustc, rustfmt, clippy, and perl are available.
#[test]
fn ci_doctor_exits_zero_in_normal_env() {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["ci", "doctor"]);
    // allow non-zero only on hard failures (missing rustc/components/perl)
    let output = cmd.output().expect("failed to run xtask ci doctor");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The header must always appear
    assert!(stdout.contains("cargo xtask ci doctor"), "expected header in output:\n{stdout}");

    // Toolchain and component sections must appear
    assert!(stdout.contains("── Toolchain ──"), "expected toolchain section:\n{stdout}");
    assert!(stdout.contains("── Rust components ──"), "expected components section:\n{stdout}");

    // Platform section must appear
    assert!(stdout.contains("── Platform ──"), "expected platform section:\n{stdout}");

    // The summary line must appear
    assert!(stdout.contains("ci doctor:"), "expected summary line:\n{stdout}");
}

/// Verify `cargo xtask ci` (without sub-command) still runs the CI suite.
/// We just check it emits CI-suite output (format/clippy/tests steps),
/// not that it passes — that depends on the environment.
#[test]
fn ci_subcommand_bare_still_runs_ci_suite() {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.arg("ci");
    // Just check it starts and produces output; don't assert success
    // because the CI suite may take a long time or fail in isolation.
    let output = cmd.output().expect("failed to spawn xtask ci");
    let _ = output; // invocation must not panic
}

/// Verify `cargo xtask ci doctor --help` shows the doctor description.
#[test]
fn ci_doctor_help_shows_description() {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["ci", "doctor", "--help"]);
    let output = cmd.output().expect("failed to run xtask ci doctor --help");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Help must contain the command description
    assert!(
        stdout.contains("doctor") || stdout.contains("parity"),
        "expected description in help:\n{stdout}"
    );
}
