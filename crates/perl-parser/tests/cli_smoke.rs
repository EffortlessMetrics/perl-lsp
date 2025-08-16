use assert_cmd::Command;

#[test]
fn health_prints_ok() {
    let mut cmd = Command::cargo_bin("perl-lsp").unwrap();
    cmd.arg("--health").assert().success().stdout(predicates::str::contains("ok"));
}

#[test]
fn version_shows_git_tag() {
    let mut cmd = Command::cargo_bin("perl-lsp").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("perl-lsp"))
        .stdout(predicates::str::contains("Git tag:"));
}
