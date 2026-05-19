//! Pragma tracker for Perl code analysis
//!
//! Tracks `use` and `no` pragmas throughout the codebase to determine
//! effective pragma state at any point in the code.

use perl_ast::ast::Node;
use std::ops::Range;

mod range_builder;

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

/// Query object describing compile-time pragma state at a byte offset.
#[derive(Debug, Clone, PartialEq)]
pub struct PragmaStateQuery {
    offset: usize,
    snapshot: PragmaSnapshot,
}

impl PragmaStateQuery {
    /// Byte offset this query was created for.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Immutable snapshot at this query position.
    #[must_use]
    pub fn snapshot(&self) -> &PragmaSnapshot {
        &self.snapshot
    }
}

/// Explicit compile-time pragma environment that can answer file-position
/// queries and expose immutable snapshots.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompileTimePragmaEnvironment {
    map: PragmaMap,
}

/// One effective pragma-state transition over a source byte range.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PragmaEntry {
    /// Source byte range covered by this snapshot.
    ///
    /// Lexical scope restores are represented as zero-length ranges at the
    /// scope end so callers can observe the restored state at that byte offset.
    pub range: Range<usize>,
    /// Immutable pragma state active for this transition.
    pub snapshot: PragmaSnapshot,
}

/// Explicit pragma transition timeline.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct PragmaMap {
    entries: Box<[PragmaEntry]>,
}

impl CompileTimePragmaEnvironment {
    /// Build a queryable environment from an AST.
    #[must_use]
    pub fn build(ast: &Node) -> Self {
        let mut ranges = Vec::new();
        let mut current_state = PragmaState::default();
        range_builder::build_ranges(ast, &mut current_state, &mut ranges);
        ranges.sort_by_key(|(range, _)| range.start);

        let entries = ranges
            .into_iter()
            .map(|(range, state)| PragmaEntry { range, snapshot: PragmaSnapshot::from(state) })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self { map: PragmaMap { entries } }
    }

    /// Return a position query object with immutable state snapshot.
    #[must_use]
    pub fn query_at(&self, offset: usize) -> PragmaStateQuery {
        PragmaStateQuery { offset, snapshot: self.snapshot_at(offset) }
    }

    /// Return the immutable snapshot active at the given byte offset.
    #[must_use]
    pub fn snapshot_at(&self, offset: usize) -> PragmaSnapshot {
        self.map.snapshot_at(offset)
    }

    /// Access the underlying transition map.
    #[must_use]
    pub fn map(&self) -> &PragmaMap {
        &self.map
    }

    /// Access the underlying range map for advanced consumers.
    #[must_use]
    pub fn as_map(&self) -> Vec<(Range<usize>, PragmaSnapshot)> {
        self.map.to_tuples()
    }
}

impl PragmaMap {
    /// Return the immutable snapshot active at the given byte offset.
    #[must_use]
    pub fn snapshot_at(&self, offset: usize) -> PragmaSnapshot {
        let idx = self.entries.partition_point(|entry| entry.range.start <= offset);
        let snapshot = if idx > 0 {
            self.entries[idx - 1].snapshot.clone()
        } else {
            PragmaSnapshot::default()
        };

        normalize_snapshot(snapshot)
    }

    /// Return the concrete pragma state active at the given byte offset.
    #[must_use]
    pub fn state_at(&self, offset: usize) -> PragmaState {
        self.snapshot_at(offset).into()
    }

    /// Return the final top-level pragma state after all lexical restores.
    #[must_use]
    pub fn final_state(&self) -> PragmaState {
        let state = self
            .entries
            .last()
            .map_or_else(PragmaState::default, |entry| entry.snapshot.clone().into());

        normalize_state(state)
    }

    /// Create a cursor for monotonic queries against this map.
    #[must_use]
    pub fn cursor(&self) -> PragmaQueryCursor {
        PragmaQueryCursor::new()
    }

    /// Return all transition entries in source order.
    #[must_use]
    pub fn entries(&self) -> &[PragmaEntry] {
        &self.entries
    }

    /// Convert this map to the legacy tuple representation.
    #[must_use]
    pub fn to_tuples(&self) -> Vec<(Range<usize>, PragmaSnapshot)> {
        self.entries.iter().map(|e| (e.range.clone(), e.snapshot.clone())).collect()
    }
}

fn normalize_snapshot(mut snapshot: PragmaSnapshot) -> PragmaSnapshot {
    if snapshot.state.signatures_strict {
        snapshot.state.strict_vars = true;
        snapshot.state.strict_subs = true;
        snapshot.state.strict_refs = true;
    }

    snapshot
}

