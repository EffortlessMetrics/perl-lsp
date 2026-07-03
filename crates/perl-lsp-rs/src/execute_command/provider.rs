//! Backend command implementations for run/debug/test and analyzer actions.

use crate::perl_critic::{
    BuiltInAnalyzer, CriticAnalyzer, CriticConfig, CriticContext, NativeCriticProfile,
    NativeCriticRegistry,
};
#[cfg(not(target_arch = "wasm32"))]
use perl_lsp_rs_core::config::PerlOracleEnv;
use perl_lsp_rs_core::config::{CriticEngine, WorkspaceConfig};
use perl_lsp_rs_core::providers::{
    ProviderDecisionConfidence, ProviderDecisionCopyablePayload, ProviderDecisionExplanation,
    ProviderDecisionFactSource, ProviderDecisionFallback, ProviderDecisionFreshness,
    ProviderDecisionOutcome, ProviderDecisionProvider, ProviderDecisionReason,
    ProviderDecisionRequestPosition, format_provider_decision_explanation,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::borrow::Cow;
#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};
#[cfg(windows)]
use std::path::{Component, Prefix};
use std::path::{Path, PathBuf};
use std::process::Command;
use url::Url;

/// Strip the Windows extended-length path prefix (`\\?\`) before passing a path
/// to an external command such as `perl`, `prove`, or `yath`.
///
/// On Windows, `Path::canonicalize` returns paths prefixed with `\\?\`, which is
/// understood by Win32 APIs but not by external programs (e.g. `perl.exe`).  This
/// helper strips that prefix so the resulting path is usable as a command-line
/// argument.  On non-Windows platforms the function is a no-op identity.
///
/// Two prefix forms are handled:
/// - `\\?\C:\...`         (local drive) → `C:\...`
/// - `\\?\UNC\server\...` (network UNC) → `\\server\...`
///
/// The UNC form requires special treatment: stripping `\\?\` alone would leave
/// `UNC\server\...` which is not a valid path.  Instead we replace `\\?\UNC\`
/// with `\\` so the result is a conventional UNC path (`\\server\share\...`).
#[cfg(windows)]
pub(crate) fn normalize_path_for_external_command(path: &Path) -> PathBuf {
    let mut components = path.components();
    let Some(Component::Prefix(prefix_component)) = components.next() else {
        return path.to_path_buf();
    };

    match prefix_component.kind() {
        Prefix::VerbatimDisk(drive) => {
            let mut normalized = PathBuf::from(format!("{}:", char::from(drive)));
            normalized.push(components.as_path());
            normalized
        }
        Prefix::VerbatimUNC(server, share) => {
            let mut wide = vec![b'\\' as u16, b'\\' as u16];
            wide.extend(server.encode_wide());
            wide.push(b'\\' as u16);
            wide.extend(share.encode_wide());

            let mut normalized = PathBuf::from(OsString::from_wide(&wide));
            normalized.push(components.as_path());
            normalized
        }
        _ => path.to_path_buf(),
    }
}

#[cfg(not(windows))]
pub(crate) fn normalize_path_for_external_command(path: &Path) -> PathBuf {
    path.to_path_buf()
}

/// Execute command provider implementing the LSP executeCommand method.
pub struct ExecuteCommandProvider {
    workspace_roots: Vec<PathBuf>,
    workspace_config: Option<WorkspaceConfig>,
    /// Configured critic engine for `perl.runCritic`.
    ///
    /// Defaults to [`CriticEngine::Native`]. The external (legacy `perlcritic`)
    /// analyzer runs only when this is [`CriticEngine::Legacy`] — merely having
    /// `perlcritic` on `PATH` must not change the default behavior.
    critic_engine: CriticEngine,
    /// Native critic rule configuration for `perl.runCritic` when the engine is
    /// [`CriticEngine::Native`]. Mirrors the `ServerConfig` values the editor's
    /// native pull-diagnostics path uses, so the command and on-type diagnostics
    /// report the same `NativeCriticRegistry` rule set.
    native_critic_config: NativeCriticCommandConfig,
}

/// Native-critic rule configuration plumbed into [`ExecuteCommandProvider`] from
/// `ServerConfig`, mirroring the values the native pull-diagnostics path reads.
#[derive(Debug, Clone)]
pub(crate) struct NativeCriticCommandConfig {
    /// Profile name (`recommended`, `strict`, …) selecting the rule set.
    pub profile: String,
    /// Rule IDs to include on top of the profile. Empty means profile-only.
    pub include: Vec<String>,
    /// Rule IDs to exclude from the profile.
    pub exclude: Vec<String>,
    /// Minimum severity (1–5) reported by the native registry.
    pub severity: u8,
}

impl Default for NativeCriticCommandConfig {
    fn default() -> Self {
        // Matches `ServerConfig::default()` native-critic defaults so a provider
        // built without explicit config behaves like the on-type native path.
        Self {
            profile: "recommended".to_string(),
            include: Vec::new(),
            exclude: Vec::new(),
            severity: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TestRunner {
    Yath,
    Prove,
    Perl,
}

#[derive(Debug, Deserialize)]
struct ExplainProviderDecisionRequest {
    provider: ProviderDecisionProvider,
    #[serde(default)]
    receipt_id: Option<String>,
    #[serde(default)]
    scenario: Option<String>,
    #[serde(default)]
    request_receipt: Option<Value>,
    #[serde(default)]
    request_position: Option<ProviderDecisionRequestPosition>,
}

impl Default for ExecuteCommandProvider {
    fn default() -> Self {
        Self::new()
    }
}

// Private helpers for PerlOracleEnv subprocess isolation.
impl ExecuteCommandProvider {
    /// Build a `Command` for a Perl subprocess using `PerlOracleEnv`.
    ///
    /// The `file_path` is used only to derive a `cwd` (its parent directory).
    /// Callers must append the actual Perl arguments after this call.
    #[cfg(not(target_arch = "wasm32"))]
    fn perl_command_for(&self, file_path: &Path) -> Result<Command, String> {
        let cwd = file_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let Some(config) = self.workspace_config.as_ref() else {
            return Err(Self::unresolved_execute_command_perl_error(file_path));
        };
        let Some(oracle) = PerlOracleEnv::for_execute_command(config, cwd) else {
            return Err(Self::unresolved_execute_command_perl_error(file_path));
        };
        Ok(oracle.into_command())
    }

    #[cfg(target_arch = "wasm32")]
    fn perl_command_for(&self, file_path: &Path) -> Result<Command, String> {
        Err(Self::unresolved_execute_command_perl_error(file_path))
    }

    fn unresolved_execute_command_perl_error(file_path: &Path) -> String {
        format!(
            "Cannot run Perl command for '{}': Perl binary could not be resolved from `perl.path` or PATH. Configure `perl.path` to an explicit Perl executable; refusing ambient fallback.",
            file_path.display()
        )
    }
}

impl ExecuteCommandProvider {
    /// Create a new execute command provider.
    pub fn new() -> Self {
        Self {
            workspace_roots: Vec::new(),
            workspace_config: None,
            critic_engine: CriticEngine::Native,
            native_critic_config: NativeCriticCommandConfig::default(),
        }
    }

    /// Create a provider with workspace root enforcement.
    pub fn with_workspace_roots(workspace_roots: Vec<PathBuf>) -> Self {
        Self {
            workspace_roots,
            workspace_config: None,
            critic_engine: CriticEngine::Native,
            native_critic_config: NativeCriticCommandConfig::default(),
        }
    }

    /// Set the critic engine used by `perl.runCritic`.
    ///
    /// Defaults to [`CriticEngine::Native`]. Pass [`CriticEngine::Legacy`] to
    /// route `perl.runCritic` through the external `perlcritic` analyzer when it
    /// is available on `PATH`.
    pub fn with_critic_engine(mut self, engine: CriticEngine) -> Self {
        self.critic_engine = engine;
        self
    }

    /// Set the native-critic rule configuration used by `perl.runCritic` under
    /// [`CriticEngine::Native`]. Plumb the same `ServerConfig` values the native
    /// pull-diagnostics path uses (profile, include/exclude rule IDs, severity)
    /// so the command and on-type diagnostics report the same rule set.
    pub fn with_native_critic_config(
        mut self,
        profile: String,
        include: Vec<String>,
        exclude: Vec<String>,
        severity: u8,
    ) -> Self {
        self.native_critic_config =
            NativeCriticCommandConfig { profile, include, exclude, severity };
        self
    }

    /// Attach a workspace configuration to enable PerlOracleEnv isolation for
    /// Perl subprocess calls (`perl.runFile`, `perl.runTestSub`).
    ///
    /// `run_file` and `run_test_sub` use `PerlOracleEnv::for_execute_command`
    /// instead of a bare `Command::new("perl")`, applying the
    /// deny-all-ambient env policy.
    pub fn with_workspace_config(mut self, config: WorkspaceConfig) -> Self {
        self.workspace_config = Some(config);
        self
    }

    /// Execute a supported command with validated JSON arguments.
    pub fn execute_command(&self, command: &str, arguments: Vec<Value>) -> Result<Value, String> {
        match command {
            "perl.runTests" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.run_tests(&file_path)
            }
            "perl.runFile" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.run_file(&file_path)
            }
            "perl.runTestSub" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                let sub_name = arguments
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing subroutine name argument".to_string())?;
                self.run_test_sub(&file_path, sub_name)
            }
            "perl.debugTests" | "perl.debugTestFile" | "perl.debugFile" | "perl.debugTest" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.debug_tests(&file_path)
            }
            "perl.runTest" | "perl.runTestFile" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.run_tests(&file_path)
            }
            "perl.runSubtest" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                let subtest_name = arguments
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing subtest name argument".to_string())?;
                self.run_subtest(&file_path, subtest_name)
            }
            "perl.runCritic" => self.run_critic_secure(&arguments),
            "perl.goToTest" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                Ok(self.go_to_test(&file_path))
            }
            "perl.goToImplementation" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                Ok(self.go_to_implementation(&file_path))
            }
            "perl.workspaceTrustReport" => {
                Err("perl.workspaceTrustReport requires the live LSP runtime state".to_string())
            }
            "perl.previewSafeDelete" => {
                Err("perl.previewSafeDelete requires the live LSP runtime workspace index"
                    .to_string())
            }
            "perl.safeDeleteSymbol" => {
                Err("perl.safeDeleteSymbol requires the live LSP runtime workspace index"
                    .to_string())
            }
            "perl.previewPackageRename" => {
                Err("perl.previewPackageRename requires the live LSP runtime workspace index"
                    .to_string())
            }
            "perl.explainMissingModuleLookup" => {
                Err("perl.explainMissingModuleLookup requires the live LSP runtime module-resolution state"
                    .to_string())
            }
            "perl.explainProviderDecision" => self.explain_provider_decision(&arguments),
            _ => Err(format!("Unknown command: {}", command)),
        }
    }

    fn explain_provider_decision(&self, arguments: &[Value]) -> Result<Value, String> {
        let request_value = arguments
            .first()
            .ok_or_else(|| "Missing explain-provider-decision argument".to_string())?;
        let request: ExplainProviderDecisionRequest = serde_json::from_value(request_value.clone())
            .map_err(|error| format!("Invalid explain-provider-decision argument: {error}"))?;

        let mut explanation = default_provider_decision_explanation(request.provider);

        if let Some(receipt_id) = request.receipt_id {
            explanation = explanation.with_receipt_id(receipt_id);
        }
        if let Some(scenario) = request.scenario {
            explanation = explanation.with_scenario(scenario);
        }
        if let Some(request_receipt) = request.request_receipt {
            if !request_receipt.is_object() {
                return Err(
                    "Invalid explain-provider-decision argument: request_receipt must be an object"
                        .to_string(),
                );
            }
            explanation = explanation.with_request_receipt(request_receipt);
        }

        let request_position = request.request_position;
        let user_message = format_provider_decision_explanation(&explanation);
        explanation = explanation.with_user_message(user_message);
        let copyable_payload = ProviderDecisionCopyablePayload::from_explanation(
            &explanation,
            env!("CARGO_PKG_VERSION"),
            workspace_root_class(&self.workspace_roots),
            workspace_root_hash(&self.workspace_roots),
            request_position,
            provider_support_tier_link(explanation.provider),
        );
        explanation = explanation.with_copyable_payload(copyable_payload);

        serde_json::to_value(explanation)
            .map_err(|error| format!("Failed to serialize provider decision explanation: {error}"))
    }

    pub(crate) fn run_tests(&self, file_path: &Path) -> Result<Value, String> {
        let file_path_str = file_path.to_string_lossy();
        let is_test_file = self.is_test_file(&file_path_str);
        let runner = select_test_runner(
            is_test_file,
            self.command_exists("yath"),
            self.command_exists("prove"),
        );
        let ext_path = normalize_path_for_external_command(file_path);

        match runner {
            TestRunner::Yath => self.run_tests_with_yath_fallback(&ext_path),
            TestRunner::Prove => self.run_tests_with_prove_fallback(&ext_path),
            TestRunner::Perl => self.run_tests_with_perl(&ext_path, "perl"),
        }
    }

    fn run_tests_with_yath_fallback(&self, ext_path: &Path) -> Result<Value, String> {
        match self.run_test_command("yath", ext_path) {
            Ok(value) => Ok(value),
            Err(yath_error) => {
                if self.command_exists("prove") {
                    match self.run_test_command("prove", ext_path) {
                        Ok(value) => Ok(value),
                        Err(prove_error) => match self.run_test_command("perl", ext_path) {
                            Ok(value) => Ok(value),
                            Err(perl_error) => Ok(self.format_command_launch_failure(
                                "yath",
                                format!(
                                    "Failed to run yath: {yath_error}; prove fallback also failed: {prove_error}; perl fallback also failed: {perl_error}"
                                ),
                            )),
                        },
                    }
                } else {
                    match self.run_test_command("perl", ext_path) {
                        Ok(value) => Ok(value),
                        Err(perl_error) => Ok(self.format_command_launch_failure(
                            "yath",
                            format!(
                                "Failed to run yath: {yath_error}; perl fallback also failed: {perl_error}"
                            ),
                        )),
                    }
                }
            }
        }
    }

    fn run_tests_with_prove_fallback(&self, ext_path: &Path) -> Result<Value, String> {
        match self.run_test_command("prove", ext_path) {
            Ok(value) => Ok(value),
            Err(prove_error) => match self.run_test_command("perl", ext_path) {
                Ok(value) => Ok(value),
                Err(perl_error) => Ok(self.format_command_launch_failure(
                    "prove",
                    format!(
                        "Failed to run prove: {prove_error}; perl fallback also failed: {perl_error}"
                    ),
                )),
            },
        }
    }

    fn run_tests_with_perl(&self, ext_path: &Path, command_name: &str) -> Result<Value, String> {
        match self.run_test_command("perl", ext_path) {
            Ok(value) => Ok(value),
            Err(error) => Ok(self.format_command_launch_failure(
                command_name,
                format!("Failed to run {command_name}: {error}"),
            )),
        }
    }

    pub(crate) fn run_test_command(&self, command: &str, ext_path: &Path) -> Result<Value, String> {
        // Resolve the bare program name to an absolute PATH-searched path before
        // passing it to Command::new.  On Windows, Command::new(bare_name)
        // triggers CreateProcess's CWD-first search — a planted perl.exe/yath.exe/
        // prove.exe in the LSP workspace root would execute instead of the real
        // tool (binary-planting RCE, #2764/#3028).  resolve_program fails closed
        // when the tool is not found on any absolute PATH directory.
        #[cfg(not(target_arch = "wasm32"))]
        let resolved_command =
            perl_subprocess_runtime::resolve_program(command).map_err(|e| e.to_string())?;
        #[cfg(target_arch = "wasm32")]
        let resolved_command = command.to_string();

        let mut cmd = Command::new(&resolved_command);
        if command == "perl" {
            cmd.arg("--").arg(ext_path.as_os_str());
        } else {
            cmd.arg("-v").arg("--").arg(ext_path.as_os_str());
        }

        crate::util::run_command_with_timeout(cmd, 30)
            .map(|result| self.format_test_command_result(result, command))
            .map_err(|error| error.to_string())
    }

    /// Format a test-runner process result, enriching the raw command output
    /// with structured TAP facts: which runner produced it, plan/pass/fail
    /// counts, and per-failure source locations. TODO/SKIP are reported but do
    /// not count as hard failures. The raw `output`/`error`/`command`/`success`
    /// fields are preserved so existing clients keep working.
    pub(crate) fn format_test_command_result(
        &self,
        result: std::process::Output,
        command: &str,
    ) -> Value {
        use perl_lsp_rs_core::providers::testing::tap::parse_tap;

        let stdout = String::from_utf8_lossy(&result.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
        let success = result.status.success();
        let exit_code = result.status.code();
        let report = parse_tap(&stdout);

        let failures: Vec<Value> = report
            .failures()
            .into_iter()
            .map(|test| {
                json!({
                    "number": test.number,
                    "description": test.description,
                    "file": test.file,
                    "line": test.line,
                    "got": test.got,
                    "expected": test.expected,
                    "depth": test.depth,
                })
            })
            .collect();

        let plan_mismatch = report
            .plan_mismatch()
            .map(|(actual, planned)| json!({ "actual": actual, "planned": planned }));

        json!({
            "success": success,
            "output": stdout,
            "error": if success { Value::Null } else { Value::String(stderr) },
            "command": command,
            "runner": command,
            "exitCode": exit_code,
            "tap": {
                "planned": report.plan.as_ref().map(|plan| plan.count),
                "skipAll": report.plan.as_ref().and_then(|plan| plan.skip_all.clone()),
                "total": report.summary.total,
                "passed": report.summary.passed,
                "failed": report.summary.failed,
                "skipped": report.summary.skipped,
                "todo": report.summary.todo,
                "bailedOut": report.bailed_out,
                "planMismatch": plan_mismatch,
            },
            "failures": failures,
        })
    }

    pub(crate) fn run_test_sub(&self, file_path: &Path, sub_name: &str) -> Result<Value, String> {
        let perl_code = r#"
            my ($file, $sub) = @ARGV;
            do $file;
            if (defined &$sub) {
                no strict 'refs';
                &$sub();
            } else {
                die "Subroutine $sub not found";
            }
        "#;
        let ext_path = normalize_path_for_external_command(file_path);

        let mut perl_cmd = self.perl_command_for(file_path)?;
        perl_cmd.arg("-e").arg(perl_code).arg("--").arg(ext_path.as_os_str()).arg(sub_name);
        match crate::util::run_command_with_timeout(perl_cmd, 30) {
            Ok(result) => {
                Ok(self.format_command_result(result, Some(("subroutine", sub_name.into()))))
            }
            Err(error) => Ok(self.format_command_launch_failure(
                "perl",
                format!("Failed to run test subroutine: {error}"),
            )),
        }
    }

    /// Run a Test2/Test::More subtest by name.
    ///
    /// A Test2 `subtest 'name' => sub { ... }` is an anonymous block that is
    /// part of the file's runtime — `perl-lsp` does **not** extract and execute
    /// it in isolation (that would misrepresent the test). Instead this runs the
    /// whole file through the configured runner and *focuses* the resulting TAP
    /// output on the named subtest, clearly labelling the result as a
    /// whole-file-focused run rather than filtered execution.
    pub(crate) fn run_subtest(
        &self,
        file_path: &Path,
        subtest_name: &str,
    ) -> Result<Value, String> {
        use perl_lsp_rs_core::providers::testing::tap::{focus_subtest, parse_tap};

        let mut result = self.run_tests(file_path)?;

        let raw = result.get("output").and_then(|value| value.as_str()).unwrap_or("");
        let report = parse_tap(raw);
        let focus = focus_subtest(&report, subtest_name);

        result["requestedSubtest"] = json!(subtest_name);
        // No cross-runner subtest filtering yet: always a whole-file run whose
        // output we focus on the requested subtest.
        result["subtestMode"] = json!("whole-file-focused");
        result["subtestFocus"] = match focus {
            Some(focus) => json!({
                "found": true,
                "passed": focus.passed,
                "innerFailed": focus.inner_failed,
                "name": focus.name,
            }),
            None => json!({
                "found": false,
                "reason": "no labelled subtest with that name in the runner output (dynamic name, or runner did not emit a subtest summary line)",
            }),
        };
        result["note"] = json!(
            "Ran the whole test file through the runner and focused output on the requested subtest. perl-lsp does not execute subtest blocks in isolation."
        );
        Ok(result)
    }

    pub(crate) fn run_file(&self, file_path: &Path) -> Result<Value, String> {
        let ext_path = normalize_path_for_external_command(file_path);
        let mut perl_cmd = self.perl_command_for(file_path)?;
        perl_cmd.arg("--").arg(ext_path.as_os_str());
        match crate::util::run_command_with_timeout(perl_cmd, 30) {
            Ok(result) => Ok(self.format_command_result(result, None)),
            Err(error) => {
                Ok(self
                    .format_command_launch_failure("perl", format!("Failed to run file: {error}")))
            }
        }
    }

    /// Build a Debug Adapter Protocol launch configuration for debugging a test
    /// file through the native `perl-dap` adapter.
    ///
    /// `perl-lsp` does not run the debugger itself: it returns a launch
    /// configuration (`action: "startDebugging"`) that the editor hands to the
    /// `perl` debug type (backed by `perl-dap`). The program is the `.t` file
    /// run under the real Perl interpreter — no Test2 runtime is emulated. The
    /// working directory follows the same policy as `runTests` (the file's
    /// directory), and `PERL_TEST_HARNESS_DUMP_TAP` is set so the session emits
    /// TAP the editor can read back.
    pub(crate) fn debug_tests(&self, file_path: &Path) -> Result<Value, String> {
        let ext_path = normalize_path_for_external_command(file_path);
        // A single-component path (bare filename) has a `Some("")` parent; an
        // empty cwd can fail to launch, so fall back to the current directory.
        let cwd = ext_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let file_name = file_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| ext_path.to_string_lossy().into_owned());

        Ok(json!({
            "success": true,
            "action": "startDebugging",
            "adapter": "perl-dap",
            "configuration": {
                "type": "perl",
                "request": "launch",
                "name": format!("Debug Test: {file_name}"),
                "program": ext_path.to_string_lossy(),
                "cwd": cwd.to_string_lossy(),
                "stopOnEntry": false,
                "args": [],
                "env": { "PERL_TEST_HARNESS_DUMP_TAP": "1" }
            }
        }))
    }

    fn run_critic_secure(&self, arguments: &[Value]) -> Result<Value, String> {
        let canonical_path = match self.resolve_path_from_args(arguments) {
            Ok(path) => path,
            Err(e) => {
                if e.contains("Missing file path argument") {
                    return Err(e);
                }

                if e.contains("File not found")
                    || e.contains("does not exist")
                    || e.contains("No such file or directory")
                    || e.contains("Failed to canonicalize")
                {
                    let error_message = if e.contains("Failed to canonicalize") {
                        if let Some(start) = e.find('\'') {
                            if let Some(end) = e[start + 1..].find('\'') {
                                let path = &e[start + 1..start + 1 + end];
                                format!("File not found: {}", path)
                            } else {
                                "File not found".to_string()
                            }
                        } else {
                            "File not found".to_string()
                        }
                    } else {
                        e.clone()
                    };
                    return Ok(self.format_critic_error(error_message, "none"));
                }

                if e.contains("Path traversal")
                    || e.contains("outside workspace")
                    || e.contains("Argument too long")
                {
                    return Err(format!("Path resolution failed: {}", e));
                }

                return Ok(self.format_critic_error(e, "none"));
            }
        };

        if self.external_critic_requested() {
            // Legacy engine: external `perlcritic` when present, else the
            // `BuiltInAnalyzer` (Perl::Critic-compatible) fallback. Legacy
            // behavior is unchanged.
            if command_exists("perlcritic") {
                if let Ok(result) = self.run_external_critic(&canonical_path) {
                    return Ok(result);
                }
            }
            return self.run_builtin_critic(&canonical_path);
        }

        // Native engine (default): route through the native rule registry so the
        // command reports the same `native.*` rule set as the editor's on-type
        // native pull diagnostics.
        self.run_native_critic(&canonical_path)
    }

    /// Whether the configured critic engine explicitly selects the external
    /// (legacy `perlcritic`) analyzer.
    ///
    /// The native analyzer is the default: merely having `perlcritic` present
    /// on `PATH` must not change the default behavior of `perl.runCritic`.
    /// External critic runs only when `.perl-lsp.toml` / server config sets the
    /// critic engine to `legacy`/`external`/`perlcritic`
    /// ([`CriticEngine::Legacy`]).
    pub(crate) fn external_critic_requested(&self) -> bool {
        matches!(self.critic_engine, CriticEngine::Legacy)
    }

    #[deprecated(since = "0.8.9", note = "Use run_critic_secure for secure path resolution")]
    #[allow(dead_code)]
    #[allow(deprecated)]
    pub(crate) fn run_critic(&self, file_path: &str) -> Result<Value, String> {
        let normalized_path = self.normalize_file_path(file_path);
        let path = Path::new(normalized_path.as_ref());

        if !path.exists() {
            return Ok(self.format_critic_error(
                format!("File not found: {}", normalized_path.as_ref()),
                "none",
            ));
        }

        if self.external_critic_requested() {
            if command_exists("perlcritic") {
                if let Ok(result) = self.run_external_critic(path) {
                    return Ok(result);
                }
            }
            return self.run_builtin_critic(path);
        }

        self.run_native_critic(path)
    }

    fn run_external_critic(&self, file_path: &Path) -> Result<Value, String> {
        let config = CriticConfig { severity: 3, verbose: true, ..Default::default() };
        let mut analyzer = CriticAnalyzer::with_os_runtime(config);

        match analyzer.analyze_file(file_path) {
            Ok(violations) => {
                let formatted_violations: Vec<_> = violations
                    .iter()
                    .map(|v| {
                        self.format_violation(
                            &v.policy,
                            &v.description,
                            &v.explanation,
                            v.severity as u8,
                            (v.range.start.line + 1) as usize,
                            (v.range.start.column + 1) as usize,
                            &v.file,
                        )
                    })
                    .collect();

                Ok(json!({
                    "status": "success",
                    "violations": formatted_violations,
                    "violationCount": formatted_violations.len(),
                    "analyzerUsed": "external"
                }))
            }
            Err(e) => Err(format!("External perlcritic failed: {}", e)),
        }
    }

    /// Run the native critic rule registry (`NativeCriticRegistry`) — the same
    /// engine the editor's on-type pull diagnostics use — and format its findings
    /// into the `perl.runCritic` result shape.
    ///
    /// This is the default `perl.runCritic` path (engine `Native`). Unlike
    /// [`Self::run_builtin_critic`] (which runs the Perl::Critic-compatible
    /// `BuiltInAnalyzer` and is kept only as the legacy fallback), this reports
    /// the profile-selected `native.*` rules — including `native.*`-only rules
    /// such as `native.variables.unused_lexical` that the built-in analyzer does
    /// not emit — so command results and native diagnostics agree.
    pub(crate) fn run_native_critic(&self, file_path: &Path) -> Result<Value, String> {
        use crate::Parser;

        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let code_text = perl_parser::util::code_slice(&content);
        let mut parser = Parser::new(code_text);

        let (ast, parse_error) = match parser.parse() {
            Ok(ast) => (ast, None),
            Err(error) => {
                let message = error.to_string();
                (
                    crate::ast::Node::new(
                        crate::ast::NodeKind::Error {
                            message,
                            expected: vec![],
                            found: None,
                            partial: None,
                        },
                        crate::ast::SourceLocation { start: 0, end: code_text.len() },
                    ),
                    Some(error),
                )
            }
        };

        // Build the native critic config from the plumbed server config, mirroring
        // the editor's native pull-diagnostics path (`add_native_critic_diagnostics`).
        // The `NativeCriticProfile` selects the rule set; `include`/`exclude`/
        // `severity` refine it. The rc-file `profile` is an external-perlcritic
        // concept the native registry does not consult, so it stays `None`.
        let cfg = &self.native_critic_config;
        let critic_config = CriticConfig {
            severity: cfg.severity.clamp(1, 5),
            profile: None,
            include: cfg.include.clone(),
            exclude: cfg.exclude.clone(),
            ..CriticConfig::default()
        };
        let critic_context = CriticContext::new(code_text, &ast, &critic_config);
        let profile =
            NativeCriticProfile::parse(&cfg.profile).unwrap_or(NativeCriticProfile::Strict);
        let registry = NativeCriticRegistry::for_profile(profile);

        let file = file_path.to_string_lossy();
        let mut formatted_violations: Vec<_> = registry
            .check(&critic_context)
            .into_iter()
            .map(|finding| {
                let violation = finding.to_violation(file.as_ref());
                self.format_violation(
                    &violation.policy,
                    &violation.description,
                    &violation.explanation,
                    violation.severity as u8,
                    (violation.range.start.line + 1) as usize,
                    (violation.range.start.column + 1) as usize,
                    &file,
                )
            })
            .collect();

        if let Some(error) = parse_error {
            let violation = self.create_syntax_error_violation(&error, code_text, file_path);
            formatted_violations.push(self.format_violation(
                &violation.policy,
                &violation.description,
                &violation.explanation,
                violation.severity as u8,
                (violation.range.start.line + 1) as usize,
                (violation.range.start.column + 1) as usize,
                &file,
            ));
        }

        Ok(json!({
            "status": "success",
            "violations": formatted_violations,
            "violationCount": formatted_violations.len(),
            "analyzerUsed": "native"
        }))
    }

    pub(crate) fn run_builtin_critic(&self, file_path: &Path) -> Result<Value, String> {
        use crate::Parser;

        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let code_text = perl_parser::util::code_slice(&content);
        let mut parser = Parser::new(code_text);

        let (ast, parse_error) = match parser.parse() {
            Ok(ast) => (ast, None),
            Err(error) => {
                let message = error.to_string();
                (
                    crate::ast::Node::new(
                        crate::ast::NodeKind::Error {
                            message,
                            expected: vec![],
                            found: None,
                            partial: None,
                        },
                        crate::ast::SourceLocation { start: 0, end: code_text.len() },
                    ),
                    Some(error),
                )
            }
        };

        let analyzer = BuiltInAnalyzer::new();
        let mut all_violations = analyzer.analyze(&ast, code_text);
        if let Some(error) = parse_error {
            all_violations.push(self.create_syntax_error_violation(&error, code_text, file_path));
        }

        let formatted_violations: Vec<_> = all_violations
            .iter()
            .map(|v| {
                self.format_violation(
                    &v.policy,
                    &v.description,
                    &v.explanation,
                    v.severity as u8,
                    (v.range.start.line + 1) as usize,
                    (v.range.start.column + 1) as usize,
                    &file_path.to_string_lossy(),
                )
            })
            .collect();

        Ok(json!({
            "status": "success",
            "violations": formatted_violations,
            "violationCount": formatted_violations.len(),
            "analyzerUsed": "native"
        }))
    }

    pub(crate) fn is_test_file(&self, file_path: &str) -> bool {
        file_path.ends_with(".t") || file_path.contains("/t/") || file_path.contains("test")
    }

    /// Convert a Perl module name to a test file stem.
    ///
    /// `Foo::Bar` -> `foo-bar` (canonical hyphen form used by many CPAN distributions)
    pub fn module_to_test_stem(&self, module_name: &str) -> String {
        module_name.replace("::", "-").to_lowercase()
    }

    /// Infer a module name from a `lib/` path component.
    ///
    /// `/path/to/lib/Foo/Bar.pm` -> `Foo::Bar`
    fn pm_path_to_module(&self, pm_path: &std::path::Path) -> Option<String> {
        // Walk up from the file to find the `lib` directory anchor.
        let components: Vec<_> = pm_path.components().collect();
        let lib_pos = components.iter().rposition(|c| c.as_os_str() == "lib")?;
        let after_lib: Vec<_> = components[lib_pos + 1..].to_vec();
        if after_lib.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        for c in &after_lib {
            let s = c.as_os_str().to_string_lossy();
            let part = if s.ends_with(".pm") {
                s.trim_end_matches(".pm").to_string()
            } else {
                s.to_string()
            };
            parts.push(part);
        }
        Some(parts.join("::"))
    }

    /// Navigate from a `.pm` implementation file to its companion test file.
    ///
    /// Probes (in order):
    ///   1. `t/<stem>.t`  where stem is the hyphen-lowercased module name
    ///   2. `t/<stem>.t`  where stem uses underscores instead of hyphens
    ///   3. `t/<leaf>.t`  where leaf is just the last module component lowercased
    ///   4. `t/lib/<Foo/Bar>.t`  where the relative path mirrors the module hierarchy
    pub(crate) fn go_to_test(&self, pm_path: &std::path::Path) -> Value {
        let module_name = match self.pm_path_to_module(pm_path) {
            Some(m) => m,
            None => {
                return json!({ "found": false, "candidates": [] });
            }
        };

        // Find workspace root: walk up until we find a `lib` or `t` sibling.
        let workspace_root = match self.find_workspace_root(pm_path) {
            Some(r) => r,
            None => {
                return json!({ "found": false, "candidates": [] });
            }
        };

        let t_dir = workspace_root.join("t");
        let stem_hyphen = self.module_to_test_stem(&module_name);
        let stem_underscore = stem_hyphen.replace('-', "_");
        // Leaf component without unwrap: split always produces at least one element.
        let leaf = match module_name.rsplit_once("::") {
            Some((_, last)) => last.to_lowercase(),
            None => module_name.to_lowercase(),
        };
        // Mirror path under t/lib/ (e.g. Foo::Bar::Baz -> t/lib/Foo/Bar/Baz.t)
        let mirror_rel = module_name.replace("::", std::path::MAIN_SEPARATOR_STR) + ".t";

        let candidates = [
            t_dir.join(format!("{stem_hyphen}.t")),
            t_dir.join(format!("{stem_underscore}.t")),
            t_dir.join(format!("{leaf}.t")),
            t_dir.join("lib").join(&mirror_rel),
        ];

        for candidate in &candidates {
            if candidate.exists() {
                return json!({
                    "found": true,
                    "path": candidate.to_string_lossy(),
                    "module": module_name,
                });
            }
        }

        let candidate_strings: Vec<_> =
            candidates.iter().map(|p| p.to_string_lossy().to_string()).collect();
        json!({ "found": false, "candidates": candidate_strings })
    }

    /// Navigate from a test file to the first local module it uses.
    ///
    /// Scans the test file for `use Foo::Bar;` statements (skipping well-known
    /// CPAN pragmas and test modules), then maps the first match to
    /// `lib/Foo/Bar.pm` relative to the workspace root.
    pub(crate) fn go_to_implementation(&self, test_path: &std::path::Path) -> Value {
        let content = match std::fs::read_to_string(test_path) {
            Ok(c) => c,
            Err(_) => return json!({ "found": false }),
        };

        let workspace_root = match self.find_workspace_root(test_path) {
            Some(r) => r,
            None => return json!({ "found": false }),
        };

        // Well-known modules that are NOT local implementations (exact matches).
        const SKIP_MODULES: &[&str] = &[
            // Core pragmas
            "strict",
            "warnings",
            "utf8",
            "feature",
            "parent",
            "base",
            "vars",
            "constant",
            "overload",
            "ok",
            // Core modules
            "Carp",
            "Exporter",
            "Scalar::Util",
            "List::Util",
            "Hash::Util",
            "POSIX",
            "Data::Dumper",
            "Storable",
            "Encode",
            "Cwd",
            "FindBin",
            "File::Basename",
            "File::Path",
            "File::Spec",
            "File::Find",
            "File::Temp",
            "File::Copy",
            "IO::File",
            "IO::Handle",
            "IO::Select",
            "Getopt::Long",
            "Getopt::Std",
            // OO frameworks
            "Moo",
            "Moose",
            "Mouse",
            // Test modules (exact)
            "Test::More",
            "Test::Simple",
            "Test::Builder",
            "Test::Deep",
            "Test::Exception",
            "Test::Warn",
            "Test::Fatal",
            "Test::MockObject",
            "Test::MockModule",
            "Test::Output",
            "Test::Differences",
            "Test::Class",
            "Test::Pod",
            "Test::Pod::Coverage",
            "Try::Tiny",
        ];

        // Module name-space prefixes whose entire hierarchy is non-local.
        // Any `use` statement whose module starts with one of these prefixes
        // will be skipped without needing every sub-module listed.
        const SKIP_PREFIXES: &[&str] = &[
            "Test2::",  // Test2::V0, Test2::Bundle::*, Test2::Tools::*
            "MooseX::", // MooseX::Types, MooseX::Declare, etc.
            "MouseX::",
            "Moo::Role",
            "Moose::Role",
            "Types::",     // Types::Standard, Types::Path::Tiny, etc.
            "namespace::", // namespace::autoclean, namespace::clean
            "Sub::",       // Sub::Exporter, Sub::Quote, etc.
            "Class::MOP",
            "DBIx::", // DBIx::Class, DBIx::Connector
            "DBI",
            "LWP::",
            "HTTP::",
            "URI::",
            "JSON::",
            "YAML::",
            "XML::",
            "DateTime::",
            "Path::Tiny",
            "Path::Class",
        ];

        for line in content.lines() {
            let trimmed = line.trim();
            // Match `use Module::Name;` or `use Module::Name qw(...);`
            if !trimmed.starts_with("use ") {
                continue;
            }
            let after_use = trimmed.trim_start_matches("use ").trim();
            // Extract the module name (stop at first whitespace or semicolon)
            let module_name: String =
                after_use.chars().take_while(|c| c.is_alphanumeric() || *c == ':').collect();

            if module_name.is_empty() {
                continue;
            }
            // Skip version-only pragmas like `use 5.010;` or `use v5.10;`
            if module_name.chars().next().is_some_and(|c| c.is_ascii_digit() || c == 'v') {
                continue;
            }
            if SKIP_MODULES.contains(&module_name.as_str()) {
                continue;
            }
            if SKIP_PREFIXES
                .iter()
                .any(|p| module_name.starts_with(p) || module_name == p.trim_end_matches("::"))
            {
                continue;
            }

            // Map Foo::Bar -> lib/Foo/Bar.pm
            let rel_path = module_name.replace("::", std::path::MAIN_SEPARATOR_STR) + ".pm";
            let candidate = workspace_root.join("lib").join(&rel_path);
            if candidate.exists() {
                return json!({
                    "found": true,
                    "path": candidate.to_string_lossy(),
                    "module": module_name,
                });
            }
        }

        json!({ "found": false })
    }

    /// Find the workspace root by walking up from `path`.
    ///
    /// Preference order:
    ///   1. Explicit workspace roots registered with the provider (normal LSP runtime path).
    ///   2. Nearest ancestor that contains a Perl project marker (`Makefile.PL`,
    ///      `Build.PL`, `cpanfile`, `dist.ini`, `META.json`, `META.yml`, `.git`).
    ///   3. Nearest ancestor that contains either a `lib/` or `t/` child directory.
    ///
    /// This multi-tier strategy avoids accidentally picking up a distant ancestor
    /// that happens to have a `lib/` or `t/` directory unrelated to the current project.
    fn find_workspace_root(&self, path: &std::path::Path) -> Option<std::path::PathBuf> {
        // Tier 1: explicit workspace roots registered with the provider.
        if !self.workspace_roots.is_empty() {
            let canonical_path = path.canonicalize().map_err(|e| {
                tracing::debug!(path = %path.display(), error = %e, "workspace root: failed to canonicalize path");
            }).ok();
            for root in &self.workspace_roots {
                let Ok(canonical_root) = root.canonicalize() else { continue };
                if canonical_path.as_ref().is_some_and(|p| p.starts_with(&canonical_root)) {
                    return Some(root.clone());
                }
            }
        }

        // Perl distribution marker files that indicate a project root.
        const PROJECT_MARKERS: &[&str] =
            &["Makefile.PL", "Build.PL", "cpanfile", "dist.ini", "META.json", "META.yml", ".git"];

        let mut current = path.parent()?;

        // Tier 2: walk up looking for a Perl project marker first.
        let mut tier3_candidate: Option<std::path::PathBuf> = None;
        loop {
            // Check for definitive project markers.
            if PROJECT_MARKERS.iter().any(|m| current.join(m).exists()) {
                return Some(current.to_path_buf());
            }
            // Remember the first ancestor with lib/ or t/ for tier-3 fallback.
            if tier3_candidate.is_none()
                && (current.join("lib").is_dir() || current.join("t").is_dir())
            {
                tier3_candidate = Some(current.to_path_buf());
            }
            current = match current.parent() {
                Some(p) => p,
                None => break,
            };
        }

        // Tier 3: fall back to the nearest lib/t ancestor found above.
        tier3_candidate
    }

    pub(crate) fn format_command_result(
        &self,
        result: std::process::Output,
        extra_field: Option<(&str, Value)>,
    ) -> Value {
        let output = String::from_utf8_lossy(&result.stdout);
        let error = if !result.status.success() {
            Some(String::from_utf8_lossy(&result.stderr).to_string())
        } else {
            None
        };

        let mut response = json!({
            "success": result.status.success(),
            "output": output.to_string(),
            "error": error
        });

        if let Some((key, value)) = extra_field {
            response[key] = value;
        }

        response
    }

    fn format_command_launch_failure(&self, command: &str, error: String) -> Value {
        json!({
            "success": false,
            "output": String::new(),
            "error": error,
            "command": command
        })
    }

    fn resolve_path_from_args(&self, arguments: &[Value]) -> Result<PathBuf, String> {
        let raw_path = arguments
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing file path argument".to_string())?;

        const MAX_ARG_LENGTH: usize = 4096;
        if raw_path.len() > MAX_ARG_LENGTH {
            return Err(format!(
                "Argument too long ({} bytes, max {})",
                raw_path.len(),
                MAX_ARG_LENGTH
            ));
        }

        let path = if raw_path.starts_with("file://") {
            crate::workspace_index::uri_to_fs_path(raw_path)
                .ok_or_else(|| format!("Failed to parse file URI: {raw_path}"))?
        } else {
            PathBuf::from(raw_path)
        };
        let normalized_path = path.to_string_lossy();
        if normalized_path.contains("..") {
            return Err("Path traversal attempt detected: path contains '..' component".to_string());
        }

        let canonical_path = path
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize path '{}': {}", normalized_path, e))?;

        let effective_roots: Vec<PathBuf> = if self.workspace_roots.is_empty() {
            match std::env::current_dir() {
                Ok(cwd) => vec![cwd],
                Err(_) => {
                    return Err(
                        "No workspace roots configured and cannot determine working directory"
                            .to_string(),
                    );
                }
            }
        } else {
            self.workspace_roots.clone()
        };

        let allowed = effective_roots.iter().any(|workspace_root| {
            workspace_root
                .canonicalize()
                .map(|canonical_root| canonical_path.starts_with(&canonical_root))
                .unwrap_or(false)
        });

        if !allowed {
            return Err(format!(
                "Path traversal detected: {} is outside workspace boundaries",
                canonical_path.display()
            ));
        }

        if !canonical_path.exists() {
            return Err(format!("File not found: {}", canonical_path.display()));
        }

        if !canonical_path.is_file() {
            return Err(format!("Path is not a file: {}", canonical_path.display()));
        }

        std::fs::metadata(&canonical_path).map_err(|e| {
            format!("Cannot read file metadata '{}': {}", canonical_path.display(), e)
        })?;

        Ok(canonical_path)
    }

    /// Resolve a debug file path using the same workspace security checks.
    pub fn resolve_debug_file_path(&self, file_path: &str) -> Result<PathBuf, String> {
        self.resolve_path_from_args(&[Value::String(file_path.to_string())])
    }

    #[deprecated(since = "0.8.9", note = "Use resolve_path_from_args for secure path resolution")]
    #[allow(dead_code)]
    pub(crate) fn normalize_file_path<'a>(&self, file_path: &'a str) -> Cow<'a, str> {
        if !file_path.starts_with("file://") {
            return Cow::Borrowed(file_path);
        }

        if let Ok(url) = Url::parse(file_path)
            && let Ok(path) = url.to_file_path()
        {
            return Cow::Owned(path.to_string_lossy().into_owned());
        }

        Cow::Borrowed(file_path.strip_prefix("file://").unwrap_or(file_path))
    }

    pub(crate) fn format_violation(
        &self,
        policy: &str,
        description: &str,
        explanation: &str,
        severity: u8,
        line: usize,
        column: usize,
        file: &str,
    ) -> Value {
        json!({
            "policy": policy,
            "description": description,
            "explanation": explanation,
            "severity": severity,
            "line": line,
            "column": column,
            "file": file
        })
    }

    pub(crate) fn format_critic_error(&self, error_message: String, analyzer_used: &str) -> Value {
        json!({
            "status": "error",
            "error": error_message,
            "violations": [],
            "violationCount": 0,
            "analyzerUsed": analyzer_used
        })
    }

    fn create_syntax_error_violation(
        &self,
        error: &perl_parser::ParseError,
        _content: &str,
        file_path: &Path,
    ) -> crate::perl_critic::Violation {
        let error_msg = format!("{}", error);
        let (line, column) = (0, 0);

        crate::perl_critic::Violation {
            policy: "Syntax::ParseError".to_string(),
            description: format!("Syntax error: {}", error_msg),
            explanation: "This code contains a syntax error that prevents parsing. Fix the syntax error before running additional checks.".to_string(),
            severity: crate::perl_critic::Severity::Brutal,
            range: crate::position::Range {
                start: crate::position::Position { byte: 0, line: line as u32, column: column as u32 },
                end: crate::position::Position { byte: 1, line: line as u32, column: (column + 1) as u32 },
            },
            file: file_path.to_string_lossy().to_string(),
        }
    }

    pub(crate) fn command_exists(&self, command: &str) -> bool {
        // On Windows, spawning `Command::new("where")` is itself a bare-name call
        // subject to the same CWD-first CreateProcess RCE (#3028).  Use the
        // hardened PATH-only resolver instead — it already answers "is this tool
        // findable on an absolute PATH directory" without touching the CWD.
        //
        // On non-Windows, the resolver is a pass-through (returns Ok unchanged),
        // so we fall back to the `which` crate for the actual PATH search there.
        #[cfg(all(windows, not(target_arch = "wasm32")))]
        {
            perl_subprocess_runtime::resolve_program(command).is_ok()
        }
        #[cfg(all(not(windows), not(target_arch = "wasm32")))]
        {
            let mut cmd = Command::new("which");
            cmd.arg(command);
            crate::util::run_command_with_timeout(cmd, 2)
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }
}

