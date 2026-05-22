//! Runtime workload tuning for latency-sensitive scenarios.
//!
//! [`RuntimeTuning`] is a small bag of dials that govern *how* the server
//! does work, not *what* features it advertises. It lets editor harnesses
//! (especially Neovim's e2e tests) opt into a faster, lower-noise runtime
//! without changing the LSP feature surface or the feature profile.
//!
//! Sources of truth, lowest priority first:
//!
//! 1. Compile-time defaults from [`RuntimeTuning::normal_defaults`] /
//!    [`RuntimeTuning::e2e_defaults`].
//! 2. Environment variables (`PERL_LSP_E2E`, `PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS`,
//!    `PERL_LSP_DIAGNOSTIC_MODE`).
//! 3. CLI overrides parsed by [`crate::runtime::launcher::parse_args`].
//!
//! CLI wins over env, env wins over compiled defaults. The shape is
//! deliberately small — six fields — because every dial we add is a new
//! interaction with the rest of the server.
//!
//! This module owns the *shape* of the config and its parsing only. The
//! actual behavioral wiring (debouncer interval, syntax-only diagnostics,
//! workspace-indexing gate, etc.) lives in the consuming runtime.

use std::time::Duration;

/// Coarse-grained runtime workload mode.
///
/// `Normal` is the default for editor sessions. `E2e` favours fast,
/// low-noise behavior for latency-focused harnesses: zero diagnostic
/// debounce, syntax-only diagnostics, no eager workspace indexing, and
/// no opt-in file watching by default.
///
/// This is **orthogonal** to [`FeatureProfile`](crate::governance::FeatureProfile):
/// the LSP capability advertisement is unchanged. Only the runtime workload
/// pattern shifts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    /// Production / normal editor behavior.
    Normal,
    /// Latency-focused harness mode (e.g. Neovim e2e tests).
    E2e,
}

impl RuntimeMode {
    /// CLI/env token for this mode.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::E2e => "e2e",
        }
    }

    /// Parse a CLI / env value into a runtime mode.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "normal" | "default" => Some(Self::Normal),
            "e2e" => Some(Self::E2e),
            _ => None,
        }
    }
}

/// Granularity of diagnostic publication.
///
/// `Normal` runs the full diagnostic stack (parse + semantic +
/// module-resolution + native critic + external critic + workspace
/// dead-code).
///
/// `SyntaxOnly` restricts diagnostics to parse errors only. Used by e2e
/// harnesses that wait for diagnostics to settle before measuring hover /
/// completion latency — running the full stack on every keystroke makes
/// the "first useful answer" appear slower than the server actually is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticMode {
    /// Full diagnostic pipeline.
    Normal,
    /// Parse errors only; skip semantic / critic / module-resolution / dead-code.
    SyntaxOnly,
}

impl DiagnosticMode {
    /// CLI/env token for this mode.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::SyntaxOnly => "syntax-only",
        }
    }

    /// Parse a CLI / env value into a diagnostic mode.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "normal" | "full" => Some(Self::Normal),
            "syntax-only" | "syntax_only" | "syntax" => Some(Self::SyntaxOnly),
            _ => None,
        }
    }
}

/// Runtime workload tuning knobs.
///
/// Treat this struct as read-only after construction. The fields are
/// intentionally simple values — the consuming runtime branches on them
/// rather than going through a trait, because each field has one or two
/// call sites and we want them legible at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeTuning {
    /// Coarse workload pattern (drives the other defaults).
    pub runtime_mode: RuntimeMode,
    /// Diagnostic publication scope.
    pub diagnostic_mode: DiagnosticMode,
    /// Debounce window in milliseconds for `publishDiagnostics`. `0`
    /// publishes immediately.
    pub diagnostic_debounce_ms: u64,
    /// Whether `initialized` should kick off a workspace-wide indexing
    /// scan eagerly. E2E defaults to `false`.
    pub eager_workspace_indexing: bool,
    /// Whether file-system watchers are registered with the client.
    /// E2E defaults to `false`.
    pub file_watchers: bool,
}

