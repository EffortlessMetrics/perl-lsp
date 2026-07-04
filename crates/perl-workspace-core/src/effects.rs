//! Compile-time effect facts: what `use strict` / `use warnings` /
//! `use feature` / `use v5.x` established for a file.
//!
//! These are computed by **reusing `perl-pragma`**, which owns the
//! strict/warnings semantics and the full `use v5.10`→`v5.42` feature-bundle
//! progression. Hand-rolling a version→feature table is exactly the
//! fabricated-fact hazard the external-truth-gate doctrine warns against, so
//! the substrate borrows perl-pragma's correctness and only projects the result
//! into its own serde-serializable fact shape.

use serde::{Deserialize, Serialize};

use crate::id::FileId;

/// The effective compile-time pragma state for a file, as a fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileEffectFacts {
    /// The file these effects apply to.
    pub file_id: FileId,
    /// Whether `use strict` (all three of vars/subs/refs) is in effect at file
    /// scope.
    pub strict: bool,
    /// Whether warnings are globally enabled.
    pub warnings: bool,
    /// Whether `use utf8` is in effect.
    pub utf8: bool,
    /// Whether `unicode_strings` (via feature or bundle) is in effect.
    pub unicode_strings: bool,
    /// Enabled features (from `use feature` and version bundles).
    pub features: Vec<String>,
    /// Warning categories explicitly disabled via `no warnings 'CATEGORY'`.
    pub disabled_warnings: Vec<String>,
    /// The Perl language version requested via `use v5.x` / `use 5.0xx`, if any.
    pub perl_version: Option<String>,
}

impl CompileEffectFacts {
    /// Project a `perl_pragma::PragmaState` into a fact for `file_id`.
    ///
    /// `perl_version` is not part of `PragmaState`; the caller passes the
    /// version it saw on a bare `use v5.x` statement, if any.
    #[must_use]
    pub fn from_pragma_state(
        file_id: FileId,
        state: &perl_pragma::PragmaState,
        perl_version: Option<String>,
    ) -> Self {
        Self {
            file_id,
            strict: state.strict_vars && state.strict_subs && state.strict_refs,
            warnings: state.warnings,
            utf8: state.utf8,
            unicode_strings: state.unicode_strings,
            features: state.features.iter().map(|s| (*s).to_string()).collect(),
            disabled_warnings: state.disabled_warning_categories.clone(),
            perl_version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Digest;

    #[test]
    fn projects_pragma_state() {
        let state = perl_pragma::PragmaState {
            strict_vars: true,
            strict_subs: true,
            strict_refs: true,
            warnings: true,
            features: vec!["say", "signatures"],
            ..Default::default()
        };
        let file_id = FileId::new("lib/App.pm", &Digest::of("x"));
        let facts =
            CompileEffectFacts::from_pragma_state(file_id, &state, Some("v5.38".to_string()));
        assert!(facts.strict);
        assert!(facts.warnings);
        assert_eq!(facts.features, vec!["say", "signatures"]);
        assert_eq!(facts.perl_version.as_deref(), Some("v5.38"));
    }

    #[test]
    fn partial_strict_is_not_full_strict() {
        // only vars, not subs/refs
        let state = perl_pragma::PragmaState { strict_vars: true, ..Default::default() };
        let file_id = FileId::new("lib/App.pm", &Digest::of("x"));
        let facts = CompileEffectFacts::from_pragma_state(file_id, &state, None);
        assert!(!facts.strict, "partial strict must not report full strict");
    }
}
