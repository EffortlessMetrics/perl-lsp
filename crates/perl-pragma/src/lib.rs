//! Pragma tracker for Perl code analysis
//!
//! Tracks `use` and `no` pragmas throughout the codebase to determine
//! effective pragma state at any point in the code.

use perl_ast::ast::Node;
use std::ops::Range;

mod args;
mod conditional;
mod features;
mod map;
mod range_builder;
mod version;

pub use map::{
    CompileTimePragmaEnvironment, PragmaEntry, PragmaMap, PragmaQueryCursor, PragmaStateQuery,
};
pub use version::{
    PerlVersion, features_enabled_by_version, parse_perl_version, version_implies_strict,
    version_implies_warnings,
};

pub(crate) use args::{
    add_disabled_warning_category, apply_builtin_imports, builtin_import_names,
    normalized_pragma_token, pragma_arg_items, remove_builtin_imports,
};
pub(crate) use conditional::conditional_pragma_target;
pub(crate) use features::{apply_feature_state, canonical_feature_query};
pub(crate) use map::normalize_state;
pub(crate) use version::enable_effective_version_semantics;

/// Pragma state at a given point in the code
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PragmaState {
    /// Whether strict vars is enabled
    pub strict_vars: bool,
    /// Whether strict subs is enabled
    pub strict_subs: bool,
    /// Whether strict refs is enabled
    pub strict_refs: bool,
    /// Whether warnings are enabled (globally)
    pub warnings: bool,
    /// Whether `use utf8` is enabled.
    pub utf8: bool,
    /// Active source encoding from `use encoding`.
    pub encoding: Option<String>,
    /// Whether `use feature 'unicode_strings'` or a matching feature bundle is enabled.
    pub unicode_strings: bool,
    /// Whether locale-sensitive behavior is enabled.
    pub locale: bool,
    /// Locale scope from `use locale`, if any.
    pub locale_scope: Option<String>,
    /// Warning categories explicitly disabled via `no warnings 'CATEGORY'`.
    ///
    /// When `no warnings` is used with specific category arguments (e.g.
    /// `no warnings 'uninitialized'`), the global `warnings` flag stays `true`
    /// and the disabled categories are recorded here. Only bare `no warnings`
    /// (no arguments) clears the global `warnings` flag.
    pub disabled_warning_categories: Vec<String>,
    /// Whether explicit `use feature 'signatures'` currently implies strictness.
    ///
    /// This is tracked separately from the raw strict flags so `no feature
    /// 'signatures'` can unwind the feature-driven implication without
    /// clobbering explicit `use strict` or version-implied strict state.
    pub signatures_strict: bool,
    /// Effective language features enabled in the current lexical scope.
    ///
    /// This starts with any features implied by `use vX.Y` declarations and is
    /// then updated by explicit `use feature` / `no feature` pragmas.
    pub features: Vec<&'static str>,
    /// Lexically imported builtin short names from `use builtin`.
    pub builtin_imports: Vec<String>,
}

/// Immutable compile-time snapshot of pragma state.
///
/// This is the stable value object returned by position queries, and the same
/// type used by lexical save/restore operations while building an environment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PragmaSnapshot {
    state: PragmaState,
}

impl PragmaSnapshot {
    /// Create a snapshot from a concrete state value.
    #[must_use]
    pub fn from_state(state: PragmaState) -> Self {
        Self { state }
    }

    /// Borrow the underlying state.
    #[must_use]
    pub fn state(&self) -> &PragmaState {
        &self.state
    }

    /// Whether all strict categories are active in this snapshot.
    #[must_use]
    pub fn strict_enabled(&self) -> bool {
        self.state.strict_vars && self.state.strict_subs && self.state.strict_refs
    }

    /// Whether warnings are globally active in this snapshot.
    #[must_use]
    pub fn warnings_enabled(&self) -> bool {
        self.state.warnings
    }

    /// Whether a feature is enabled in this snapshot.
    #[must_use]
    pub fn has_feature(&self, feature: &str) -> bool {
        self.state.has_feature(feature)
    }

    /// Returns true if warnings are active for the given category.
    #[must_use]
    pub fn is_warning_active(&self, category: &str) -> bool {
        self.state.is_warning_active(category)
    }
}

impl From<PragmaState> for PragmaSnapshot {
    fn from(state: PragmaState) -> Self {
        Self::from_state(state)
    }
}

impl From<PragmaSnapshot> for PragmaState {
    fn from(snapshot: PragmaSnapshot) -> Self {
        snapshot.state
    }
}

impl PragmaState {
    /// Create a new pragma state with all strict modes enabled
    pub fn all_strict() -> Self {
        Self {
            strict_vars: true,
            strict_subs: true,
            strict_refs: true,
            warnings: false,
            utf8: false,
            encoding: None,
            unicode_strings: false,
            locale: false,
            locale_scope: None,
            disabled_warning_categories: Vec::new(),
            signatures_strict: false,
            features: Vec::new(),
            builtin_imports: Vec::new(),
        }
    }

    /// Returns `true` if warnings are active for the given category.
    ///
    /// Warnings for a category are active when:
    /// - The global `warnings` flag is `true`, **and**
    /// - The category is not listed in `disabled_warning_categories`.
    ///
    /// If the global `warnings` flag is `false` (i.e. `no warnings` with no
    /// arguments was used), all categories are considered inactive regardless of
    /// the `disabled_warning_categories` list.
    #[must_use]
    pub fn is_warning_active(&self, category: &str) -> bool {
        self.warnings && !self.disabled_warning_categories.iter().any(|c| c == category)
    }

    /// Returns `true` if the given feature name is currently enabled.
    #[must_use]
    pub fn has_feature(&self, feature: &str) -> bool {
        let feature = canonical_feature_query(feature);
        self.features.contains(&feature)
    }

    /// Returns `true` when a builtin short name was lexically imported in scope.
    #[must_use]
    pub fn has_builtin_import(&self, name: &str) -> bool {
        self.builtin_imports.iter().any(|import| import == name)
    }
}

/// Tracks pragma state throughout a Perl file
pub struct PragmaTracker;

impl PragmaTracker {
    /// Build a range-indexed pragma map from an AST
    pub fn build(ast: &Node) -> Vec<(Range<usize>, PragmaState)> {
        CompileTimePragmaEnvironment::build(ast)
            .as_map()
            .iter()
            .map(|(range, snapshot)| (range.clone(), snapshot.clone().into()))
            .collect()
    }

    /// Get the pragma state at a specific byte offset
    pub fn state_for_offset(
        pragma_map: &[(Range<usize>, PragmaState)],
        offset: usize,
    ) -> PragmaState {
        let idx = pragma_map.partition_point(|(range, _)| range.start <= offset);
        let state = if idx > 0 { pragma_map[idx - 1].1.clone() } else { PragmaState::default() };

        normalize_state(state)
    }

    /// Get the final top-level pragma state after all lexical scopes close.
    #[must_use]
    pub fn final_state(pragma_map: &[(Range<usize>, PragmaState)]) -> PragmaState {
        let state = pragma_map.last().map_or_else(PragmaState::default, |(_, s)| s.clone());

        normalize_state(state)
    }
}
