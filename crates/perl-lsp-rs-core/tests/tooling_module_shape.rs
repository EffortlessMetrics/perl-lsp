//! Integration test: `perl-lsp-tooling` public API reachable via `perl_lsp_rs_core::tooling`.
//!
//! NOTE(G3-API-fix): Red-TDD assumed `LintProvider` and `FormattingProvider` as type names,
//! but the actual API uses `CriticAnalyzer` and `PerlTidyFormatter`. Tests updated to match.

use perl_lsp_rs_core::tooling::*;

#[test]
fn tooling_module_exposes_performance_submodule() {
    // Verify that performance submodule is accessible via tooling post-absorption
    let _: Option<performance::AstCache> = None;
}

#[test]
fn tooling_module_exposes_perl_critic_submodule() {
    // Verify that perl_critic submodule is accessible via tooling post-absorption.
    // NOTE(G3-API-fix): Actual type is CriticAnalyzer, not LintProvider.
    let _: Option<perl_critic::CriticAnalyzer> = None;
}

#[test]
fn tooling_module_exposes_perltidy_submodule() {
    // Verify that perltidy submodule is accessible via tooling post-absorption.
    // NOTE(G3-API-fix): Actual type is PerlTidyFormatter, not FormattingProvider.
    let _: Option<perltidy::PerlTidyFormatter> = None;
}

#[test]
fn tooling_module_exposes_subprocess_runtime_trait() {
    // Verify that SubprocessRuntime trait is accessible via tooling post-absorption
    let _: Option<Box<dyn SubprocessRuntime>> = None;
}

#[test]
fn tooling_module_exposes_subprocess_error() {
    // Verify that SubprocessError is accessible via tooling post-absorption
    let _: Option<SubprocessError> = None;
}

#[test]
fn tooling_module_exposes_subprocess_output() {
    // Verify that SubprocessOutput is accessible via tooling post-absorption
    let _: Option<SubprocessOutput> = None;
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn tooling_module_exposes_os_subprocess_runtime() {
    // Verify that OsSubprocessRuntime is accessible (non-WASM only)
    let _: Option<OsSubprocessRuntime> = None;
}

#[test]
fn tooling_module_exposes_mock_submodule() {
    // Verify that mock submodule is accessible via tooling post-absorption
    let _: Option<mock::MockSubprocessRuntime> = None;
}
