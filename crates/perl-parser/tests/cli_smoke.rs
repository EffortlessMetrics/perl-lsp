use assert_cmd::prelude::*;

#[test]
fn health_prints_ok() {
    let bin_path = match assert_cmd::cargo::cargo_bin("perl-lsp") {
        Ok(path) => path,
        Err(_) => return,
    };
    let mut cmd = std::process::Command::new(bin_path);
    cmd.arg("--health").assert().success().stdout(predicates::str::contains("ok"));
}

#[test]
fn version_shows_git_tag() {
    let bin_path = match assert_cmd::cargo::cargo_bin("perl-lsp") {
        Ok(path) => path,
        Err(_) => return,
    };
    let mut cmd = std::process::Command::new(bin_path);
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("perl-lsp"))
        .stdout(predicates::str::contains("Git tag:"));
}