fn normalize_state(mut state: PragmaState) -> PragmaState {
    if state.signatures_strict {
        state.strict_vars = true;
        state.strict_subs = true;
        state.strict_refs = true;
    }

    state
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
/// Features that were removed from a bundle (for example `switch` removed in
/// v5.36 and `smartmatch` removed in v5.42) are **not** included for that
/// version and above. Versions older than v5.10 load the `:default` bundle.
///
/// Reference: <https://perldoc.perl.org/feature#FEATURE-BUNDLES>
#[must_use]
pub fn features_enabled_by_version(version: PerlVersion) -> Vec<&'static str> {
    let bundle = if version < PerlVersion::new(5, 10) {
        DEFAULT_FEATURES
    } else if version >= PerlVersion::new(5, 42) {
        BUNDLE_5_42_FEATURES
    } else if version >= PerlVersion::new(5, 40) {
        BUNDLE_5_40_FEATURES
    } else if version >= PerlVersion::new(5, 38) {
        BUNDLE_5_38_FEATURES
    } else if version >= PerlVersion::new(5, 36) {
        BUNDLE_5_36_FEATURES
    } else if version >= PerlVersion::new(5, 34) {
        BUNDLE_5_34_FEATURES
    } else if version >= PerlVersion::new(5, 28) {
        BUNDLE_5_28_FEATURES
    } else if version >= PerlVersion::new(5, 24) {
        BUNDLE_5_24_FEATURES
    } else if version >= PerlVersion::new(5, 16) {
        BUNDLE_5_16_FEATURES
    } else if version >= PerlVersion::new(5, 12) {
        BUNDLE_5_12_FEATURES
    } else {
        BUNDLE_5_10_FEATURES
    };

    bundle.to_vec()
}

const DEFAULT_FEATURES: &[&str] = &[
    "indirect",
    "multidimensional",
    "bareword_filehandles",
    "apostrophe_as_package_separator",
    "smartmatch",
];

const BUNDLE_5_10_FEATURES: &[&str] = &[
    "apostrophe_as_package_separator",
    "bareword_filehandles",
    "indirect",
    "multidimensional",
    "say",
    "smartmatch",
    "state",
    "switch",
];

const BUNDLE_5_12_FEATURES: &[&str] = &[
    "apostrophe_as_package_separator",
    "bareword_filehandles",
    "indirect",
    "multidimensional",
    "say",
    "smartmatch",
    "state",
    "switch",
    "unicode_strings",
];

const BUNDLE_5_16_FEATURES: &[&str] = &[
    "apostrophe_as_package_separator",
    "bareword_filehandles",
    "current_sub",
    "evalbytes",
    "fc",
    "indirect",
    "multidimensional",
    "say",
    "smartmatch",
    "state",
    "switch",
    "unicode_eval",
    "unicode_strings",
];

const BUNDLE_5_24_FEATURES: &[&str] = &[
    "apostrophe_as_package_separator",
    "bareword_filehandles",
    "current_sub",
    "evalbytes",
    "fc",
    "indirect",
    "multidimensional",
    "postderef_qq",
    "say",
    "smartmatch",
    "state",
    "switch",
    "unicode_eval",
    "unicode_strings",
];

const BUNDLE_5_28_FEATURES: &[&str] = &[
    "apostrophe_as_package_separator",
    "bareword_filehandles",
    "bitwise",
    "current_sub",
    "evalbytes",
    "fc",
    "indirect",
    "multidimensional",
    "postderef_qq",
    "say",
    "smartmatch",
    "state",
    "switch",
    "unicode_eval",
    "unicode_strings",
];

const BUNDLE_5_34_FEATURES: &[&str] = BUNDLE_5_28_FEATURES;

const BUNDLE_5_36_FEATURES: &[&str] = &[
    "apostrophe_as_package_separator",
    "bareword_filehandles",
    "bitwise",
    "current_sub",
    "evalbytes",
    "fc",
    "isa",
    "postderef_qq",
    "say",
    "signatures",
    "smartmatch",
    "state",
    "unicode_eval",
    "unicode_strings",
];

const BUNDLE_5_38_FEATURES: &[&str] = &[
    "apostrophe_as_package_separator",
    "bitwise",
    "current_sub",
    "evalbytes",
    "fc",
    "isa",
    "module_true",
    "postderef_qq",
    "say",
    "signatures",
    "smartmatch",
    "state",
    "unicode_eval",
    "unicode_strings",
];

