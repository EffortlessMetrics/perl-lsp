//! Pragma tracker for Perl code analysis
//!
//! Tracks `use` and `no` pragmas throughout the codebase to determine
//! effective pragma state at any point in the code.

use perl_ast::ast::{Node, NodeKind};
use std::ops::Range;

const MAX_DISABLED_WARNING_CATEGORIES: usize = 256;

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
    let mut parts = s.splitn(3, '.');

    let major = parse_version_component(parts.next()?)?;
    let minor = match parts.next() {
        Some(part) => parse_version_component(part)?,
        None => 0,
    };

    Some(PerlVersion::new(major, minor))
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

    // v5.12 bundle adds: unicode_strings
    if version >= PerlVersion::new(5, 12) {
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
    state.unicode_strings = state.has_feature("unicode_strings");
    state.signatures_strict = false;
}

fn feature_items(arg: &str) -> Vec<String> {
    pragma_arg_items(arg)
}

fn known_feature_name(name: &str) -> Option<&'static str> {
    match name {
        "say" => Some("say"),
        "state" => Some("state"),
        "switch" => Some("switch"),
        "unicode_strings" => Some("unicode_strings"),
        "unicode_eval" => Some("unicode_eval"),
        "evalbytes" => Some("evalbytes"),
        "current_sub" => Some("current_sub"),
        "fc" => Some("fc"),
        "postfix_deref" => Some("postfix_deref"),
        "try" => Some("try"),
        "signatures" => Some("signatures"),
        "defer" => Some("defer"),
        "isa" => Some("isa"),
        "class" => Some("class"),
        "field" => Some("field"),
        "method" => Some("method"),
        "builtin" => Some("builtin"),
        _ => None,
    }
}

fn enable_feature_name(state: &mut PragmaState, name: &str) -> bool {
    if name == "signatures" {
        state.signatures_strict = true;
    }
    if name == "unicode_strings" {
        state.unicode_strings = true;
    }

    if let Some(feature) = known_feature_name(name) {
        if state.features.iter().all(|existing| existing != &feature) {
            state.features.push(feature);
        }
        true
    } else {
        false
    }
}

fn disable_feature_name(state: &mut PragmaState, name: &str) -> bool {
    if name == "signatures" {
        state.signatures_strict = false;
    }
    if name == "unicode_strings" {
        state.unicode_strings = false;
    }

    if let Some(feature) = known_feature_name(name) {
        let before = state.features.len();
        state.features.retain(|existing| *existing != feature);
        before != state.features.len()
    } else {
        false
    }
}

