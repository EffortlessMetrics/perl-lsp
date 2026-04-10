# EffectiveSemantics Implementation Specification

**Architecture:** 1 - EffectiveSemantics Layer  
**Date:** 2026-04-09  
**Status:** Implementation Specification  
**Source:** perl-lsp-architecture-rfc.md Architecture 1

---

## 1. Data Structures (Rust Code)

### 1.1 PerlVersion

Location: `crates/perl-pragma/src/version.rs` (new file) or inline in `lib.rs`

```rust
use std::fmt;
use std::str::FromStr;

/// A Perl version in comparable form.
/// Supports forms: v5.36, 5.036, v5.36.0, 5.036000
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PerlVersion {
    pub major: u32,
    pub minor: u32,
    /// Patch version (None for "v5.36", Some(0) for "v5.36.0")
    pub patch: Option<u32>,
}

impl PerlVersion {
    // Common version constants
    pub const V5_8: Self = Self { major: 5, minor: 8, patch: None };
    pub const V5_10: Self = Self { major: 5, minor: 10, patch: None };
    pub const V5_12: Self = Self { major: 5, minor: 12, patch: None };
    pub const V5_14: Self = Self { major: 5, minor: 14, patch: None };
    pub const V5_16: Self = Self { major: 5, minor: 16, patch: None };
    pub const V5_18: Self = Self { major: 5, minor: 18, patch: None };
    pub const V5_20: Self = Self { major: 5, minor: 20, patch: None };
    pub const V5_22: Self = Self { major: 5, minor: 22, patch: None };
    pub const V5_24: Self = Self { major: 5, minor: 24, patch: None };
    pub const V5_26: Self = Self { major: 5, minor: 26, patch: None };
    pub const V5_28: Self = Self { major: 5, minor: 28, patch: None };
    pub const V5_30: Self = Self { major: 5, minor: 30, patch: None };
    pub const V5_32: Self = Self { major: 5, minor: 32, patch: None };
    pub const V5_34: Self = Self { major: 5, minor: 34, patch: None };
    pub const V5_36: Self = Self { major: 5, minor: 36, patch: None };
    pub const V5_38: Self = Self { major: 5, minor: 38, patch: None };
    pub const V5_40: Self = Self { major: 5, minor: 40, patch: None };

    /// Parse from various Perl version string formats.
    /// 
    /// Supported formats:
    /// - "v5.36" → PerlVersion { major: 5, minor: 36, patch: None }
    /// - "5.036" → PerlVersion { major: 5, minor: 36, patch: None }
    /// - "v5.36.0" → PerlVersion { major: 5, minor: 36, patch: Some(0) }
    /// - "5.036000" → PerlVersion { major: 5, minor: 36, patch: Some(0) }
    /// - "v5.36.1" → PerlVersion { major: 5, minor: 36, patch: Some(1) }
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        
        // Handle v-string format: v5.36 or v5.36.0
        if let Some(rest) = s.strip_prefix('v') {
            let parts: Vec<&str> = rest.split('.').collect();
            match parts.len() {
                2 => {
                    let major = parts[0].parse().ok()?;
                    let minor = parts[1].parse().ok()?;
                    Some(Self { major, minor, patch: None })
                }
                3 => {
                    let major = parts[0].parse().ok()?;
                    let minor = parts[1].parse().ok()?;
                    let patch = parts[2].parse().ok()?;
                    Some(Self { major, minor, patch: Some(patch) })
                }
                _ => None,
            }
        }
        // Handle numeric format: 5.036 or 5.036000
        else if s.contains('.') {
            let parts: Vec<&str> = s.split('.').collect();
            if parts.len() != 2 {
                return None;
            }
            let major = parts[0].parse().ok()?;
            let minor_part = parts[1];
            
            // 5.036 or 5.036000 style
            if minor_part.len() == 3 {
                // 5.036 style
                let minor = minor_part.parse().ok()?;
                Some(Self { major, minor, patch: None })
            } else if minor_part.len() == 6 {
                // 5.036000 style
                let minor = minor_part[..3].parse().ok()?;
                let patch = minor_part[3..].parse().ok()?;
                Some(Self { major, minor, patch: if patch > 0 { Some(patch) } else { None } })
            } else {
                None
            }
        }
        // Handle single number: 5.036000 without decimal (shouldn't happen in practice)
        else if let Ok(num) = s.parse::<f32>() {
            let major = num as u32;
            let minor = ((num - major as f32) * 1000.0).round() as u32;
            Some(Self { major, minor, patch: None })
        } else {
            None
        }
    }

    /// Format as v-string: "v5.36"
    pub fn to_vstring(&self) -> String {
        match self.patch {
            Some(patch) => format!("v{}.{}.{}", self.major, self.minor, patch),
            None => format!("v{}.{}", self.major, self.minor),
        }
    }

    /// Format as numeric: "5.036"
    pub fn to_numeric(&self) -> String {
        format!("5.{:03}", self.minor)
    }

    /// Normalize for comparison (treats patch:None as patch:0)
    fn normalize(&self) -> (u32, u32, u32) {
        (self.major, self.minor, self.patch.unwrap_or(0))
    }
}

impl PartialOrd for PerlVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PerlVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.normalize().cmp(&other.normalize())
    }
}

impl fmt::Display for PerlVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_vstring())
    }
}

impl FromStr for PerlVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("Invalid Perl version: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vstring() {
        assert_eq!(PerlVersion::parse("v5.36"), Some(PerlVersion::V5_36));
        assert_eq!(PerlVersion::parse("v5.36.0"), Some(PerlVersion { major: 5, minor: 36, patch: Some(0) }));
        assert_eq!(PerlVersion::parse("v5.36.1"), Some(PerlVersion { major: 5, minor: 36, patch: Some(1) }));
    }

    #[test]
    fn test_parse_numeric() {
        assert_eq!(PerlVersion::parse("5.036"), Some(PerlVersion::V5_36));
        assert_eq!(PerlVersion::parse("5.036000"), Some(PerlVersion::V5_36));
    }

    #[test]
    fn test_comparison() {
        assert!(PerlVersion::V5_36 > PerlVersion::V5_20);
        assert!(PerlVersion::V5_36 > PerlVersion::V5_34);
        assert!(PerlVersion::V5_36 == PerlVersion::V5_36);
        assert!(PerlVersion { major: 5, minor: 36, patch: Some(1) } > PerlVersion::V5_36);
    }
}
```

### 1.2 Feature Enum

Location: `crates/perl-pragma/src/feature.rs` (new file)