const BUNDLE_5_40_FEATURES: &[&str] = &[
    "apostrophe_as_package_separator",
    "bitwise",
    "current_sub",
    "evalbytes",
    "fc",
    "isa",
    "module_true",
    "postderef_qq",
    "say",
    "signatures",
    "smartmatch",
    "state",
    "try",
    "unicode_eval",
    "unicode_strings",
];

const BUNDLE_5_42_FEATURES: &[&str] = &[
    "bitwise",
    "current_sub",
    "evalbytes",
    "fc",
    "isa",
    "module_true",
    "postderef_qq",
    "say",
    "signatures",
    "state",
    "try",
    "unicode_eval",
    "unicode_strings",
];

pub(crate) fn enable_effective_version_semantics(state: &mut PragmaState, version: PerlVersion) {
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
        "smartmatch" => Some("smartmatch"),
        "unicode_strings" => Some("unicode_strings"),
        "unicode_eval" => Some("unicode_eval"),
        "evalbytes" => Some("evalbytes"),
        "current_sub" => Some("current_sub"),
        "fc" => Some("fc"),
        "lexical_subs" => Some("lexical_subs"),
        "postderef" => Some("postderef"),
        "postderef_qq" | "postfix_deref" => Some("postderef_qq"),
        "refaliasing" => Some("refaliasing"),
        "bitwise" => Some("bitwise"),
        "declared_refs" => Some("declared_refs"),
        "try" => Some("try"),
        "signatures" => Some("signatures"),
        "defer" => Some("defer"),
        "isa" => Some("isa"),
        "class" => Some("class"),
        "field" => Some("field"),
        "method" => Some("method"),
        "builtin" | "module_true" => Some("module_true"),
        "indirect" => Some("indirect"),
        "multidimensional" => Some("multidimensional"),
        "bareword_filehandles" => Some("bareword_filehandles"),
        "extra_paired_delimiters" => Some("extra_paired_delimiters"),
        "apostrophe_as_package_separator" => Some("apostrophe_as_package_separator"),
        "keyword_any" => Some("keyword_any"),
        "keyword_all" => Some("keyword_all"),
        _ => None,
    }
}

const ALL_KNOWN_FEATURES: &[&str] = &[
    "say",
    "state",
    "smartmatch",
    "switch",
    "unicode_strings",
    "unicode_eval",
    "evalbytes",
    "current_sub",
    "fc",
    "lexical_subs",
    "postderef",
    "postderef_qq",
    "signatures",
    "refaliasing",
    "bitwise",
    "declared_refs",
    "isa",
    "indirect",
    "multidimensional",
    "bareword_filehandles",
    "try",
    "defer",
    "extra_paired_delimiters",
    "module_true",
    "class",
    "field",
    "method",
    "apostrophe_as_package_separator",
    "keyword_any",
    "keyword_all",
];

