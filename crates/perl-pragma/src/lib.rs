//! Pragma tracker for Perl code analysis
//!
//! Tracks `use` and `no` pragmas throughout the codebase to determine
//! effective pragma state at any point in the code.

use perl_ast::ast::{Node, NodeKind};
use std::ops::Range;

/// Parsed Perl version from a lexical `use v...;` or `use 5.xxx;` pragma.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PerlVersion {
    /// Major Perl version component.
    pub major: u32,
    /// Minor Perl version component.
    pub minor: u32,
}

impl PerlVersion {
    /// Create a new Perl version value.
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }
}

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
    /// and the disabled categories are recorded here.  Only bare `no warnings`
    /// (no arguments) clears the global `warnings` flag.
    pub disabled_warning_categories: Vec<String>,
    /// Language features implicitly enabled by a `use vX.Y` version declaration.
    ///
    /// Populated by [`features_enabled_by_version`] when a version pragma is
    /// encountered. Entries are static string slices from the known feature
    /// table. `use feature` declarations are **not** tracked here — only
    /// version-bundle implications.
    pub features: Vec<&'static str>,
    /// Lexically imported builtin short names from `use builtin`.
    pub builtin_imports: Vec<String>,
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

    /// Returns `true` if the given feature name is in the version-implied feature set.
    ///
    /// This only reflects features implied by `use vX.Y` version declarations.
    /// Explicit `use feature 'X'` calls are not tracked here.
    #[must_use]
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.contains(&feature)
    }

    /// Returns `true` when a builtin short name was lexically imported in scope.
    #[must_use]
    pub fn has_builtin_import(&self, name: &str) -> bool {
        self.builtin_imports.iter().any(|import| import == name)
    }
}

/// Parse a Perl version string into a major/minor pair.
///
/// Handles lexical version pragmas such as:
/// - `v5.36`
/// - `v5.36.0`
/// - `5.036`
/// - `5.10`
/// - developer releases like `5.012_001`
pub fn parse_perl_version(module: &str) -> Option<PerlVersion> {
    let s = module.strip_prefix('v').unwrap_or(module);

    let parts: Vec<&str> = s.splitn(3, '.').collect();
    let major: u32 = parse_version_component(parts.first()?)?;
    let minor: u32 = match parts.get(1) {
        Some(part) => parse_version_component(part)?,
        None => 0,
    };

    Some(PerlVersion { major, minor })
}

fn parse_version_component(component: &str) -> Option<u32> {
    let component = component.split_once('_').map_or(component, |(head, _)| head);
    component.parse().ok()
}

/// Whether `use VERSION` implies `strict` for this version.
#[must_use]
pub fn version_implies_strict(version: PerlVersion) -> bool {
    version >= PerlVersion::new(5, 12)
}

/// Whether `use VERSION` implies `warnings` for this version.
#[must_use]
pub fn version_implies_warnings(version: PerlVersion) -> bool {
    version >= PerlVersion::new(5, 35)
}

/// Returns the language features implicitly enabled by declaring `use VERSION`.
///
/// Mirrors the Perl `feature` pragma bundle semantics: each `use vX.Y`
/// declaration implicitly enables the same features as `use feature ':X.Y'`.
/// Features that were removed from a bundle (e.g. `switch` removed in v5.38)
/// are **not** included for that version and above.
///
/// Reference: <https://perldoc.perl.org/feature#FEATURE-BUNDLES>
#[must_use]
pub fn features_enabled_by_version(version: PerlVersion) -> Vec<&'static str> {
    let mut features = Vec::new();

    // v5.10 bundle: say, state, switch (given/when)
    if version >= PerlVersion::new(5, 10) {
        features.extend_from_slice(&["say", "state", "switch"]);
    }

    // v5.14 bundle adds: unicode_strings
    if version >= PerlVersion::new(5, 14) {
        features.push("unicode_strings");
    }

    // v5.16 bundle adds: unicode_eval, evalbytes, current_sub, fc
    if version >= PerlVersion::new(5, 16) {
        features.extend_from_slice(&["unicode_eval", "evalbytes", "current_sub", "fc"]);
    }

    // v5.20 bundle adds: postfix_deref (experimental; stable-bundled at v5.26)
    // We track it from v5.20 so explicit `use feature 'postfix_deref'` on v5.20 works.
    if version >= PerlVersion::new(5, 20) {
        features.push("postfix_deref");
    }

    // v5.34 bundle adds: try (experimental)
    if version >= PerlVersion::new(5, 34) {
        features.push("try");
    }

    // v5.36 bundle adds: signatures (stable), defer, isa
    if version >= PerlVersion::new(5, 36) {
        features.extend_from_slice(&["signatures", "defer", "isa"]);
    }

    // v5.38 bundle adds: class, field, method; removes: switch (given/when deprecated)
    if version >= PerlVersion::new(5, 38) {
        features.extend_from_slice(&["class", "field", "method"]);
        features.retain(|&f| f != "switch");
    }

    // v5.40 bundle adds: builtin
    if version >= PerlVersion::new(5, 40) {
        features.push("builtin");
    }

    features
}

