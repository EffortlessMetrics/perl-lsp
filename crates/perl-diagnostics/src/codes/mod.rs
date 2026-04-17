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

/// Severity level of a diagnostic.
///
/// Maps to LSP DiagnosticSeverity values (1=Error, 2=Warning, 3=Info, 4=Hint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum DiagnosticSeverity {
    /// Critical error that prevents parsing/execution.
    #[default]
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
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Information => write!(f, "info"),
            Self::Hint => write!(f, "hint"),
        }
    }
}

/// Diagnostic tags for additional classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticTag {
    /// Code that can be safely removed (unused variables, imports).
    Unnecessary = 1,
    /// Code using deprecated features.
    Deprecated = 2,
}

impl DiagnosticTag {
    /// Get the LSP numeric value for this tag.
    pub fn to_lsp_value(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for DiagnosticTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unnecessary => write!(f, "unnecessary"),
            Self::Deprecated => write!(f, "deprecated"),
        }
    }
}

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

impl DiagnosticCode {
    /// Get the string representation of this code.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ParseError => "PL001",
            Self::SyntaxError => "PL002",
            Self::UnexpectedEof => "PL003",
            Self::MissingStrict => "PL100",
            Self::MissingWarnings => "PL101",
            Self::UnusedVariable => "PL102",
            Self::UndefinedVariable => "PL103",
            Self::VariableShadowing => "PL104",
            Self::VariableRedeclaration => "PL105",
            Self::DuplicateParameter => "PL106",
            Self::ParameterShadowsGlobal => "PL107",
            Self::UnusedParameter => "PL108",
            Self::UnquotedBareword => "PL109",
            Self::UninitializedVariable => "PL110",
            Self::MisspelledPragma => "PL111",
            Self::CaptureVarWithoutRegexMatch => "PL112",
            Self::MissingPackageDeclaration => "PL200",
            Self::DuplicatePackage => "PL201",
            Self::DuplicateSubroutine => "PL300",
            Self::MissingReturn => "PL301",
            Self::InvalidPrototype => "PL302",
            Self::RoleConflict => "PL303",
            Self::BarewordFilehandle => "PL400",
            Self::TwoArgOpen => "PL401",
            Self::ImplicitReturn => "PL402",
            Self::AssignmentInCondition => "PL403",
            Self::NumericComparisonWithUndef => "PL404",
            Self::PrintfFormatMismatch => "PL405",
            Self::UnreachableCode => "PL406",
            Self::EvalErrorFlow => "PL407",
            Self::DuplicateHashKey => "PL408",
            Self::GotoUndefinedLabel => "PL409",
            Self::DeprecatedDefined => "PL500",
            Self::DeprecatedArrayBase => "PL501",
            Self::PhaseScopedStrictPragma => "PL502",
            Self::PhaseScopedWarningsPragma => "PL503",
            Self::SecurityStringEval => "PL600",
            Self::SecurityBacktickExec => "PL601",
            Self::SecuritySignalHandler => "PL602",
            Self::SecuritySystemCall => "PL603",
            Self::SecurityExecCall => "PL604",
            Self::SecurityPipeOpen => "PL605",
            Self::SecurityReadpipe => "PL606",
            Self::UnusedImport => "PL700",
            Self::ModuleNotFound => "PL701",
            Self::HeredocInFormat => "PL800",
            Self::HeredocInBegin => "PL801",
            Self::HeredocDynamicDelimiter => "PL802",
            Self::HeredocInSourceFilter => "PL803",
            Self::HeredocInRegexCode => "PL804",
            Self::HeredocInEval => "PL805",
            Self::HeredocTiedHandle => "PL806",
            Self::VersionIncompatFeature => "PL900",
            Self::CriticSeverity1 => "PC001",
            Self::CriticSeverity2 => "PC002",
            Self::CriticSeverity3 => "PC003",
            Self::CriticSeverity4 => "PC004",
            Self::CriticSeverity5 => "PC005",
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
            "PL502" => "https://docs.perl-lsp.org/errors/PL502",
            "PL503" => "https://docs.perl-lsp.org/errors/PL503",
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
            Self::ParseError
            | Self::SyntaxError
            | Self::UnexpectedEof
            | Self::UndefinedVariable
            | Self::VariableRedeclaration
            | Self::DuplicateParameter
            | Self::UnquotedBareword => DiagnosticSeverity::Error,

