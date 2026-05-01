/// Represents a code action (quick-fix) that can be applied to resolve a diagnostic
///
/// Code actions provide automated fixes and refactoring operations for Perl code.
#[derive(Debug, Clone)]
pub struct CodeAction {
    /// Human-readable title describing the action
    pub title: String,
    /// The kind/category of code action
    pub kind: CodeActionKind,
    /// The text edit to apply
    pub edit: TextEdit,
    /// ID of the diagnostic this action fixes
    pub diagnostic_id: Option<String>,
    /// Exact diagnostic range this action was derived from
    pub diagnostic_range: Option<(usize, usize)>,
}

/// Kind of code action
///
/// Categorizes the type of code action to help editors organize actions.
#[derive(Debug, Clone, PartialEq)]
pub enum CodeActionKind {
    /// Quick fix for a diagnostic issue
    QuickFix,
    /// General refactoring operation
    Refactor,
    /// Extract code into a new construct
    RefactorExtract,
    /// Inline a construct into its usage sites
    RefactorInline,
    /// Rewrite code using a different pattern
    RefactorRewrite,
}

/// Text edit operation
///
/// Represents a change to be made to source code.
#[derive(Debug, Clone)]
pub struct TextEdit {
    /// The range of text to replace (start, end)
    pub range: (usize, usize),
    /// The new text to insert
    pub new_text: String,
}