```rust
use crate::version::PerlVersion;

/// A named feature that can be enabled/disabled via `use feature` or version bundles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Feature {
    // v5.10 features
    Say,
    State,
    Switch,  // given/when (deprecated in 5.38)

    // v5.12 features
    UnicodeStrings,
    ArrayBaseWord,
    
    // v5.14 features
    Say14,  // say (reconfirmed)
    State14,
    Switch14,
    
    // v5.16 features
    CurrentSub,  // __SUB__
    Evalbytes,
    
    // v5.20 features
    PostfixDeref,
    Signatures20,  // experimental in 5.20
    
    // v5.22 features
    Bitwise,
    <<>>_operator,  // safe diamond operator (name escaped)
    
    // v5.24 features
    PostfixDeref24,
    
    // v5.26 features
    DeclaredRefs,
    
    // v5.28 features
    Refaliasing,

    // v5.32 features
    TrueBoolean,
    
    // v5.34 features
    Try,           // try/catch
    
    // v5.36 features
    Signatures,    // stable signatures
    
    // v5.38 features
    Class,
    Field,
    Method,
    Builtin,
    ExtraPairedDelims,
    
    // v5.40 features (future)
    // Add as needed
}

impl Feature {
    /// Get the feature name as a string for use in `use feature 'name'`
    pub fn as_str(&self) -> &'static str {
        match self {
            Feature::Say | Feature::Say14 => "say",
            Feature::State | Feature::State14 => "state",
            Feature::Switch | Feature::Switch14 => "switch",
            Feature::UnicodeStrings => "unicode_strings",
            Feature::ArrayBaseWord => "array_base_word",
            Feature::CurrentSub => "current_sub",
            Feature::Evalbytes => "evalbytes",
            Feature::PostfixDeref | Feature::PostfixDeref24 => "postfix_deref",
            Feature::Signatures20 => "signatures",  // experimental
            Feature::Bitwise => "bitwise",
            Feature::<<>>_operator => "<<>>",
            Feature::DeclaredRefs => "declared_refs",
            Feature::Refaliasing => "refaliasing",
            Feature::TrueBoolean => "true_boolean",
            Feature::Try => "try",
            Feature::Signatures => "signatures",
            Feature::Class => "class",
            Feature::Field => "field",
            Feature::Method => "method",
            Feature::Builtin => "builtin",
            Feature::ExtraPairedDelims => "extra_paired_delims",
        }
    }

    /// Parse a feature name string into a Feature variant.
    pub fn from_name(name: &str) -> Option<Self> {
        // Remove quotes if present
        let name = name.trim_matches('\'').trim_matches('"');
        
        match name {
            "say" => Some(Feature::Say),
            "state" => Some(Feature::State),
            "switch" => Some(Feature::Switch),
            "unicode_strings" => Some(Feature::UnicodeStrings),
            "array_base_word" => Some(Feature::ArrayBaseWord),
            "current_sub" | "__SUB__" => Some(Feature::CurrentSub),
            "evalbytes" => Some(Feature::Evalbytes),
            "postfix_deref" | "postderef" => Some(Feature::PostfixDeref),
            "signatures" => Some(Feature::Signatures),
            "bitwise" => Some(Feature::Bitwise),
            "<<>>" => Some(Feature::<<>>_operator),
            "declared_refs" => Some(Feature::DeclaredRefs),
            "refaliasing" => Some(Feature::Refaliasing),
            "true_boolean" => Some(Feature::TrueBoolean),
            "try" => Some(Feature::Try),
            "class" => Some(Feature::Class),
            "field" => Some(Feature::Field),
            "method" => Some(Feature::Method),
            "builtin" => Some(Feature::Builtin),
            "extra_paired_delims" => Some(Feature::ExtraPairedDelims),
            _ => None,
        }
    }

    /// Minimum Perl version where this feature is available.
    /// Returns None if the feature is not version-gated (always available).
    pub fn min_version(&self) -> Option<PerlVersion> {
        match self {
            Feature::Say | Feature::State | Feature::Switch => Some(PerlVersion::V5_10),
            Feature::UnicodeStrings | Feature::ArrayBaseWord => Some(PerlVersion::V5_12),
            Feature::CurrentSub | Feature::Evalbytes => Some(PerlVersion::V5_16),
            Feature::PostfixDeref | Feature::Signatures20 => Some(PerlVersion::V5_20),
            Feature::Bitwise | Feature::<<>>_operator => Some(PerlVersion::V5_22),
            Feature::PostfixDeref24 => Some(PerlVersion::V5_24),
            Feature::DeclaredRefs => Some(PerlVersion::V5_26),
            Feature::Refaliasing => Some(PerlVersion::V5_28),
            Feature::TrueBoolean => Some(PerlVersion::V5_32),
            Feature::Try => Some(PerlVersion::V5_34),
            Feature::Signatures => Some(PerlVersion::V5_36),
            Feature::Class | Feature::Field | Feature::Method | 
            Feature::Builtin | Feature::ExtraPairedDelims => Some(PerlVersion::V5_38),
            Feature::Say14 | Feature::State14 | Feature::Switch14 => Some(PerlVersion::V5_14),
        }
    }

    /// Check if this feature is experimental (may require explicit enable).
    pub fn is_experimental(&self) -> bool {
        matches!(self, Feature::Signatures20)  // experimental in 5.20
    }

    /// Get all features implicitly enabled by a version declaration.
    /// 
    /// This mirrors Perl's behavior where `use v5.36` automatically enables
    /// the feature bundle for that version.
    pub fn features_enabled_by_version(version: PerlVersion) -> Vec<Feature> {
        let mut features = Vec::new();

        // v5.10 bundle
        if version >= PerlVersion::V5_10 {
            features.push(Feature::Say);
            features.push(Feature::State);
            features.push(Feature::Switch);
        }

        // v5.12 bundle
        if version >= PerlVersion::V5_12 {
            features.push(Feature::UnicodeStrings);
            // array_base_word is implicitly on in 5.16+ but can be disabled
        }

        // v5.14 - features confirmed
        if version >= PerlVersion::V5_14 {
            // say, state, switch reconfirmed
        }

        // v5.16 features
        if version >= PerlVersion::V5_16 {
            features.push(Feature::CurrentSub);
            features.push(Feature::Evalbytes);
        }

        // v5.20 features
        if version >= PerlVersion::V5_20 {
            features.push(Feature::PostfixDeref);
        }

        // v5.22 features
        if version >= PerlVersion::V5_22 {
            features.push(Feature::Bitwise);
        }

        // v5.24 - postfix_deref reconfirmed
        if version >= PerlVersion::V5_24 {
            // No new features, but postfix_deref is now stable
        }

        // v5.26 features
        if version >= PerlVersion::V5_26 {
            features.push(Feature::DeclaredRefs);
        }

        // v5.28 features
        if version >= PerlVersion::V5_28 {
            features.push(Feature::Refaliasing);
        }

        // v5.32 features
        if version >= PerlVersion::V5_32 {
            features.push(Feature::TrueBoolean);
        }

        // v5.34 features
        if version >= PerlVersion::V5_34 {
            features.push(Feature::Try);
        }

        // v5.36 bundle (major feature bundle)
        if version >= PerlVersion::V5_36 {
            features.push(Feature::Signatures);
        }

        // v5.38 bundle
        if version >= PerlVersion::V5_38 {
            features.push(Feature::Class);
            features.push(Feature::Field);
            features.push(Feature::Method);
            features.push(Feature::Builtin);
            features.push(Feature::ExtraPairedDelims);
        }

        features
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_bundle_5_36() {
        let features = Feature::features_enabled_by_version(PerlVersion::V5_36);
        assert!(features.contains(&Feature::Say));
        assert!(features.contains(&Feature::Signatures));
        assert!(!features.contains(&Feature::Class));  // 5.38+
    }

    #[test]
    fn test_from_name() {
        assert_eq!(Feature::from_name("say"), Some(Feature::Say));
        assert_eq!(Feature::from_name("'say'"), Some(Feature::Say));  // quoted
        assert_eq!(Feature::from_name("\"say\""), Some(Feature::Say));
        assert_eq!(Feature::from_name("unknown"), None);
    }
}
```

### 1.3 ExtendedPragmaState

Location: `crates/perl-pragma/src/pragma_state.rs` (new file, or extend existing)

