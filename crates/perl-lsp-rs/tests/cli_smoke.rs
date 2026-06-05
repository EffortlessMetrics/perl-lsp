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
    cmd.arg("--help").assert().success().stdout(predicates::str::contains("Usage:"));
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
        .stderr(predicates::str::contains("error reading file"))
        .stderr(predicates::str::contains("help: check the path"));
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
fn perltidy_compat_report_prints_native_mapping() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let profile = dir.path().join(".perltidyrc");
    std::fs::write(&profile, "-l=100\n-nsok\n-q\n")?;

    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--perltidy-compat-report", profile.to_str().ok_or("non-UTF-8 temp path")?])
        .assert()
        .success()
        .stdout(predicates::str::contains("# Native Format Perltidy Compatibility"))
        .stdout(predicates::str::contains("format.line_width"))
        .stdout(predicates::str::contains("format.keyword_spacing"));
    Ok(())
}

#[test]
fn perlcritic_compat_report_prints_native_mapping() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let profile = dir.path().join(".perlcriticrc");
    std::fs::write(
        &profile,
        "severity = 3\n[TestingAndDebugging::RequireUseStrict]\n[InputOutput::RequireCheckedOpen]\n",
    )?;

    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--perlcritic-compat-report", profile.to_str().ok_or("non-UTF-8 temp path")?])
        .assert()
        .success()
        .stdout(predicates::str::contains("# Native Critic Perlcritic Compatibility"))
        .stdout(predicates::str::contains("native.testing.require_use_strict"))
        .stdout(predicates::str::contains("native.io.unchecked_open_close"));
    Ok(())
}

#[test]
fn check_project_missing_dir_errors() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let missing = dir.path().join("missing-project");
    let missing_str = missing.to_str().ok_or("non-UTF-8 temp path")?;

    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--check-project", missing_str])
        .assert()
        .failure()
        .stderr(predicates::str::contains("directory not found"));

    Ok(())
}

#[test]
fn check_project_file_path_errors() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("not-a-directory.pl");
    std::fs::write(&file, "use strict;\n")?;
    let file_str = file.to_str().ok_or("non-UTF-8 temp path")?;

    let mut cmd = cargo_bin_cmd!("perl-lsp");
    cmd.args(["--check-project", file_str])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not a directory"));

    Ok(())
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
