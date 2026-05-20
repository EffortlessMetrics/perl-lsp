use std::fmt;

use super::DiagnosticCode;

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

impl DiagnosticCode {
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
            | Self::RoleConflict
            | Self::MissingPodCoverage => DiagnosticCategory::Subroutine,

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
            | Self::LoopControlUndefinedLabel
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