```rust
use crate::version::PerlVersion;
use crate::feature::{Feature, FeatureBundle};
use rustc_hash::FxHashSet;

/// Extended pragma state that includes all pragma categories.
/// 
/// This extends the existing `PragmaState` (which only tracks strict/warnings)
/// to include additional pragmas like utf8, re, and feature bundles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtendedPragmaState {
    // === Strict pragma ===
    /// strict 'vars' is enabled
    pub strict_vars: bool,
    /// strict 'subs' is enabled  
    pub strict_subs: bool,
    /// strict 'refs' is enabled
    pub strict_refs: bool,

    // === Warnings pragma ===
    /// warnings are enabled (can be category-specific)
    pub warnings: bool,
    /// Specific warning categories enabled
    pub warning_categories: FxHashSet<String>,

    // === New: Additional pragmas ===
    /// utf8 pragma (source encoding)
    pub utf8: bool,
    /// bytes pragma (disable utf8 semantics)
    pub bytes: bool,
    
    // === re pragma ===
    /// re 'strict' mode
    pub re_strict: bool,
    /// re 'eval' mode (allow code in regex)
    pub re_eval: bool,
    /// re '/a' or '/aa' (ASCII-safe matching)
    pub re_ascii: bool,
    
    // === features ===
    /// Feature bundle state
    pub feature_bundle: Option<FeatureBundle>,
    
    // === misc ===
    /// integer pragma (use integer math)
    pub integer: bool,
    /// open pragma (I/O disciplines)
    pub open_settings: Option<String>,
}

/// How features were enabled.
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureBundle {
    /// Implied by version declaration (e.g., `use v5.36`)
    Implied(PerlVersion),
    /// Explicit feature list (e.g., `use feature qw(say state)`)
    Explicit(Vec<Feature>),
    /// Both: explicit list applied after version bundle
    Combined {
        version: PerlVersion,
        added: Vec<Feature>,
        removed: Vec<Feature>,  // via `no feature 'x'`
    },
}

/// Categories of strictness that can be queried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrictCategory {
    Vars,
    Subs,
    Refs,
    /// Any strict category is enabled
    Any,
}

impl ExtendedPragmaState {
    /// Create a new state with all strict categories disabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new state with all strict categories enabled.
    pub fn strict() -> Self {
        Self {
            strict_vars: true,
            strict_subs: true,
            strict_refs: true,
            warnings: true,
            ..Default::default()
        }
    }

    /// Check if a specific strict category is enabled.
    pub fn is_strict(&self, category: StrictCategory) -> bool {
        match category {
            StrictCategory::Vars => self.strict_vars,
            StrictCategory::Subs => self.strict_subs,
            StrictCategory::Refs => self.strict_refs,
            StrictCategory::Any => self.strict_vars || self.strict_subs || self.strict_refs,
        }
    }

    /// Enable a strict category.
    pub fn enable_strict(&mut self, category: StrictCategory) {
        match category {
            StrictCategory::Vars => self.strict_vars = true,
            StrictCategory::Subs => self.strict_subs = true,
            StrictCategory::Refs => self.strict_refs = true,
            StrictCategory::Any => {
                self.strict_vars = true;
                self.strict_subs = true;
                self.strict_refs = true;
            }
        }
    }

    /// Disable a strict category (from `no strict 'x'`).
    pub fn disable_strict(&mut self, category: StrictCategory) {
        match category {
            StrictCategory::Vars => self.strict_vars = false,
            StrictCategory::Subs => self.strict_subs = false,
            StrictCategory::Refs => self.strict_refs = false,
            StrictCategory::Any => {
                self.strict_vars = false;
                self.strict_subs = false;
                self.strict_refs = false;
            }
        }
    }

    /// Parse strict category from pragma argument.
    pub fn parse_strict_category(s: &str) -> Option<StrictCategory> {
        match s.trim_matches('\'').trim_matches('"') {
            "vars" => Some(StrictCategory::Vars),
            "subs" => Some(StrictCategory::Subs),
            "refs" => Some(StrictCategory::Refs),
            _ => None,
        }
    }

    /// Update state from `use strict ...` arguments.
    pub fn apply_use_strict(&mut self, args: &[String]) {
        if args.is_empty() {
            // `use strict` without args = all categories
            self.enable_strict(StrictCategory::Any);
        } else {
            for arg in args {
                if let Some(cat) = Self::parse_strict_category(arg) {
                    self.enable_strict(cat);
                }
            }
        }
    }

    /// Update state from `no strict ...` arguments.
    pub fn apply_no_strict(&mut self, args: &[String]) {
        if args.is_empty() {
            // `no strict` without args = disable all
            self.disable_strict(StrictCategory::Any);
        } else {
            for arg in args {
                if let Some(cat) = Self::parse_strict_category(arg) {
                    self.disable_strict(cat);
                }
            }
        }
    }

    /// Check if a specific warning category is enabled.
    pub fn has_warning(&self, category: &str) -> bool {
        self.warnings || self.warning_categories.contains(category)
    }

    /// Check if utf8 source encoding is enabled.
    pub fn is_utf8(&self) -> bool {
        self.utf8 && !self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_queries() {
        let mut state = ExtendedPragmaState::new();
        assert!(!state.is_strict(StrictCategory::Any));
        
        state.enable_strict(StrictCategory::Vars);
        assert!(state.is_strict(StrictCategory::Vars));
        assert!(!state.is_strict(StrictCategory::Subs));
        assert!(state.is_strict(StrictCategory::Any));
    }

    #[test]
    fn test_use_strict_parsing() {
        let mut state = ExtendedPragmaState::new();
        state.apply_use_strict(&["vars".to_string(), "refs".to_string()]);
        assert!(state.strict_vars);
        assert!(!state.strict_subs);
        assert!(state.strict_refs);
    }
}
```

### 1.4 EffectiveSemantics (Main Struct)

Location: `crates/perl-pragma/src/effective_semantics.rs` (new file)

