//! Comprehensive unit tests for `perl-lsp-feature-profile`.
//!
//! Covers: re-exports, `supported_cli_profiles()`, `from_str_name()`,
//! `parse_profile_token()`, `FeatureProfileKind` methods,
//! `FeatureProfileSpec` metadata, and `feature_profile_specs()`.

use perl_lsp_feature_profile::{
    FeatureProfileKind, feature_profile_specs, from_str_name, parse_profile_token,
    supported_cli_profiles,
};

// ---------------------------------------------------------------------------
// Re-export smoke tests
// ---------------------------------------------------------------------------

#[test]
fn reexported_feature_profile_kind_has_three_variants() -> Result<(), String> {
    let all = FeatureProfileKind::all();
    if all.len() != 3 {
        return Err(format!("expected 3 variants, got {}", all.len()));
    }
    Ok(())
}

#[test]
fn reexported_feature_profile_specs_returns_non_empty() -> Result<(), String> {
    let specs = feature_profile_specs();
    if specs.is_empty() {
        return Err("feature_profile_specs() returned empty slice".into());
    }
    Ok(())
}

#[test]
fn feature_profile_specs_length_matches_variants() -> Result<(), String> {
    let specs = feature_profile_specs();
    let variants = FeatureProfileKind::all();
    if specs.len() != variants.len() {
        return Err(format!(
            "specs len ({}) != variants len ({})",
            specs.len(),
            variants.len()
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// supported_cli_profiles()
// ---------------------------------------------------------------------------

#[test]
fn supported_cli_profiles_is_non_empty() -> Result<(), String> {
    let profiles = supported_cli_profiles();
    if profiles.is_empty() {
        return Err("supported_cli_profiles() returned empty slice".into());
    }
    Ok(())
}

#[test]
fn supported_cli_profiles_contains_auto() -> Result<(), String> {
    let profiles = supported_cli_profiles();
    if !profiles.contains(&"auto") {
        return Err("missing 'auto' token".into());
    }
    Ok(())
}

#[test]
fn supported_cli_profiles_contains_all_expected_tokens() -> Result<(), String> {
    let profiles = supported_cli_profiles();
    let expected = [
        "auto",
        "ga-lock",
        "ga",
        "ga_lock",
        "prod",
        "production",
        "all",
    ];
    for token in &expected {
        if !profiles.contains(token) {
            return Err(format!("missing expected token '{token}'"));
        }
    }
    Ok(())
}

#[test]
fn supported_cli_profiles_has_no_empty_strings() -> Result<(), String> {
    for token in supported_cli_profiles() {
        if token.is_empty() {
            return Err("found empty string in supported_cli_profiles".into());
        }
    }
    Ok(())
}

#[test]
fn supported_cli_profiles_has_no_duplicates() -> Result<(), String> {
    let profiles = supported_cli_profiles();
    let mut seen = std::collections::HashSet::new();
    for token in profiles {
        if !seen.insert(token) {
            return Err(format!("duplicate token: '{token}'"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// from_str_name() — canonical and alias lookups
// ---------------------------------------------------------------------------

#[test]
fn from_str_name_auto_resolves_to_current() -> Result<(), String> {
    let result = from_str_name("auto");
    let expected = FeatureProfileKind::current();
    if result != Some(expected) {
        return Err(format!("expected Some({expected:?}), got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_ga_lock_hyphen() -> Result<(), String> {
    let result = from_str_name("ga-lock");
    if result != Some(FeatureProfileKind::GaLock) {
        return Err(format!("expected GaLock, got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_ga_lock_underscore() -> Result<(), String> {
    let result = from_str_name("ga_lock");
    if result != Some(FeatureProfileKind::GaLock) {
        return Err(format!("expected GaLock, got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_ga_alias() -> Result<(), String> {
    let result = from_str_name("ga");
    if result != Some(FeatureProfileKind::GaLock) {
        return Err(format!("expected GaLock, got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_production() -> Result<(), String> {
    let result = from_str_name("production");
    if result != Some(FeatureProfileKind::Production) {
        return Err(format!("expected Production, got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_prod_alias() -> Result<(), String> {
    let result = from_str_name("prod");
    if result != Some(FeatureProfileKind::Production) {
        return Err(format!("expected Production, got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_all() -> Result<(), String> {
    let result = from_str_name("all");
    if result != Some(FeatureProfileKind::All) {
        return Err(format!("expected All, got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_unknown_returns_none() -> Result<(), String> {
    let result = from_str_name("unknown");
    if result.is_some() {
        return Err(format!("expected None, got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_empty_returns_none() -> Result<(), String> {
    let result = from_str_name("");
    if result.is_some() {
        return Err(format!("expected None for empty string, got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_is_case_sensitive() -> Result<(), String> {
    // The raw from_str_name does NOT normalize case — uppercase should fail
    let result = from_str_name("GA-LOCK");
    if result.is_some() {
        return Err(format!(
            "from_str_name should be case-sensitive, got {result:?}"
        ));
    }
    Ok(())
}

#[test]
fn from_str_name_rejects_whitespace_padded_input() -> Result<(), String> {
    let result = from_str_name("  ga-lock  ");
    if result.is_some() {
        return Err("from_str_name should not trim whitespace".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// parse_profile_token() — normalization layer
// ---------------------------------------------------------------------------

#[test]
fn parse_profile_token_trims_whitespace() -> Result<(), String> {
    let result = parse_profile_token("  ga-lock  ");
    if result != Some(FeatureProfileKind::GaLock) {
        return Err(format!("expected GaLock, got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_normalizes_uppercase() -> Result<(), String> {
    let result = parse_profile_token("GA-LOCK");
    if result != Some(FeatureProfileKind::GaLock) {
        return Err(format!("expected GaLock, got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_normalizes_mixed_case() -> Result<(), String> {
    let result = parse_profile_token("Production");
    if result != Some(FeatureProfileKind::Production) {
        return Err(format!("expected Production, got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_replaces_underscores_with_hyphens() -> Result<(), String> {
    let result = parse_profile_token("ga_lock");
    if result != Some(FeatureProfileKind::GaLock) {
        return Err(format!("expected GaLock, got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_combined_normalization() -> Result<(), String> {
    // Uppercase + underscore + whitespace all at once
    let result = parse_profile_token("  GA_LOCK  ");
    if result != Some(FeatureProfileKind::GaLock) {
        return Err(format!("expected GaLock, got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_auto() -> Result<(), String> {
    let result = parse_profile_token("auto");
    let expected = FeatureProfileKind::current();
    if result != Some(expected) {
        return Err(format!("expected Some({expected:?}), got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_auto_uppercase() -> Result<(), String> {
    let result = parse_profile_token("AUTO");
    let expected = FeatureProfileKind::current();
    if result != Some(expected) {
        return Err(format!("expected Some({expected:?}), got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_all() -> Result<(), String> {
    let result = parse_profile_token("all");
    if result != Some(FeatureProfileKind::All) {
        return Err(format!("expected All, got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_prod_alias() -> Result<(), String> {
    let result = parse_profile_token("PROD");
    if result != Some(FeatureProfileKind::Production) {
        return Err(format!("expected Production, got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_unknown_returns_none() -> Result<(), String> {
    let result = parse_profile_token("bogus");
    if result.is_some() {
        return Err(format!("expected None, got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_empty_returns_none() -> Result<(), String> {
    let result = parse_profile_token("");
    if result.is_some() {
        return Err(format!("expected None for empty, got {result:?}"));
    }
    Ok(())
}

#[test]
fn parse_profile_token_whitespace_only_returns_none() -> Result<(), String> {
    let result = parse_profile_token("   ");
    if result.is_some() {
        return Err(format!("expected None for whitespace-only, got {result:?}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — as_str() round-trip
// ---------------------------------------------------------------------------

#[test]
fn as_str_ga_lock() -> Result<(), String> {
    if FeatureProfileKind::GaLock.as_str() != "ga-lock" {
        return Err(format!(
            "expected 'ga-lock', got '{}'",
            FeatureProfileKind::GaLock.as_str()
        ));
    }
    Ok(())
}

#[test]
fn as_str_production() -> Result<(), String> {
    if FeatureProfileKind::Production.as_str() != "production" {
        return Err(format!(
            "expected 'production', got '{}'",
            FeatureProfileKind::Production.as_str()
        ));
    }
    Ok(())
}

#[test]
fn as_str_all() -> Result<(), String> {
    if FeatureProfileKind::All.as_str() != "all" {
        return Err(format!(
            "expected 'all', got '{}'",
            FeatureProfileKind::All.as_str()
        ));
    }
    Ok(())
}

#[test]
fn as_str_round_trips_through_from_str_name() -> Result<(), String> {
    for kind in FeatureProfileKind::all() {
        let canonical = kind.as_str();
        let parsed = from_str_name(canonical);
        if parsed != Some(*kind) {
            return Err(format!(
                "round-trip failed for {kind:?}: as_str='{canonical}', parsed={parsed:?}"
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// FeatureProfileKind::all()
// ---------------------------------------------------------------------------

#[test]
fn all_variants_are_distinct() -> Result<(), String> {
    let all = FeatureProfileKind::all();
    for (i, a) in all.iter().enumerate() {
        for (j, b) in all.iter().enumerate() {
            if i != j && a == b {
                return Err(format!("duplicate variant at index {i} and {j}: {a:?}"));
            }
        }
    }
    Ok(())
}

#[test]
fn all_contains_ga_lock() -> Result<(), String> {
    if !FeatureProfileKind::all().contains(&FeatureProfileKind::GaLock) {
        return Err("all() missing GaLock".into());
    }
    Ok(())
}

#[test]
fn all_contains_production() -> Result<(), String> {
    if !FeatureProfileKind::all().contains(&FeatureProfileKind::Production) {
        return Err("all() missing Production".into());
    }
    Ok(())
}

#[test]
fn all_contains_all_variant() -> Result<(), String> {
    if !FeatureProfileKind::all().contains(&FeatureProfileKind::All) {
        return Err("all() missing All".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// FeatureProfileKind::current()
// ---------------------------------------------------------------------------

#[test]
fn current_returns_a_valid_variant() -> Result<(), String> {
    let current = FeatureProfileKind::current();
    if !FeatureProfileKind::all().contains(&current) {
        return Err(format!("current() = {current:?} not in all()"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// FeatureProfileKind::from_ga_lock_enabled()
// ---------------------------------------------------------------------------

#[test]
fn from_ga_lock_enabled_true_returns_ga_lock() -> Result<(), String> {
    let result = FeatureProfileKind::from_ga_lock_enabled(true);
    if result != FeatureProfileKind::GaLock {
        return Err(format!("expected GaLock, got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_ga_lock_enabled_false_returns_production() -> Result<(), String> {
    let result = FeatureProfileKind::from_ga_lock_enabled(false);
    if result != FeatureProfileKind::Production {
        return Err(format!("expected Production, got {result:?}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// FeatureProfileKind::aliases()
// ---------------------------------------------------------------------------

#[test]
fn aliases_are_non_empty_for_every_variant() -> Result<(), String> {
    for kind in FeatureProfileKind::all() {
        if kind.aliases().is_empty() {
            return Err(format!("{kind:?} has empty aliases"));
        }
    }
    Ok(())
}

#[test]
fn aliases_all_resolve_back_to_their_profile() -> Result<(), String> {
    for kind in FeatureProfileKind::all() {
        for alias in kind.aliases() {
            let parsed = from_str_name(alias);
            if parsed != Some(*kind) {
                return Err(format!(
                    "alias '{alias}' for {kind:?} resolved to {parsed:?}"
                ));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// FeatureProfileSpec metadata
// ---------------------------------------------------------------------------

#[test]
fn specs_have_non_empty_canonical_names() -> Result<(), String> {
    for spec in feature_profile_specs() {
        if spec.canonical.is_empty() {
            return Err("found spec with empty canonical name".into());
        }
    }
    Ok(())
}

#[test]
fn specs_have_non_empty_descriptions() -> Result<(), String> {
    for spec in feature_profile_specs() {
        if spec.description.is_empty() {
            return Err(format!("spec '{}' has empty description", spec.canonical));
        }
    }
    Ok(())
}

#[test]
fn specs_have_non_empty_aliases() -> Result<(), String> {
    for spec in feature_profile_specs() {
        if spec.aliases.is_empty() {
            return Err(format!("spec '{}' has no aliases", spec.canonical));
        }
    }
    Ok(())
}

#[test]
fn spec_canonical_names_are_unique() -> Result<(), String> {
    let specs = feature_profile_specs();
    let mut seen = std::collections::HashSet::new();
    for spec in specs {
        if !seen.insert(spec.canonical) {
            return Err(format!("duplicate canonical name: '{}'", spec.canonical));
        }
    }
    Ok(())
}

#[test]
fn spec_canonical_matches_variant_as_str() -> Result<(), String> {
    let specs = feature_profile_specs();
    let canonical_names: std::collections::HashSet<&str> =
        specs.iter().map(|s| s.canonical).collect();
    for kind in FeatureProfileKind::all() {
        if !canonical_names.contains(kind.as_str()) {
            return Err(format!(
                "variant {kind:?} as_str='{}' not found in specs",
                kind.as_str()
            ));
        }
    }
    Ok(())
}

#[test]
fn every_supported_cli_profile_resolves() -> Result<(), String> {
    for token in supported_cli_profiles() {
        if from_str_name(token).is_none() {
            return Err(format!(
                "supported CLI token '{token}' does not resolve via from_str_name"
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Edge cases and regression guards
// ---------------------------------------------------------------------------

#[test]
fn parse_profile_token_tab_and_newline_trimming() -> Result<(), String> {
    let result = parse_profile_token("\t ga-lock \n");
    if result != Some(FeatureProfileKind::GaLock) {
        return Err(format!(
            "expected GaLock after tab/newline trim, got {result:?}"
        ));
    }
    Ok(())
}

#[test]
fn parse_profile_token_multiple_underscores_rejected() -> Result<(), String> {
    // "ga__lock" normalizes to "ga--lock" which is unknown
    let result = parse_profile_token("ga__lock");
    if result.is_some() {
        return Err(format!(
            "double-underscore should not match, got {result:?}"
        ));
    }
    Ok(())
}

#[test]
fn parse_profile_token_hyphen_preserved() -> Result<(), String> {
    // Hyphens are preserved, only underscores are replaced
    let result = parse_profile_token("ga-lock");
    if result != Some(FeatureProfileKind::GaLock) {
        return Err(format!("expected GaLock, got {result:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_special_characters_rejected() -> Result<(), String> {
    for input in &["ga lock", "ga.lock", "ga/lock", "ga\\lock", "ga@lock"] {
        if from_str_name(input).is_some() {
            return Err(format!("expected None for '{input}'"));
        }
    }
    Ok(())
}

#[test]
fn parse_profile_token_numeric_input_rejected() -> Result<(), String> {
    let result = parse_profile_token("12345");
    if result.is_some() {
        return Err(format!("expected None for numeric input, got {result:?}"));
    }
    Ok(())
}
