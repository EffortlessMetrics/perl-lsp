use assert_cmd::Command;

fn perl_lsp_cmd() -> Command {
    if let Ok(p) =
        std::env::var("CARGO_BIN_EXE_perl-lsp").or_else(|_| std::env::var("CARGO_BIN_EXE_perl_lsp"))
    {
        Command::new(p)
    } else if which::which("perl-lsp").is_ok() {
        Command::new("perl-lsp")
    } else {
        let mut c = Command::new("cargo");
        c.args(["run", "-q", "-p", "perl-lsp", "--"]);
        c
    }
}

#[test]
fn health_prints_ok() {
    let mut cmd = perl_lsp_cmd();
    cmd.arg("--health").assert().success().stdout(predicates::str::contains("ok"));
}

#[test]
fn version_shows_git_tag() {
    let mut cmd = perl_lsp_cmd();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("perl-lsp"))
        .stdout(predicates::str::contains("Git tag:"));
}
