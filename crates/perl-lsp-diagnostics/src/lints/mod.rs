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
//! - **security**: Security anti-patterns (two-arg open, string eval, backtick execution)
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
//! ## Security (`security.rs`)
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `security-two-arg-open` | Warning | `open(FH, ">file")` -- use 3-arg open |
//! | `security-string-eval` | Warning | `eval "$string"` is a security risk |
//! | `security-backtick-exec` | Information | Backtick/qx command execution detected |
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
pub mod security;
pub mod strict_warnings;
