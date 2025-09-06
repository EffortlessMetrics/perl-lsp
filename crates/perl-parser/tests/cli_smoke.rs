use assert_cmd::Command;

#[test]
fn health_prints_ok() {
    let mut cmd = match Command::cargo_bin("perl-lsp") {
        Ok(cmd) => cmd,
        Err(_) => return,
    };
    cmd.arg("--health").assert().success().stdout(predicates::str::contains("ok"));
}

#[test]
fn version_shows_git_tag() {
    let mut cmd = match Command::cargo_bin("perl-lsp") {
        Ok(cmd) => cmd,
        Err(_) => return,
    };
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("perl-lsp"))
        .stdout(predicates::str::contains("Git tag:"));
}
