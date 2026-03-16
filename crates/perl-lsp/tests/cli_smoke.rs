use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn health_prints_ok() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("--health").assert().success().stdout(predicates::str::contains("ok"));
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
        .stdout(predicates::str::contains("LSP coverage:"));
}

#[test]
fn check_no_files_exits_with_error() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("--check").assert().failure().stderr(predicates::str::contains("No files specified"));
}

#[test]
fn check_valid_perl_file() {
    let dir = tempfile::tempdir().expect("failed to create tempdir");
    let file = dir.path().join("test.pl");
    std::fs::write(&file, "use strict;\nprint \"hello\\n\";\n")
        .expect("failed to write test file");
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.arg("--check")
        .arg(file.to_str().expect("non-UTF-8 temp path"))
        .assert()
        .success()
        .stdout(predicates::str::contains("ok"));
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
    cmd.args(["--completion", "powershell"]).assert().failure();
}

#[test]
fn help_mentions_new_flags() {
    let mut cmd = cargo_bin_cmd!("perl-lsp");
    let output = cmd.arg("--help").output().expect("failed to run perl-lsp --help");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--info"), "help should mention --info");
    assert!(stdout.contains("--check"), "help should mention --check");
    assert!(stdout.contains("--completion"), "help should mention --completion");
}
