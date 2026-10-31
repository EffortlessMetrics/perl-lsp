//! The existing single-crate hook checks, under managed Linux storage supervision.

use super::{
    GateResult, Receipt, ReceiptSummary, collect_metadata, is_cargo_package_context_variable,
    storage,
};
use chrono::Utc;
use color_eyre::eyre::{Result, bail, eyre};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

/// Keep the caller's cwd/Git context and exact original Cargo arguments.
pub fn run(package: String) -> i32 {
    match run_checks(package) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(io::stderr(), "Cannot initialize single-crate hook: {error:#}");
            70
        }
    }
}

fn persist_receipt(receipt: &Receipt, path: &Path) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

fn run_checks(package: String) -> Result<i32> {
    if package.is_empty() {
        bail!("The single-crate hook requires a package name");
    }
    let directory = std::env::current_dir()?.join("target/receipts/single-crate-hook");
    let logs = directory.join("logs");
    let receipt_path = directory.join("receipt.json");
    fs::create_dir_all(&logs)?;
    let commands = [
        vec!["fmt", "-p", &package, "--", "--check"],
        vec!["clippy", "-p", &package, "--", "-D", "warnings"],
        vec!["test", "-p", &package],
    ];
    let mut receipt = Receipt {
        schema_version: "1.0.0".to_owned(),
        metadata: collect_metadata(Utc::now())?,
        gates: commands
            .iter()
            .map(|args| GateResult {
                gate_name: args[0].to_owned(),
                tier: "single_crate_hook".to_owned(),
                status: "skip".to_owned(),
                required: Some(true),
                duration_ms: 0,
                command: format!(
                    "cargo {}",
                    args.iter()
                        .map(|arg| format!("'{}'", arg.replace('\'', "'\"'\"'")))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
                exit_code: None,
                output_summary: Some("NOT RUN: hook has not completed this check".to_owned()),
                log_path: None,
                metrics: None,
                artifacts: None,
                first_failure: None,
            })
            .collect(),
        summary: ReceiptSummary {
            total_gates: 3,
            passed: 0,
            failed: 0,
            skipped: 3,
            timeout: None,
            error: None,
            total_duration_ms: 0,
            tier_results: None,
            overall_status: "fail".to_owned(),
            blocking_failures: None,
            aggregate_metrics: None,
        },
        agent_receipt: None,
        diff_config: None,
    };
    persist_receipt(&receipt, &receipt_path)?;
    let started = Instant::now();
    for (index, args) in commands.iter().enumerate() {
        let log = logs.join(format!("{}.log", args[0]));
        let mut command = Command::new("cargo");
        command.args(args);
        // Remove only metadata injected by the supervisor's cargo run. Caller
        // routing, profile settings, Git selectors and parent environment survive.
        for (key, _) in std::env::vars_os() {
            if is_cargo_package_context_variable(&key, false) {
                command.env_remove(key);
            }
        }
        let start = Instant::now();
        let execution = storage::Policy::from_environment(&log, &receipt_path)
            .and_then(|policy| {
                policy.ok_or_else(|| eyre!("Use scripts/cargo-safe for managed hook routing"))
            })
            .and_then(|policy| {
                storage::run_direct(
                    &mut command,
                    &receipt.gates[index].command,
                    &log,
                    &receipt_path,
                    &policy,
                )
            })
            .unwrap_or_else(|error| storage::Execution {
                output: format!("Storage guard setup failed; command not completed: {error:#}\n"),
                exit_code: 75,
                timed_out: false,
                supervision_error: true,
            });
        // Output is bounded and delivered after each check. Reporting failures
        // must not hide a substantive failure, or turn a successful check green.
        let output_error = io::stdout().write_all(execution.output.as_bytes()).err();
        let exit_code = if execution.exit_code == 0 && output_error.is_some() {
            70
        } else {
            execution.exit_code
        };
        let result = &mut receipt.gates[index];
        result.status = if execution.supervision_error
            || (execution.exit_code == 0 && output_error.is_some())
        {
            "error"
        } else if execution.exit_code == 0 {
            "pass"
        } else {
            "fail"
        }
        .to_owned();
        result.exit_code = Some(exit_code);
        result.duration_ms = start.elapsed().as_millis() as u64;
        result.log_path = log.is_file().then(|| format!("logs/{}.log", args[0]));
        result.output_summary = Some(
            execution.output.chars().rev().take(4096).collect::<String>().chars().rev().collect(),
        );
        if let Some(error) = output_error {
            let message = format!("Cannot deliver single-crate hook output: {error}");
            let _ = writeln!(io::stderr(), "{message}");
            result.output_summary.get_or_insert_with(String::new).push_str(&format!("\n{message}"));
        }
        receipt.summary.passed = receipt.gates.iter().filter(|g| g.status == "pass").count() as u32;
        receipt.summary.failed = receipt.gates.iter().filter(|g| g.status == "fail").count() as u32;
        receipt.summary.skipped =
            receipt.gates.iter().filter(|g| g.status == "skip").count() as u32;
        receipt.summary.error =
            Some(receipt.gates.iter().filter(|g| g.status == "error").count() as u32);
        receipt.summary.total_duration_ms = started.elapsed().as_millis() as u64;
        receipt.summary.overall_status =
            if receipt.summary.passed == 3 { "pass" } else { "fail" }.to_owned();
        receipt.summary.blocking_failures = Some(
            receipt
                .gates
                .iter()
                .filter(|g| g.status != "pass")
                .map(|g| g.gate_name.clone())
                .collect(),
        );
        // A normal receipt write failure is non-green; the independent runtime
        // emergency record is retained when storage supervision failed.
        if let Err(error) = persist_receipt(&receipt, &receipt_path) {
            let _ = writeln!(io::stderr(), "Cannot persist single-crate hook receipt: {error:#}");
            return Ok(if exit_code == 0 { 70 } else { exit_code });
        }
        if exit_code != 0 {
            return Ok(exit_code);
        }
    }
    Ok(0)
}