impl RuntimeTuning {
    /// Default tuning for normal editor sessions.
    ///
    /// 250ms debounce, full diagnostics, eager workspace indexing on,
    /// file watchers on.
    pub const fn normal_defaults() -> Self {
        Self {
            runtime_mode: RuntimeMode::Normal,
            diagnostic_mode: DiagnosticMode::Normal,
            diagnostic_debounce_ms: 250,
            eager_workspace_indexing: true,
            file_watchers: true,
        }
    }

    /// Default tuning for e2e harness sessions.
    ///
    /// Zero debounce, syntax-only diagnostics, no eager indexing, no file
    /// watchers — every dial pointed at "first useful answer arrives fast,
    /// background noise stays quiet."
    pub const fn e2e_defaults() -> Self {
        Self {
            runtime_mode: RuntimeMode::E2e,
            diagnostic_mode: DiagnosticMode::SyntaxOnly,
            diagnostic_debounce_ms: 0,
            eager_workspace_indexing: false,
            file_watchers: false,
        }
    }

    /// Defaults for a given runtime mode.
    pub const fn defaults_for(mode: RuntimeMode) -> Self {
        match mode {
            RuntimeMode::Normal => Self::normal_defaults(),
            RuntimeMode::E2e => Self::e2e_defaults(),
        }
    }

    /// The diagnostic debounce interval as a [`Duration`].
    pub const fn diagnostic_debounce(self) -> Duration {
        Duration::from_millis(self.diagnostic_debounce_ms)
    }

    /// Is debounce effectively immediate (zero)?
    pub const fn diagnostic_debounce_is_immediate(self) -> bool {
        self.diagnostic_debounce_ms == 0
    }

    /// Resolve tuning by layering env vars on top of compiled defaults.
    ///
    /// CLI overrides applied after this should win; see
    /// [`Self::apply_cli_overrides`].
    pub fn from_env() -> Self {
        Self::from_env_with(|name| std::env::var(name).ok())
    }

    /// Resolve tuning using an injected env reader (for tests).
    pub fn from_env_with<F>(mut read_env: F) -> Self
    where
        F: FnMut(&str) -> Option<String>,
    {
        let e2e_env = read_env("PERL_LSP_E2E");
        let runtime_mode = match env_truthy(&e2e_env) {
            Some(true) => RuntimeMode::E2e,
            _ => RuntimeMode::Normal,
        };

        let mut tuning = Self::defaults_for(runtime_mode);

        if let Some(raw) = read_env("PERL_LSP_DIAGNOSTIC_MODE") {
            if let Some(mode) = DiagnosticMode::parse(&raw) {
                tuning.diagnostic_mode = mode;
            }
        }

        if let Some(raw) = read_env("PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS") {
            if let Ok(parsed) = raw.trim().parse::<u64>() {
                tuning.diagnostic_debounce_ms = parsed;
            }
        }

        if let Some(raw) = read_env("PERL_LSP_EAGER_WORKSPACE_INDEXING") {
            if let Some(flag) = env_truthy(&Some(raw)) {
                tuning.eager_workspace_indexing = flag;
            }
        }

        if let Some(raw) = read_env("PERL_LSP_FILE_WATCHERS") {
            if let Some(flag) = env_truthy(&Some(raw)) {
                tuning.file_watchers = flag;
            }
        }

        tuning
    }

