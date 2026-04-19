// Tests use panic! as structured test failure reporters.
#![allow(clippy::panic)]

//! Tests for pre-launch syntax checking (issue #3477)
//!
//! The DAP server should run `perl -c` on the target script before launching
//! `perl -d`, and report syntax errors clearly instead of failing mid-execution.

use perl_dap::DapMessage;
use perl_dap::DebugAdapter;
use perl_tdd_support::must_some;
use serde_json::json;
use std::fs;

/// Helper: create a Perl script in a temp dir.
fn write_script(dir: &tempfile::TempDir, filename: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(filename);
    if let Err(e) = fs::write(&path, content) {
        panic!("write test script failed: {e}");
    }
    path
}

// ── syntax error cases ──────────────────────────────────────────────────────

#[test]
fn test_launch_rejects_missing_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();
    let tmp = tempfile::tempdir()?;

    // Missing semicolon between statements — perl -c catches this
    let script = write_script(&tmp, "missing_semicolon.pl", "my $x = 1\nprint $x;\n");

    let args = json!({
        "program": must_some(script.to_str()),
        "cwd":     must_some(tmp.path().to_str()),
        "args":    []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response {
            success, message, ..
        } => {
            assert!(
                !success,
                "Launch should fail: script has a missing semicolon"
            );
            let msg = must_some(message);
            assert!(
                msg.to_lowercase().contains("syntax") || msg.contains("line"),
                "Error message should mention 'syntax' or a line number, got: {msg}"
            );
        }
        _ => return Err("Expected a Response message".into()),
    }
    Ok(())
}

#[test]
fn test_launch_rejects_unclosed_brace() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();
    let tmp = tempfile::tempdir()?;

    // Unclosed block brace
    let script = write_script(
        &tmp,
        "unclosed_brace.pl",
        "sub foo {\n    my $x = 1;\n# missing closing brace\n",
    );

    let args = json!({
        "program": must_some(script.to_str()),
        "cwd":     must_some(tmp.path().to_str()),
        "args":    []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response {
            success, message, ..
        } => {
            assert!(!success, "Launch should fail: script has an unclosed brace");
            let msg = must_some(message);
            assert!(
                msg.to_lowercase().contains("syntax") || msg.contains("line"),
                "Error message should mention 'syntax' or a line number, got: {msg}"
            );
        }
        _ => return Err("Expected a Response message".into()),
    }
    Ok(())
}

#[test]
fn test_launch_rejects_simple_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();
    let tmp = tempfile::tempdir()?;

    // `use strict` + bareword: perl -c exits non-zero with a clear error.
    // Without strict, barewords are valid Perl, so we must use strict here.
    let script = write_script(
        &tmp,
        "simple_syntax_error.pl",
        "use strict;\nmy $x = BAREWORD_NOT_DEFINED;\n",
    );

    let args = json!({
        "program": must_some(script.to_str()),
        "cwd":     must_some(tmp.path().to_str()),
        "args":    []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response {
            success, message, ..
        } => {
            assert!(
                !success,
                "Launch should fail: script has a syntax error under strict"
            );
            let msg = must_some(message);
            assert!(
                msg.to_lowercase().contains("syntax") || msg.contains("line"),
                "Error message should mention 'syntax' or a line number, got: {msg}"
            );
        }
        _ => return Err("Expected a Response message".into()),
    }
    Ok(())
}

// ── valid script passes through ─────────────────────────────────────────────

#[test]
fn test_launch_allows_syntactically_valid_script() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();
    let tmp = tempfile::tempdir()?;

    // Valid Perl — no syntax error.  If perl is not on PATH the launch will
    // fail for a different reason, which is fine; the test asserts that the
    // failure is NOT attributed to a syntax error.
    let script = write_script(
        &tmp,
        "valid.pl",
        "use strict;\nuse warnings;\nmy $x = 42;\nprint \"$x\\n\";\n",
    );

    let args = json!({
        "program": must_some(script.to_str()),
        "cwd":     must_some(tmp.path().to_str()),
        "args":    []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response {
            success, message, ..
        } => {
            if !success {
                let msg = message.unwrap_or_default();
                assert!(
                    !msg.to_lowercase().contains("syntax error in"),
                    "Valid script was incorrectly rejected for syntax: {msg}"
                );
            }
            // success is fine too — perl was found and the session launched
        }
        _ => return Err("Expected a Response message".into()),
    }
    Ok(())
}