fn enable_effective_version_semantics(state: &mut PragmaState, version: PerlVersion) {
    if version_implies_strict(version) {
        state.strict_vars = true;
        state.strict_subs = true;
        state.strict_refs = true;
    }
    if version_implies_warnings(version) {
        state.warnings = true;
    }
    // Populate the version-implied feature set.
    // Replace (not merge) so the highest `use vX.Y` wins if multiple appear.
    state.features = features_enabled_by_version(version);
}

fn builtin_import_names(arg: &str) -> Vec<String> {
    let trimmed = arg.trim();

    if let Some(inner) = trimmed.strip_prefix("qw(").and_then(|s| s.strip_suffix(')')) {
        return inner
            .split_whitespace()
            .filter(|name| !name.is_empty())
            .map(|name| name.trim_matches('\'').trim_matches('"').to_string())
            .collect();
    }

    let name = trimmed.trim_matches('\'').trim_matches('"');
    if name.is_empty() { Vec::new() } else { vec![name.to_string()] }
}

fn apply_builtin_imports(state: &mut PragmaState, args: &[String]) {
    for arg in args {
        for name in builtin_import_names(arg) {
            if !state.builtin_imports.iter().any(|import| import == &name) {
                state.builtin_imports.push(name);
            }
        }
    }
}

fn pragma_arg_items(arg: &str) -> Vec<String> {
    let trimmed = arg.trim().trim_matches('\'').trim_matches('"');

    if let Some(inner) = trimmed.strip_prefix("qw(").and_then(|s| s.strip_suffix(')')) {
        return inner.split_whitespace().map(|item| item.to_string()).collect();
    }

    vec![trimmed.to_string()]
}

/// Tracks pragma state throughout a Perl file
pub struct PragmaTracker;

impl PragmaTracker {
    /// Build a range-indexed pragma map from an AST
    pub fn build(ast: &Node) -> Vec<(Range<usize>, PragmaState)> {
        let mut ranges = Vec::new();
        let mut current_state = PragmaState::default();

        // Build the pragma map by walking the AST
        Self::build_ranges(ast, &mut current_state, &mut ranges);

        // Sort by start offset
        ranges.sort_by_key(|(range, _)| range.start);

        ranges
    }

    /// Get the pragma state at a specific byte offset
    pub fn state_for_offset(
        pragma_map: &[(Range<usize>, PragmaState)],
        offset: usize,
    ) -> PragmaState {
        // Find the last pragma state that starts before this offset.
        // pragma_map is sorted by start offset (guaranteed by build()).
        // We use partition_point to find the first element where start > offset,
        // then take the element before it.
        let idx = pragma_map.partition_point(|(range, _)| range.start <= offset);

        if idx > 0 { pragma_map[idx - 1].1.clone() } else { PragmaState::default() }
    }