fn canonical_feature_query(feature: &str) -> &str {
    match feature {
        "builtin" => "module_true",
        "postfix_deref" => "postderef_qq",
        _ => feature,
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

pub(crate) fn apply_feature_state(state: &mut PragmaState, args: &[String], enabled: bool) -> bool {
    if !enabled && args.is_empty() {
        let default_features = DEFAULT_FEATURES.to_vec();
        let changed =
            state.features != default_features || state.unicode_strings || state.signatures_strict;
        state.features = default_features;
        state.unicode_strings = state.has_feature("unicode_strings");
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

            if enabled && item == ":default" {
                for feature in DEFAULT_FEATURES {
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

            if !enabled && item == ":default" {
                for feature in DEFAULT_FEATURES {
                    changed |= disable_feature_name(state, feature);
                }
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
    let trimmed = normalized_pragma_token(arg);

    if let Some(inner) = qw_list_inner(trimmed) {
        return pragma_words(inner).into_iter().map(|name| name.to_string()).collect();
    }

    if trimmed.is_empty() { Vec::new() } else { vec![trimmed.to_string()] }
}

pub(crate) fn apply_builtin_imports(state: &mut PragmaState, args: &[String]) {
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
pub(crate) fn add_disabled_warning_category(state: &mut PragmaState, category: &str) {
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

pub(crate) fn remove_builtin_imports(state: &mut PragmaState, args: &[String]) {
    if args.is_empty() {
        state.builtin_imports.clear();
        return;
    }

    let names_to_remove: Vec<String> =
        args.iter().flat_map(|arg| builtin_import_names(arg)).collect();
    state.builtin_imports.retain(|import| !names_to_remove.iter().any(|name| name == import));
}

pub(crate) fn pragma_arg_items(arg: &str) -> Vec<String> {
    let trimmed = normalized_pragma_token(arg);

    if let Some(inner) = qw_list_inner(trimmed) {
        return pragma_words(inner).into_iter().map(|item| item.to_string()).collect();
    }

    if trimmed.contains(char::is_whitespace) {
        return pragma_words(trimmed).into_iter().map(|item| item.to_string()).collect();
    }

    vec![trimmed.to_string()]
}

fn qw_list_inner(arg: &str) -> Option<&str> {
    let rest = arg.strip_prefix("qw")?.trim_start();
    let opener = rest.chars().next()?;
    let closer = qw_closer(opener)?;
    let after_opener = &rest[opener.len_utf8()..];

    after_opener.strip_suffix(closer)
}

fn qw_closer(opener: char) -> Option<char> {
    match opener {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '<' => Some('>'),
        delimiter if !delimiter.is_alphanumeric() && !delimiter.is_whitespace() => Some(delimiter),
        _ => None,
    }
}

fn pragma_words(value: &str) -> Vec<&str> {
    value
        .split_whitespace()
        .map(|item| item.trim_matches('\'').trim_matches('"'))
        .filter(|item| !item.is_empty())
        .collect()
}

pub(crate) fn normalized_pragma_token(arg: &str) -> &str {
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

pub(crate) fn conditional_pragma_target(args: &[String]) -> Option<(&str, &[String])> {
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

/// Monotonic query cursor for repeated pragma lookups.
///
/// Reuse a single cursor when querying offsets in non-decreasing order to
/// avoid repeated binary searches over the pragma map.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PragmaQueryCursor {
    index: usize,
}

impl PragmaQueryCursor {
    /// Create a new cursor positioned before the start of the map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Query state at `offset` assuming lookups are mostly non-decreasing.
    ///
    /// This is the primary cursor API for the explicit pragma map.
    /// If the caller queries an older offset, this falls back to a binary
    /// search and repositions the cursor.
    pub fn snapshot_at(&mut self, pragma_map: &PragmaMap, offset: usize) -> PragmaSnapshot {
        let snapshot = self
            .entry_for_offset(pragma_map.entries(), offset)
            .map_or_else(PragmaSnapshot::default, |entry| entry.snapshot.clone());

        normalize_snapshot(snapshot)
    }

    /// Query state at `offset` against the explicit pragma map.
    pub fn state_at(&mut self, pragma_map: &PragmaMap, offset: usize) -> PragmaState {
        self.snapshot_at(pragma_map, offset).into()
    }

    /// Query state at `offset` assuming lookups are mostly non-decreasing.
    ///
    /// This legacy tuple API is retained for existing `PragmaTracker` callers.
    /// If the caller queries an older offset, this falls back to a binary
    /// search and repositions the cursor.
    pub fn state_for_offset(
        &mut self,
        pragma_map: &[(Range<usize>, PragmaState)],
        offset: usize,
    ) -> PragmaState {
        if pragma_map.is_empty() {
            return PragmaState::default();
        }

        if self.index >= pragma_map.len() {
            self.index = pragma_map.len() - 1;
        }

        if pragma_map[self.index].0.start > offset {
            self.index = pragma_map.partition_point(|(range, _)| range.start <= offset);
            if self.index > 0 {
                self.index -= 1;
            }
        } else {
            while self.index + 1 < pragma_map.len() && pragma_map[self.index + 1].0.start <= offset
            {
                self.index += 1;
            }
        }

        let state = if pragma_map[self.index].0.start <= offset {
            pragma_map[self.index].1.clone()
        } else {
            PragmaState::default()
        };

        normalize_state(state)
    }

    fn entry_for_offset<'a>(
        &mut self,
        entries: &'a [PragmaEntry],
        offset: usize,
    ) -> Option<&'a PragmaEntry> {
        if entries.is_empty() {
            return None;
        }

        if self.index >= entries.len() {
            self.index = entries.len() - 1;
        }

        if entries[self.index].range.start > offset {
            self.index = entries.partition_point(|entry| entry.range.start <= offset);
            if self.index > 0 {
                self.index -= 1;
            }
        } else {
            while self.index + 1 < entries.len() && entries[self.index + 1].range.start <= offset {
                self.index += 1;
            }
        }

        if entries[self.index].range.start <= offset { Some(&entries[self.index]) } else { None }
    }
}

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
