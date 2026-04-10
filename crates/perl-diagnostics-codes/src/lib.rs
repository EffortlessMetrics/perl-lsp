//! Stable diagnostic codes and severity levels for Perl LSP.
//!
//! This crate provides the canonical definitions of diagnostic codes used
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
//! | PC001-PC005 | Perl::Critic violations   |
//!
//! # Example
//!
//! ```
//! use perl_diagnostics_codes::{DiagnosticCode, DiagnosticSeverity};
//!
//! let code = DiagnosticCode::ParseError;
//! assert_eq!(code.as_str(), "PL001");
//! assert_eq!(code.severity(), DiagnosticSeverity::Error);
//! ```

use std::fmt;

/// Severity level of a diagnostic.
///
/// Maps to LSP DiagnosticSeverity values (1=Error, 2=Warning, 3=Info, 4=Hint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum DiagnosticSeverity {
    /// Critical error that prevents parsing/execution.
    Error = 1,
    /// Non-critical issue that should be addressed.
    Warning = 2,
    /// Informational message.
    Information = 3,
    /// Subtle suggestion or hint.
    Hint = 4,
}

impl DiagnosticSeverity {
    /// Get the LSP numeric value for this severity.
    pub fn to_lsp_value(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticSeverity::Error => write!(f, "error"),
            DiagnosticSeverity::Warning => write!(f, "warning"),
            DiagnosticSeverity::Information => write!(f, "info"),
            DiagnosticSeverity::Hint => write!(f, "hint"),
        }
    }
}

/// Diagnostic tags for additional classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticTag {
    /// Code that can be safely removed (unused variables, imports).
    Unnecessary,
    /// Code using deprecated features.
    Deprecated,
}

impl DiagnosticTag {
    /// Get the LSP numeric value for this tag.
    pub fn to_lsp_value(self) -> u8 {
        match self {
            DiagnosticTag::Unnecessary => 1,
            DiagnosticTag::Deprecated => 2,
        }
    }
}

/// Stable diagnostic codes for Perl LSP.
///
/// Each code has a fixed string representation and associated metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticCode {
    // Parser diagnostics (PL001-PL099)
    /// General parse error
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
    /// Same-file Moo/Moose roles provide conflicting methods
    RoleConflict,
    /// Invalid character(s) in a subroutine prototype
    ///
    /// Perl only allows `$`, `@`, `%`, `&`, `*`, `\`, `;`, `+`, `_`, and
    /// spaces in old-style prototypes.  Any other character triggers Perl's
    /// "Illegal character in prototype" warning.
    InvalidPrototype,

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

    // Deprecated syntax (PL500-PL599)
    /// Use of deprecated defined(@array) / defined(%hash)
    DeprecatedDefined,
    /// Use of deprecated $[ array base variable
    DeprecatedArrayBase,

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