// ── message quality: line number ────────────────────────────────────────────

#[test]
fn test_syntax_error_message_contains_line_number() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();
    let tmp = tempfile::tempdir()?;

    // Assignment with no right-hand side on line 2
    let script = write_script(
        &tmp,
        "line_number_test.pl",
        "my $a = 1;\nmy $b = ;\nmy $c = 3;\n",
    );

    let args = json!({
        "program": must_some(script.to_str()),
        "cwd":     must_some(tmp.path().to_str()),
        "args":    []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response {
            success, message, ..
        } => {
            assert!(
                !success,
                "Launch should fail: assignment with no right-hand side"
            );
            let msg = must_some(message);
            assert!(
                msg.contains("line") || msg.contains("Line"),
                "Syntax error message should include a line reference, got: {msg}"
            );
        }
        _ => return Err("Expected a Response message".into()),
    }
    Ok(())
}

#[test]
fn test_launch_syntax_check_honors_perl5lib_env_override() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = DebugAdapter::new();
    let tmp = tempfile::tempdir()?;
    let lib_dir = tmp.path().join("lib");
    fs::create_dir_all(lib_dir.join("Local"))?;

    fs::write(
        lib_dir.join("Local").join("Helper.pm"),
        "package Local::Helper;\nsub ok { 1 }\n1;\n",
    )?;

    let script = write_script(
        &tmp,
        "needs_perl5lib.pl",
        "use strict;\nuse warnings;\nuse Local::Helper;\nprint Local::Helper::ok();\n",
    );

    let args = json!({
        "program": must_some(script.to_str()),
        "cwd": must_some(tmp.path().to_str()),
        "args": [],
        "env": {
            "PERL5LIB": must_some(lib_dir.to_str())
        }
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response {
            success, message, ..
        } => {
            if !success {
                let msg = message.unwrap_or_default();
                assert!(
                    !msg.contains("Local/Helper.pm")
                        && !msg.contains("Can't locate Local/Helper.pm"),
                    "Launch should honor PERL5LIB during syntax check, got: {msg}"
                );
            }
        }
        _ => return Err("Expected a Response message".into()),
    }
    Ok(())
}

#[test]
fn test_launch_reports_missing_module_with_install_hint() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = DebugAdapter::new();
    let tmp = tempfile::tempdir()?;

    for module_name in [
        "Some::Missing::Module",
        "Optional::Dep",
        "Tied::Hash::With::Spaces",
    ] {
        let script = write_script(
            &tmp,
            "missing_module.pl",
            &format!("use strict;\nuse warnings;\nuse {module_name};\n"),
        );

        let args = json!({
            "program": must_some(script.to_str()),
            "cwd": must_some(tmp.path().to_str()),
            "args": []
        });

        let response = adapter.handle_request(1, "launch", Some(args));

        match response {
            DapMessage::Response {
                success, message, ..
            } => {
                assert!(
                    !success,
                    "Launch should fail for missing module {module_name}"
                );
                let msg = must_some(message);
                assert!(
                    msg.contains(&format!("Module {module_name} not found")),
                    "Launch error should name the missing module, got: {msg}"
                );
                assert!(
                    msg.contains(&format!("cpan {module_name}")),
                    "Launch error should suggest cpan install, got: {msg}"
                );
                assert!(
                    msg.contains(&format!("metacpan.org/pod/{module_name}")),
                    "Launch error should link to MetaCPAN, got: {msg}"
                );
            }
            _ => return Err("Expected a Response message".into()),
        }
    }
    Ok(())
}
