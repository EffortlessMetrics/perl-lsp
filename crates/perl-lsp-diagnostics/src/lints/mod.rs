//! Lint checks for Perl code analysis
//!
//! This module provides various linting checks for detecting deprecated syntax,
//! strict/warnings issues, common mistakes, and security anti-patterns in Perl code.
//!
//! # Architecture
//!
//! Lints are organized into focused submodules:
//!
//! - **deprecated**: Deprecated syntax warnings (e.g., `defined(@array)`)
//! - **strict_warnings**: Missing `use strict` / `use warnings` advisories and
//!   misspelled pragma detection
//! - **common_mistakes**: Frequent programming errors (assignment in conditions, etc.)
//! - **security**: Security anti-patterns (two-arg open, string eval, backtick execution, global signal handlers)
//! - **eval_error_flow**: Conservative `$@` / `$EVAL_ERROR` flow checks after `eval` / `try`
//! - **goto_label**: Conservative `goto LABEL` validation when no matching label exists in-file
//!
//! # Diagnostic Code Reference
//!
//! Every diagnostic carries a `code` field that IDEs can use for quick-fix
//! integration, filtering, and documentation lookup.
//!
//! ## Parse errors (`diagnostics.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `parse-error` | Error | Generic parse error from the parser |
//!
//! ## Scope issues (`scope.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `undeclared-variable` | Error | Variable used without declaration |
//! | `variable-redeclaration` | Error | Duplicate `my` in same scope |
//! | `duplicate-parameter` | Error | Same parameter name twice |
//! | `unquoted-bareword` | Error | Bareword not allowed under strict |
//! | `variable-shadowing` | Warning | Inner variable hides outer |
//! | `unused-variable` | Warning | Declared but never read |
//! | `unused-parameter` | Warning | Subroutine parameter never used |
//! | `parameter-shadows-global` | Warning | Parameter hides package var |
//! | `uninitialized-variable` | Warning | Used before assignment |
//!
//! ## Deprecated syntax (`deprecated.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `deprecated-defined` | Warning | `defined(@array)` or `defined(%hash)` |
//! | `deprecated-array-base` | Warning | Use of `$[` variable |
//!
//! ## Strict / warnings (`strict_warnings.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `missing-strict` | Information | `use strict` not found |
//! | `missing-warnings` | Information | `use warnings` not found |
//! | `misspelled-pragma` | Warning | Pragma name appears misspelled |
//!
//! ## Common mistakes (`common_mistakes.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `assignment-in-condition` | Warning | `=` used where `==` likely intended |
//! | `numeric-undef` | Warning | `==`/`!=` with potentially undef value |
//!
//! ## Eval / try flow (`eval_error_flow.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `PL407` | Warning | `$@` / `$EVAL_ERROR` read without a nearby `eval` / `try` |
//!
//! ## Security (`security.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `security-two-arg-open` | Warning | `open(FH, ">file")` -- use 3-arg open |
//! | `security-string-eval` | Warning | `eval "$string"` is a security risk |
//! | `security-backtick-exec` | Information | Backtick/qx command execution detected |
//! | `security-signal-handler` | Warning | Global `$SIG{__DIE__}` / `$SIG{__WARN__}` assignment |
//!
//! ## Package / subroutine (`package_subroutine.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `PL200` | Warning | File has no package declaration |
//! | `PL201` | Warning | Package name declared more than once |
//! | `PL300` | Warning | Subroutine name defined more than once |
//! | `PL303` | Warning | Same-file Moo/Moose roles provide conflicting methods |
//!
//! ## Dead code (`dead_code.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `dead-code-subroutine` | Hint | Subroutine with no callers |
//! | `dead-code-variable` | Hint | Package variable with no references |
//! | `dead-code-constant` | Hint | Constant with no references |
//! | `dead-code-package` | Hint | Package with no references |
//!
//! ## Unreachable code (`unreachable_code.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `PL406` | Hint | Statement cannot be reached due to preceding unconditional exit |
//!
//! ## Duplicate hash keys (`duplicate_hash_keys.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `PL408` | Warning | Hash key appears more than once in the same literal |
//!
//! ## Goto labels (`goto_label.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `PL409` | Warning | `goto LABEL` references a label that is not defined in the file |
//!
//! # Severity Levels
//!
//! Each lint produces diagnostics with appropriate severity:
//!
//! - **Error**: Issues that will cause runtime failures
//! - **Warning**: Potential bugs or deprecated patterns
//! - **Information**: Best practice suggestions
//! - **Hint**: Style recommendations
//!
//! # Integration
//!
//! Lints integrate with the diagnostics pipeline and provide:
//!
//! - Diagnostic codes for IDE quick-fix integration
//! - Related information with suggestions and explanations
//! - Diagnostic tags (Deprecated, Unnecessary) for IDE rendering

pub mod common_mistakes;
pub mod deprecated;
/// Duplicate hash key detection (PL408)
pub mod duplicate_hash_keys;
/// Conservative `$@` / `$EVAL_ERROR` flow checks after `eval` / `try`
pub mod eval_error_flow;
/// FFI::CheckLib native-library validation hints
pub mod ffi_checklib;
/// Conservative `goto LABEL` validation
pub mod goto_label;
/// Missing module detection (PL701)
pub mod missing_module;
/// Package and subroutine diagnostics (PL200, PL201, PL300, PL303)
pub mod package_subroutine;
/// printf/sprintf format specifier arity validation (PL405)
pub mod printf_format;
/// Same-file Moo/Moose role conflict detection (PL303)
pub mod role_conflicts;
pub mod security;
pub mod strict_warnings;
/// Unreachable code detection (PL406)
pub mod unreachable_code;
/// Unused import detection
pub mod unused_imports;
/// Perl version compatibility warnings (PL900)
pub mod version_compat;