```rust
use crate::version::PerlVersion;
use crate::feature::Feature;
use crate::pragma_state::{ExtendedPragmaState, StrictCategory};
use rustc_hash::FxHashSet;
use std::ops::Range;

/// The unified effective semantics at a specific source location.
/// 
/// This struct combines version information, enabled features, and pragma state
/// into a single queryable structure for use by lints, analyzers, and diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveSemantics {
    /// The declared Perl version (from `use v5.36` or `use 5.036`).
    /// None if no version declared.
    pub declared_version: Option<PerlVersion>,
    
    /// The effective feature set at this location.
    /// Includes: version-implied features + explicit features - disabled features.
    pub features: FxHashSet<Feature>,
    
    /// Extended pragma state (strict, warnings, utf8, re, etc.)
    pub pragmas: ExtendedPragmaState,
    
    /// The source byte offset where this state becomes effective.
    pub effective_from: usize,
    
    /// The source byte offset where this state ends (exclusive).
    /// None means "until end of file".
    pub effective_until: Option<usize>,
}

impl Default for EffectiveSemantics {
    fn default() -> Self {
        Self {
            declared_version: None,
            features: FxHashSet::default(),
            pragmas: ExtendedPragmaState::default(),
            effective_from: 0,
            effective_until: None,
        }
    }
}

impl EffectiveSemantics {
    // ==================== Query Methods ====================

    /// Check if a specific feature is enabled at this location.
    /// 
    /// # Example
    /// ```
    /// if semantics.has_feature(Feature::Signatures) {
    ///     // Can use subroutine signatures
    /// }
    /// ```
    pub fn has_feature(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }

    /// Check if a feature is enabled by name.
    /// 
    /// Useful for lint integration where feature names come from configuration.
    pub fn has_feature_by_name(&self, name: &str) -> bool {
        Feature::from_name(name)
            .map(|f| self.has_feature(f))
            .unwrap_or(false)
    }

    /// Check if any of the given features are enabled.
    pub fn has_any_feature(&self, features: &[Feature]) -> bool {
        features.iter().any(|f| self.has_feature(*f))
    }

    /// Check if all of the given features are enabled.
    pub fn has_all_features(&self, features: &[Feature]) -> bool {
        features.iter().all(|f| self.has_feature(*f))
    }

    /// Check if strict mode is active for a specific category.
    pub fn is_strict(&self, category: StrictCategory) -> bool {
        self.pragmas.is_strict(category)
    }

    /// Check if any strict category is enabled.
    pub fn is_strict_any(&self) -> bool {
        self.is_strict(StrictCategory::Any)
    }

    /// Check if warnings are enabled.
    pub fn has_warnings(&self) -> bool {
        self.pragmas.warnings
    }

    /// Check if a specific warning category is enabled.
    pub fn has_warning_category(&self, category: &str) -> bool {
        self.pragmas.has_warning(category)
    }

    /// Get the minimum Perl version required for all enabled features.
    /// 
    /// This calculates the version floor based on which features are enabled.
    /// Useful for suggesting version declarations or checking compatibility.
    pub fn min_required_version(&self) -> PerlVersion {
        let mut min = PerlVersion::V5_8;  // baseline

        for feature in &self.features {
            if let Some(feature_min) = feature.min_version() {
                if feature_min > min {
                    min = feature_min;
                }
            }
        }

        // Also consider declared version
        if let Some(declared) = self.declared_version {
            if declared > min {
                min = declared;
            }
        }

        min
    }

    /// Get the declared version, or the minimum required version if none declared.
    pub fn effective_version(&self) -> PerlVersion {
        self.declared_version.unwrap_or_else(|| self.min_required_version())
    }

    /// Check if this location is within a specific version range.
    pub fn version_matches<F>(&self, predicate: F) -> bool
    where
        F: FnOnce(PerlVersion) -> bool,
    {
        predicate(self.effective_version())
    }

    /// Check if the source encoding is UTF-8.
    pub fn is_utf8(&self) -> bool {
        self.pragmas.is_utf8()
    }

    // ==================== Builder Methods ====================

    /// Create a new EffectiveSemantics with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the declared version.
    pub fn with_version(mut self, version: PerlVersion) -> Self {
        self.declared_version = Some(version);
        // Auto-populate features from version bundle
        for feature in Feature::features_enabled_by_version(version) {
            self.features.insert(feature);
        }
        self.pragmas.feature_bundle = Some(crate::pragma_state::FeatureBundle::Implied(version));
        self
    }

    /// Add a feature explicitly.
    pub fn with_feature(mut self, feature: Feature) -> Self {
        self.features.insert(feature);
        self
    }

    /// Add multiple features.
    pub fn with_features(mut self, features: &[Feature]) -> Self {
        for f in features {
            self.features.insert(*f);
        }
        self
    }

    /// Remove a feature (from `no feature 'x'`).
    pub fn without_feature(mut self, feature: Feature) -> Self {
        self.features.remove(&feature);
        self
    }

    /// Enable strict mode for a category.
    pub fn with_strict(mut self, category: StrictCategory) -> Self {
        self.pragmas.enable_strict(category);
        self
    }

    /// Enable all strict categories.
    pub fn with_strict_all(mut self) -> Self {
        self.pragmas.enable_strict(StrictCategory::Any);
        self
    }

    /// Enable warnings.
    pub fn with_warnings(mut self, enabled: bool) -> Self {
        self.pragmas.warnings = enabled;
        self
    }

    /// Set the effective range.
    pub fn with_range(mut self, from: usize, until: Option<usize>) -> Self {
        self.effective_from = from;
        self.effective_until = until;
        self
    }

    /// Set pragma state directly.
    pub fn with_pragmas(mut self, pragmas: ExtendedPragmaState) -> Self {
        self.pragmas = pragmas;
        self
    }

    // ==================== Mutation Methods ====================

    /// Add a feature (in-place mutation).
    pub fn add_feature(&mut self, feature: Feature) {
        self.features.insert(feature);
    }

    /// Remove a feature (in-place mutation).
    pub fn remove_feature(&mut self, feature: Feature) {
        self.features.remove(&feature);
    }

    /// Merge another EffectiveSemantics into this one.
    /// Features are unioned, pragmas are overwritten.
    pub fn merge(&mut self, other: &Self) {
        for feature in &other.features {
            self.features.insert(*feature);
        }
        self.pragmas = other.pragmas.clone();
        if other.declared_version.is_some() {
            self.declared_version = other.declared_version;
        }
    }

    // ==================== Convenience ====================

    /// Get the effective range.
    pub fn range(&self) -> Range<usize> {
        self.effective_from..self.effective_until.unwrap_or(usize::MAX)
    }

    /// Check if a byte offset is within this semantics' range.
    pub fn contains_offset(&self, offset: usize) -> bool {
        offset >= self.effective_from && 
        self.effective_until.map(|u| offset < u).unwrap_or(true)
    }
}

/// A collection of EffectiveSemantics ranges for a source file.
#[derive(Debug, Clone, Default)]
pub struct EffectiveSemanticsMap {
    /// Sorted ranges of effective semantics.
    ranges: Vec<(Range<usize>, EffectiveSemantics)>,
}

impl EffectiveSemanticsMap {
    /// Create an empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a semantics range.
    /// 
    /// Note: ranges should be added in source order for correct lookup.
    pub fn add(&mut self, range: Range<usize>, semantics: EffectiveSemantics) {
        self.ranges.push((range, semantics));
    }

    /// Get the effective semantics at a specific byte offset.
    /// 
    /// Uses binary search for O(log n) lookup.
    pub fn at_offset(&self, offset: usize) -> Option<&EffectiveSemantics> {
        // partition_point gives us the first range with start > offset
        let idx = self.ranges.partition_point(|(r, _)| r.start <= offset);
        
        // The semantics we want is at idx - 1 (the last range that started before offset)
        if idx > 0 {
            let (range, semantics) = &self.ranges[idx - 1];
            if range.contains(&offset) {
                return Some(semantics);
            }
        }
        
        None
    }

    /// Get the effective semantics at a specific byte offset, or return default.
    pub fn at_offset_or_default(&self, offset: usize) -> EffectiveSemantics {
        self.at_offset(offset).cloned().unwrap_or_default()
    }

    /// Get all ranges (for iteration).
    pub fn iter(&self) -> impl Iterator<Item = &(Range<usize>, EffectiveSemantics)> {
        self.ranges.iter()
    }

    /// Returns true if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Returns the number of ranges.
    pub fn len(&self) -> usize {
        self.ranges.len()
    }
}

/// Builder for constructing EffectiveSemantics ranges from AST.
/// 
/// This walks the AST and builds a range-indexed map of effective semantics.
pub struct EffectiveSemanticsBuilder;

impl EffectiveSemanticsBuilder {
    /// Build a range-indexed map from AST nodes.
    /// 
    /// Traverses use/no statements and builds the semantics state machine.
    pub fn build(ast: &perl_ast::Node) -> EffectiveSemanticsMap {
        let mut map = EffectiveSemanticsMap::new();
        let mut current = EffectiveSemantics::new();
        let mut range_start = 0;

        Self::build_recursive(
            ast,
            &mut current,
            &mut map,
            &mut range_start,
        );

        map
    }