fn default_provider_decision_explanation(
    provider: ProviderDecisionProvider,
) -> ProviderDecisionExplanation {
    let (
        decision,
        reason,
        fact_source,
        confidence,
        freshness,
        dynamic_boundary,
        fallback,
        receipt_id,
        scenario,
    ) = match provider {
        ProviderDecisionProvider::Completion => (
            ProviderDecisionOutcome::Fallback,
            ProviderDecisionReason::FallbackPolicy,
            ProviderDecisionFactSource::CompilerFact,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::LegacyProvider,
            Some("docs/project/status/provider_confidence_matrix.md#completion"),
            Some("ux_scenario_28_mojolicious_completion_ranking"),
        ),
        ProviderDecisionProvider::GotoDefinition => (
            ProviderDecisionOutcome::Acted,
            ProviderDecisionReason::SourceBackedHighConfidence,
            ProviderDecisionFactSource::CompilerFact,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::None,
            Some("docs/project/status/provider_cutover.md#navigation-live-quality-dashboard"),
            Some("ux_scenario_30_mojolicious_navigation_quality"),
        ),
        ProviderDecisionProvider::TypeDefinition => (
            ProviderDecisionOutcome::Acted,
            ProviderDecisionReason::SourceBackedHighConfidence,
            ProviderDecisionFactSource::ParserSyntax,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::None,
            Some("docs/project/status/provider_confidence_matrix.md#type-definition"),
            Some("type-definition-provider-decision-receipt"),
        ),
        ProviderDecisionProvider::References => (
            ProviderDecisionOutcome::Acted,
            ProviderDecisionReason::SourceBackedHighConfidence,
            ProviderDecisionFactSource::CompilerFact,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::None,
            Some("docs/project/status/provider_cutover.md#navigation-live-quality-dashboard"),
            Some("ux_scenario_30_mojolicious_navigation_quality"),
        ),
        ProviderDecisionProvider::Hover => (
            ProviderDecisionOutcome::Fallback,
            ProviderDecisionReason::FallbackPolicy,
            ProviderDecisionFactSource::CompilerFact,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::LegacyProvider,
            Some("docs/project/status/provider_confidence_matrix.md#hover"),
            Some("ux_scenario_29_mojolicious_hover_provenance"),
        ),
        ProviderDecisionProvider::Diagnostics => (
            ProviderDecisionOutcome::Fallback,
            ProviderDecisionReason::FallbackPolicy,
            ProviderDecisionFactSource::CompilerFact,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::LegacyProvider,
            Some("docs/project/status/provider_confidence_matrix.md#diagnostics"),
            Some("ux_scenario_31_mojolicious_diagnostics_quality"),
        ),
        ProviderDecisionProvider::Rename => (
            ProviderDecisionOutcome::Fallback,
            ProviderDecisionReason::FallbackPolicy,
            ProviderDecisionFactSource::ParserSyntax,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::LegacyProvider,
            Some("docs/project/status/provider_confidence_matrix.md#rename"),
            Some("ux_scenario_35_mojolicious_rename_unsafe_edit"),
        ),
        ProviderDecisionProvider::SafeDelete => (
            ProviderDecisionOutcome::Blocked,
            ProviderDecisionReason::UnsafeEditBlocked,
            ProviderDecisionFactSource::CompilerFact,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::NoEdit,
            Some("docs/project/status/provider_confidence_matrix.md#safe-delete"),
            Some("realbaseline-safe-delete-imported-symbol"),
        ),
        ProviderDecisionProvider::WorkspaceSymbols => (
            ProviderDecisionOutcome::Acted,
            ProviderDecisionReason::SourceBackedHighConfidence,
            ProviderDecisionFactSource::CompilerFact,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::None,
            Some("docs/project/status/provider_confidence_matrix.md#workspace-symbols"),
            Some("ux_scenario_33_mojolicious_workspace_symbol_noise"),
        ),
        ProviderDecisionProvider::DocumentSymbols => (
            ProviderDecisionOutcome::Acted,
            ProviderDecisionReason::SourceBackedHighConfidence,
            ProviderDecisionFactSource::ParserSyntax,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::None,
            Some("docs/project/status/provider_confidence_matrix.md#document-symbols"),
            Some("ux_scenario_32_mojolicious_document_symbols_quality"),
        ),
        ProviderDecisionProvider::SemanticTokens => (
            ProviderDecisionOutcome::Shadowed,
            ProviderDecisionReason::ShadowOnly,
            ProviderDecisionFactSource::SemanticFact,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::ShadowReceiptOnly,
            Some("docs/project/status/provider_confidence_matrix.md#semantic-tokens"),
            Some("ux_scenario_34_mojolicious_semantic_tokens_quality"),
        ),
        ProviderDecisionProvider::ModuleResolution => (
            ProviderDecisionOutcome::Acted,
            ProviderDecisionReason::SourceBackedHighConfidence,
            ProviderDecisionFactSource::SemanticFact,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::None,
            Some("docs/project/status/module_resolution.md"),
            Some("ux_scenario_14_inc_conformance"),
        ),
        ProviderDecisionProvider::DapModulePaths => (
            ProviderDecisionOutcome::Acted,
            ProviderDecisionReason::SourceBackedHighConfidence,
            ProviderDecisionFactSource::SemanticFact,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::None,
            Some("docs/development/PERL_ORACLE_RAIL.md"),
            Some("dap-module-resolution-smoke"),
        ),
        ProviderDecisionProvider::PerlSubprocess => (
            ProviderDecisionOutcome::Acted,
            ProviderDecisionReason::SourceBackedHighConfidence,
            ProviderDecisionFactSource::LegacyWorkspace,
            ProviderDecisionConfidence::High,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::None,
            Some("docs/architecture/perl-subprocess-seams.md"),
            Some("perl-oracle-env"),
        ),
        ProviderDecisionProvider::WorkspaceTrustReport => (
            // Report-only boundary (PLSP-SPEC-0016): the trust report aggregates
            // existing runtime state without driving live provider behavior, so it
            // is shadowed rather than acted, and must not promote support tiers.
            ProviderDecisionOutcome::Shadowed,
            ProviderDecisionReason::ShadowOnly,
            ProviderDecisionFactSource::LegacyWorkspace,
            ProviderDecisionConfidence::Low,
            ProviderDecisionFreshness::NotApplicable,
            false,
            ProviderDecisionFallback::ShadowReceiptOnly,
            Some("docs/project/status/SUPPORT_TIERS.md"),
            Some("workspace-trust-report-boundary"),
        ),
        ProviderDecisionProvider::Unknown => (
            ProviderDecisionOutcome::Fallback,
            ProviderDecisionReason::MissingFact,
            ProviderDecisionFactSource::Unknown,
            ProviderDecisionConfidence::Low,
            ProviderDecisionFreshness::Unknown,
            false,
            ProviderDecisionFallback::NoResult,
            None,
            None,
        ),
        _ => (
            ProviderDecisionOutcome::Fallback,
            ProviderDecisionReason::Unknown,
            ProviderDecisionFactSource::Unknown,
            ProviderDecisionConfidence::Low,
            ProviderDecisionFreshness::Unknown,
            false,
            ProviderDecisionFallback::NoResult,
            None,
            None,
        ),
    };

    let mut explanation = ProviderDecisionExplanation::new(
        provider,
        decision,
        reason,
        fact_source,
        confidence,
        freshness,
        dynamic_boundary,
        fallback,
    );
    if let Some(receipt_id) = receipt_id {
        explanation = explanation.with_receipt_id(receipt_id);
    }
    if let Some(scenario) = scenario {
        explanation = explanation.with_scenario(scenario);
    }
    explanation
}

