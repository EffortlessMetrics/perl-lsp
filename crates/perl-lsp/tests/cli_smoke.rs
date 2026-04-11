use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn health_prints_ok() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("--health")
        .assert()
        .success()
        .stdout(predicates::str::contains("ok"))
        .stdout(predicates::str::contains("Perl::Critic:"))
        .stdout(predicates::str::contains("status:"))
        .stdout(predicates::str::contains("enabled:"))
        .stdout(predicates::str::contains("severity:"));
}

#[test]
fn version_shows_git_tag() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("perl-lsp"))
        .stdout(predicates::str::contains("Git tag:"));
}

#[test]
fn help_prints_to_stdout() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("--help").assert().success().stdout(predicates::str::contains("Usage: perl-lsp"));
}

#[test]
fn info_shows_version_and_features() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("--info")
        .assert()
        .success()
        .stdout(predicates::str::contains("perl-lsp"))
        .stdout(predicates::str::contains("Features:"))
        .stdout(predicates::str::contains("LSP spec coverage:"));
}

#[test]
fn check_no_files_exits_with_error() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("--check").assert().failure().stderr(predicates::str::contains("No files specified"));
}

#[test]
fn check_valid_perl_file() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("test.pl");
    std::fs::write(&file, "use strict;\nprint \"hello\\n\";\n")?;
    let file_str = file.to_str().ok_or("non-UTF-8 temp path")?;
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("--check").arg(file_str).assert().success().stdout(predicates::str::contains("ok"));
    Ok(())
}

#[test]
fn check_nonexistent_file() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("--check")
        .arg("/nonexistent/path/to/file.pl")
        .assert()
        .failure()
        .stderr(predicates::str::contains("error reading file"));
}

#[test]
fn completion_bash_produces_output() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--completion", "bash"])
        .assert()
        .success()
        .stdout(predicates::str::contains("complete"));
}

#[test]
fn completion_zsh_produces_output() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--completion", "zsh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("compdef"));
}

#[test]
fn completion_fish_produces_output() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--completion", "fish"])
        .assert()
        .success()
        .stdout(predicates::str::contains("complete"));
}

#[test]
fn completion_unknown_shell_fails() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--completion", "unknown-shell"]).assert().failure();
}

#[test]
fn help_mentions_new_flags() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    let output = cmd.arg("--help").output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--info"), "help should mention --info");
    assert!(stdout.contains("--check"), "help should mention --check");
    assert!(stdout.contains("--completion"), "help should mention --completion");
    Ok(())
}

#[test]
fn trailing_files_without_check_flag_errors() {
    // Trailing file arguments should require --check
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("somefile.pl").assert().failure();
}