    fn build_recursive(
        node: &perl_ast::Node,
        current: &mut EffectiveSemantics,
        map: &mut EffectiveSemanticsMap,
        range_start: &mut usize,
    ) {
        use perl_ast::NodeKind;

        match &node.kind {
            // Use statements: use v5.36, use strict, use feature, etc.
            NodeKind::Use { module, args, .. } => {
                let module_name = module.as_str();
                
                // Version declaration: use v5.36 or use 5.036
                if let Some(version) = Self::try_parse_version_use(module_name, args) {
                    // Close previous range
                    let range_end = node.location.start;
                    if *range_start < range_end {
                        map.add(
                            *range_start..range_end,
                            current.clone(),
                        );
                    }
                    
                    // Update state
                    current.declared_version = Some(version);
                    current.features = Feature::features_enabled_by_version(version)
                        .into_iter()
                        .collect();
                    current.pragmas.feature_bundle = 
                        Some(crate::pragma_state::FeatureBundle::Implied(version));
                    
                    *range_start = range_end;
                }
                // Feature pragma: use feature 'say'
                else if module_name == "feature" {
                    let range_end = node.location.start;
                    if *range_start < range_end {
                        map.add(*range_start..range_end, current.clone());
                    }
                    
                    // Parse features from args
                    for arg in args {
                        if let Some(feature) = Feature::from_name(arg) {
                            current.add_feature(feature);
                        }
                    }
                    current.pragmas.feature_bundle = 
                        Some(crate::pragma_state::FeatureBundle::Explicit(
                            args.iter()
                                .filter_map(|a| Feature::from_name(a))
                                .collect()
                        ));
                    
                    *range_start = range_end;
                }
                // Strict pragma: use strict, use strict 'vars'
                else if module_name == "strict" {
                    let range_end = node.location.start;
                    if *range_start < range_end {
                        map.add(*range_start..range_end, current.clone());
                    }
                    
                    current.pragmas.apply_use_strict(args);
                    
                    *range_start = range_end;
                }
                // Warnings pragma: use warnings
                else if module_name == "warnings" {
                    let range_end = node.location.start;
                    if *range_start < range_end {
                        map.add(*range_start..range_end, current.clone());
                    }
                    
                    current.pragmas.warnings = true;
                    for arg in args {
                        current.pragmas.warning_categories.insert(arg.clone());
                    }
                    
                    *range_start = range_end;
                }
                // UTF8 pragma
                else if module_name == "utf8" {
                    let range_end = node.location.start;
                    if *range_start < range_end {
                        map.add(*range_start..range_end, current.clone());
                    }
                    
                    current.pragmas.utf8 = true;
                    current.pragmas.bytes = false;
                    
                    *range_start = range_end;
                }
            }
            
            // No statements: no strict, no feature, etc.
            NodeKind::No { module, args, .. } => {
                let module_name = module.as_str();
                let range_end = node.location.start;
                
                if *range_start < range_end {
                    map.add(*range_start..range_end, current.clone());
                }
                
                if module_name == "feature" {
                    for arg in args {
                        if let Some(feature) = Feature::from_name(arg) {
                            current.remove_feature(feature);
                        }
                    }
                } else if module_name == "strict" {
                    current.pragmas.apply_no_strict(args);
                } else if module_name == "warnings" {
                    if args.is_empty() {
                        current.pragmas.warnings = false;
                    } else {
                        for arg in args {
                            current.pragmas.warning_categories.remove(arg);
                        }
                    }
                } else if module_name == "utf8" {
                    current.pragmas.utf8 = false;
                } else if module_name == "bytes" {
                    current.pragmas.bytes = true;
                }
                
                *range_start = range_end;
            }
            
            // Block scoping: pragmas may reset
            NodeKind::Block { statements, .. } => {
                let saved = current.clone();
                let saved_start = *range_start;
                
                for stmt in statements {
                    Self::build_recursive(stmt, current, map, range_start);
                }
                
                // Restore after block (unless it's a file-level scope)
                *current = saved;
                // Note: we don't restore range_start here - the block's changes
                // to range_start are correct for the next sibling
            }
            
            // Subroutine: scoping
            NodeKind::SubDefinition { body, .. } => {
                let saved = current.clone();
                
                if let Some(body_node) = body {
                    Self::build_recursive(body_node, current, map, range_start);
                }
                
                *current = saved;
            }
            
            // Default: recurse into children
            _ => {
                for child in node.children() {
                    Self::build_recursive(child, current, map, range_start);
                }
            }
        }
    }

