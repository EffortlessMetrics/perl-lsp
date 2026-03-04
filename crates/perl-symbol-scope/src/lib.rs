//! Shared scope identifiers and lexical scope classification for Perl analysis.

/// Unique identifier for a scope.
pub type ScopeId = usize;

/// Classification of lexical scope types in Perl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScopeKind {
    /// Global/file scope
    Global,
    /// Package scope
    Package,
    /// Subroutine scope
    Subroutine,
    /// Block scope (if, while, for, etc.)
    Block,
    /// Eval scope
    Eval,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_kind_is_copy() {
        let kind = ScopeKind::Block;
        let copied = kind;
        assert_eq!(kind, copied);
    }
}
