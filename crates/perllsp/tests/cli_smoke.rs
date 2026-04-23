use std::process::Command;

fn run_perllsp(args: &[&str]) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_perllsp")).args(args).output()?;
    Ok(output)
}

fn successful_stdout(output: std::process::Output) -> Result<String, Box<dyn std::error::Error>> {
    if output.status.success() {
        return String::from_utf8(output.stdout).map_err(Into::into);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("command failed with status {}: {}", output.status, stderr).into())
}

#[test]
fn help_mentions_perllsp() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = successful_stdout(run_perllsp(&["--help"])?)?;
    assert!(stdout.contains("Usage: perllsp"), "help should mention the facade name");
    Ok(())
}

#[test]
fn version_mentions_facade_name_and_git_tag() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = successful_stdout(run_perllsp(&["--version"])?)?;
    assert!(stdout.contains("perllsp "), "version should print the facade name");
    assert!(stdout.contains("Git tag:"), "version should include the git tag line");
    Ok(())
}