    fn build_ranges(
        node: &Node,
        current_state: &mut PragmaState,
        ranges: &mut Vec<(Range<usize>, PragmaState)>,
    ) {
        match &node.kind {
            NodeKind::Use { module, args, .. } => {
                // Handle use statements
                match module.as_str() {
                    "strict" => {
                        if args.is_empty() {
                            // use strict; enables all categories
                            current_state.strict_vars = true;
                            current_state.strict_subs = true;
                            current_state.strict_refs = true;
                        } else {
                            // Parse specific categories
                            for arg in args {
                                match arg.as_str() {
                                    "vars" | "'vars'" | "\"vars\"" => {
                                        current_state.strict_vars = true
                                    }
                                    "subs" | "'subs'" | "\"subs\"" => {
                                        current_state.strict_subs = true
                                    }
                                    "refs" | "'refs'" | "\"refs\"" => {
                                        current_state.strict_refs = true
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Record the state change at this location
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    "warnings" => {
                        current_state.warnings = true;
                        // `use warnings` re-enables all warnings; clear any previously
                        // disabled categories so they are active again.
                        current_state.disabled_warning_categories.clear();
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    "utf8" => {
                        current_state.utf8 = true;
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    "encoding" => {
                        current_state.encoding = args
                            .first()
                            .map(|arg| arg.trim().trim_matches('\'').trim_matches('"').to_string());
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    "locale" => {
                        current_state.locale = true;
                        current_state.locale_scope = args
                            .first()
                            .map(|arg| arg.trim().trim_matches('\'').trim_matches('"').to_string());
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    "feature" => {
                        // Track the small set of feature pragma effects that this
                        // crate currently needs for semantic consumers.
                        let mut changed = false;
                        for arg in args {
                            for item in pragma_arg_items(arg) {
                                match item.as_str() {
                                    "signatures" => {
                                        current_state.strict_vars = true;
                                        current_state.strict_subs = true;
                                        current_state.strict_refs = true;
                                        changed = true;
                                    }
                                    "unicode_strings" => {
                                        current_state.unicode_strings = true;
                                        changed = true;
                                    }
                                    item if item.starts_with(':') => {
                                        if let Some(version) =
                                            parse_perl_version(item.trim_start_matches(':'))
                                        {
                                            if version >= PerlVersion::new(5, 12) {
                                                current_state.unicode_strings = true;
                                                changed = true;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }

                        if changed {
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                        }
                    }
                    "builtin" => {
                        apply_builtin_imports(current_state, args);
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    _ => {
                        if let Some(version) = parse_perl_version(module) {
                            enable_effective_version_semantics(current_state, version);
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                        }
                    }
                }
            }
            NodeKind::No { module, args, .. } => {
                // Handle no statements
                match module.as_str() {
                    "strict" => {
                        if args.is_empty() {
                            // no strict; disables all categories
                            current_state.strict_vars = false;
                            current_state.strict_subs = false;
                            current_state.strict_refs = false;
                        } else {
                            // Parse specific categories
                            for arg in args {
                                match arg.as_str() {
                                    "vars" | "'vars'" | "\"vars\"" => {
                                        current_state.strict_vars = false
                                    }
                                    "subs" | "'subs'" | "\"subs\"" => {
                                        current_state.strict_subs = false
                                    }
                                    "refs" | "'refs'" | "\"refs\"" => {
                                        current_state.strict_refs = false
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Record the state change at this location
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    "warnings" => {
                        if args.is_empty() {
                            // `no warnings;` — disable all warnings globally
                            current_state.warnings = false;
                            current_state.disabled_warning_categories.clear();
                        } else {
                            // `no warnings 'CATEGORY'` — disable only the named
                            // categories; the global flag stays true so that other
                            // categories remain active.
                            for arg in args {
                                // Strip any surrounding single or double quotes that
                                // the parser may have left on the argument.
                                let category = arg.trim_matches('\'').trim_matches('"');
                                if !current_state
                                    .disabled_warning_categories
                                    .iter()
                                    .any(|c| c == category)
                                {
                                    current_state
                                        .disabled_warning_categories
                                        .push(category.to_string());
                                }
                            }
                        }
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    "utf8" => {
                        current_state.utf8 = false;
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    "encoding" => {
                        current_state.encoding = None;
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    "locale" => {
                        current_state.locale = false;
                        current_state.locale_scope = None;
                        ranges
                            .push((node.location.start..node.location.end, current_state.clone()));
                    }
                    _ => {}
                }
            }
            NodeKind::Block { statements } => {
                // Save current state
                let saved_state = current_state.clone();

                // Process statements in the block
                for stmt in statements {
                    Self::build_ranges(stmt, current_state, ranges);
                }

                // Restore state after block
                *current_state = saved_state;
            }
            NodeKind::Program { statements } => {
                // Process all top-level statements
                for stmt in statements {
                    Self::build_ranges(stmt, current_state, ranges);
                }
            }
            // For subroutines and other container nodes, recurse into their bodies
            NodeKind::Subroutine { body, .. } => {
                Self::build_ranges(body, current_state, ranges);
            }
            NodeKind::If { then_branch, else_branch, .. } => {
                Self::build_ranges(then_branch, current_state, ranges);
                if let Some(else_b) = else_branch {
                    Self::build_ranges(else_b, current_state, ranges);
                }
            }
            NodeKind::While { body, .. }
            | NodeKind::For { body, .. }
            | NodeKind::Foreach { body, .. } => {
                Self::build_ranges(body, current_state, ranges);
            }
            // `package Foo { ... }` — the block form is lexically scoped.
            // Save/restore state around the block so pragmas declared inside
            // don't leak out, just like a regular braced block.
            //
            // `package Foo;` (no block) has no inner scope to walk — its
            // siblings in `Program` already accumulate state normally.
            NodeKind::Package { block: Some(pkg_block), .. } => {
                let saved_state = current_state.clone();
                Self::build_ranges(pkg_block, current_state, ranges);
                *current_state = saved_state;
            }
            // Other node types don't contain use/no statements
            _ => {}
        }
    }
}