    /// Try to parse a version from `use v5.36` or `use 5.036`.
    fn try_parse_version_use(module: &str, _args: &[String]) -> Option<PerlVersion> {
        // Try parsing the module name as a version
        if let Some(version) = PerlVersion::parse(module) {
            return Some(version);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_methods() {
        let semantics = EffectiveSemantics::new()
            .with_version(PerlVersion::V5_36)
            .with_strict(StrictCategory::Vars);

        assert!(semantics.has_feature(Feature::Say));
        assert!(semantics.has_feature(Feature::Signatures));
        assert!(!semantics.has_feature(Feature::Class));
        
        assert!(semantics.is_strict(StrictCategory::Vars));
        assert!(!semantics.is_strict(StrictCategory::Subs));
        
        assert_eq!(semantics.min_required_version(), PerlVersion::V5_36);
    }

    #[test]
    fn test_feature_by_name() {
        let semantics = EffectiveSemantics::new()
            .with_version(PerlVersion::V5_36);

        assert!(semantics.has_feature_by_name("say"));
        assert!(semantics.has_feature_by_name("'say'"));  // with quotes
        assert!(!semantics.has_feature_by_name("unknown"));
    }

    #[test]
    fn test_map_lookup() {
        let mut map = EffectiveSemanticsMap::new();
        
        map.add(0..100, EffectiveSemantics::new().with_version(PerlVersion::V5_20));
        map.add(100..200, EffectiveSemantics::new().with_version(PerlVersion::V5_36));
        
        let s50 = map.at_offset(50).unwrap();
        assert_eq!(s50.declared_version, Some(PerlVersion::V5_20));
        
        let s150 = map.at_offset(150).unwrap();
        assert_eq!(s150.declared_version, Some(PerlVersion::V5_36));
    }
}
```

### 1.5 PragmaState Extension (Backward Compatibility)

Location: Update existing `PragmaState` in `crates/perl-pragma/src/lib.rs`

```rust
// Add these traits and implementations for backward compatibility:

use crate::effective_semantics::{EffectiveSemantics, ExtendedPragmaState};

/// Conversion from EffectiveSemantics to the legacy PragmaState.
/// 
/// This allows gradual migration of existing code.
impl From<&EffectiveSemantics> for PragmaState {
    fn from(semantics: &EffectiveSemantics) -> Self {
        Self {
            strict_vars: semantics.pragmas.strict_vars,
            strict_subs: semantics.pragmas.strict_subs,
            strict_refs: semantics.pragmas.strict_refs,
            warnings: semantics.pragmas.warnings,
        }
    }
}

/// Conversion from ExtendedPragmaState to legacy PragmaState.
impl From<&ExtendedPragmaState> for PragmaState {
    fn from(pragmas: &ExtendedPragmaState) -> Self {
        Self {
            strict_vars: pragmas.strict_vars,
            strict_subs: pragmas.strict_subs,
            strict_refs: pragmas.strict_refs,
            warnings: pragmas.warnings,
        }
    }
}

/// Extension trait for PragmaTracker to support EffectiveSemantics.
pub trait PragmaTrackerExt {
    /// Get full EffectiveSemantics instead of just PragmaState.
    fn effective_semantics_for_offset(&self, offset: usize) -> EffectiveSemantics;
}

impl PragmaTrackerExt for PragmaTracker {
    fn effective_semantics_for_offset(&self, offset: usize) -> EffectiveSemantics {
        // If the tracker has been upgraded to store EffectiveSemantics internally,
        // return that. Otherwise, convert from legacy PragmaState.
        
        // Option 1: If tracker stores EffectiveSemanticsMap internally
        if let Some(map) = self.effective_semantics_map() {
            return map.at_offset_or_default(offset);
        }
        
        // Option 2: Convert from legacy state
        let legacy_state = self.state_for_offset(offset);
        let extended = ExtendedPragmaState {
            strict_vars: legacy_state.strict_vars,
            strict_subs: legacy_state.strict_subs,
            strict_refs: legacy_state.strict_refs,
            warnings: legacy_state.warnings,
            ..Default::default()
        };
        
        EffectiveSemantics {
            declared_version: None,
            features: FxHashSet::default(),
            pragmas: extended,
            effective_from: offset,
            effective_until: None,
        }
    }
}
```

---

## 2. API Surface

### 2.1 Public API Summary

```rust
// Top-level re-exports for convenience
pub use crate::version::PerlVersion;
pub use crate::feature::{Feature, FeatureBundle};
pub use crate::pragma_state::{ExtendedPragmaState, StrictCategory, FeatureBundle};
pub use crate::effective_semantics::{
    EffectiveSemantics, 
    EffectiveSemanticsMap,
    EffectiveSemanticsBuilder,
};

// Legacy compatibility
pub use crate::legacy::{PragmaState, PragmaTracker};
```

### 2.2 Query Interface Methods

| Method | Returns | Purpose |
|--------|---------|---------|
| `has_feature(Feature)` | `bool` | Check if a specific feature is enabled |
| `has_feature_by_name(&str)` | `bool` | Check by string name (for lint config) |
| `has_any_feature(&[Feature])` | `bool` | Check if any feature in list is enabled |
| `has_all_features(&[Feature])` | `bool` | Check if all features in list are enabled |
| `is_strict(StrictCategory)` | `bool` | Check strict state for category |
| `is_strict_any()` | `bool` | Check if any strict is enabled |
| `has_warnings()` | `bool` | Check if warnings are enabled |
| `has_warning_category(&str)` | `bool` | Check specific warning category |
| `min_required_version()` | `PerlVersion` | Calculate version floor from features |
| `effective_version()` | `PerlVersion` | Get declared or calculated version |
| `is_utf8()` | `bool` | Check UTF-8 source encoding |
| `range()` | `Range<usize>` | Get effective byte range |
| `contains_offset(usize)` | `bool` | Check if offset is in range |

### 2.3 Builder Pattern Methods

| Method | Returns | Purpose |
|--------|---------|---------|
| `new()` | `Self` | Create with defaults |
| `with_version(PerlVersion)` | `Self` | Set declared version (chainable) |
| `with_feature(Feature)` | `Self` | Add single feature (chainable) |
| `with_features(&[Feature])` | `Self` | Add multiple features (chainable) |
| `without_feature(Feature)` | `Self` | Remove feature (chainable) |
| `with_strict(StrictCategory)` | `Self` | Enable strict category (chainable) |
| `with_strict_all()` | `Self` | Enable all strict (chainable) |
| `with_warnings(bool)` | `Self` | Set warnings state (chainable) |
| `with_range(usize, Option<usize>)` | `Self` | Set byte range (chainable) |
| `with_pragmas(ExtendedPragmaState)` | `Self` | Set full pragma state (chainable) |

### 2.4 Mutation Methods

| Method | Purpose |
|--------|---------|
| `add_feature(Feature)` | Add feature in-place |
| `remove_feature(Feature)` | Remove feature in-place |
| `merge(&Self)` | Union features, overwrite pragmas |

### 2.5 Map Lookup Methods

| Method | Returns | Purpose |
|--------|---------|---------|
| `EffectiveSemanticsMap::new()` | `Self` | Create empty map |
| `add(Range<usize>, EffectiveSemantics)` | `()` | Add range |
| `at_offset(usize)` | `Option<&EffectiveSemantics>` | Lookup semantics |
| `at_offset_or_default(usize)` | `EffectiveSemantics` | Lookup with fallback |
| `iter()` | `Iterator` | Iterate all ranges |

### 2.6 Lint Integration Example

```rust
// In version_compat.rs or similar lint
use perl_pragma::{EffectiveSemanticsMap, Feature, StrictCategory};

pub fn check_feature_usage(
    semantics_map: &EffectiveSemanticsMap,
    node: &Node,
    feature: Feature,
    diagnostic: &mut Vec<Diagnostic>,
) {
    let semantics = semantics_map.at_offset_or_default(node.location.start);
    
    if !semantics.has_feature(feature) {
        let min_version = feature.min_version();
        diagnostic.push(Diagnostic::error(
            node.location.clone(),
            format!(
                "Feature '{}' requires Perl {} or later",
                feature.as_str(),
                min_version.map(|v| v.to_string()).unwrap_or_else(|| "unknown".to_string())
            ),
        ));
    }
}

pub fn check_strict(
    semantics_map: &EffectiveSemanticsMap,
    variable_name: &str,
    offset: usize,
    diagnostic: &mut Vec<Diagnostic>,
) {
    let semantics = semantics_map.at_offset_or_default(offset);
    
    if semantics.is_strict(StrictCategory::Vars) {
        // Require variable declaration
    }
}
```

---

## 3. Integration Points

### 3.1 Parser Integration

**File:** `crates/perl-parser-core/src/engine/parser/statements.rs`

**Where to populate:**

```rust
impl Parser<'_> {
    /// Parse a use statement and extract version/pragma info.
    /// 
    /// Populates EffectiveSemantics context for downstream analysis.
    fn parse_use_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.expect(TokenKind::Use)?;
        
        // Check if this is a version declaration (v-string or numeric)
        if let Some(version_token) = self.peek_if_version() {
            let version_str = version_token.text.to_string();
            let location = self.make_location(start);
            
            // Return Use node with version info
            return Ok(Node::new(
                NodeKind::Use {
                    module: version_str.clone(),
                    args: Vec::new(),
                    version: PerlVersion::parse(&version_str),  // NEW FIELD
                },
                location,
            ));
        }
        
        // Standard module use
        let module = self.parse_module_name()?;
        let args = self.parse_optional_use_args()?;
        
        Ok(Node::new(
            NodeKind::Use {
                module,
                args,
                version: None,
            },
            self.make_location(start),
        ))
    }
}
```

**AST Node Changes:**

```rust
// In perl-ast/src/ast.rs

pub enum NodeKind {
    Use {
        module: String,
        args: Vec<String>,
        /// NEW: Parsed version if this is a version declaration
        version: Option<PerlVersion>,
    },
    No {
        module: String,
        args: Vec<String>,
    },
    // ...
}
```

### 3.2 Scope Analyzer Integration

**File:** `crates/perl-semantic-analyzer/src/scope_analyzer.rs`

**How to query:**

```rust
use perl_pragma::{EffectiveSemanticsMap, StrictCategory, Feature};

pub struct ScopeAnalyzer {
    scope_stack: Vec<Rc<Scope>>,
    /// NEW: Effective semantics for the current file
    semantics_map: EffectiveSemanticsMap,
    /// NEW: Cache for quick pragma lookups
    pragma_cache: RefCell<HashMap<usize, PragmaState>>,
}

impl ScopeAnalyzer {
    /// NEW: Set the semantics map before analysis.
    pub fn with_semantics_map(mut self, map: EffectiveSemanticsMap) -> Self {
        self.semantics_map = map;
        self
    }
    
    /// UPDATED: Get pragma state with EffectiveSemantics awareness.
    fn pragma_state_at(&self, offset: usize) -> PragmaState {
        // Check cache first
        if let Some(cached) = self.pragma_cache.borrow().get(&offset) {
            return *cached;
        }
        
        // Query EffectiveSemantics
        let semantics = self.semantics_map.at_offset_or_default(offset);
        let pragma_state: PragmaState = (&semantics).into();
        
        // Cache and return
        self.pragma_cache.borrow_mut().insert(offset, pragma_state);
        pragma_state
    }
    
    /// NEW: Check if a feature is enabled at a location.
    fn has_feature_at(&self, offset: usize, feature: Feature) -> bool {
        let semantics = self.semantics_map.at_offset_or_default(offset);
        semantics.has_feature(feature)
    }
    
    /// UPDATED: Variable declaration check with feature awareness.
    fn check_variable_declaration(
        &self,
        node: &Node,
        sigil: &str,
        name: &str,
        offset: usize,
    ) -> Option<ScopeIssue> {
        let semantics = self.semantics_map.at_offset_or_default(offset);
        
        // Check strict vars
        if semantics.is_strict(StrictCategory::Vars) {
            // Existing strict vars logic...
        }
        
        // NEW: Check for signatures feature
        if sigil == "$" && name.starts_with('_') {
            // In signature context with Feature::Signatures, this is valid
            if self.in_signature_context() && semantics.has_feature(Feature::Signatures) {
                return None;
            }
        }
        
        None
    }
}
```

### 3.3 Lint Integration

**File:** `crates/perl-lsp-diagnostics/src/lints/version_compat.rs`

**Current:**

```rust
// Current version_compat.rs has FEATURE_VERSIONS table and manual parsing