fn apply_feature_state(state: &mut PragmaState, args: &[String], enabled: bool) -> bool {
    const ALL_KNOWN_FEATURES: &[&str] = &[
        "say",
        "state",
        "switch",
        "unicode_strings",
        "unicode_eval",
        "evalbytes",
        "current_sub",
        "fc",
        "postfix_deref",
        "try",
        "signatures",
        "defer",
        "isa",
        "class",
        "field",
        "method",
        "builtin",
    ];

    if !enabled && args.is_empty() {
        let changed =
            !state.features.is_empty() || state.unicode_strings || state.signatures_strict;
        state.features.clear();
        state.unicode_strings = false;
        state.signatures_strict = false;
        return changed;
    }

    let mut changed = false;

    for arg in args {
        for item in feature_items(arg) {
            if enabled && item == ":all" {
                for feature in ALL_KNOWN_FEATURES {
                    changed |= enable_feature_name(state, feature);
                }
                continue;
            }

            if !enabled && item == ":all" {
                let had_features =
                    !state.features.is_empty() || state.unicode_strings || state.signatures_strict;
                state.features.clear();
                state.unicode_strings = false;
                state.signatures_strict = false;
                changed |= had_features;
                continue;
            }

            if let Some(version) = item.strip_prefix(':').and_then(parse_perl_version) {
                for feature in features_enabled_by_version(version) {
                    changed |= if enabled {
                        enable_feature_name(state, feature)
                    } else {
                        disable_feature_name(state, feature)
                    };
                }
                continue;
            }

            changed |= if enabled {
                enable_feature_name(state, &item)
            } else {
                disable_feature_name(state, &item)
            };
        }
    }

    changed
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

/// Insert `category` into `state.disabled_warning_categories` if not already present and
/// within the hard cap of [`MAX_DISABLED_WARNING_CATEGORIES`].
///
/// Categories beyond the cap are silently dropped. In valid Perl code this is never reached
/// (Perl's own warning hierarchy has ~30 leaf categories); the cap is a safety guard against
/// pathological or adversarial AST input that would otherwise cause O(n²) clone cost.
fn add_disabled_warning_category(state: &mut PragmaState, category: &str) {
    if category.is_empty() {
        return;
    }

    if state.disabled_warning_categories.iter().any(|c| c == category) {
        return;
    }

    if state.disabled_warning_categories.len() >= MAX_DISABLED_WARNING_CATEGORIES {
        return;
    }

    state.disabled_warning_categories.push(category.to_string());
}

fn pragma_arg_items(arg: &str) -> Vec<String> {
    let trimmed = arg.trim().trim_matches('\'').trim_matches('"');

    if let Some(inner) = trimmed.strip_prefix("qw(").and_then(|s| s.strip_suffix(')')) {
        return inner.split_whitespace().map(|item| item.to_string()).collect();
    }

    vec![trimmed.to_string()]
}

fn normalized_pragma_token(arg: &str) -> &str {
    arg.trim().trim_matches('\'').trim_matches('"')
}

fn is_tracked_pragma_module(module: &str) -> bool {
    matches!(module, "strict" | "warnings" | "utf8" | "encoding" | "locale" | "feature" | "builtin")
}

fn valid_strict_args(args: &[String]) -> bool {
    args.iter()
        .flat_map(|arg| pragma_arg_items(arg))
        .all(|item| matches!(item.as_str(), "vars" | "subs" | "refs"))
}

fn conditional_target_tail_is_valid(module: &str, tail: &[String]) -> bool {
    if parse_perl_version(module).is_some() {
        return tail.is_empty();
    }

    match module {
        "strict" => tail.is_empty() || valid_strict_args(tail),
        "warnings" => true,
        "utf8" => tail.is_empty(),
        "encoding" => tail.len() == 1 && !normalized_pragma_token(&tail[0]).is_empty(),
        "locale" => {
            tail.is_empty() || (tail.len() == 1 && !normalized_pragma_token(&tail[0]).is_empty())
        }
        "feature" => !tail.is_empty(),
        "builtin" => tail.iter().any(|arg| !builtin_import_names(arg).is_empty()),
        _ => false,
    }
}

fn conditional_pragma_target(args: &[String]) -> Option<(&str, &[String])> {
    args.iter().enumerate().find_map(|(idx, arg)| {
        let module = normalized_pragma_token(arg);
        let tail = &args[idx + 1..];
        if (is_tracked_pragma_module(module) || parse_perl_version(module).is_some())
            && conditional_target_tail_is_valid(module, tail)
        {
            Some((module, tail))
        } else {
            None
        }
    })
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

        let mut state =
            if idx > 0 { pragma_map[idx - 1].1.clone() } else { PragmaState::default() };

        if state.signatures_strict {
            state.strict_vars = true;
            state.strict_subs = true;
            state.strict_refs = true;
        }

        state
    }

    /// Process a lexically scoped body and then restore the caller state.
    ///
    /// This applies to ordinary blocks and phase blocks alike. `BEGIN`/`END`/
    /// `INIT`/`CHECK`/`UNITCHECK` execute at special times, but their pragma
    /// effects are still lexical to the block body rather than file-wide.
    fn build_scoped_body(
        body: &Node,
        current_state: &mut PragmaState,
        ranges: &mut Vec<(Range<usize>, PragmaState)>,
    ) {
        let saved_state = current_state.clone();
        Self::build_ranges(body, current_state, ranges);
        *current_state = saved_state;
        ranges.push((body.location.end..body.location.end, current_state.clone()));
    }

    fn build_ranges(
        node: &Node,
        current_state: &mut PragmaState,
        ranges: &mut Vec<(Range<usize>, PragmaState)>,
    ) {
        match &node.kind {
            NodeKind::Use { module, args, .. } => {
                if (module == "if" || module == "unless")
                    && let Some((conditional_module, conditional_args)) =
                        conditional_pragma_target(args)
                {
                    match conditional_module {
                        "strict" => {
                            if conditional_args.is_empty() {
                                current_state.strict_vars = true;
                                current_state.strict_subs = true;
                                current_state.strict_refs = true;
                            } else {
                                for arg in conditional_args {
                                    match normalized_pragma_token(arg) {
                                        "vars" => current_state.strict_vars = true,
                                        "subs" => current_state.strict_subs = true,
                                        "refs" => current_state.strict_refs = true,
                                        _ => {}
                                    }
                                }
                            }
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        "warnings" => {
                            current_state.warnings = true;
                            current_state.disabled_warning_categories.clear();
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        "utf8" => {
                            current_state.utf8 = true;
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        "encoding" => {
                            current_state.encoding = conditional_args
                                .first()
                                .map(|arg| normalized_pragma_token(arg).to_string());
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        "locale" => {
                            current_state.locale = true;
                            current_state.locale_scope = conditional_args
                                .first()
                                .map(|arg| normalized_pragma_token(arg).to_string());
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        "feature" => {
                            if apply_feature_state(current_state, conditional_args, true) {
                                ranges.push((
                                    node.location.start..node.location.end,
                                    current_state.clone(),
                                ));
                            }
                            return;
                        }
                        "builtin" => {
                            apply_builtin_imports(current_state, conditional_args);
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        _ => {
                            if let Some(version) = parse_perl_version(conditional_module) {
                                enable_effective_version_semantics(current_state, version);
                                ranges.push((
                                    node.location.start..node.location.end,
                                    current_state.clone(),
                                ));
                            }
                            return;
                        }
                    }
                }

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
                            for item in args.iter().flat_map(|arg| pragma_arg_items(arg)) {
                                match item.as_str() {
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
                        if apply_feature_state(current_state, args, true) {
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
                if (module == "if" || module == "unless")
                    && let Some((conditional_module, conditional_args)) =
                        conditional_pragma_target(args)
                {
                    match conditional_module {
                        "strict" => {
                            if conditional_args.is_empty() {
                                current_state.strict_vars = false;
                                current_state.strict_subs = false;
                                current_state.strict_refs = false;
                            } else {
                                for arg in conditional_args {
                                    match normalized_pragma_token(arg) {
                                        "vars" => current_state.strict_vars = false,
                                        "subs" => current_state.strict_subs = false,
                                        "refs" => current_state.strict_refs = false,
                                        _ => {}
                                    }
                                }
                            }
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        "warnings" => {
                            if conditional_args.is_empty() {
                                current_state.warnings = false;
                                current_state.disabled_warning_categories.clear();
                            } else {
                                for arg in conditional_args {
                                    let category = normalized_pragma_token(arg);
                                    add_disabled_warning_category(current_state, category);
                                }
                            }
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        "utf8" => {
                            current_state.utf8 = false;
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        "encoding" => {
                            current_state.encoding = None;
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        "locale" => {
                            current_state.locale = false;
                            current_state.locale_scope = None;
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        "feature" => {
                            if apply_feature_state(current_state, conditional_args, false) {
                                ranges.push((
                                    node.location.start..node.location.end,
                                    current_state.clone(),
                                ));
                            }
                            return;
                        }
                        "builtin" => {
                            if conditional_args.is_empty() {
                                current_state.builtin_imports.clear();
                            } else {
                                for arg in conditional_args {
                                    for name in builtin_import_names(arg) {
                                        current_state
                                            .builtin_imports
                                            .retain(|existing| existing != &name);
                                    }
                                }
                            }
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                            return;
                        }
                        _ => return,
                    }
                }

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
                            for item in args.iter().flat_map(|arg| pragma_arg_items(arg)) {
                                match item.as_str() {
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
                                add_disabled_warning_category(current_state, category);
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
                    "feature" => {
                        if apply_feature_state(current_state, args, false) {
                            ranges.push((
                                node.location.start..node.location.end,
                                current_state.clone(),
                            ));
                        }
                    }
                    "builtin" => {
                        if args.is_empty() {
                            current_state.builtin_imports.clear();
                        } else {
                            for arg in args {
                                for name in builtin_import_names(arg) {
                                    current_state
                                        .builtin_imports
                                        .retain(|existing| existing != &name);
                                }
                            }
                        }
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
                ranges.push((node.location.end..node.location.end, current_state.clone()));
            }
            NodeKind::Program { statements } => {
                // Process all top-level statements
                for stmt in statements {
                    Self::build_ranges(stmt, current_state, ranges);
                }
            }
            // For subroutines and other container nodes, recurse into their bodies
            NodeKind::Subroutine { body, .. } => {
                Self::build_scoped_body(body, current_state, ranges);
            }
            NodeKind::Method { body, .. } => {
                Self::build_scoped_body(body, current_state, ranges);
            }
            NodeKind::Class { body, .. } => {
                Self::build_scoped_body(body, current_state, ranges);
            }
            NodeKind::If { then_branch, elsif_branches, else_branch, .. } => {
                Self::build_scoped_body(then_branch, current_state, ranges);
                for (_, elsif_body) in elsif_branches {
                    Self::build_scoped_body(elsif_body, current_state, ranges);
                }
                if let Some(else_b) = else_branch {
                    Self::build_scoped_body(else_b, current_state, ranges);
                }
            }
            NodeKind::While { body, continue_block, .. }
            | NodeKind::For { body, continue_block, .. }
            | NodeKind::Foreach { body, continue_block, .. } => {
                Self::build_scoped_body(body, current_state, ranges);
                if let Some(continue_block) = continue_block {
                    Self::build_scoped_body(continue_block, current_state, ranges);
                }
            }
            NodeKind::Eval { block } => {
                if matches!(block.kind, NodeKind::Block { .. }) {
                    Self::build_scoped_body(block, current_state, ranges);
                }
            }
            NodeKind::Do { block } | NodeKind::Defer { block } => {
                Self::build_scoped_body(block, current_state, ranges);
            }
            NodeKind::PhaseBlock { block, .. } => {
                Self::build_scoped_body(block, current_state, ranges);
            }
            NodeKind::Given { body, .. }
            | NodeKind::When { body, .. }
            | NodeKind::Default { body } => {
                Self::build_scoped_body(body, current_state, ranges);
            }
            NodeKind::Try { body, catch_blocks, finally_block } => {
                Self::build_scoped_body(body, current_state, ranges);
                for (_, catch_body) in catch_blocks {
                    Self::build_scoped_body(catch_body, current_state, ranges);
                }
                if let Some(finally_block) = finally_block {
                    Self::build_scoped_body(finally_block, current_state, ranges);
                }
            }
            NodeKind::LabeledStatement { statement, .. } => {
                Self::build_ranges(statement, current_state, ranges);
            }
            NodeKind::StatementModifier { statement, condition, .. } => {
                Self::build_ranges(statement, current_state, ranges);
                Self::build_ranges(condition, current_state, ranges);
            }
            // `package Foo { ... }` — the block form is lexically scoped.
            // Save/restore state around the block so pragmas declared inside
            // don't leak out, just like a regular braced block.
            //
            // `package Foo;` (no block) has no inner scope to walk — its
            // siblings in `Program` already accumulate state normally.
            NodeKind::Package { block: Some(pkg_block), .. } => {
                Self::build_scoped_body(pkg_block, current_state, ranges);
            }
            // Other node types don't contain use/no statements
            _ => {}
        }
    }
}
