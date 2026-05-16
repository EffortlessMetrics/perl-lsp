use crate::PragmaState;

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