static FEATURE_VERSIONS: &[(&str, PerlVersion)] = &[
    ("say", PerlVersion { major: 5, minor: 10 }),
    ("state", PerlVersion { major: 5, minor: 10 }),
    // ... duplicated data
];
```

**NEW:**

```rust
use perl_pragma::{EffectiveSemanticsMap, Feature, PerlVersion};

pub struct VersionCompatLint {
    semantics_map: Option<EffectiveSemanticsMap>,
}

impl VersionCompatLint {
    /// NEW: Accept pre-built semantics map from analyzer.
    pub fn with_semantics_map(&mut self, map: EffectiveSemanticsMap) {
        self.semantics_map = Some(map);
    }
    
    /// UPDATED: Check feature usage against effective semantics.
    fn check_feature(
        &self,
        node: &Node,
        feature_name: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(ref map) = self.semantics_map else {
            return; // Can't check without semantics
        };
        
        let semantics = map.at_offset_or_default(node.location.start);
        
        if !semantics.has_feature_by_name(feature_name) {
            if let Some(feature) = Feature::from_name(feature_name) {
                let min_version = feature.min_version();
                diagnostics.push(Diagnostic::error(
                    node.location.clone(),
                    format!(
                        "Feature '{}' requires Perl {} (enabled by `use {}`)",
                        feature_name,
                        min_version.map(|v| v.to_string()).unwrap_or_else(|| "unknown".to_string()),
                        min_version.map(|v| v.to_vstring()).unwrap_or_else(|| "feature".to_string()),
                    ),
                    Some("add-version-declaration".to_string()),
                ));
            }
        }
    }
    
    /// DEPRECATED: Remove this once migration complete.
    #[deprecated(since = "0.1.0", note = "Use EffectiveSemantics instead")]
    fn features_enabled_by_version(&self, _version: PerlVersion) -> Vec<&'static str> {
        // Keep for backward compat during migration
        unimplemented!("Use Feature::features_enabled_by_version")
    }
}
```

### 3.4 LSP Server Integration

**File:** `crates/perl-lsp-server/src/server.rs`

```rust
use perl_pragma::EffectiveSemanticsBuilder;

impl PerlLanguageServer {
    /// UPDATED: Analyze document with EffectiveSemantics.
    async fn analyze_document(&self, uri: &Url) -> Result<AnalysisResult, Error> {
        let content = self.get_document_content(uri).await?;
        let ast = self.parse(&content)?;
        
        // NEW: Build effective semantics from AST
        let semantics_map = EffectiveSemanticsBuilder::build(&ast);
        
        // Pass to analyzer
        let mut analyzer = ScopeAnalyzer::new()
            .with_semantics_map(semantics_map.clone());
        let scope_issues = analyzer.analyze(&ast);
        
        // Pass to lints
        let mut version_compat = VersionCompatLint::new();
        version_compat.with_semantics_map(semantics_map);
        let version_diagnostics = version_compat.check(&ast);
        
        // Combine results
        Ok(AnalysisResult {
            diagnostics: merge_diagnostics(scope_issues, version_diagnostics),
        })
    }
}
```

---

## 4. Migration Strategy

### 4.1 Phase 1: Type Definition (Week 1)

**Goal:** Define types without breaking changes.

1. **Create new files in `perl-pragma`:**
   - `src/version.rs` - `PerlVersion`
   - `src/feature.rs` - `Feature` enum
   - `src/pragma_state.rs` - `ExtendedPragmaState`
   - `src/effective_semantics.rs` - `EffectiveSemantics`

2. **Update `src/lib.rs`:**
   - Add module declarations
   - Add re-exports
   - Keep existing `PragmaState` and `PragmaTracker` untouched

3. **Add to `Cargo.toml`:**
   ```toml
   [dependencies]
   rustc-hash = "1.1"  # If not already present
   perl-ast = { path = "../perl-ast" }  # For builder
   ```

4. **Tests:**
   - Unit tests for PerlVersion parsing
   - Unit tests for Feature::from_name
   - Unit tests for EffectiveSemantics queries

### 4.2 Phase 2: Builder Implementation (Week 1-2)

**Goal:** Enable building semantics from AST.

1. **Implement `EffectiveSemanticsBuilder`**
   - Handle `use v5.36`, `use 5.036`
   - Handle `use feature 'say'`
   - Handle `use strict`, `no strict`
   - Handle `use warnings`, `no warnings`
   - Handle `use utf8`, `no bytes`

2. **Block scoping:**
   - Save/restore semantics at block boundaries
   - Handle subroutine scoping

3. **Integration tests:**
   - Parse sample files, build semantics, verify queries

### 4.3 Phase 3: Backward Compatibility (Week 2)

**Goal:** Allow gradual migration.

1. **Implement `From<&EffectiveSemantics> for PragmaState`**
2. **Add `PragmaTrackerExt` trait** with `effective_semantics_for_offset()`
3. **Add feature flag** in `perl-pragma/Cargo.toml`:
   ```toml
   [features]
   effective-semantics = []  # Enable new API
   ```

4. **Update existing code to use trait:**
   ```rust
   // Old code can remain:
   let state = tracker.state_for_offset(offset);
   
   // New code can use:
   let semantics = tracker.effective_semantics_for_offset(offset);
   ```

### 4.4 Phase 4: Lint Migration (Week 3)

**Goal:** Migrate `version_compat.rs` to use new API.

1. **Update `VersionCompatLint`:**
   - Add `semantics_map` field
   - Replace `FEATURE_VERSIONS` table with `Feature` enum
   - Update `features_enabled_by_version()` to use `Feature::features_enabled_by_version()`

2. **Update `scope_analyzer.rs`:**
   - Accept `EffectiveSemanticsMap` in constructor
   - Add feature-aware checks
   - Keep `PragmaState` conversion for existing code paths

3. **Update `diagnostics.rs`:**
   - Build semantics map once, pass to all lints

### 4.5 Phase 5: Full Migration (Week 4)

**Goal:** Complete the migration, deprecate old API.

1. **Remove feature flag default-on**
2. **Update all internal callers** to use EffectiveSemantics directly
3. **Mark old methods deprecated** with migration hints
4. **Documentation:**
   - Migration guide for downstream crates
   - API documentation
   - Examples

### 4.6 Rollback Plan

If issues arise:

1. **Disable feature flag** to revert to old implementation
2. **Fix issues** in isolation
3. **Re-enable** when stable

---

## 5. File-by-File Changes

### 5.1 New Files

| File | Purpose |
|------|---------|
| `crates/perl-pragma/src/version.rs` | `PerlVersion` struct with parsing |
| `crates/perl-pragma/src/feature.rs` | `Feature` enum and bundles |
| `crates/perl-pragma/src/pragma_state.rs` | `ExtendedPragmaState` |
| `crates/perl-pragma/src/effective_semantics.rs` | Main `EffectiveSemantics` types |
| `crates/perl-pragma/src/compat.rs` | Backward compatibility shims |

### 5.2 Modified Files in `perl-pragma`

**`crates/perl-pragma/src/lib.rs`:**

```rust
// Add at top of file
pub mod version;
pub mod feature;
pub mod pragma_state;
pub mod effective_semantics;
pub mod compat;

// Re-exports
pub use version::PerlVersion;
pub use feature::{Feature, FeatureBundle};
pub use pragma_state::{ExtendedPragmaState, StrictCategory};
pub use effective_semantics::{EffectiveSemantics, EffectiveSemanticsMap, EffectiveSemanticsBuilder};
pub use compat::{PragmaTrackerExt, EffectiveSemanticsFromLegacy};

// Keep existing exports for backward compatibility
pub use crate::legacy::{PragmaState, PragmaTracker};  // assuming this exists
```

**`crates/perl-pragma/Cargo.toml`:**

```toml
[dependencies]
rustc-hash = "1.1"
perl-ast = { path = "../perl-ast", optional = true }

