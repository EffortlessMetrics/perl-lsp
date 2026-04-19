use crate::perl_critic::{Violation, insertion_range};
use perl_parser_core::position::Range;

/// A quick fix for a violation
#[derive(Debug, Clone)]
pub struct QuickFix {
    /// Human-readable title describing the fix action
    pub title: String,
    /// The text edit to apply as a fix
    pub edit: TextEdit,
}

/// A text edit
#[derive(Debug, Clone)]
pub struct TextEdit {
    /// The range of text to replace
    pub range: Range,
    /// The replacement text (empty string for deletion)
    pub new_text: String,
}

#[cfg(feature = "lsp-compat")]
pub(crate) fn perlcritic_quick_fix(violation: &Violation) -> Option<QuickFix> {
    match violation.policy.as_str() {
        "Variables::ProhibitUnusedVariables" => Some(QuickFix {
            title: "Remove unused variable".to_string(),
            edit: TextEdit {
                range: violation.range,
                new_text: String::new(),
            },
        }),
        "Subroutines::ProhibitUnusedPrivateSubroutines" => Some(QuickFix {
            title: "Remove unused subroutine".to_string(),
            edit: TextEdit {
                range: violation.range,
                new_text: String::new(),
            },
        }),
        "TestingAndDebugging::RequireUseStrict" => Some(use_statement_fix("strict")),
        "TestingAndDebugging::RequireUseWarnings" => Some(use_statement_fix("warnings")),
        _ => None,
    }
}

pub(crate) fn built_in_quick_fix(violation: &Violation) -> Option<QuickFix> {
    match violation.policy.as_str() {
        "TestingAndDebugging::RequireUseStrict" => Some(use_statement_fix("strict")),
        "TestingAndDebugging::RequireUseWarnings" => Some(use_statement_fix("warnings")),
        _ => None,
    }
}

fn use_statement_fix(feature: &str) -> QuickFix {
    QuickFix {
        title: format!("Add 'use {feature}'"),
        edit: TextEdit {
            range: insertion_range(),
            new_text: format!("use {feature};\n"),
        },
    }
}
