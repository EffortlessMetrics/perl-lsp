//! Diagnostic codes, severity levels, tags, and categories.
//!
//! This module contains the canonical definitions of all diagnostic codes used
//! throughout the Perl LSP ecosystem. These codes are stable and can be
//! referenced in documentation and error messages.
//!
//! # Code Ranges
//!
//! | Range       | Category                  |
//! |-------------|---------------------------|
//! | PL001-PL099 | Parser diagnostics        |
//! | PL100-PL199 | Strict/warnings           |
//! | PL200-PL299 | Package/module            |
//! | PL300-PL399 | Subroutine                |
//! | PL400-PL499 | Best practices            |
//! | PL500-PL599 | Deprecated syntax         |
//! | PL600-PL699 | Security                  |
//! | PL700-PL799 | Import                    |
//! | PL800-PL899 | Heredoc anti-patterns     |
//! | PL900-PL999 | Version compatibility     |
//! | PC001-PC005 | Perl::Critic violations   |

use std::fmt;

mod category;
mod metadata;
mod severity;
mod tag;

pub use category::DiagnosticCategory;
pub use severity::DiagnosticSeverity;
pub use tag::DiagnosticTag;

/// Stable diagnostic codes for Perl LSP.
///
/// Each code has a fixed string representation and associated metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticCode {
    // Parser diagnostics (PL001-PL099)
    /// General parse error
    #[default]
    ParseError,
    /// Syntax error
    SyntaxError,
    /// Unexpected end-of-file
    UnexpectedEof,

    // Strict/warnings (PL100-PL199)
    /// Missing 'use strict' pragma
    MissingStrict,
    /// Missing 'use warnings' pragma
    MissingWarnings,
    /// Unused variable
    UnusedVariable,
    /// Undefined variable
    UndefinedVariable,
    /// Variable shadowing an outer declaration
    VariableShadowing,
    /// Variable redeclared in the same scope
    VariableRedeclaration,
    /// Duplicate parameter in a subroutine signature
    DuplicateParameter,
    /// Subroutine parameter shadows a global variable
    ParameterShadowsGlobal,
    /// Subroutine parameter is declared but never used
    UnusedParameter,
    /// Bareword used where a quoted string is expected (under strict)
    UnquotedBareword,
    /// Variable used before being initialized
    UninitializedVariable,
    /// Pragma name appears to be misspelled
    MisspelledPragma,
    /// Capture variable ($1, $2, etc.) used without a preceding regex match in scope
    CaptureVarWithoutRegexMatch,

    // Package/module (PL200-PL299)
    /// Missing package declaration
    MissingPackageDeclaration,
    /// Duplicate package declaration
    DuplicatePackage,

    // Subroutine (PL300-PL399)
    /// Duplicate subroutine definition
    DuplicateSubroutine,
    /// Missing explicit return statement
    MissingReturn,
    /// Invalid character(s) in a subroutine prototype
    ///
    /// Perl only allows `$`, `@`, `%`, `&`, `*`, `\`, `;`, `+`, `_`, and
    /// spaces in old-style prototypes.  Any other character triggers Perl's
    /// "Illegal character in prototype" warning.
    InvalidPrototype,
    /// Same-file Moo/Moose roles provide conflicting methods
    RoleConflict,
    /// Exported subroutine lacks POD documentation
    MissingPodCoverage,

    // Best practices (PL400-PL499)
    /// Bareword filehandle usage
    BarewordFilehandle,
    /// Two-argument open() call
    TwoArgOpen,
    /// Implicit return value
    ImplicitReturn,
    /// Assignment used where a comparison was likely intended
    AssignmentInCondition,
    /// Numeric comparison against a potentially undefined value
    NumericComparisonWithUndef,
    /// printf/sprintf format specifier count does not match argument count
    PrintfFormatMismatch,
    /// Statement that cannot be reached due to preceding unconditional exit
    UnreachableCode,
    /// `$@` / `$EVAL_ERROR` reads that are not paired with a nearby `eval`/`try`
    EvalErrorFlow,
    /// Duplicate key in a hash literal or hash reference constructor
    DuplicateHashKey,
    /// `goto LABEL` references a label that does not exist in this file
    GotoUndefinedLabel,
    /// `next`/`last`/`redo LABEL` references a label that does not exist in this file
    LoopControlUndefinedLabel,

    // Pragma pitfalls / deprecated syntax (PL500-PL599)
    /// Use of deprecated defined(@array) / defined(%hash)
    DeprecatedDefined,
    /// Use of deprecated $[ array base variable
    DeprecatedArrayBase,
    /// `use strict` appears only inside a phase block and does not affect file scope
    PhaseScopedStrictPragma,
    /// `use warnings` appears only inside a phase block and does not affect file scope
    PhaseScopedWarningsPragma,

    // Security (PL600-PL699)
    /// String eval is a security risk
    SecurityStringEval,
    /// Backtick/qx command execution detected
    SecurityBacktickExec,
    /// Global assignment to `$SIG{__DIE__}` / `$SIG{__WARN__}`
    SecuritySignalHandler,
    /// `system()` call executes shell commands
    SecuritySystemCall,
    /// `exec()` call replaces the current process with a shell command
    SecurityExecCall,
    /// Pipe-open `open(FH, "|-", ...)` / `open(FH, "-|", ...)` executes shell commands
    SecurityPipeOpen,
    /// `readpipe()` function call executes shell commands (equivalent to qx//)
    SecurityReadpipe,

    // Import (PL700-PL799)
    /// Module appears to be unused
    UnusedImport,
    /// Module not found in workspace or configured include paths
    ModuleNotFound,

    // Heredoc anti-patterns (PL800-PL899)
    /// Heredoc used inside a format block
    HeredocInFormat,
    /// Heredoc used inside a BEGIN block
    HeredocInBegin,
    /// Heredoc delimiter is dynamic (variable interpolation)
    HeredocDynamicDelimiter,
    /// Heredoc used inside a source filter
    HeredocInSourceFilter,
    /// Heredoc used inside a regex code block
    HeredocInRegexCode,
    /// Heredoc used inside string eval
    HeredocInEval,
    /// Heredoc used with a tied filehandle
    HeredocTiedHandle,

    // Version compatibility (PL900-PL999)
    /// Use of a Perl feature not available in the declared version
    VersionIncompatFeature,

    // Perl::Critic violations (PC001-PC005)
    /// Perl::Critic brutal (severity 1) violation
    CriticSeverity1,
    /// Perl::Critic cruel (severity 2) violation
    CriticSeverity2,
    /// Perl::Critic harsh (severity 3) violation
    CriticSeverity3,
    /// Perl::Critic stern (severity 4) violation
    CriticSeverity4,
    /// Perl::Critic gentle (severity 5) violation
    CriticSeverity5,
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