[features]
default = []
effective-semantics = ["dep:perl-ast"]
```

### 5.3 Modified Files in `perl-parser-core`

**`crates/perl-parser-core/src/engine/parser/statements.rs`:**

```rust
// In parse_use_statement, around line ~XXX
fn parse_use_statement(&mut self) -> ParseResult<Node> {
    let start = self.current_position();
    self.expect(TokenKind::Use)?;
    
    // NEW: Check for version declaration first
    if self.peek_is_version_string() {
        let version_token = self.consume_token()?;
        let version_str = version_token.text.to_string();
        
        return Ok(Node::new(
            NodeKind::Use {
                module: version_str.clone(),
                args: Vec::new(),
                version: PerlVersion::parse(&version_str),  // NEW FIELD
            },
            self.make_location(start),
        ));
    }
    
    // ... rest of existing code
}
```

### 5.4 Modified Files in `perl-ast`

**`crates/perl-ast/src/ast.rs`:**

```rust
pub enum NodeKind {
    Use {
        module: String,
        args: Vec<String>,
        version: Option<PerlVersion>,  // NEW FIELD
    },
    No {
        module: String,
        args: Vec<String>,
        // Could add version field for completeness
    },
    // ...
}

impl Node {
    // NEW: Helper to get children for traversal
    pub fn children(&self) -> Vec<&Node> {
        match &self.kind {
            NodeKind::Block { statements, .. } => statements.iter().collect(),
            NodeKind::SubDefinition { body, .. } => body.as_ref().into_iter().collect(),
            // ... other variants with children
            _ => Vec::new(),
        }
    }
}
```

### 5.5 Modified Files in `perl-semantic-analyzer`

**`crates/perl-semantic-analyzer/src/scope_analyzer.rs`:**

```rust
pub struct ScopeAnalyzer {
    scope_stack: Vec<Rc<Scope>>,
    current_scope_id: usize,
    /// NEW: Effective semantics for feature/version queries
    semantics_map: EffectiveSemanticsMap,
}

impl ScopeAnalyzer {
    /// NEW: Constructor with semantics map
    pub fn with_semantics_map(mut self, map: EffectiveSemanticsMap) -> Self {
        self.semantics_map = map;
        self
    }
    
    /// UPDATED: Get pragma state
    fn pragma_state_at(&self, offset: usize) -> PragmaState {
        // Use semantics map instead of legacy tracker
        let semantics = self.semantics_map.at_offset_or_default(offset);
        (&semantics).into()
    }
    
    /// NEW: Feature check method
    pub fn has_feature_at(&self, offset: usize, feature: Feature) -> bool {
        self.semantics_map
            .at_offset_or_default(offset)
            .has_feature(feature)
    }
}
```

### 5.6 Modified Files in `perl-lsp-diagnostics`

**`crates/perl-lsp-diagnostics/src/lints/version_compat.rs`:**

```rust
pub struct VersionCompatLint {
    semantics_map: Option<EffectiveSemanticsMap>,
}

impl VersionCompatLint {
    pub fn new() -> Self {
        Self { semantics_map: None }
    }
    
    /// NEW: Set semantics map
    pub fn with_semantics_map(&mut self, map: EffectiveSemanticsMap) {
        self.semantics_map = Some(map);
    }
    
    /// UPDATED: Check method
    pub fn check(&mut self, ast: &Node) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        // Ensure we have semantics
        if self.semantics_map.is_none() {
            self.semantics_map = Some(EffectiveSemanticsBuilder::build(ast));
        }
        
        self.check_node(ast, &mut diagnostics);
        diagnostics
    }
    
    /// UPDATED: Use semantics for feature checks
    fn check_feature_usage(&self, node: &Node, feature_name: &str) -> Option<Diagnostic> {
        let semantics = self.semantics_map
            .as_ref()?
            .at_offset_or_default(node.location.start);
        
        if !semantics.has_feature_by_name(feature_name) {
            let feature = Feature::from_name(feature_name)?;
            let min_version = feature.min_version()?;
            
            Some(Diagnostic::error(
                node.location.clone(),
                format!(
                    "Feature '{}' requires Perl {} (try adding `use {}`)",
                    feature_name,
                    min_version,
                    min_version.to_vstring()
                ),
            ))
        } else {
            None
        }
    }
}

// DEPRECATED: Remove after migration
#[allow(deprecated)]
static FEATURE_VERSIONS: &[(&str, &str)] = &[
    ("say", "v5.10"),
    // ... old table, keep during migration
];
```

### 5.7 Modified Files in `perl-lsp-server`

**`crates/perl-lsp-server/src/server.rs`:**

```rust
use perl_pragma::EffectiveSemanticsBuilder;
use perl_semantic_analyzer::ScopeAnalyzer;

impl PerlLanguageServer {
    async fn analyze_document(&self, uri: &Url) -> Result<Vec<Diagnostic>, Error> {
        let content = self.documents.get(uri).ok_or(Error::DocumentNotFound)?;
        let ast = self.parser.parse(&content.text)?;
        
        // NEW: Build effective semantics once
        let semantics_map = EffectiveSemanticsBuilder::build(&ast);
        
        // Pass to analyzer
        let analyzer = ScopeAnalyzer::new()
            .with_semantics_map(semantics_map.clone());
        let mut issues = analyzer.analyze(&ast);
        
        // Pass to lints
        let mut version_lint = VersionCompatLint::new();
        version_lint.with_semantics_map(semantics_map);
        let version_issues = version_lint.check(&ast);
        
        // Convert to LSP diagnostics
        issues.extend(version_issues);
        Ok(self.convert_diagnostics(issues))
    }
}
```

### 5.8 Test Files to Add

| File | Purpose |
|------|---------|
| `crates/perl-pragma/tests/version_tests.rs` | PerlVersion parsing edge cases |
| `crates/perl-pragma/tests/feature_tests.rs` | Feature enum and bundles |
| `crates/perl-pragma/tests/semantics_tests.rs` | EffectiveSemantics queries |
| `crates/perl-pragma/tests/builder_tests.rs` | EffectiveSemanticsBuilder with sample code |
| `crates/perl-lsp-diagnostics/tests/version_compat_tests.rs` | Integration tests for version_compat lint |

---

## Appendix: Type Dependencies

```
EffectiveSemantics
├── declared_version: Option<PerlVersion>
├── features: FxHashSet<Feature>
├── pragmas: ExtendedPragmaState
│   ├── strict_vars/sub/refs: bool
│   ├── warnings: bool
│   ├── warning_categories: FxHashSet<String>
│   ├── utf8/bytes: bool
│   ├── re_strict/re_eval/re_ascii: bool
│   ├── feature_bundle: Option<FeatureBundle>
│   └── integer/open_settings: various
└── effective_from/until: usize

Feature
├── Say, State, Switch, etc. (variants)
├── min_version() -> Option<PerlVersion>
└── features_enabled_by_version(v) -> Vec<Feature>

PerlVersion
├── major/minor/patch: u32
├── parse(s: &str) -> Option<Self>
├── V5_8, V5_10, ... V5_40 (constants)
└── implements Ord for comparison

EffectiveSemanticsMap
├── ranges: Vec<(Range<usize>, EffectiveSemantics)>
├── at_offset(usize) -> Option<&EffectiveSemantics>
└── O(log n) binary search lookup

EffectiveSemanticsBuilder
└── build(ast: &Node) -> EffectiveSemanticsMap
    └── traverses use/no/feature/strict statements
```

---

## Summary

This specification provides:

1. **Complete data structures** with full Rust implementations
2. **Comprehensive API** with builder pattern, queries, and mutations
3. **Clear integration points** at parser, scope analyzer, lints, and LSP server
4. **Phased migration strategy** minimizing risk and disruption
5. **Detailed file-by-file changes** including function signatures

The implementation follows the Architecture 1 design from the RFC, resolving ~62 of the 113 filed issues related to version handling and pragma state tracking.