    /// Apply CLI-supplied overrides on top of the current tuning.
    ///
    /// Used by the launcher so CLI flags win over env vars. Each `Option`
    /// is "user provided this on the command line" — `None` means "leave
    /// whatever env / defaults gave us."
    pub fn apply_cli_overrides(
        &mut self,
        runtime_mode: Option<RuntimeMode>,
        diagnostic_mode: Option<DiagnosticMode>,
        diagnostic_debounce_ms: Option<u64>,
        eager_workspace_indexing: Option<bool>,
        file_watchers: Option<bool>,
    ) {
        if let Some(mode) = runtime_mode {
            let new_defaults = Self::defaults_for(mode);
            // Switching modes from CLI resets every dial the user did not
            // explicitly override — that's the whole point of `--runtime-mode e2e`.
            self.runtime_mode = mode;
            self.diagnostic_mode = new_defaults.diagnostic_mode;
            self.diagnostic_debounce_ms = new_defaults.diagnostic_debounce_ms;
            self.eager_workspace_indexing = new_defaults.eager_workspace_indexing;
            self.file_watchers = new_defaults.file_watchers;
        }

        if let Some(mode) = diagnostic_mode {
            self.diagnostic_mode = mode;
        }
        if let Some(ms) = diagnostic_debounce_ms {
            self.diagnostic_debounce_ms = ms;
        }
        if let Some(flag) = eager_workspace_indexing {
            self.eager_workspace_indexing = flag;
        }
        if let Some(flag) = file_watchers {
            self.file_watchers = flag;
        }
    }
}

impl Default for RuntimeTuning {
    fn default() -> Self {
        Self::normal_defaults()
    }
}

