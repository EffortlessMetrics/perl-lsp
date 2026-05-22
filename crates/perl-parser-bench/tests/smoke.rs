use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(kind: &str, extension: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::temp_dir();
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    path.push(format!("perl-parser-bench-{kind}-{}-{unique}.{extension}", std::process::id()));
    Ok(path)
}

fn write_temp_perl_file() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = temp_path("smoke", "pl")?;
    fs::write(&path, "use strict;\nprint \"hello\\n\";\n")?;
    Ok(path)
}

fn create_temp_perl_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = temp_path("dir", "d")?;
    fs::create_dir(&path)?;
    fs::write(path.join("one.pl"), "use strict;\nmy $x = 1;\n")?;
    fs::write(path.join("two.pm"), "package Two;\nsub value { 2 }\n1;\n")?;
    Ok(path)
}

#[test]
fn file_smoke_reports_success() -> Result<(), Box<dyn std::error::Error>> {
    let file = write_temp_perl_file()?;
    let output = Command::new(env!("CARGO_BIN_EXE_perl-parser-bench")).arg(&file).output()?;

    let _ = fs::remove_file(&file);

    assert!(output.status.success(), "benchmark binary should succeed on a valid file");

    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("status=success"), "expected the success status line");
    assert!(stdout.contains("error=false"), "expected a non-error parse result");
    Ok(())
}

#[test]
fn directory_smoke_reports_aggregate_success() -> Result<(), Box<dyn std::error::Error>> {
    let dir = create_temp_perl_dir()?;
    let output = Command::new(env!("CARGO_BIN_EXE_perl-parser-bench")).arg(&dir).output()?;

    let _ = fs::remove_dir_all(&dir);

    assert!(output.status.success(), "benchmark binary should succeed on a directory");

    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("total_files=2"), "expected both files to be counted: {stdout}");
    assert!(stdout.contains("error_files=0"), "expected valid files to parse cleanly: {stdout}");
    assert!(
        stdout.contains("success_rate=100.0"),
        "expected all valid files to count as successful: {stdout}"
    );
    Ok(())
}

#[test]
fn missing_path_reports_error_status() -> Result<(), Box<dyn std::error::Error>> {
    let missing = temp_path("missing", "pl")?;
    let output = Command::new(env!("CARGO_BIN_EXE_perl-parser-bench")).arg(&missing).output()?;

    assert!(!output.status.success(), "missing path should fail");

    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("Path does not exist"), "expected missing-path diagnostic: {stderr}");
    Ok(())
}

#[test]
fn missing_argument_reports_usage_error() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_perl-parser-bench")).output()?;

    assert!(!output.status.success(), "missing argument should fail");

    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("Usage: perl-parser-bench"), "expected usage diagnostic: {stderr}");
    Ok(())
}
