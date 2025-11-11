use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::prelude::*;

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