            // Warnings
            Self::MissingStrict
            | Self::MissingWarnings
            | Self::UnusedVariable
            | Self::VariableShadowing
            | Self::ParameterShadowsGlobal
            | Self::UnusedParameter
            | Self::UninitializedVariable
            | Self::MisspelledPragma
            | Self::MissingPackageDeclaration
            | Self::DuplicatePackage
            | Self::DuplicateSubroutine
            | Self::MissingReturn
            | Self::InvalidPrototype
            | Self::RoleConflict
            | Self::BarewordFilehandle
            | Self::TwoArgOpen
            | Self::ImplicitReturn
            | Self::AssignmentInCondition
            | Self::NumericComparisonWithUndef
            | Self::PrintfFormatMismatch
            | Self::DuplicateHashKey
            | Self::GotoUndefinedLabel
            | Self::DeprecatedDefined
            | Self::DeprecatedArrayBase
            | Self::PhaseScopedStrictPragma
            | Self::PhaseScopedWarningsPragma
            | Self::SecurityStringEval
            | Self::SecurityBacktickExec
            | Self::SecuritySignalHandler
            | Self::SecuritySystemCall
            | Self::SecurityExecCall
            | Self::SecurityPipeOpen
            | Self::SecurityReadpipe
            | Self::ModuleNotFound
            | Self::VersionIncompatFeature
            | Self::EvalErrorFlow
            | Self::CriticSeverity1
            | Self::CriticSeverity2 => DiagnosticSeverity::Warning,

            // Information
            Self::CaptureVarWithoutRegexMatch
            | Self::HeredocInFormat
            | Self::HeredocInBegin
            | Self::HeredocDynamicDelimiter
            | Self::HeredocInSourceFilter
            | Self::HeredocInRegexCode
            | Self::HeredocInEval
            | Self::HeredocTiedHandle => DiagnosticSeverity::Information,

