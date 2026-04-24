#[cfg(feature = "lsp-compat")]
use super::QuickFix;
#[cfg(not(feature = "lsp-compat"))]
use super::ViolationSummary;
#[cfg(feature = "lsp-compat")]
use super::perlcritic_quick_fix;
use super::{CriticConfig, Severity, Violation};
use crate::critic_parser::parse_perlcritic_output;
use perl_parser_core::position::{Position, Range};
use perl_subprocess_runtime::SubprocessRuntime;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "lsp-compat")]
use lsp_types;

/// Perl::Critic analyzer
pub struct CriticAnalyzer {
    /// Configuration settings for the analyzer
    config: CriticConfig,
    /// Cache of violations keyed by file path
    cache: HashMap<String, Vec<Violation>>,
    /// Subprocess runtime for executing perlcritic
    runtime: Arc<dyn SubprocessRuntime>,
}

impl CriticAnalyzer {
    /// Creates a new analyzer with the given configuration and runtime.
    pub fn new(config: CriticConfig, runtime: Arc<dyn SubprocessRuntime>) -> Self {
        Self { config, cache: HashMap::new(), runtime }
    }

    /// Creates a new analyzer with the OS subprocess runtime (non-WASM only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_os_runtime(config: CriticConfig) -> Self {
        use perl_subprocess_runtime::OsSubprocessRuntime;
        let timeout = config.timeout_secs;
        Self::new(config, Arc::new(OsSubprocessRuntime::with_timeout(timeout)))
    }

    /// Run Perl::Critic on a file
    pub fn analyze_file(&mut self, file_path: &Path) -> Result<Vec<Violation>, String> {
        let path_str = file_path.to_string_lossy().to_string();
        if let Some(cached) = self.cache.get(&path_str) {
            return Ok(cached.clone());
        }

        let args = build_perlcritic_args(&self.config, &path_str);
        let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output =
            self.runtime.run_command("perlcritic", &args_refs, None).map_err(|e| e.message)?;
        let violations = self.parse_output(&output.stdout, &path_str)?;
        self.cache.insert(path_str, violations.clone());
        Ok(violations)
    }

    /// Parse perlcritic output
    fn parse_output(&self, output: &[u8], file_path: &str) -> Result<Vec<Violation>, String> {
        let output_str = String::from_utf8_lossy(output);
        Ok(parse_perlcritic_output(&output_str)
            .into_iter()
            .map(|parsed| Violation {
                policy: parsed.policy.clone(),
                description: parsed.message,
                explanation: self.get_policy_explanation(&parsed.policy),
                severity: Severity::from_number(parsed.severity),
                range: Range {
                    start: Position { byte: 0, line: parsed.line - 1, column: parsed.column - 1 },
                    end: Position { byte: 0, line: parsed.line - 1, column: parsed.column },
                },
                file: file_path.to_string(),
            })
            .collect())
    }

    /// Get explanation for a policy
    fn get_policy_explanation(&self, policy: &str) -> String {
        format!("See perldoc Perl::Critic::Policy::{policy}")
    }

    /// Clear cache for a file
    pub fn invalidate_cache(&mut self, file_path: &str) {
        self.cache.remove(file_path);
    }

    /// Convert violations to diagnostics
    #[cfg(feature = "lsp-compat")]
    pub fn to_diagnostics(&self, violations: &[Violation]) -> Vec<lsp_types::Diagnostic> {
        violations
            .iter()
            .map(|v| {
                let lsp_range = lsp_types::Range::new(
                    lsp_types::Position::new(v.range.start.line, v.range.start.column),
                    lsp_types::Position::new(v.range.end.line, v.range.end.column),
                );
                lsp_types::Diagnostic {
                    range: lsp_range,
                    severity: Some(v.severity.to_diagnostic_severity()),
                    code: Some(lsp_types::NumberOrString::String(v.policy.clone())),
                    source: Some("perlcritic".to_string()),
                    message: v.description.clone(),
                    related_information: None,
                    tags: None,
                    code_description: None,
                    data: None,
                }
            })
            .collect()
    }

    /// Convert violations to violation summaries (for non-LSP contexts)
    #[cfg(not(feature = "lsp-compat"))]
    pub fn to_violation_summaries(&self, violations: &[Violation]) -> Vec<ViolationSummary> {
        violations
            .iter()
            .map(|v| ViolationSummary {
                policy: v.policy.clone(),
                description: v.description.clone(),
                severity: v.severity.to_severity_level(),
                line: v.range.start.line as usize,
            })
            .collect()
    }

    /// Get quick fix for a violation
    #[cfg(feature = "lsp-compat")]
    pub fn get_quick_fix(&self, violation: &Violation, _content: &str) -> Option<QuickFix> {
        perlcritic_quick_fix(violation)
    }
}

fn build_perlcritic_args(config: &CriticConfig, path_str: &str) -> Vec<String> {
    let mut args = vec![format!("--severity={}", config.severity)];

    if let Some(profile) = &config.profile {
        args.push(format!("--profile={profile}"));
    }
    if let Some(theme) = &config.theme {
        args.push(format!("--theme={theme}"));
    }
    for policy in &config.include {
        args.push(format!("--include={policy}"));
    }
    for policy in &config.exclude {
        args.push(format!("--exclude={policy}"));
    }

    args.push("--verbose=%f:%l:%c:%s:%p:%m\\n".to_string());
    args.push("--".to_string());
    args.push(path_str.to_string());
    args
}
