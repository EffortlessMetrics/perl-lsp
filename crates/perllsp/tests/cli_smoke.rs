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

fn failed_stderr(output: std::process::Output) -> Result<String, Box<dyn std::error::Error>> {
    if !output.status.success() {
        return String::from_utf8(output.stderr).map_err(Into::into);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(format!("command unexpectedly succeeded with stdout: {stdout}").into())
}

#[test]
fn help_mentions_perllsp() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = successful_stdout(run_perllsp(&["--help"])?)?;
    assert!(stdout.contains("Usage: perllsp"), "help should mention the facade name");
    Ok(())
}

#[test]
fn help_examples_use_facade_name() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = successful_stdout(run_perllsp(&["--help"])?)?;
    assert!(stdout.contains("perllsp --stdio"), "stdio example should use facade name");
    assert!(
        stdout.contains("perllsp --completion bash"),
        "completion example should use facade name"
    );
    assert!(!stdout.contains("perl-lsp --"), "facade help should not leak perl-lsp examples");
    Ok(())
}

#[test]
fn version_mentions_facade_name_and_git_tag() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = successful_stdout(run_perllsp(&["--version"])?)?;
    assert!(stdout.contains("perllsp "), "version should print the facade name");
    assert!(stdout.contains("Git tag:"), "version should include the git tag line");
    Ok(())
}

#[test]
fn bash_completion_uses_facade_command_and_function() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = successful_stdout(run_perllsp(&["--completion", "bash"])?)?;
    assert!(stdout.contains("_perllsp()"), "bash completion should rename the function");
    assert!(
        stdout.contains("complete -F _perllsp perllsp"),
        "bash completion should register the facade binary"
    );
    assert!(
        !stdout.contains("perl-lsp"),
        "bash completion should not leak the implementation binary"
    );
    Ok(())
}

#[test]
fn zsh_completion_uses_facade_command_and_function() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = successful_stdout(run_perllsp(&["--completion", "zsh"])?)?;
    assert!(stdout.contains("#compdef perllsp"), "zsh completion should target facade binary");
    assert!(stdout.contains("_perllsp()"), "zsh completion should rename the function");
    assert!(stdout.contains("_perllsp \"$@\""), "zsh completion should invoke renamed function");
    assert!(
        !stdout.contains("perl-lsp"),
        "zsh completion should not leak the implementation binary"
    );
    Ok(())
}

#[test]
fn fish_completion_uses_facade_command() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = successful_stdout(run_perllsp(&["--completion", "fish"])?)?;
    assert!(!stdout.trim().is_empty(), "fish completion should render at least one command");
    assert!(
        stdout.lines().all(|line| line.contains("-c perllsp")),
        "every fish completion line should target the facade binary: {stdout}"
    );
    assert!(
        !stdout.contains("perl-lsp"),
        "fish completion should not leak the implementation binary"
    );
    Ok(())
}

#[test]
fn powershell_completion_uses_facade_command() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = successful_stdout(run_perllsp(&["--completion", "powershell"])?)?;
    assert!(
        stdout.contains("-CommandName perllsp"),
        "powershell completion should register the facade binary"
    );
    assert!(
        !stdout.contains("CommandName perl-lsp"),
        "powershell completion should not register the implementation binary"
    );
    Ok(())
}

#[test]
fn unknown_completion_shell_reports_supported_values() -> Result<(), Box<dyn std::error::Error>> {
    let stderr = failed_stderr(run_perllsp(&["--completion", "nushell"])?)?;
    assert!(stderr.contains("Unknown shell: nushell"), "stderr should name the invalid shell");
    assert!(
        stderr.contains("Supported: bash, zsh, fish, powershell"),
        "stderr should list supported shells"
    );
    assert!(stderr.contains("Usage: perllsp"), "error help should use the facade name");
    Ok(())
}