fn workspace_root_class(workspace_roots: &[PathBuf]) -> &'static str {
    match workspace_roots.len() {
        0 => "none",
        1 => "single_root",
        _ => "multi_root",
    }
}

fn workspace_root_hash(workspace_roots: &[PathBuf]) -> Option<String> {
    if workspace_roots.is_empty() {
        return None;
    }

    let mut roots = workspace_roots
        .iter()
        .map(|root| root.to_string_lossy().replace('\\', "/"))
        .collect::<Vec<_>>();
    roots.sort();
    Some(format!("{:x}", md5::compute(roots.join("\n"))))
}

fn provider_support_tier_link(_provider: ProviderDecisionProvider) -> &'static str {
    "docs/project/status/SUPPORT_TIERS.md#claim-rows"
}

pub(crate) fn select_test_runner(
    is_test_file: bool,
    yath_available: bool,
    prove_available: bool,
) -> TestRunner {
    if !is_test_file {
        TestRunner::Perl
    } else if yath_available {
        TestRunner::Yath
    } else if prove_available {
        TestRunner::Prove
    } else {
        TestRunner::Perl
    }
}

/// Check whether a command exists in the current PATH.
pub fn command_exists(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Return the supported executeCommand identifiers.
pub fn get_supported_commands() -> Vec<String> {
    // Keep in sync with perl_lsp_rs_core::protocol::capabilities::get_supported_commands
    vec![
        "perl.runTests".to_string(),
        "perl.runFile".to_string(),
        "perl.runTestSub".to_string(),
        "perl.runCritic".to_string(),
        "perl.runTest".to_string(),
        "perl.runTestFile".to_string(),
        "perl.runSubtest".to_string(),
        "perl.debugFile".to_string(),
        "perl.debugTest".to_string(),
        "perl.debugTestFile".to_string(),
        "perl.goToTest".to_string(),
        "perl.goToImplementation".to_string(),
        "perl.explainProviderDecision".to_string(),
        "perl.workspaceTrustReport".to_string(),
        "perl.previewSafeDelete".to_string(),
        "perl.safeDeleteSymbol".to_string(),
        "perl.previewPackageRename".to_string(),
        "perl.explainMissingModuleLookup".to_string(),
    ]
}

#[cfg(test)]
mod normalize_path_tests;