fn env_truthy(value: &Option<String>) -> Option<bool> {
    let raw = value.as_deref()?;
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    Some(!matches!(normalized.as_str(), "0" | "false" | "no" | "off"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_some;

    #[test]
    fn runtime_mode_normal_defaults_unchanged() {
        let tuning = RuntimeTuning::normal_defaults();
        assert_eq!(tuning.runtime_mode, RuntimeMode::Normal);
        assert_eq!(tuning.diagnostic_mode, DiagnosticMode::Normal);
        assert_eq!(tuning.diagnostic_debounce_ms, 250);
        assert!(tuning.eager_workspace_indexing);
        assert!(tuning.file_watchers);
        assert_eq!(tuning, RuntimeTuning::default());
    }

    #[test]
    fn runtime_mode_e2e_defaults() {
        let tuning = RuntimeTuning::e2e_defaults();
        assert_eq!(tuning.runtime_mode, RuntimeMode::E2e);
        assert_eq!(tuning.diagnostic_mode, DiagnosticMode::SyntaxOnly);
        assert_eq!(tuning.diagnostic_debounce_ms, 0);
        assert!(!tuning.eager_workspace_indexing);
        assert!(!tuning.file_watchers);
    }

    #[test]
    fn diagnostic_debounce_zero_is_immediate() {
        let mut tuning = RuntimeTuning::normal_defaults();
        tuning.diagnostic_debounce_ms = 0;
        assert!(tuning.diagnostic_debounce_is_immediate());
        assert_eq!(tuning.diagnostic_debounce(), Duration::from_millis(0));

        tuning.diagnostic_debounce_ms = 1;
        assert!(!tuning.diagnostic_debounce_is_immediate());

        // e2e defaults imply zero debounce.
        let e2e = RuntimeTuning::e2e_defaults();
        assert!(e2e.diagnostic_debounce_is_immediate());
    }

    #[test]
    fn runtime_mode_parse_round_trips() {
        for mode in [RuntimeMode::Normal, RuntimeMode::E2e] {
            let parsed = must_some(RuntimeMode::parse(mode.as_str()));
            assert_eq!(parsed, mode);
        }
        assert!(RuntimeMode::parse("E2E").is_some());
        assert_eq!(RuntimeMode::parse("default"), Some(RuntimeMode::Normal));
        assert!(RuntimeMode::parse("bogus").is_none());
    }

    #[test]
    fn diagnostic_mode_parse_round_trips() {
        for mode in [DiagnosticMode::Normal, DiagnosticMode::SyntaxOnly] {
            let parsed = must_some(DiagnosticMode::parse(mode.as_str()));
            assert_eq!(parsed, mode);
        }
        assert_eq!(DiagnosticMode::parse("Syntax_Only"), Some(DiagnosticMode::SyntaxOnly));
        assert_eq!(DiagnosticMode::parse("full"), Some(DiagnosticMode::Normal));
        assert!(DiagnosticMode::parse("noisy").is_none());
    }

    fn env_map<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| pairs.iter().find(|(k, _)| *k == name).map(|(_, v)| (*v).to_string())
    }

    #[test]
    fn from_env_default_when_no_vars_set() {
        let tuning = RuntimeTuning::from_env_with(|_| None);
        assert_eq!(tuning, RuntimeTuning::normal_defaults());
    }

    #[test]
    fn from_env_e2e_sets_e2e_defaults() {
        let tuning = RuntimeTuning::from_env_with(env_map(&[("PERL_LSP_E2E", "1")]));
        assert_eq!(tuning, RuntimeTuning::e2e_defaults());
    }

    #[test]
    fn from_env_e2e_then_diagnostic_overrides_wins_over_e2e_defaults() {
        let tuning = RuntimeTuning::from_env_with(env_map(&[
            ("PERL_LSP_E2E", "1"),
            ("PERL_LSP_DIAGNOSTIC_MODE", "normal"),
            ("PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS", "120"),
        ]));
        assert_eq!(tuning.runtime_mode, RuntimeMode::E2e);
        assert_eq!(tuning.diagnostic_mode, DiagnosticMode::Normal);
        assert_eq!(tuning.diagnostic_debounce_ms, 120);
        // Knobs not overridden retain e2e values:
        assert!(!tuning.eager_workspace_indexing);
        assert!(!tuning.file_watchers);
    }

    #[test]
    fn from_env_invalid_values_ignored() {
        let tuning = RuntimeTuning::from_env_with(env_map(&[
            ("PERL_LSP_DIAGNOSTIC_MODE", "carbonara"),
            ("PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS", "fast"),
        ]));
        assert_eq!(tuning, RuntimeTuning::normal_defaults());
    }

    #[test]
    fn from_env_truthy_falsy_tokens() {
        let off = RuntimeTuning::from_env_with(env_map(&[("PERL_LSP_E2E", "0")]));
        assert_eq!(off.runtime_mode, RuntimeMode::Normal);

        let off2 = RuntimeTuning::from_env_with(env_map(&[("PERL_LSP_E2E", "false")]));
        assert_eq!(off2.runtime_mode, RuntimeMode::Normal);

        let on = RuntimeTuning::from_env_with(env_map(&[("PERL_LSP_E2E", "yes")]));
        assert_eq!(on.runtime_mode, RuntimeMode::E2e);
    }

    #[test]
    fn cli_runtime_mode_resets_defaults_then_other_cli_dials_apply() {
        let mut tuning = RuntimeTuning::normal_defaults();
        tuning.apply_cli_overrides(
            Some(RuntimeMode::E2e),
            Some(DiagnosticMode::Normal),
            Some(75),
            None,
            None,
        );
        assert_eq!(tuning.runtime_mode, RuntimeMode::E2e);
        // CLI diag mode wins over the e2e default of syntax-only.
        assert_eq!(tuning.diagnostic_mode, DiagnosticMode::Normal);
        assert_eq!(tuning.diagnostic_debounce_ms, 75);
        // Knobs the CLI did not provide picked up e2e defaults.
        assert!(!tuning.eager_workspace_indexing);
        assert!(!tuning.file_watchers);
    }

    #[test]
    fn cli_overrides_preserve_existing_when_none() {
        let mut tuning = RuntimeTuning::e2e_defaults();
        tuning.apply_cli_overrides(None, None, None, None, None);
        assert_eq!(tuning, RuntimeTuning::e2e_defaults());
    }

    #[test]
    fn cli_partial_override_keeps_env_choice_for_non_overridden_dials() {
        // Simulate "env said e2e, CLI bumped debounce to 50".
        let mut tuning = RuntimeTuning::e2e_defaults();
        tuning.apply_cli_overrides(None, None, Some(50), None, None);
        assert_eq!(tuning.runtime_mode, RuntimeMode::E2e);
        assert_eq!(tuning.diagnostic_mode, DiagnosticMode::SyntaxOnly);
        assert_eq!(tuning.diagnostic_debounce_ms, 50);
        assert!(!tuning.eager_workspace_indexing);
        assert!(!tuning.file_watchers);
    }
}
