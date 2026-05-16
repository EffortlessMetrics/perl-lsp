//! Semantic token types and modifiers for LSP syntax highlighting.

use crate::SourceLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Semantic token types for syntax highlighting in the Parse/Complete workflow.
pub enum SemanticTokenType {
    // Variables
    /// Variable reference (scalar, array, or hash)
    Variable,
    /// Variable declaration site
    VariableDeclaration,
    /// Read-only variable (constant)
    VariableReadonly,
    /// Function parameter
    Parameter,

    // Functions
    /// Function/subroutine reference
    Function,
    /// Function/subroutine declaration
    FunctionDeclaration,
    /// Object method call
    Method,

    // Types
    /// Class/package name
    Class,
    /// Package namespace
    Namespace,
    /// Type annotation (modern Perl)
    Type,

    // Keywords
    /// Language keyword (if, while, etc.)
    Keyword,
    /// Control flow keyword (return, next, last)
    KeywordControl,
    /// Variable modifier (my, our, local, state)
    Modifier,

    // Literals
    /// Numeric literal
    Number,
    /// String literal
    String,
    /// Regular expression
    Regex,

    // Comments
    /// Regular comment
    Comment,
    /// Documentation comment (POD)
    CommentDoc,

    // Other
    /// Operator (+, -, =~, etc.)
    Operator,
    /// Punctuation marks and delimiters
    Punctuation,
    /// Code label for goto statements
    Label,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Semantic token modifiers for Analyze/Complete stage highlighting.
///
/// Provides additional context about semantic tokens beyond their base type,
/// enabling rich editor highlighting with detailed symbol information.
///
/// # LSP Integration
/// Maps to LSP `SemanticTokenModifiers` for consistent editor experience
/// across different LSP clients with full Perl language semantics.
pub enum SemanticTokenModifier {
    /// Symbol is being declared at this location
    Declaration,
    /// Symbol is being defined at this location
    Definition,
    /// Symbol is read-only (constant)
    Readonly,
    /// Symbol has static storage duration (state variables)
    Static,
    /// Symbol is deprecated and should not be used
    Deprecated,
    /// Symbol is abstract (method without implementation)
    Abstract,
    /// Symbol represents an asynchronous operation
    Async,
    /// Symbol is being modified (written to)
    Modification,
    /// Symbol is documentation-related (POD)
    Documentation,
    /// Symbol is from the Perl standard library
    DefaultLibrary,
}

#[derive(Debug, Clone)]
/// A semantic token with type and modifiers for LSP syntax highlighting.
///
/// Represents a single semantic unit in Perl source code with precise location
/// and rich type information for enhanced editor experience.
///
/// # Performance Characteristics
/// - Memory: ~32 bytes per token (optimized for large files)
/// - Serialization: Direct LSP protocol mapping
/// - Batch processing: Efficient delta updates for incremental parsing
///
/// # LSP Workflow Integration
/// Core component in Parse → Index → Navigate → Complete → Analyze pipeline
/// for real-time syntax highlighting with ≤1ms update latency.
pub struct SemanticToken {
    /// Source location of the token
    pub location: SourceLocation,
    /// Semantic classification of the token
    pub token_type: SemanticTokenType,
    /// Additional modifiers for enhanced highlighting
    pub modifiers: Vec<SemanticTokenModifier>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SemanticTokenType ──────────────────────────────────────────────────────

    #[test]
    fn token_type_equality_same_variant() {
        assert_eq!(SemanticTokenType::Variable, SemanticTokenType::Variable);
        assert_eq!(SemanticTokenType::Function, SemanticTokenType::Function);
        assert_eq!(SemanticTokenType::Comment, SemanticTokenType::Comment);
    }

    #[test]
    fn token_type_inequality_different_variants() {
        assert_ne!(SemanticTokenType::Variable, SemanticTokenType::Function);
        assert_ne!(SemanticTokenType::String, SemanticTokenType::Number);
        assert_ne!(SemanticTokenType::Keyword, SemanticTokenType::Operator);
    }

    #[test]
    fn token_type_is_copy() {
        let t = SemanticTokenType::Class;
        let t2 = t; // Copy — no move
        assert_eq!(t, t2);
    }

    #[test]
    fn token_type_debug_contains_variant_name() {
        let formatted = format!("{:?}", SemanticTokenType::FunctionDeclaration);
        assert!(formatted.contains("FunctionDeclaration"), "got: {formatted}");

        let formatted2 = format!("{:?}", SemanticTokenType::CommentDoc);
        assert!(formatted2.contains("CommentDoc"), "got: {formatted2}");
    }

    // ── SemanticTokenModifier ─────────────────────────────────────────────────

    #[test]
    fn token_modifier_equality_same_variant() {
        assert_eq!(SemanticTokenModifier::Declaration, SemanticTokenModifier::Declaration);
        assert_eq!(SemanticTokenModifier::Readonly, SemanticTokenModifier::Readonly);
        assert_eq!(SemanticTokenModifier::Deprecated, SemanticTokenModifier::Deprecated);
    }

    #[test]
    fn token_modifier_inequality_different_variants() {
        assert_ne!(SemanticTokenModifier::Declaration, SemanticTokenModifier::Definition);
        assert_ne!(SemanticTokenModifier::Static, SemanticTokenModifier::Async);
        assert_ne!(SemanticTokenModifier::Documentation, SemanticTokenModifier::DefaultLibrary);
    }

    #[test]
    fn token_modifier_is_copy() {
        let m = SemanticTokenModifier::Modification;
        let m2 = m; // Copy — no move
        assert_eq!(m, m2);
    }

    #[test]
    fn token_modifier_debug_contains_variant_name() {
        let formatted = format!("{:?}", SemanticTokenModifier::DefaultLibrary);
        assert!(formatted.contains("DefaultLibrary"), "got: {formatted}");

        let formatted2 = format!("{:?}", SemanticTokenModifier::Abstract);
        assert!(formatted2.contains("Abstract"), "got: {formatted2}");
    }

    // ── SemanticToken ─────────────────────────────────────────────────────────

    #[test]
    fn semantic_token_construction_round_trips_fields() {
        let location = SourceLocation { start: 10, end: 20 };
        let tok = SemanticToken {
            location,
            token_type: SemanticTokenType::Variable,
            modifiers: vec![SemanticTokenModifier::Declaration],
        };
        assert_eq!(tok.location.start, 10);
        assert_eq!(tok.location.end, 20);
        assert_eq!(tok.token_type, SemanticTokenType::Variable);
        assert_eq!(tok.modifiers.len(), 1);
        assert_eq!(tok.modifiers[0], SemanticTokenModifier::Declaration);
    }

    #[test]
    fn semantic_token_no_modifiers() {
        let tok = SemanticToken {
            location: SourceLocation { start: 0, end: 5 },
            token_type: SemanticTokenType::Keyword,
            modifiers: Vec::new(),
        };
        assert!(tok.modifiers.is_empty());
    }

    #[test]
    fn semantic_token_clone_is_independent() {
        let original = SemanticToken {
            location: SourceLocation { start: 1, end: 3 },
            token_type: SemanticTokenType::Number,
            modifiers: vec![SemanticTokenModifier::Readonly],
        };
        let mut cloned = original.clone();
        cloned.modifiers.push(SemanticTokenModifier::Static);
        // Original must not be affected by mutation of the clone
        assert_eq!(original.modifiers.len(), 1);
        assert_eq!(cloned.modifiers.len(), 2);
    }
}