impl DiagnosticCode {
    /// Get the string representation of this code.
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticCode::ParseError => "PL001",
            DiagnosticCode::SyntaxError => "PL002",
            DiagnosticCode::UnexpectedEof => "PL003",
            DiagnosticCode::MissingStrict => "PL100",
            DiagnosticCode::MissingWarnings => "PL101",
            DiagnosticCode::UnusedVariable => "PL102",
            DiagnosticCode::UndefinedVariable => "PL103",
            DiagnosticCode::VariableShadowing => "PL104",
            DiagnosticCode::VariableRedeclaration => "PL105",
            DiagnosticCode::DuplicateParameter => "PL106",
            DiagnosticCode::ParameterShadowsGlobal => "PL107",
            DiagnosticCode::UnusedParameter => "PL108",
            DiagnosticCode::UnquotedBareword => "PL109",
            DiagnosticCode::UninitializedVariable => "PL110",
            DiagnosticCode::MisspelledPragma => "PL111",
            DiagnosticCode::CaptureVarWithoutRegexMatch => "PL112",
            DiagnosticCode::MissingPackageDeclaration => "PL200",
            DiagnosticCode::DuplicatePackage => "PL201",
            DiagnosticCode::DuplicateSubroutine => "PL300",
            DiagnosticCode::MissingReturn => "PL301",
            DiagnosticCode::InvalidPrototype => "PL302",
            DiagnosticCode::RoleConflict => "PL303",
            DiagnosticCode::BarewordFilehandle => "PL400",
            DiagnosticCode::TwoArgOpen => "PL401",
            DiagnosticCode::ImplicitReturn => "PL402",
            DiagnosticCode::AssignmentInCondition => "PL403",
            DiagnosticCode::NumericComparisonWithUndef => "PL404",
            DiagnosticCode::PrintfFormatMismatch => "PL405",
            DiagnosticCode::UnreachableCode => "PL406",
            DiagnosticCode::EvalErrorFlow => "PL407",
            DiagnosticCode::DuplicateHashKey => "PL408",
            DiagnosticCode::GotoUndefinedLabel => "PL409",
            DiagnosticCode::DeprecatedDefined => "PL500",
            DiagnosticCode::DeprecatedArrayBase => "PL501",
            DiagnosticCode::SecurityStringEval => "PL600",
            DiagnosticCode::SecurityBacktickExec => "PL601",
            DiagnosticCode::SecuritySignalHandler => "PL602",
            DiagnosticCode::SecuritySystemCall => "PL603",
            DiagnosticCode::SecurityExecCall => "PL604",
            DiagnosticCode::SecurityPipeOpen => "PL605",
            DiagnosticCode::SecurityReadpipe => "PL606",
            DiagnosticCode::UnusedImport => "PL700",
            DiagnosticCode::ModuleNotFound => "PL701",
            DiagnosticCode::HeredocInFormat => "PL800",
            DiagnosticCode::HeredocInBegin => "PL801",
            DiagnosticCode::HeredocDynamicDelimiter => "PL802",
            DiagnosticCode::HeredocInSourceFilter => "PL803",
            DiagnosticCode::HeredocInRegexCode => "PL804",
            DiagnosticCode::HeredocInEval => "PL805",
            DiagnosticCode::HeredocTiedHandle => "PL806",
            DiagnosticCode::VersionIncompatFeature => "PL900",
            DiagnosticCode::CriticSeverity1 => "PC001",
            DiagnosticCode::CriticSeverity2 => "PC002",
            DiagnosticCode::CriticSeverity3 => "PC003",
            DiagnosticCode::CriticSeverity4 => "PC004",
            DiagnosticCode::CriticSeverity5 => "PC005",
        }
    }

    /// Get the documentation URL for this code, if available.
    pub fn documentation_url(&self) -> Option<&'static str> {
        let code = self.as_str();
        // Perl::Critic codes don't have centralized documentation
        if code.starts_with("PC") {
            return None;
        }
        // Build URL from stable code string for all PL codes
        Some(match code {
            "PL001" => "https://docs.perl-lsp.org/errors/PL001",
            "PL002" => "https://docs.perl-lsp.org/errors/PL002",
            "PL003" => "https://docs.perl-lsp.org/errors/PL003",
            "PL100" => "https://docs.perl-lsp.org/errors/PL100",
            "PL101" => "https://docs.perl-lsp.org/errors/PL101",
            "PL102" => "https://docs.perl-lsp.org/errors/PL102",
            "PL103" => "https://docs.perl-lsp.org/errors/PL103",
            "PL104" => "https://docs.perl-lsp.org/errors/PL104",
            "PL105" => "https://docs.perl-lsp.org/errors/PL105",
            "PL106" => "https://docs.perl-lsp.org/errors/PL106",
            "PL107" => "https://docs.perl-lsp.org/errors/PL107",
            "PL108" => "https://docs.perl-lsp.org/errors/PL108",
            "PL109" => "https://docs.perl-lsp.org/errors/PL109",
            "PL110" => "https://docs.perl-lsp.org/errors/PL110",
            "PL111" => "https://docs.perl-lsp.org/errors/PL111",
            "PL112" => "https://docs.perl-lsp.org/errors/PL112",
            "PL200" => "https://docs.perl-lsp.org/errors/PL200",
            "PL201" => "https://docs.perl-lsp.org/errors/PL201",
            "PL300" => "https://docs.perl-lsp.org/errors/PL300",
            "PL301" => "https://docs.perl-lsp.org/errors/PL301",
            "PL302" => "https://docs.perl-lsp.org/errors/PL302",
            "PL303" => "https://docs.perl-lsp.org/errors/PL303",
            "PL400" => "https://docs.perl-lsp.org/errors/PL400",
            "PL401" => "https://docs.perl-lsp.org/errors/PL401",
            "PL402" => "https://docs.perl-lsp.org/errors/PL402",
            "PL403" => "https://docs.perl-lsp.org/errors/PL403",
            "PL404" => "https://docs.perl-lsp.org/errors/PL404",
            "PL405" => "https://docs.perl-lsp.org/errors/PL405",
            "PL406" => "https://docs.perl-lsp.org/errors/PL406",
            "PL407" => "https://docs.perl-lsp.org/errors/PL407",
            "PL408" => "https://docs.perl-lsp.org/errors/PL408",
            "PL409" => "https://docs.perl-lsp.org/errors/PL409",
            "PL500" => "https://docs.perl-lsp.org/errors/PL500",
            "PL501" => "https://docs.perl-lsp.org/errors/PL501",
            "PL600" => "https://docs.perl-lsp.org/errors/PL600",
            "PL601" => "https://docs.perl-lsp.org/errors/PL601",
            "PL602" => "https://docs.perl-lsp.org/errors/PL602",
            "PL603" => "https://docs.perl-lsp.org/errors/PL603",
            "PL604" => "https://docs.perl-lsp.org/errors/PL604",
            "PL605" => "https://docs.perl-lsp.org/errors/PL605",
            "PL606" => "https://docs.perl-lsp.org/errors/PL606",
            "PL700" => "https://docs.perl-lsp.org/errors/PL700",
            "PL701" => "https://docs.perl-lsp.org/errors/PL701",
            "PL800" => "https://docs.perl-lsp.org/errors/PL800",
            "PL801" => "https://docs.perl-lsp.org/errors/PL801",
            "PL802" => "https://docs.perl-lsp.org/errors/PL802",
            "PL803" => "https://docs.perl-lsp.org/errors/PL803",
            "PL804" => "https://docs.perl-lsp.org/errors/PL804",
            "PL805" => "https://docs.perl-lsp.org/errors/PL805",
            "PL806" => "https://docs.perl-lsp.org/errors/PL806",
            "PL900" => "https://docs.perl-lsp.org/errors/PL900",
            _ => return None,
        })
    }

    /// Get the default severity for this diagnostic code.
    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            // Errors
            DiagnosticCode::ParseError
            | DiagnosticCode::SyntaxError
            | DiagnosticCode::UnexpectedEof
            | DiagnosticCode::UndefinedVariable
            | DiagnosticCode::VariableRedeclaration
            | DiagnosticCode::DuplicateParameter
            | DiagnosticCode::UnquotedBareword => DiagnosticSeverity::Error,

            // Warnings
            DiagnosticCode::MissingStrict
            | DiagnosticCode::MissingWarnings
            | DiagnosticCode::UnusedVariable
            | DiagnosticCode::VariableShadowing
            | DiagnosticCode::ParameterShadowsGlobal
            | DiagnosticCode::UnusedParameter
            | DiagnosticCode::UninitializedVariable
            | DiagnosticCode::MisspelledPragma
            | DiagnosticCode::MissingPackageDeclaration
            | DiagnosticCode::DuplicatePackage
            | DiagnosticCode::DuplicateSubroutine
            | DiagnosticCode::MissingReturn
            | DiagnosticCode::InvalidPrototype
            | DiagnosticCode::RoleConflict
            | DiagnosticCode::BarewordFilehandle
            | DiagnosticCode::TwoArgOpen
            | DiagnosticCode::ImplicitReturn
            | DiagnosticCode::AssignmentInCondition
            | DiagnosticCode::NumericComparisonWithUndef
            | DiagnosticCode::PrintfFormatMismatch
            | DiagnosticCode::DuplicateHashKey
            | DiagnosticCode::GotoUndefinedLabel
            | DiagnosticCode::DeprecatedDefined
            | DiagnosticCode::DeprecatedArrayBase
            | DiagnosticCode::SecurityStringEval
            | DiagnosticCode::SecurityBacktickExec
            | DiagnosticCode::SecuritySignalHandler
            | DiagnosticCode::SecuritySystemCall
            | DiagnosticCode::SecurityExecCall
            | DiagnosticCode::SecurityPipeOpen
            | DiagnosticCode::SecurityReadpipe
            | DiagnosticCode::ModuleNotFound
            | DiagnosticCode::VersionIncompatFeature
            | DiagnosticCode::EvalErrorFlow
            | DiagnosticCode::CriticSeverity1
            | DiagnosticCode::CriticSeverity2 => DiagnosticSeverity::Warning,

            // Information
            DiagnosticCode::CaptureVarWithoutRegexMatch
            | DiagnosticCode::HeredocInFormat
            | DiagnosticCode::HeredocInBegin
            | DiagnosticCode::HeredocDynamicDelimiter
            | DiagnosticCode::HeredocInSourceFilter
            | DiagnosticCode::HeredocInRegexCode
            | DiagnosticCode::HeredocInEval
            | DiagnosticCode::HeredocTiedHandle => DiagnosticSeverity::Information,

            // Hints
            DiagnosticCode::UnusedImport
            | DiagnosticCode::UnreachableCode
            | DiagnosticCode::CriticSeverity3
            | DiagnosticCode::CriticSeverity4
            | DiagnosticCode::CriticSeverity5 => DiagnosticSeverity::Hint,
        }
    }

    /// Get any diagnostic tags associated with this code.
    pub fn tags(&self) -> &'static [DiagnosticTag] {
        match self {
            DiagnosticCode::UnusedVariable
            | DiagnosticCode::UnusedParameter
            | DiagnosticCode::UnusedImport
            | DiagnosticCode::UnreachableCode => &[DiagnosticTag::Unnecessary],
            DiagnosticCode::DeprecatedDefined | DiagnosticCode::DeprecatedArrayBase => {
                &[DiagnosticTag::Deprecated]
            }
            _ => &[],
        }
    }

    /// Return a human-readable context hint for this diagnostic code.
    ///
    /// Hints are short, actionable explanations that help users understand
    /// what the diagnostic means and how to resolve it.  Perl::Critic codes
    /// return `None` because their per-policy descriptions already serve this
    /// purpose.
    pub fn context_hint(&self) -> Option<&'static str> {
        match self {
            DiagnosticCode::ParseError => Some(
                "The parser could not understand this code. \
                Check for missing semicolons, unmatched brackets, or incorrect syntax.",
            ),
            DiagnosticCode::SyntaxError => Some(
                "Perl syntax error. Check for typos, missing operators, \
                or unbalanced parentheses near this location.",
            ),
            DiagnosticCode::UnexpectedEof => Some(
                "The file ended unexpectedly. Check for unclosed blocks `{}`, \
                heredocs, or multi-line strings.",
            ),
            DiagnosticCode::MissingStrict => Some(
                "Add `use strict;` at the top of your file. \
                Strict mode catches common variable mistakes at compile time.",
            ),
            DiagnosticCode::MissingWarnings => Some(
                "Add `use warnings;` at the top of your file. \
                Warnings highlight many common programming mistakes.",
            ),
            DiagnosticCode::UnusedVariable => Some(
                "This variable is declared but never used. \
                Remove it, or prefix with `_` (e.g., `$_unused`) to suppress.",
            ),
            DiagnosticCode::UndefinedVariable => Some(
                "This variable was not declared with `my`, `our`, or `local`. \
                Add `use strict;` and declare all variables before use.",
            ),
            DiagnosticCode::MissingPackageDeclaration => Some(
                "This file has no `package` declaration. \
                Add `package MyModule;` at the top for module files.",
            ),
            DiagnosticCode::DuplicatePackage => Some(
                "This package name is declared more than once in the same file. \
                Each package should appear once, or split into separate files.",
            ),
            DiagnosticCode::DuplicateSubroutine => Some(
                "A subroutine with this name is defined more than once. \
                The later definition silently replaces the earlier one.",
            ),
            DiagnosticCode::MissingReturn => Some(
                "This subroutine has no explicit `return` statement. \
                Add `return $value;` to make the return value clear.",
            ),
            DiagnosticCode::RoleConflict => Some(
                "Two or more consumed Moo/Moose roles provide the same method. \
                Define the method in the class or remove one of the conflicting roles.",
            ),
            DiagnosticCode::InvalidPrototype => Some(
                "The prototype contains a character that Perl does not recognise. \
                Valid prototype characters are: $, @, %, &, *, \\, ;, +, _ and spaces. \
                See perlsub for the full prototype syntax.",
            ),
            DiagnosticCode::BarewordFilehandle => Some(
                "Bareword filehandles (e.g., `open FH, ...`) are global and unsafe. \
                Use a lexical filehandle instead: `open my $fh, '<', $file or die $!;`",
            ),
            DiagnosticCode::TwoArgOpen => Some(
                "Two-argument `open()` is vulnerable to injection. \
                Use three-argument form: `open my $fh, '<', $filename or die $!;`",
            ),
            DiagnosticCode::ImplicitReturn => Some(
                "The return value of this expression is used implicitly. \
                Make it explicit with `return` or assign it to a variable.",
            ),
            DiagnosticCode::AssignmentInCondition => Some(
                "This looks like an assignment `=` inside a condition where \
                a comparison `==` or `eq` was likely intended.",
            ),
            DiagnosticCode::NumericComparisonWithUndef => Some(
                "Comparing a potentially undefined value with a numeric operator \
                produces a warning at runtime. Check for definedness first with `defined()`.",
            ),
            DiagnosticCode::EvalErrorFlow => Some(
                "Read `$@` or `$EVAL_ERROR` immediately after an `eval` or `try` \
                block; intervening statements can clobber the exception state.",
            ),
            DiagnosticCode::UnreachableCode => Some(
                "This statement cannot be executed because a preceding statement \
                unconditionally exits (return, die, exit, croak). Remove or relocate it.",
            ),
            DiagnosticCode::DuplicateHashKey => Some(
                "This hash key appears more than once in the same literal. \
                Only the last value will be used; the earlier assignment is silently discarded.",
            ),
            DiagnosticCode::GotoUndefinedLabel => Some(
                "This goto target label is not defined in the current file. \
                Define the label or use a dynamic goto form only when the target is known at runtime.",
            ),
            DiagnosticCode::PrintfFormatMismatch => Some(
                "The number of format specifiers does not match the number of arguments. \
                Each %s/%d/%f/etc. consumes one argument (except %% which consumes none).",
            ),
            DiagnosticCode::VariableShadowing => Some(
                "This variable shadows an outer variable with the same name. \
                Rename it to avoid confusion, or use the outer variable directly.",
            ),
            DiagnosticCode::VariableRedeclaration => Some(
                "This variable is declared again in the same scope. \
                Remove the duplicate `my` declaration and reuse the existing variable.",
            ),
            DiagnosticCode::DuplicateParameter => Some(
                "This subroutine signature has a duplicate parameter name. \
                Each parameter must have a unique name.",
            ),
            DiagnosticCode::ParameterShadowsGlobal => Some(
                "This subroutine parameter shadows a global (`our`) variable. \
                Rename the parameter to avoid confusion with the global.",
            ),
            DiagnosticCode::UnusedParameter => Some(
                "This subroutine parameter is declared but never used. \
                Remove it or prefix with `_` (e.g., `$_unused`) to suppress.",
            ),
            DiagnosticCode::UnquotedBareword => Some(
                "This bareword is used where a quoted string is expected. \
                Under `use strict`, barewords are not allowed. Quote it: `'word'`.",
            ),
            DiagnosticCode::UninitializedVariable => Some(
                "This variable is used before being assigned a value. \
                Initialize it before use to avoid `Use of uninitialized value` warnings.",
            ),
            DiagnosticCode::MisspelledPragma => Some(
                "This pragma name appears to be misspelled. \
                Check the spelling and ensure the module is installed.",
            ),
            DiagnosticCode::CaptureVarWithoutRegexMatch => Some(
                "Capture variables ($1, $2, etc.) are only meaningful after a successful regex match. \
                Perform a regex match with =~ /.../ before using $1 or $2.",
            ),
            DiagnosticCode::DeprecatedDefined => Some(
                "`defined(@array)` and `defined(%hash)` are deprecated since Perl 5.6. \
                Use `@array` or `%hash` directly in boolean context instead.",
            ),
            DiagnosticCode::DeprecatedArrayBase => Some(
                "The `$[` variable is deprecated. Array indices always start at 0 \
                in modern Perl. Remove any assignment to `$[`.",
            ),
            DiagnosticCode::SecurityStringEval => Some(
                "String `eval` executes arbitrary code and is a security risk. \
                Use block eval `eval { ... }` or safer alternatives.",
            ),
            DiagnosticCode::SecurityBacktickExec => Some(
                "Backticks/`qx()` execute shell commands and can be exploited. \
                Use `system()` with a list form or IPC::Run for safer execution.",
            ),
            DiagnosticCode::SecuritySignalHandler => Some(
                "Assigning to $SIG{__DIE__} or $SIG{__WARN__} globally changes exception \
                and warning handling for the whole process. Use `local` to scope the handler.",
            ),
            DiagnosticCode::SecuritySystemCall => Some(
                "`system()` executes a shell command. If the arguments include user input, \
                use the list form `system($cmd, @args)` to avoid shell injection.",
            ),
            DiagnosticCode::SecurityExecCall => Some(
                "`exec()` replaces the current process with a shell command. If arguments \
                include user input, use the list form `exec($cmd, @args)` to avoid shell injection.",
            ),
            DiagnosticCode::SecurityPipeOpen => Some(
                "Pipe-open executes a shell command. Pass a list to `open` for safe argument \
                handling: `open(my $fh, '-|', $cmd, @args)` instead of `open(my $fh, \"|$cmd\")`.",
            ),
            DiagnosticCode::SecurityReadpipe => Some(
                "`readpipe()` executes a shell command (equivalent to backticks/qx//). \
                Use `open(my $fh, '-|', $cmd, @args)` or IPC::Run for safer command execution.",
            ),
            DiagnosticCode::UnusedImport => Some(
                "This module is imported but none of its exports appear to be used. \
                Remove the `use` statement to reduce unnecessary dependencies.",
            ),
            DiagnosticCode::ModuleNotFound => Some(
                "This module was not found in the workspace or configured include paths. \
                Install it with cpanm or add it to cpanfile.",
            ),
            DiagnosticCode::HeredocInFormat => Some(
                "Heredocs inside `format` blocks can cause subtle parsing issues. \
                Extract the heredoc content into a variable before the format.",
            ),
            DiagnosticCode::HeredocInBegin => Some(
                "Heredocs inside `BEGIN` blocks may behave unexpectedly due to \
                compile-time execution. Move the heredoc outside the BEGIN block.",
            ),
            DiagnosticCode::HeredocDynamicDelimiter => Some(
                "The heredoc delimiter contains a variable, making it dynamic. \
                Use a static delimiter string to avoid surprising behavior.",
            ),
            DiagnosticCode::HeredocInSourceFilter => Some(
                "Heredocs inside source-filtered code may be mangled by the filter. \
                Avoid combining heredocs with source filters.",
            ),
            DiagnosticCode::HeredocInRegexCode => Some(
                "Heredocs inside regex code blocks `(?{ ... })` can cause parsing failures. \
                Move the heredoc content outside the regex.",
            ),
            DiagnosticCode::HeredocInEval => Some(
                "Heredocs inside string `eval` are fragile and error-prone. \
                Use a variable or block eval instead.",
            ),
            DiagnosticCode::HeredocTiedHandle => Some(
                "Heredocs written to tied filehandles may not behave as expected. \
                The tie interface may not handle multi-line heredoc output correctly.",
            ),
            DiagnosticCode::VersionIncompatFeature => Some(
                "This Perl feature requires a newer Perl version than declared. \
                Update 'use vN.NN' or 'use feature' to enable it.",
            ),
            // Perl::Critic codes carry per-policy descriptions; no generic hint needed.
            DiagnosticCode::CriticSeverity1
            | DiagnosticCode::CriticSeverity2
            | DiagnosticCode::CriticSeverity3
            | DiagnosticCode::CriticSeverity4
            | DiagnosticCode::CriticSeverity5 => None,
        }
    }

    /// Try to infer a diagnostic code from a message.
    pub fn from_message(msg: &str) -> Option<DiagnosticCode> {
        let msg_lower = msg.to_lowercase();
        if msg_lower.contains("use strict") {
            Some(DiagnosticCode::MissingStrict)
        } else if msg_lower.contains("use warnings") {
            Some(DiagnosticCode::MissingWarnings)
        } else if msg_lower.contains("unused variable") || msg_lower.contains("never used") {
            Some(DiagnosticCode::UnusedVariable)
        } else if msg_lower.contains("undefined") || msg_lower.contains("not declared") {
            Some(DiagnosticCode::UndefinedVariable)
        } else if msg_lower.contains("bareword filehandle") {
            Some(DiagnosticCode::BarewordFilehandle)
        } else if msg_lower.contains("two-argument") || msg_lower.contains("2-arg") {
            Some(DiagnosticCode::TwoArgOpen)
        } else if msg_lower.contains("invalid prototype character")
            || msg_lower.contains("illegal character in prototype")
        {
            Some(DiagnosticCode::InvalidPrototype)
        } else if msg_lower.contains("parse error") || msg_lower.contains("syntax error") {
            Some(DiagnosticCode::ParseError)
        } else {
            None
        }
    }

    /// Try to parse a code string into a DiagnosticCode.
    pub fn parse_code(code: &str) -> Option<DiagnosticCode> {
        match code {
            "PL001" => Some(DiagnosticCode::ParseError),
            "PL002" => Some(DiagnosticCode::SyntaxError),
            "PL003" => Some(DiagnosticCode::UnexpectedEof),
            "PL100" => Some(DiagnosticCode::MissingStrict),
            "PL101" => Some(DiagnosticCode::MissingWarnings),
            "PL102" => Some(DiagnosticCode::UnusedVariable),
            "PL103" => Some(DiagnosticCode::UndefinedVariable),
            "PL104" => Some(DiagnosticCode::VariableShadowing),
            "PL105" => Some(DiagnosticCode::VariableRedeclaration),
            "PL106" => Some(DiagnosticCode::DuplicateParameter),
            "PL107" => Some(DiagnosticCode::ParameterShadowsGlobal),
            "PL108" => Some(DiagnosticCode::UnusedParameter),
            "PL109" => Some(DiagnosticCode::UnquotedBareword),
            "PL110" => Some(DiagnosticCode::UninitializedVariable),
            "PL111" => Some(DiagnosticCode::MisspelledPragma),
            "PL112" => Some(DiagnosticCode::CaptureVarWithoutRegexMatch),
            "PL200" => Some(DiagnosticCode::MissingPackageDeclaration),
            "PL201" => Some(DiagnosticCode::DuplicatePackage),
            "PL300" => Some(DiagnosticCode::DuplicateSubroutine),
            "PL301" => Some(DiagnosticCode::MissingReturn),
            "PL302" => Some(DiagnosticCode::InvalidPrototype),
            "PL303" => Some(DiagnosticCode::RoleConflict),
            "PL400" => Some(DiagnosticCode::BarewordFilehandle),
            "PL401" => Some(DiagnosticCode::TwoArgOpen),
            "PL402" => Some(DiagnosticCode::ImplicitReturn),
            "PL403" => Some(DiagnosticCode::AssignmentInCondition),
            "PL404" => Some(DiagnosticCode::NumericComparisonWithUndef),
            "PL405" => Some(DiagnosticCode::PrintfFormatMismatch),
            "PL406" => Some(DiagnosticCode::UnreachableCode),
            "PL407" => Some(DiagnosticCode::EvalErrorFlow),
            "PL408" => Some(DiagnosticCode::DuplicateHashKey),
            "PL409" => Some(DiagnosticCode::GotoUndefinedLabel),
            "PL500" => Some(DiagnosticCode::DeprecatedDefined),
            "PL501" => Some(DiagnosticCode::DeprecatedArrayBase),
            "PL600" => Some(DiagnosticCode::SecurityStringEval),
            "PL601" => Some(DiagnosticCode::SecurityBacktickExec),
            "PL602" => Some(DiagnosticCode::SecuritySignalHandler),
            "PL603" => Some(DiagnosticCode::SecuritySystemCall),
            "PL604" => Some(DiagnosticCode::SecurityExecCall),
            "PL605" => Some(DiagnosticCode::SecurityPipeOpen),
            "PL606" => Some(DiagnosticCode::SecurityReadpipe),
            "PL700" => Some(DiagnosticCode::UnusedImport),
            "PL701" => Some(DiagnosticCode::ModuleNotFound),
            "PL800" => Some(DiagnosticCode::HeredocInFormat),
            "PL801" => Some(DiagnosticCode::HeredocInBegin),
            "PL802" => Some(DiagnosticCode::HeredocDynamicDelimiter),
            "PL803" => Some(DiagnosticCode::HeredocInSourceFilter),
            "PL804" => Some(DiagnosticCode::HeredocInRegexCode),
            "PL805" => Some(DiagnosticCode::HeredocInEval),
            "PL806" => Some(DiagnosticCode::HeredocTiedHandle),
            "PL900" => Some(DiagnosticCode::VersionIncompatFeature),
            "PC001" => Some(DiagnosticCode::CriticSeverity1),
            "PC002" => Some(DiagnosticCode::CriticSeverity2),
            "PC003" => Some(DiagnosticCode::CriticSeverity3),
            "PC004" => Some(DiagnosticCode::CriticSeverity4),
            "PC005" => Some(DiagnosticCode::CriticSeverity5),
            _ => None,
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Category of diagnostic codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticCategory {
    /// Parser-related diagnostics (PL001-PL099)
    Parser,
    /// Strict/warnings pragmas and scope analysis (PL100-PL199)
    StrictWarnings,
    /// Package/module issues (PL200-PL299)
    PackageModule,
    /// Subroutine issues (PL300-PL399)
    Subroutine,
    /// Best practices and common mistakes (PL400-PL499)
    BestPractices,
    /// Deprecated syntax (PL500-PL599)
    Deprecated,
    /// Security anti-patterns (PL600-PL699)
    Security,
    /// Import/use diagnostics (PL700-PL799)
    Import,
    /// Heredoc anti-patterns (PL800-PL899)
    Heredoc,
    /// Perl::Critic violations (PC001-PC005)
    PerlCritic,
}

impl DiagnosticCode {
    /// Get the category of this diagnostic code.
    pub fn category(&self) -> DiagnosticCategory {
        match self {
            DiagnosticCode::ParseError
            | DiagnosticCode::SyntaxError
            | DiagnosticCode::UnexpectedEof => DiagnosticCategory::Parser,

            DiagnosticCode::MissingStrict
            | DiagnosticCode::MissingWarnings
            | DiagnosticCode::UnusedVariable
            | DiagnosticCode::UndefinedVariable
            | DiagnosticCode::VariableShadowing
            | DiagnosticCode::VariableRedeclaration
            | DiagnosticCode::DuplicateParameter
            | DiagnosticCode::ParameterShadowsGlobal
            | DiagnosticCode::UnusedParameter
            | DiagnosticCode::UnquotedBareword
            | DiagnosticCode::UninitializedVariable
            | DiagnosticCode::MisspelledPragma
            | DiagnosticCode::CaptureVarWithoutRegexMatch => DiagnosticCategory::StrictWarnings,

            DiagnosticCode::MissingPackageDeclaration | DiagnosticCode::DuplicatePackage => {
                DiagnosticCategory::PackageModule
            }

            DiagnosticCode::DuplicateSubroutine
            | DiagnosticCode::MissingReturn
            | DiagnosticCode::InvalidPrototype => DiagnosticCategory::Subroutine,
            DiagnosticCode::RoleConflict => DiagnosticCategory::Subroutine,

            DiagnosticCode::BarewordFilehandle
            | DiagnosticCode::TwoArgOpen
            | DiagnosticCode::ImplicitReturn
            | DiagnosticCode::AssignmentInCondition
            | DiagnosticCode::NumericComparisonWithUndef
            | DiagnosticCode::PrintfFormatMismatch
            | DiagnosticCode::UnreachableCode
            | DiagnosticCode::EvalErrorFlow
            | DiagnosticCode::DuplicateHashKey
            | DiagnosticCode::GotoUndefinedLabel => DiagnosticCategory::BestPractices,

            DiagnosticCode::DeprecatedDefined | DiagnosticCode::DeprecatedArrayBase => {
                DiagnosticCategory::Deprecated
            }

            DiagnosticCode::SecurityStringEval
            | DiagnosticCode::SecurityBacktickExec
            | DiagnosticCode::SecuritySignalHandler
            | DiagnosticCode::SecuritySystemCall
            | DiagnosticCode::SecurityExecCall
            | DiagnosticCode::SecurityPipeOpen
            | DiagnosticCode::SecurityReadpipe => DiagnosticCategory::Security,

            DiagnosticCode::UnusedImport | DiagnosticCode::ModuleNotFound => {
                DiagnosticCategory::Import
            }

            DiagnosticCode::HeredocInFormat
            | DiagnosticCode::HeredocInBegin
            | DiagnosticCode::HeredocDynamicDelimiter
            | DiagnosticCode::HeredocInSourceFilter
            | DiagnosticCode::HeredocInRegexCode
            | DiagnosticCode::HeredocInEval
            | DiagnosticCode::HeredocTiedHandle => DiagnosticCategory::Heredoc,

            DiagnosticCode::VersionIncompatFeature => DiagnosticCategory::BestPractices,

            DiagnosticCode::CriticSeverity1
            | DiagnosticCode::CriticSeverity2
            | DiagnosticCode::CriticSeverity3
            | DiagnosticCode::CriticSeverity4
            | DiagnosticCode::CriticSeverity5 => DiagnosticCategory::PerlCritic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_strings() {
        assert_eq!(DiagnosticCode::ParseError.as_str(), "PL001");
        assert_eq!(DiagnosticCode::MissingStrict.as_str(), "PL100");
        assert_eq!(DiagnosticCode::EvalErrorFlow.as_str(), "PL407");
        assert_eq!(DiagnosticCode::CriticSeverity1.as_str(), "PC001");
        assert_eq!(DiagnosticCode::SecuritySignalHandler.as_str(), "PL602");
        assert_eq!(DiagnosticCode::GotoUndefinedLabel.as_str(), "PL409");
    }

    #[test]
    fn test_severity() {
        assert_eq!(DiagnosticCode::ParseError.severity(), DiagnosticSeverity::Error);
        assert_eq!(DiagnosticCode::UnusedVariable.severity(), DiagnosticSeverity::Warning);
        assert_eq!(DiagnosticCode::EvalErrorFlow.severity(), DiagnosticSeverity::Warning);
        assert_eq!(DiagnosticCode::CriticSeverity5.severity(), DiagnosticSeverity::Hint);
    }

    #[test]
    fn test_from_message() {
        assert_eq!(
            DiagnosticCode::from_message("Missing 'use strict' pragma"),
            Some(DiagnosticCode::MissingStrict)
        );
        assert_eq!(
            DiagnosticCode::from_message("Unused variable $foo"),
            Some(DiagnosticCode::UnusedVariable)
        );
    }

    #[test]
    fn test_from_str() {
        assert_eq!(DiagnosticCode::parse_code("PL001"), Some(DiagnosticCode::ParseError));
        assert_eq!(
            DiagnosticCode::parse_code("PL602"),
            Some(DiagnosticCode::SecuritySignalHandler)
        );
        assert_eq!(DiagnosticCode::parse_code("INVALID"), None);
    }

    #[test]
    fn test_category() {
        assert_eq!(DiagnosticCode::ParseError.category(), DiagnosticCategory::Parser);
        assert_eq!(DiagnosticCode::MissingStrict.category(), DiagnosticCategory::StrictWarnings);
        assert_eq!(DiagnosticCode::SecuritySignalHandler.category(), DiagnosticCategory::Security);
        assert_eq!(DiagnosticCode::EvalErrorFlow.category(), DiagnosticCategory::BestPractices);
        assert_eq!(DiagnosticCode::CriticSeverity1.category(), DiagnosticCategory::PerlCritic);
    }

    #[test]
    fn test_tags() {
        assert!(DiagnosticCode::UnusedVariable.tags().contains(&DiagnosticTag::Unnecessary));
        assert!(DiagnosticCode::ParseError.tags().is_empty());
    }
}
