// `select_path_candidate` and `candidate_priority` are cross-platform and
// exported for test use on all platforms.  Import them unconditionally so the
// ripr quality gate can observe call paths to these seams on Linux CI runners.
use crate::os_runtime::{candidate_priority, select_path_candidate};
#[cfg(windows)]
use crate::os_runtime::{
    resolve_cmd_exe, resolve_command_invocation, windows_program_priority, windows_quote_for_cmd,
};
use crate::*;

#[test]
fn test_subprocess_output_success() {
    let output = SubprocessOutput { stdout: vec![1, 2, 3], stderr: vec![], status_code: 0 };
    assert!(output.success());
}

#[test]
fn test_subprocess_output_failure() {
    let output = SubprocessOutput { stdout: vec![], stderr: b"error".to_vec(), status_code: 1 };
    assert!(!output.success());
    assert_eq!(output.stderr_lossy(), "error");
}

#[test]
fn test_subprocess_error_display() {
    let error = SubprocessError::new("test error");
    assert_eq!(format!("{}", error), "test error");
}

#[test]
fn test_mock_runtime() {
    use mock::*;

    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"formatted code".to_vec()));

    let result = runtime.run_command("perltidy", &["-st"], Some(b"my $x = 1;"));

    assert!(result.is_ok());
    let output = perl_tdd_support::must(result);
    assert!(output.success());
    assert_eq!(output.stdout_lossy(), "formatted code");

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].program, "perltidy");
    assert_eq!(invocations[0].args, vec!["-st"]);
    assert_eq!(invocations[0].stdin, Some(b"my $x = 1;".to_vec()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_os_runtime_echo() {
    let runtime = OsSubprocessRuntime::new();
    #[cfg(windows)]
    let result = runtime.run_command("cmd.exe", &["/C", "echo", "hello"], None);
    #[cfg(not(windows))]
    let result = runtime.run_command("echo", &["hello"], None);

    assert!(result.is_ok());
    let output = perl_tdd_support::must(result);
    assert!(output.success());
    assert!(output.stdout_lossy().trim() == "hello");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_os_runtime_nonexistent() {
    let runtime = OsSubprocessRuntime::new();

    let result = runtime.run_command("nonexistent_program_xyz", &[], None);

    assert!(result.is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_os_runtime_rejects_empty_program_name() {
    let runtime = OsSubprocessRuntime::new();
    let result = runtime.run_command("   ", &["--version"], None);
    assert!(result.is_err());
    let err = result.expect_err("empty program name must be rejected");
    assert!(err.message.contains("must not be empty"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_os_runtime_rejects_nul_bytes_in_program_or_args() {
    let runtime = OsSubprocessRuntime::new();

    let bad_program = runtime.run_command("perl\0", &["--version"], None);
    assert!(bad_program.is_err());
    let bad_program_err = bad_program.expect_err("NUL in program must be rejected");
    assert!(bad_program_err.message.contains("NUL"));

    let bad_arg = runtime.run_command("perl", &["-e", "print \"ok\"\0"], None);
    assert!(bad_arg.is_err());
    let bad_arg_err = bad_arg.expect_err("NUL in arg must be rejected");
    assert!(bad_arg_err.message.contains("NUL"));
}

#[cfg(windows)]
#[test]
fn test_resolve_command_invocation_uses_cmd_for_batch_wrappers() {
    let (program, args) = perl_tdd_support::must(resolve_command_invocation(
        r"C:\Strawberry\perl\bin\perltidy.bat",
        &["-st", "-se"],
    ));

    // The cmd.exe used for batch wrappers is resolved to an ABSOLUTE path —
    // never the bare "cmd.exe", which CreateProcess would search CWD-first.
    assert!(
        std::path::Path::new(&program).is_absolute()
            && program.to_ascii_lowercase().ends_with("cmd.exe"),
        "batch wrapper must run via an absolute cmd.exe path, not a bare name; got: {program}"
    );
    assert_eq!(
        args,
        vec![
            "/D".to_string(),
            "/V:OFF".to_string(),
            "/S".to_string(),
            "/C".to_string(),
            "\"C:\\Strawberry\\perl\\bin\\perltidy.bat\" \"-st\" \"-se\"".to_string(),
        ]
    );
}

/// Verify that cmd.exe shell metacharacters are handled correctly inside
/// double-quoted tokens.
///
/// Inside a cmd.exe double-quoted region, shell metacharacters (`&`, `|`,
/// `<`, `>`, `(`, `)`) are already literal — no `^` prefix is used.
/// `^` is also literal and must not be doubled.
/// `%` is doubled to prevent `%VAR%` expansion.
/// `"` is doubled (`""`) per the cmd.exe shell convention.
#[cfg(windows)]
#[test]
fn test_windows_quote_for_cmd_metacharacters_are_literal_inside_quotes() {
    // Metacharacters & | < > are passed through literally — no ^ prefix.
    // ^ is literal — must NOT be doubled.
    // % is doubled to prevent %VAR% expansion.
    // " is doubled (cmd.exe "" convention), not backslash-escaped.
    let quoted = windows_quote_for_cmd(r#"profile&name|1>%TEMP%^"x""#);
    assert_eq!(quoted, r#""profile&name|1>%%TEMP%%^""x""""#);
}

/// Verify that a caret in an argument is not doubled.
///
/// The original PR erroneously included `'^'` in the metacharacter match
/// arm, which caused `windows_quote_for_cmd("foo^bar")` to return
/// `"foo^^bar"` — delivering two carets to the program.  Inside a
/// cmd.exe double-quoted region `^` is literal and must not be escaped.
#[cfg(windows)]
#[test]
fn test_windows_quote_for_cmd_caret_not_doubled() {
    let quoted = windows_quote_for_cmd(r"foo^bar");
    assert_eq!(quoted, r#""foo^bar""#);
}

/// Verify that an embedded double-quote uses the cmd.exe `""` convention.
///
/// The original PR used `\"` which is the `CommandLineToArgvW` / C-runtime
/// convention.  In cmd.exe context the backslash is literal and the `"`
/// terminates the quoted region, breaking argument boundaries.
#[cfg(windows)]
#[test]
fn test_windows_quote_for_cmd_embedded_quote_uses_doubling() {
    let quoted = windows_quote_for_cmd(r#"arg"with"quotes"#);
    // cmd.exe convention: "" represents a literal " inside a quoted token.
    assert_eq!(quoted, r#""arg""with""quotes""#);
}

/// Verify that an attacker-controlled injection attempt is rendered inert.
///
/// An arg like `&calc.exe` must not break out of the quoted token.
/// After quoting, cmd.exe sees `&` as a literal character inside the
/// double-quoted region.
#[cfg(windows)]
#[test]
fn test_windows_quote_for_cmd_injection_attempt_is_inert() {
    let quoted = windows_quote_for_cmd("&calc.exe");
    assert_eq!(quoted, "\"&calc.exe\"");
}

/// Verify that /V:OFF is present in the cmd.exe argument list.
///
/// Without /V:OFF, cmd.exe with delayed expansion enabled would expand
/// `!VAR!` patterns inside arguments, which is an information-disclosure
/// vector and, in edge cases, an injection vector.
#[cfg(windows)]
#[test]
fn test_resolve_command_invocation_includes_v_off_flag() {
    let (program, args) = perl_tdd_support::must(resolve_command_invocation(
        r"C:\tools\perlcritic.bat",
        &["--profile=!TEMP!"],
    ));

    assert!(
        std::path::Path::new(&program).is_absolute()
            && program.to_ascii_lowercase().ends_with("cmd.exe"),
        "batch wrapper must run via an absolute cmd.exe path; got: {program}"
    );
    assert!(
        args.contains(&"/V:OFF".to_string()),
        "/V:OFF must be present to disable delayed expansion; got: {:?}",
        args
    );
}

#[cfg(windows)]
#[test]
fn test_resolve_command_invocation_preserves_executable_paths() {
    let (program, args) = perl_tdd_support::must(resolve_command_invocation(
        r"C:\tools\perlcritic.exe",
        &["--version"],
    ));

    assert_eq!(program, r"C:\tools\perlcritic.exe");
    assert_eq!(args, vec!["--version".to_string()]);
}

#[cfg(windows)]
#[test]
fn test_windows_program_priority_prefers_real_wrappers_over_extensionless_shims() {
    let mut candidates = [
        r"C:\Strawberry\perl\bin\perltidy".to_string(),
        r"C:\Strawberry\perl\bin\perltidy.bat".to_string(),
        r"C:\tools\perltidy.exe".to_string(),
    ];
    candidates.sort_by_key(|candidate| windows_program_priority(candidate));

    assert_eq!(candidates.last().map(String::as_str), Some(r"C:\tools\perltidy.exe"));
    assert!(
        windows_program_priority(r"C:\Strawberry\perl\bin\perltidy.bat")
            > windows_program_priority(r"C:\Strawberry\perl\bin\perltidy")
    );
}

// --- Lossy UTF-8 conversion contracts (invalid bytes) ---

#[test]
fn test_stdout_lossy_replaces_invalid_utf8() {
    // 0xFF is never valid UTF-8; lossy conversion must substitute U+FFFD and
    // preserve the valid prefix rather than panic or drop bytes.
    let output =
        SubprocessOutput { stdout: vec![b'o', b'k', 0xFF], stderr: Vec::new(), status_code: 0 };
    let text = output.stdout_lossy();
    assert!(text.starts_with("ok"), "valid prefix preserved: {text:?}");
    assert!(text.contains('\u{FFFD}'), "invalid byte replaced with U+FFFD: {text:?}");
}

#[test]
fn test_stderr_lossy_replaces_each_invalid_byte() {
    let output = SubprocessOutput { stdout: Vec::new(), stderr: vec![0xFF, 0xFE], status_code: 1 };
    // Two independent invalid bytes become two replacement characters.
    assert_eq!(output.stderr_lossy(), "\u{FFFD}\u{FFFD}");
}

#[test]
fn test_lossy_helpers_on_empty_streams_return_empty() {
    let output = SubprocessOutput { stdout: Vec::new(), stderr: Vec::new(), status_code: 0 };
    assert_eq!(output.stdout_lossy(), "");
    assert_eq!(output.stderr_lossy(), "");
}

// --- MockResponse constructors ---

#[test]
fn test_mock_response_success_sets_zero_status_and_empty_stderr() {
    let response = mock::MockResponse::success(b"out".to_vec());
    assert_eq!(response.stdout, b"out");
    assert!(response.stderr.is_empty(), "success carries no stderr");
    assert_eq!(response.status_code, 0);
}

#[test]
fn test_mock_response_failure_sets_stderr_and_status_with_empty_stdout() {
    let response = mock::MockResponse::failure(b"boom".to_vec(), 2);
    assert!(response.stdout.is_empty(), "failure carries no stdout");
    assert_eq!(response.stderr, b"boom");
    assert_eq!(response.status_code, 2);
}

// --- SubprocessError as a std::error::Error trait object ---

#[test]
fn test_subprocess_error_usable_as_std_error_trait_object() {
    let error = SubprocessError::new("disk full");
    let dyn_error: &dyn std::error::Error = &error;
    assert_eq!(dyn_error.to_string(), "disk full");
    assert!(dyn_error.source().is_none(), "leaf error has no source");
}

// --- CWD-exclusion security invariant: cross-platform call-observation tests ---
//
// `select_path_candidate` and `candidate_priority` are pure functions that use
// only cross-platform APIs (`Path::is_absolute`, `std::fs::canonicalize`,
// `Path::parent`).  These tests exercise the security-critical seams on ALL
// platforms — including Linux CI — so the ripr quality gate can observe the
// call paths without requiring a Windows runner.
//
// The tests use POSIX-absolute paths (`/usr/bin/perltidy`) on non-Windows and
// Windows-absolute paths (`C:\tools\perltidy.exe`) on Windows.  The underlying
// logic is identical on both; only the path syntax differs.

/// Layer 1 — absolute check: a relative candidate is always rejected, returning
/// `None`.  This pins the `!Path::new(c).is_absolute() { return false; }` seam.
#[test]
fn test_select_path_candidate_rejects_relative_candidates_on_all_platforms() {
    let cwd = std::path::PathBuf::from(std::env::temp_dir());
    let candidates = &["relative_tool"];
    let result = select_path_candidate(candidates, &cwd);
    assert!(
        result.is_none(),
        "a relative candidate must be rejected on all platforms; got: {result:?}"
    );
}

/// Layer 1 — empty list: no candidates means `None` (tool not found, fail closed).
#[test]
fn test_select_path_candidate_empty_candidates_returns_none() {
    let cwd = std::env::temp_dir();
    let result = select_path_candidate(&[], &cwd);
    assert!(result.is_none(), "empty candidate list must return None; got: {result:?}");
}

/// Layer 2 — CWD exclusion: a candidate whose parent is the CWD is rejected.
///
/// Uses a synthetic non-existent path so no real file system lookup is needed.
/// When `canonicalize` fails for a non-existent dir the function falls back to
/// raw path comparison, which is sufficient for this layer-2 proof.
#[test]
fn test_select_path_candidate_cwd_parent_candidate_rejected() {
    // Build a fake CWD using the real temp dir so the absolute-path check passes,
    // then construct a candidate whose parent IS that CWD.
    let cwd = std::env::temp_dir();
    // Construct a candidate path that has `cwd` as its parent directory.
    let candidate = cwd.join("planted_tool");
    let candidate_str = candidate.to_string_lossy().to_string();
    let candidate_refs: Vec<&str> = vec![candidate_str.as_str()];
    let result = select_path_candidate(&candidate_refs, &cwd);
    // The candidate's parent == cwd, so it must be excluded (Layer 2).
    assert!(
        result.is_none(),
        "a candidate whose parent is the CWD must be rejected; got: {result:?}"
    );
}

/// Priority: `candidate_priority` assigns `.exe` > `.com` > `.cmd` > `.bat` >
/// any other extension > no extension.  Pin all six priority tiers.
#[test]
fn test_candidate_priority_ordering_all_tiers() {
    assert_eq!(candidate_priority("tool.exe"), 5, ".exe must be highest priority");
    assert_eq!(candidate_priority("tool.com"), 4, ".com must be second");
    assert_eq!(candidate_priority("tool.cmd"), 3, ".cmd must be third");
    assert_eq!(candidate_priority("tool.bat"), 2, ".bat must be fourth");
    assert_eq!(candidate_priority("tool.sh"), 1, "other extension must be fifth");
    assert_eq!(candidate_priority("tool"), 0, "no extension must be lowest");
}

/// Priority is case-insensitive: `.EXE` and `.Exe` must score the same as `.exe`.
#[test]
fn test_candidate_priority_case_insensitive() {
    assert_eq!(candidate_priority("tool.EXE"), candidate_priority("tool.exe"));
    assert_eq!(candidate_priority("tool.BAT"), candidate_priority("tool.bat"));
    assert_eq!(candidate_priority("tool.CMD"), candidate_priority("tool.cmd"));
}

// --- Windows binary-planting RCE regression (CWD exclusion) ---
//
// These tests use `select_path_candidate`, the pure inner function of
// `resolve_windows_program`, so they run on any OS and require no real PATH
// or file system setup.  They validate the security invariant: a binary
// located in the current working directory must NEVER be selected for a
// bare-name tool resolution, even when it has a higher-priority extension.

/// RCE regression: a planted `perltidy.exe` in the CWD must not be selected
/// when a legitimate `perltidy.exe` exists on PATH.
#[cfg(windows)]
#[test]
fn test_select_path_candidate_planted_cwd_binary_is_rejected_in_favor_of_path() {
    let cwd = std::path::PathBuf::from(r"C:\Users\user\project");
    let candidates = &[
        r"C:\Users\user\project\perltidy.exe", // planted — must be excluded
        r"C:\Strawberry\perl\bin\perltidy.exe", // legitimate PATH install
    ];
    let result = select_path_candidate(candidates, &cwd);
    assert_eq!(
        result.as_deref(),
        Some(r"C:\Strawberry\perl\bin\perltidy.exe"),
        "planted CWD binary must not be selected; legitimate PATH binary must win"
    );
}

/// Security invariant: when the only candidate is under the CWD, resolve to
/// `None` rather than executing a planted binary.
#[cfg(windows)]
#[test]
fn test_select_path_candidate_cwd_only_returns_none() {
    let cwd = std::path::PathBuf::from(r"C:\Users\user\untrusted-workspace");
    let candidates = &[r"C:\Users\user\untrusted-workspace\perltidy.exe"];
    let result = select_path_candidate(candidates, &cwd);
    assert!(
        result.is_none(),
        "a planted-only candidate list must return None, not execute the planted binary; got: {result:?}"
    );
}

/// Baseline: a candidate on a real PATH directory (not CWD) resolves correctly.
#[cfg(windows)]
#[test]
fn test_select_path_candidate_legitimate_path_binary_resolves() {
    let cwd = std::path::PathBuf::from(r"C:\Users\user\project");
    let candidates = &[r"C:\Strawberry\perl\bin\perltidy.exe"];
    let result = select_path_candidate(candidates, &cwd);
    assert_eq!(
        result.as_deref(),
        Some(r"C:\Strawberry\perl\bin\perltidy.exe"),
        "a legitimate PATH binary must resolve when CWD is different"
    );
}

/// Extension priority: among PATH candidates, `.exe` must beat `.bat`.
/// The CWD exclusion must not disturb the priority ordering among safe
/// candidates.
#[cfg(windows)]
#[test]
fn test_select_path_candidate_extension_priority_preserved_among_path_candidates() {
    let cwd = std::path::PathBuf::from(r"C:\Users\user\project");
    let candidates =
        &[r"C:\Strawberry\perl\bin\perltidy.bat", r"C:\Strawberry\perl\bin\perltidy.exe"];
    let result = select_path_candidate(candidates, &cwd);
    assert_eq!(
        result.as_deref(),
        Some(r"C:\Strawberry\perl\bin\perltidy.exe"),
        ".exe must beat .bat in extension priority ordering"
    );
}

// --- Empty-PATH-entry bypass regression (relative-candidate rejection) ---
//
// An empty `;;` or trailing `;` PATH entry causes `split_paths` to yield an
// empty dir, which `dir.join("perltidy")` turns into a RELATIVE path.  The
// relative candidate resolves against the CWD at `.is_file()` time, so a
// planted workspace binary survives into the candidate list.  These tests
// verify that `select_path_candidate` rejects all relative candidates —
// regardless of whether they happen to coincide with a CWD binary.

/// A bare relative candidate (as produced by an empty PATH entry) must be
/// rejected and return `None` — never executed.
#[cfg(windows)]
#[test]
fn test_select_path_candidate_relative_candidate_returns_none() {
    // `"perltidy.EXE"` is relative — no drive letter, no leading backslash.
    // This is exactly what an empty PATH entry produces via `dir.join(program)`.
    let cwd = std::path::PathBuf::from(r"C:\Users\user\project");
    let candidates = &["perltidy.EXE"];
    let result = select_path_candidate(candidates, &cwd);
    assert!(result.is_none(), "a relative candidate must always be rejected; got: {result:?}");
}

/// A relative candidate alongside a legitimate absolute PATH candidate: only
/// the absolute one is selected; the relative one is silently dropped.
#[cfg(windows)]
#[test]
fn test_select_path_candidate_relative_candidate_dropped_absolute_path_wins() {
    let cwd = std::path::PathBuf::from(r"C:\Users\user\project");
    let candidates = &[
        "perltidy.EXE",                         // relative — must be dropped
        r"C:\Strawberry\perl\bin\perltidy.exe", // absolute PATH install — must win
    ];
    let result = select_path_candidate(candidates, &cwd);
    assert_eq!(
        result.as_deref(),
        Some(r"C:\Strawberry\perl\bin\perltidy.exe"),
        "relative candidate must be dropped; absolute PATH binary must win"
    );
}

// --- Has-extension bare-name bypass regression ---
//
// A bare name WITH an extension but NO separator (e.g. "perltidy.exe") must
// not be passed through to `Command::new` unchanged.  Windows' CreateProcess
// searches the CWD before PATH, so `Command::new("perltidy.exe")` with a
// planted workspace binary is an RCE.  `resolve_windows_program` now routes
// these through the same PATH-only search as extensionless bare names.
//
// These tests exercise `select_path_candidate` directly (the pure fn) because
// the real `resolve_windows_program` needs the binary to exist on disk.  The
// security property being tested is that the selection layer rejects any
// candidate whose parent is the CWD — identical to the planted-binary tests
// above — confirming that extensioned bare names feed the same safe path.

/// A candidate for an extensioned bare name (`perltidy.exe`) that lives in the
/// CWD must be rejected in favour of a PATH-directory copy.
#[cfg(windows)]
#[test]
fn test_select_path_candidate_extensioned_bare_cwd_binary_rejected_in_favor_of_path() {
    let cwd = std::path::PathBuf::from(r"C:\Users\user\project");
    let candidates = &[
        r"C:\Users\user\project\perltidy.exe", // planted in CWD — must be dropped
        r"C:\Strawberry\perl\bin\perltidy.exe", // legitimate PATH install
    ];
    let result = select_path_candidate(candidates, &cwd);
    assert_eq!(
        result.as_deref(),
        Some(r"C:\Strawberry\perl\bin\perltidy.exe"),
        "extensioned bare name: CWD-planted binary must be rejected; PATH binary must win"
    );
}

/// When an extensioned bare name is not found on PATH at all, `None` is
/// returned — never falls back to the CWD.
#[cfg(windows)]
#[test]
fn test_select_path_candidate_extensioned_bare_cwd_only_returns_none() {
    let cwd = std::path::PathBuf::from(r"C:\Users\user\project");
    let candidates = &[r"C:\Users\user\project\perltidy.exe"]; // only in CWD
    let result = select_path_candidate(candidates, &cwd);
    assert!(
        result.is_none(),
        "extensioned bare name not on PATH must return None, not execute CWD binary; got: {result:?}"
    );
}

// --- Caller-chain fail-closed regression (the seam the selector tests miss) ---
//
// Every test above exercises `select_path_candidate` — the pure SELECTOR.  None
// drives the actual invocation chain, which is exactly where the live RCE
// survived: the selector correctly returned `None` for a not-on-PATH tool, but
// the caller's old `resolve_windows_program(program).unwrap_or_else(|| program
// .to_string())` restored the bare name → `Command::new(bare)` → CWD search.
// These tests drive `resolve_command_invocation` and the real `run_command`
// entry point so the caller can never silently re-arm the bypass.

/// Caller fail-closed: a bare tool name not present on PATH must return an
/// error, NOT a `(bare_name, args)` invocation that `Command::new` would resolve
/// against the current working directory.
#[cfg(windows)]
#[test]
fn test_resolve_command_invocation_fails_closed_when_tool_not_on_path() {
    let result = resolve_command_invocation("definitely_not_a_real_tool_zzz_2764", &["--version"]);
    assert!(
        result.is_err(),
        "an unresolved bare tool name must fail closed, not return a bare-name invocation; got: {result:?}"
    );
    let err = result.expect_err("must be Err");
    assert!(
        err.message.contains("current directory excluded"),
        "error must explain the CWD-exclusion security refusal; got: {}",
        err.message
    );
}

/// `resolve_cmd_exe` must yield an ABSOLUTE path ending in `cmd.exe` — never the
/// bare name, which would itself be subject to the CWD-first CreateProcess
/// search.
#[cfg(windows)]
#[test]
fn test_resolve_cmd_exe_returns_absolute_path_never_bare() {
    let resolved = resolve_cmd_exe().expect("cmd.exe must resolve on a Windows test host");
    assert!(
        std::path::Path::new(&resolved).is_absolute(),
        "cmd.exe must resolve to an absolute path, never the bare name; got: {resolved}"
    );
    assert!(
        resolved.to_ascii_lowercase().ends_with("cmd.exe"),
        "resolved shell must be cmd.exe; got: {resolved}"
    );
    assert_ne!(
        resolved.to_ascii_lowercase(),
        "cmd.exe",
        "must not be the bare CWD-searchable name"
    );
}

/// Full-chain RCE regression: drive the real `run_command` entry point with a
/// bare name that is NOT on PATH while a same-named batch file is planted in the
/// current working directory.  The old fail-open caller would have executed the
/// planted file via `cmd.exe /C "pwned.bat"` (CWD-resolved); the fixed chain
/// must fail closed and leave the marker absent.  Serialized because it mutates
/// the process-global CWD.
#[cfg(windows)]
#[test]
fn test_run_command_does_not_execute_planted_cwd_binary() {
    use std::io::Write as _;
    use std::sync::Mutex;
    static CWD_LOCK: Mutex<()> = Mutex::new(());
    let _guard = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let unique = format!("rce_chain_{}", std::process::id());
    let workspace = std::env::temp_dir().join(unique);
    let _ = std::fs::remove_dir_all(&workspace);
    std::fs::create_dir_all(&workspace).expect("create temp workspace");

    let marker = workspace.join("PWNED_MARKER.txt");
    let planted = workspace.join("pwned.bat");
    {
        let mut f = std::fs::File::create(&planted).expect("write planted .bat");
        writeln!(f, "@echo off").expect("write bat line");
        // Absolute marker path so execution is detectable regardless of CWD.
        writeln!(f, "echo pwned> \"{}\"", marker.display()).expect("write bat line");
    }

    let original_cwd = std::env::current_dir().expect("capture original cwd");
    std::env::set_current_dir(&workspace).expect("enter temp workspace");

    let runtime = OsSubprocessRuntime::new();
    let result = runtime.run_command("pwned.bat", &[], None);

    // Restore CWD before asserting so a failure cannot leave the suite in the
    // temp directory.
    std::env::set_current_dir(&original_cwd).expect("restore original cwd");

    let marker_exists = marker.exists();
    let _ = std::fs::remove_dir_all(&workspace);

    assert!(
        result.is_err(),
        "a not-on-PATH bare name must fail closed at the chain entry; got: {result:?}"
    );
    assert!(
        !marker_exists,
        "SECURITY: planted CWD batch file was EXECUTED through run_command — the RCE is live"
    );
}

// --- Relative path-with-separator bypass regression ---
//
// `resolve_windows_program` passes a program through unchanged when it contains
// a path separator — but ONLY when it is absolute.  A relative path that merely
// contains a separator (".\pwned.exe", "..\x", "sub\tool", "/x", "\tool") is
// still resolved by CreateProcess against the CWD (workspace root), so it must
// fail closed rather than reach `Command::new`.

/// Each relative path-with-separator form must fail closed at the invocation
/// resolver — never handed to `Command::new` for a CWD-relative spawn.
#[cfg(windows)]
#[test]
fn test_resolve_command_invocation_relative_separator_path_fails_closed() {
    for rel in [r".\pwned.exe", r"..\pwned.exe", r"sub\pwned.exe", "pwned/evil.bat", r"\pwned.exe"]
    {
        let result = resolve_command_invocation(rel, &[]);
        assert!(
            result.is_err(),
            "relative path-with-separator {rel:?} must fail closed (CWD-relative exec); got: {result:?}"
        );
    }
}

/// An absolute path with separators is still a legitimate caller-resolved
/// location and must pass through unchanged.
#[cfg(windows)]
#[test]
fn test_resolve_command_invocation_absolute_separator_path_passes_through() {
    let (program, _args) = perl_tdd_support::must(resolve_command_invocation(
        r"C:\tools\perltidy.exe",
        &["--version"],
    ));
    assert_eq!(program, r"C:\tools\perltidy.exe");
}

// --- Call-observation tests for resolve_program public API ---
//
// ripr requires call-observation tests that drive `resolve_program` directly.
// The selector and command-invocation tests above use inner functions; these
// tests target the public entry point so the seam is gripped end-to-end.

/// On non-Windows, `resolve_program` is a pass-through: it returns
/// `Ok(program.to_string())` unchanged so callers can use it unconditionally
/// without `#[cfg(windows)]` guards.
#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
#[test]
fn test_resolve_program_non_windows_pass_through() {
    // Any name is returned verbatim on non-Windows — the OS PATH search happens
    // later at Command::new time, not in resolve_program.
    let result = crate::resolve_program("perl");
    assert!(result.is_ok(), "resolve_program must succeed on non-Windows; got: {result:?}");
    assert_eq!(
        result.expect("non-windows passthrough"),
        "perl",
        "non-Windows: resolve_program must return the name unchanged"
    );
}

/// On non-Windows, `resolve_program` passes through even a nonexistent tool
/// name — it defers to the OS rather than pre-searching PATH.
#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
#[test]
fn test_resolve_program_non_windows_pass_through_nonexistent_name() {
    let result = crate::resolve_program("definitely_not_a_real_tool_zzz_3028");
    assert!(
        result.is_ok(),
        "non-Windows resolve_program must pass through even unknown names; got: {result:?}"
    );
    let resolved = result.expect("non-windows passthrough");
    assert_eq!(
        resolved, "definitely_not_a_real_tool_zzz_3028",
        "non-Windows: resolve_program must return the name unchanged"
    );
}

/// On Windows, `resolve_program` must return `Err` for a tool name that does
/// not exist on any absolute PATH directory — failing closed rather than
/// returning a bare name that CreateProcess would resolve against the CWD.
#[cfg(all(not(target_arch = "wasm32"), windows))]
#[test]
fn test_resolve_program_windows_fails_closed_for_unknown_tool() {
    let result = crate::resolve_program("definitely_not_a_real_tool_zzz_3028");
    assert!(
        result.is_err(),
        "Windows resolve_program must fail closed for an unknown tool; got: {result:?}"
    );
    let err = result.expect_err("must be Err");
    assert!(
        err.message.contains("current directory excluded"),
        "error must mention CWD exclusion; got: {}",
        err.message
    );
}

/// On Windows, `resolve_program` must return an absolute path for a tool that
/// genuinely exists on PATH (e.g. `cmd.exe` is always present on Windows CI).
#[cfg(all(not(target_arch = "wasm32"), windows))]
#[test]
fn test_resolve_program_windows_returns_absolute_path_for_real_tool() {
    // `cmd.exe` is guaranteed present on any Windows CI runner.
    let result = crate::resolve_program("cmd");
    assert!(result.is_ok(), "Windows resolve_program must find cmd on PATH; got: {result:?}");
    let resolved = result.expect("cmd must resolve");
    assert!(
        std::path::Path::new(&resolved).is_absolute(),
        "Windows resolve_program must return an absolute path; got: {resolved}"
    );
    assert!(
        resolved.to_ascii_lowercase().ends_with("cmd.exe"),
        "resolved path must end with cmd.exe; got: {resolved}"
    );
}