            // Hints
            Self::UnusedImport
            | Self::UnreachableCode
            | Self::CriticSeverity3
            | Self::CriticSeverity4
            | Self::CriticSeverity5 => DiagnosticSeverity::Hint,
        }
    }

    /// Get any diagnostic tags associated with this code.
    pub fn tags(&self) -> &'static [DiagnosticTag] {
        match self {
            Self::UnusedVariable
            | Self::UnusedParameter
            | Self::UnusedImport
            | Self::UnreachableCode => &[DiagnosticTag::Unnecessary],
            Self::DeprecatedDefined | Self::DeprecatedArrayBase => &[DiagnosticTag::Deprecated],
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
            Self::ParseError => Some(
                "The parser could not understand this code. \
                Check for missing semicolons, unmatched brackets, or incorrect syntax.",
            ),
            Self::SyntaxError => Some(
                "Perl syntax error. Check for typos, missing operators, \
                or unbalanced parentheses near this location.",
            ),
            Self::UnexpectedEof => Some(
                "The file ended unexpectedly. Check for unclosed blocks `{}`, \
                heredocs, or multi-line strings.",
            ),
            Self::MissingStrict => Some(
                "Add `use strict;` at the top of your file. \
                Strict mode catches common variable mistakes at compile time.",
            ),
            Self::MissingWarnings => Some(
                "Add `use warnings;` at the top of your file. \
                Warnings highlight many common programming mistakes.",
            ),
            Self::UnusedVariable => Some(
                "This variable is declared but never used. \
                Remove it, or prefix with `_` (e.g., `$_unused`) to suppress.",
            ),
            Self::UndefinedVariable => Some(
                "This variable was not declared with `my`, `our`, or `local`. \
                Add `use strict;` and declare all variables before use.",
            ),
            Self::MissingPackageDeclaration => Some(
                "This file has no `package` declaration. \
                Add `package MyModule;` at the top for module files.",
            ),
            Self::DuplicatePackage => Some(
                "This package name is declared more than once in the same file. \
                Each package should appear once, or split into separate files.",
            ),
            Self::DuplicateSubroutine => Some(
                "A subroutine with this name is defined more than once. \
                The later definition silently replaces the earlier one.",
            ),
            Self::MissingReturn => Some(
                "This subroutine has no explicit `return` statement. \
                Add `return $value;` to make the return value clear.",
            ),
            Self::RoleConflict => Some(
                "Two or more consumed Moo/Moose roles provide the same method. \
                Define the method in the class or remove one of the conflicting roles.",
            ),
            Self::InvalidPrototype => Some(
                "The prototype contains a character that Perl does not recognise. \
                Valid prototype characters are: $, @, %, &, *, \\, ;, +, _ and spaces. \
                See perlsub for the full prototype syntax.",
            ),
            Self::BarewordFilehandle => Some(
                "Bareword filehandles (e.g., `open FH, ...`) are global and unsafe. \
                Use a lexical filehandle instead: `open my $fh, '<', $file or die $!;`",
            ),
            Self::TwoArgOpen => Some(
                "Two-argument `open()` is vulnerable to injection. \
                Use three-argument form: `open my $fh, '<', $filename or die $!;`",
            ),
            Self::ImplicitReturn => Some(
                "The return value of this expression is used implicitly. \
                Make it explicit with `return` or assign it to a variable.",
            ),
            Self::AssignmentInCondition => Some(
                "This looks like an assignment `=` inside a condition where \
                a comparison `==` or `eq` was likely intended.",
            ),
            Self::NumericComparisonWithUndef => Some(
                "Comparing a potentially undefined value with a numeric operator \
                produces a warning at runtime. Check for definedness first with `defined()`.",
            ),
            Self::EvalErrorFlow => Some(
                "Read `$@` or `$EVAL_ERROR` immediately after an `eval` or `try` \
                block; intervening statements can clobber the exception state.",
            ),
            Self::UnreachableCode => Some(
                "This statement cannot be executed because a preceding statement \
                unconditionally exits (return, die, exit, croak). Remove or relocate it.",
            ),
            Self::DuplicateHashKey => Some(
                "This hash key appears more than once in the same literal. \
                Only the last value will be used; the earlier assignment is silently discarded.",
            ),
            Self::GotoUndefinedLabel => Some(
                "This goto target label is not defined in the current file. \
                Define the label or use a dynamic goto form only when the target is known at runtime.",
            ),
            Self::PrintfFormatMismatch => Some(
                "The number of format specifiers does not match the number of arguments. \
                Each %s/%d/%f/etc. consumes one argument (except %% which consumes none).",
            ),
            Self::VariableShadowing => Some(
                "This variable shadows an outer variable with the same name. \
                Rename it to avoid confusion, or use the outer variable directly.",
            ),
            Self::VariableRedeclaration => Some(
                "This variable is declared again in the same scope. \
                Remove the duplicate `my` declaration and reuse the existing variable.",
            ),
            Self::DuplicateParameter => Some(
                "This subroutine signature has a duplicate parameter name. \
                Each parameter must have a unique name.",
            ),
            Self::ParameterShadowsGlobal => Some(
                "This subroutine parameter shadows a global (`our`) variable. \
                Rename the parameter to avoid confusion with the global.",
            ),
            Self::UnusedParameter => Some(
                "This subroutine parameter is declared but never used. \
                Remove it or prefix with `_` (e.g., `$_unused`) to suppress.",
            ),
            Self::UnquotedBareword => Some(
                "This bareword is used where a quoted string is expected. \
                Under `use strict`, barewords are not allowed. Quote it: `'word'`.",
            ),
            Self::UninitializedVariable => Some(
                "This variable is used before being assigned a value. \
                Initialize it before use to avoid `Use of uninitialized value` warnings.",
            ),
            Self::MisspelledPragma => Some(
                "This pragma name appears to be misspelled. \
                Check the spelling and ensure the module is installed.",
            ),
            Self::CaptureVarWithoutRegexMatch => Some(
                "Capture variables ($1, $2, etc.) are only meaningful after a successful regex match. \
                Perform a regex match with =~ /.../ before using $1 or $2.",
            ),
            Self::DeprecatedDefined => Some(
                "`defined(@array)` and `defined(%hash)` are deprecated since Perl 5.6. \
                Use `@array` or `%hash` directly in boolean context instead.",
            ),
            Self::DeprecatedArrayBase => Some(
                "The `$[` variable is deprecated. Array indices always start at 0 \
                in modern Perl. Remove any assignment to `$[`.",
            ),
            Self::PhaseScopedStrictPragma => Some(
                "`use strict` inside a phase block only applies inside that block. \
                Move `use strict;` to file scope for file-wide strict enforcement.",
            ),
            Self::PhaseScopedWarningsPragma => Some(
                "`use warnings` inside a phase block only applies inside that block. \
                Move `use warnings;` to file scope for file-wide warnings coverage.",
            ),
            Self::SecurityStringEval => Some(
                "String `eval` executes arbitrary code and is a security risk. \
                Use block eval `eval { ... }` or safer alternatives.",
            ),
            Self::SecurityBacktickExec => Some(
                "Backticks/`qx()` execute shell commands and can be exploited. \
                Use `system()` with a list form or IPC::Run for safer execution.",
            ),
            Self::SecuritySignalHandler => Some(
                "Assigning to $SIG{__DIE__} or $SIG{__WARN__} globally changes exception \
                and warning handling for the whole process. Use `local` to scope the handler.",
            ),
            Self::SecuritySystemCall => Some(
                "`system()` executes a shell command. If the arguments include user input, \
                use the list form `system($cmd, @args)` to avoid shell injection.",
            ),
            Self::SecurityExecCall => Some(
                "`exec()` replaces the current process with a shell command. If arguments \
                include user input, use the list form `exec($cmd, @args)` to avoid shell injection.",
            ),
            Self::SecurityPipeOpen => Some(
                "Pipe-open executes a shell command. Pass a list to `open` for safe argument \
                handling: `open(my $fh, '-|', $cmd, @args)` instead of `open(my $fh, \"|$cmd\")`.",
            ),
            Self::SecurityReadpipe => Some(
                "`readpipe()` executes a shell command (equivalent to backticks/qx//). \
                Use `open(my $fh, '-|', $cmd, @args)` or IPC::Run for safer command execution.",
            ),
            Self::UnusedImport => Some(
                "This module is imported but none of its exports appear to be used. \
                Remove the `use` statement to reduce unnecessary dependencies.",
            ),
            Self::ModuleNotFound => Some(
                "This module was not found in the workspace or configured include paths. \
                Install it with cpanm or add it to cpanfile.",
            ),
            Self::HeredocInFormat => Some(
                "Heredocs inside `format` blocks can cause subtle parsing issues. \
                Extract the heredoc content into a variable before the format.",
            ),
            Self::HeredocInBegin => Some(
                "Heredocs inside `BEGIN` blocks may behave unexpectedly due to \
                compile-time execution. Move the heredoc outside the BEGIN block.",
            ),
            Self::HeredocDynamicDelimiter => Some(
                "The heredoc delimiter contains a variable, making it dynamic. \
                Use a static delimiter string to avoid surprising behavior.",
            ),
            Self::HeredocInSourceFilter => Some(
                "Heredocs inside source-filtered code may be mangled by the filter. \
                Avoid combining heredocs with source filters.",
            ),
            Self::HeredocInRegexCode => Some(
                "Heredocs inside regex code blocks `(?{ ... })` can cause parsing failures. \
                Move the heredoc content outside the regex.",
            ),
            Self::HeredocInEval => Some(
                "Heredocs inside string `eval` are fragile and error-prone. \
                Use a variable or block eval instead.",
            ),
            Self::HeredocTiedHandle => Some(
                "Heredocs written to tied filehandles may not behave as expected. \
                The tie interface may not handle multi-line heredoc output correctly.",
            ),
            Self::VersionIncompatFeature => Some(
                "This Perl feature requires a newer Perl version than declared. \
                Update 'use vN.NN' or 'use feature' to enable it.",
            ),
            // Perl::Critic codes carry per-policy descriptions; no generic hint needed.
            Self::CriticSeverity1
            | Self::CriticSeverity2
            | Self::CriticSeverity3
            | Self::CriticSeverity4
            | Self::CriticSeverity5 => None,
        }
    }

    /// Try to infer a diagnostic code from a message.
    pub fn from_message(msg: &str) -> Option<Self> {
        let msg_lower = msg.to_lowercase();
        if msg_lower.contains("inside a begin block does not enable strict")
            || msg_lower.contains("inside a phase block does not enable strict")
        {
            Some(Self::PhaseScopedStrictPragma)
        } else if msg_lower.contains("inside a begin block does not enable warnings")
            || msg_lower.contains("inside a phase block does not enable warnings")
        {
            Some(Self::PhaseScopedWarningsPragma)
        } else if msg_lower.contains("use strict") {
            Some(Self::MissingStrict)
        } else if msg_lower.contains("use warnings") {
            Some(Self::MissingWarnings)
        } else if msg_lower.contains("unused variable") || msg_lower.contains("never used") {
            Some(Self::UnusedVariable)
        } else if msg_lower.contains("undefined") || msg_lower.contains("not declared") {
            Some(Self::UndefinedVariable)
        } else if msg_lower.contains("bareword filehandle") {
            Some(Self::BarewordFilehandle)
        } else if msg_lower.contains("two-argument") || msg_lower.contains("2-arg") {
            Some(Self::TwoArgOpen)
        } else if msg_lower.contains("invalid prototype character")
            || msg_lower.contains("illegal character in prototype")
        {
            Some(Self::InvalidPrototype)
        } else if msg_lower.contains("parse error") || msg_lower.contains("syntax error") {
            Some(Self::ParseError)
        } else {
            None
        }
    }

    /// Try to parse a code string into a DiagnosticCode.
    pub fn parse_code(code: &str) -> Option<Self> {
        match code {
            "PL001" => Some(Self::ParseError),
            "PL002" => Some(Self::SyntaxError),
            "PL003" => Some(Self::UnexpectedEof),
            "PL100" => Some(Self::MissingStrict),
            "PL101" => Some(Self::MissingWarnings),
            "PL102" => Some(Self::UnusedVariable),
            "PL103" => Some(Self::UndefinedVariable),
            "PL104" => Some(Self::VariableShadowing),
            "PL105" => Some(Self::VariableRedeclaration),
            "PL106" => Some(Self::DuplicateParameter),
            "PL107" => Some(Self::ParameterShadowsGlobal),
            "PL108" => Some(Self::UnusedParameter),
            "PL109" => Some(Self::UnquotedBareword),
            "PL110" => Some(Self::UninitializedVariable),
            "PL111" => Some(Self::MisspelledPragma),
            "PL112" => Some(Self::CaptureVarWithoutRegexMatch),
            "PL200" => Some(Self::MissingPackageDeclaration),
            "PL201" => Some(Self::DuplicatePackage),
            "PL300" => Some(Self::DuplicateSubroutine),
            "PL301" => Some(Self::MissingReturn),
            "PL302" => Some(Self::InvalidPrototype),
            "PL303" => Some(Self::RoleConflict),
            "PL400" => Some(Self::BarewordFilehandle),
            "PL401" => Some(Self::TwoArgOpen),
            "PL402" => Some(Self::ImplicitReturn),
            "PL403" => Some(Self::AssignmentInCondition),
            "PL404" => Some(Self::NumericComparisonWithUndef),
            "PL405" => Some(Self::PrintfFormatMismatch),
            "PL406" => Some(Self::UnreachableCode),
            "PL407" => Some(Self::EvalErrorFlow),
            "PL408" => Some(Self::DuplicateHashKey),
            "PL409" => Some(Self::GotoUndefinedLabel),
            "PL500" => Some(Self::DeprecatedDefined),
            "PL501" => Some(Self::DeprecatedArrayBase),
            "PL502" => Some(Self::PhaseScopedStrictPragma),
            "PL503" => Some(Self::PhaseScopedWarningsPragma),
            "PL600" => Some(Self::SecurityStringEval),
            "PL601" => Some(Self::SecurityBacktickExec),
            "PL602" => Some(Self::SecuritySignalHandler),
            "PL603" => Some(Self::SecuritySystemCall),
            "PL604" => Some(Self::SecurityExecCall),
            "PL605" => Some(Self::SecurityPipeOpen),
            "PL606" => Some(Self::SecurityReadpipe),
            "PL700" => Some(Self::UnusedImport),
            "PL701" => Some(Self::ModuleNotFound),
            "PL800" => Some(Self::HeredocInFormat),
            "PL801" => Some(Self::HeredocInBegin),
            "PL802" => Some(Self::HeredocDynamicDelimiter),
            "PL803" => Some(Self::HeredocInSourceFilter),
            "PL804" => Some(Self::HeredocInRegexCode),
            "PL805" => Some(Self::HeredocInEval),
            "PL806" => Some(Self::HeredocTiedHandle),
            "PL900" => Some(Self::VersionIncompatFeature),
            "PC001" => Some(Self::CriticSeverity1),
            "PC002" => Some(Self::CriticSeverity2),
            "PC003" => Some(Self::CriticSeverity3),
            "PC004" => Some(Self::CriticSeverity4),
            "PC005" => Some(Self::CriticSeverity5),
            _ => None,
        }
    }

    /// Get the category of this diagnostic code.
    pub fn category(&self) -> DiagnosticCategory {
        match self {
            Self::ParseError | Self::SyntaxError | Self::UnexpectedEof => {
                DiagnosticCategory::Parser
            }

            Self::MissingStrict
            | Self::MissingWarnings
            | Self::UnusedVariable
            | Self::UndefinedVariable
            | Self::VariableShadowing
            | Self::VariableRedeclaration
            | Self::DuplicateParameter
            | Self::ParameterShadowsGlobal
            | Self::UnusedParameter
            | Self::UnquotedBareword
            | Self::UninitializedVariable
            | Self::MisspelledPragma
            | Self::CaptureVarWithoutRegexMatch
            | Self::PhaseScopedStrictPragma
            | Self::PhaseScopedWarningsPragma => DiagnosticCategory::StrictWarnings,

            Self::MissingPackageDeclaration | Self::DuplicatePackage => {
                DiagnosticCategory::PackageModule
            }

            Self::DuplicateSubroutine
            | Self::MissingReturn
            | Self::InvalidPrototype
            | Self::RoleConflict => DiagnosticCategory::Subroutine,

            Self::BarewordFilehandle
            | Self::TwoArgOpen
            | Self::ImplicitReturn
            | Self::AssignmentInCondition
            | Self::NumericComparisonWithUndef
            | Self::PrintfFormatMismatch
            | Self::UnreachableCode
            | Self::EvalErrorFlow
            | Self::DuplicateHashKey
            | Self::GotoUndefinedLabel
            | Self::VersionIncompatFeature => DiagnosticCategory::BestPractices,

            Self::DeprecatedDefined | Self::DeprecatedArrayBase => DiagnosticCategory::Deprecated,

            Self::SecurityStringEval
            | Self::SecurityBacktickExec
            | Self::SecuritySignalHandler
            | Self::SecuritySystemCall
            | Self::SecurityExecCall
            | Self::SecurityPipeOpen
            | Self::SecurityReadpipe => DiagnosticCategory::Security,

            Self::UnusedImport | Self::ModuleNotFound => DiagnosticCategory::Import,

            Self::HeredocInFormat
            | Self::HeredocInBegin
            | Self::HeredocDynamicDelimiter
            | Self::HeredocInSourceFilter
            | Self::HeredocInRegexCode
            | Self::HeredocInEval
            | Self::HeredocTiedHandle => DiagnosticCategory::Heredoc,

            Self::CriticSeverity1
            | Self::CriticSeverity2
            | Self::CriticSeverity3
            | Self::CriticSeverity4
            | Self::CriticSeverity5 => DiagnosticCategory::PerlCritic,
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

impl fmt::Display for DiagnosticCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parser => write!(f, "Parser"),
            Self::StrictWarnings => write!(f, "Strict/Warnings"),
            Self::PackageModule => write!(f, "Package/Module"),
            Self::Subroutine => write!(f, "Subroutine"),
            Self::BestPractices => write!(f, "Best Practices"),
            Self::Deprecated => write!(f, "Deprecated"),
            Self::Security => write!(f, "Security"),
            Self::Import => write!(f, "Import"),
            Self::Heredoc => write!(f, "Heredoc"),
            Self::PerlCritic => write!(f, "Perl::Critic"),
        }
    }
}
