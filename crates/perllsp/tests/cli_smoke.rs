use std::process::Command;

fn run_perllsp(args: &[&str]) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_perllsp"))
        .args(args)
        .output()?;
    Ok(output)
}

#[test]
fn help_mentions_perllsp() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_perllsp(&["--help"])?;
    assert!(output.status.success(), "help should succeed");

    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("Usage: perllsp"),
        "help should mention the facade name"
    );
    Ok(())
}

#[test]
fn version_mentions_facade_name_and_git_tag() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_perllsp(&["--version"])?;
    assert!(output.status.success(), "version should succeed");

    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("perllsp "),
        "version should print the facade name"
    );
    assert!(
        stdout.contains("Git tag:"),
        "version should include the git tag line"
    );
    Ok(())
}
