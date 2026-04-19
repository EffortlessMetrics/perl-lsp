use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_temp_perl_file() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::temp_dir();
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    path.push(format!(
        "perl-parser-bench-smoke-{}-{unique}.pl",
        std::process::id()
    ));
    fs::write(&path, "use strict;\nprint \"hello\\n\";\n")?;
    Ok(path)
}

#[test]
fn file_smoke_reports_success() -> Result<(), Box<dyn std::error::Error>> {
    let file = write_temp_perl_file()?;
    let output = Command::new(env!("CARGO_BIN_EXE_perl-parser-bench"))
        .arg(&file)
        .output()?;

    let _ = fs::remove_file(&file);

    assert!(
        output.status.success(),
        "benchmark binary should succeed on a valid file"
    );

    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("status=success"),
        "expected the success status line"
    );
    assert!(
        stdout.contains("error=false"),
        "expected a non-error parse result"
    );
    Ok(())
}
