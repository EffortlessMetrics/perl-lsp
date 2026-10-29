//! Exercise managed storage setup through the actual gate CLI without a build.
#![cfg(target_os = "linux")]

use anyhow::{Context, Result, bail};
use std::ffi::CString;
use std::fs::{self, File};
use std::os::unix::{ffi::OsStrExt, process::CommandExt};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn fifo_without_writer_fails_closed_instead_of_hanging_admission() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let fifo = temp.path().join("unusable-output-volume");
    let name = CString::new(fifo.as_os_str().as_bytes())?;
    // SAFETY: a valid NUL-terminated path in this test's private directory.
    if unsafe { libc::mkfifo(name.as_ptr(), 0o600) } != 0 {
        return Err(std::io::Error::last_os_error()).context("create private FIFO");
    }
    let receipt = temp.path().join("receipt.json");
    let log = temp.path().join("cli.log");
    let output = File::create(&log)?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "gates",
            "--gate",
            "readme_heading_check",
            "--receipt",
            "--format",
            "json",
            "--receipt-path",
        ])
        .arg(&receipt)
        .env("DEVPLANE", &fifo)
        .env("CARGO_HOME", temp.path())
        .env("CARGO_TARGET_DIR", temp.path())
        .env("CARGO_BUILD_BUILD_DIR", temp.path())
        .env("TMPDIR", temp.path())
        .env_remove("SCCACHE_DIR")
        .env("MIN_FREE_GB", "20")
        .env("MAX_USED_PCT", "95")
        .stdout(Stdio::from(output.try_clone()?))
        .stderr(Stdio::from(output))
        .process_group(0)
        .spawn()?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() >= Duration::from_secs(5) {
            let pid = i32::try_from(child.id())?;
            // SAFETY: the still-owned child leads this fixture's private group.
            if unsafe { libc::kill(-pid, libc::SIGKILL) } != 0 {
                child.kill()?;
            }
            child.wait()?;
            bail!(
                "FIFO admission hung; fixture group terminated. Output: {}",
                fs::read_to_string(log)?
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(!status.success(), "FIFO must not be accepted as an output destination");
    let receipt: serde_json::Value = serde_json::from_slice(&fs::read(receipt)?)?;
    assert_eq!(receipt["summary"]["overall_status"], "fail");
    assert_eq!(receipt["gates"][0]["status"], "error");
    assert_eq!(receipt["gates"][0]["exit_code"], 75);
    assert!(
        receipt["gates"][0]["output_summary"]
            .as_str()
            .is_some_and(|summary| summary.contains("Unsupported storage destination type"))
    );
    Ok(())
}
