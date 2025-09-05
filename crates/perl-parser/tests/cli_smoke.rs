use assert_cmd::Command;

#[test]
fn health_prints_ok() {
    let Ok(mut cmd) = Command::cargo_bin("perl-lsp") else {
        eprintln!("perl-lsp binary not built; skipping test");
        return;
    };
    cmd.arg("--health").assert().success().stdout(predicates::str::contains("ok"));
}

#[test]
fn version_shows_git_tag() {
    let Ok(mut cmd) = Command::cargo_bin("perl-lsp") else {
        eprintln!("perl-lsp binary not built; skipping test");
        return;
    };
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("perl-lsp"))
        .stdout(predicates::str::contains("Git tag:"));
}
