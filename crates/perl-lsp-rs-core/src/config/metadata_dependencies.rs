//! Static project metadata dependency extraction.
//!
//! These extractors intentionally read common literal metadata shapes without
//! executing Perl project files.

use serde_json::Value;
use std::fs;
use std::path::Path;

/// A dependency declared by project metadata.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredDependency {
    /// Perl module name, for example `JSON::PP`.
    pub module: String,
    /// Optional version constraint exactly as written in metadata.
    pub version: Option<String>,
    /// Requirement family from the metadata source, for example `requires`.
    pub kind: String,
    /// Metadata file family that declared this dependency.
    pub source: DeclaredDependencySource,
}

impl DeclaredDependency {
    /// Create a declared dependency fact.
    #[must_use]
    pub fn new(
        module: impl Into<String>,
        version: Option<&str>,
        kind: impl Into<String>,
        source: DeclaredDependencySource,
    ) -> Self {
        Self {
            module: module.into(),
            version: version.map(ToOwned::to_owned),
            kind: kind.into(),
            source,
        }
    }
}

/// Source metadata file for a declared dependency.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeclaredDependencySource {
    /// `cpanfile`.
    Cpanfile,
    /// `Makefile.PL`.
    MakefilePl,
    /// `Build.PL`.
    BuildPl,
    /// `dist.ini`.
    DistIni,
    /// `META.json`.
    MetaJson,
    /// `META.yml`.
    MetaYml,
}

impl DeclaredDependencySource {
    /// User-facing file label for this metadata source.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Cpanfile => "cpanfile",
            Self::MakefilePl => "Makefile.PL",
            Self::BuildPl => "Build.PL",
            Self::DistIni => "dist.ini",
            Self::MetaJson => "META.json",
            Self::MetaYml => "META.yml",
        }
    }
}

const CPANFILE_KEYS: &[&str] =
    &["requires", "test_requires", "recommends", "build_requires", "configure_requires"];
const MAKEFILE_KEYS: &[&str] =
    &["PREREQ_PM", "BUILD_REQUIRES", "TEST_REQUIRES", "CONFIGURE_REQUIRES"];
const BUILD_PL_KEYS: &[&str] =
    &["requires", "build_requires", "test_requires", "configure_requires"];
const META_RELATION_KEYS: &[&str] =
    &["requires", "recommends", "build_requires", "test_requires", "configure_requires"];

/// Detect declared dependencies from common workspace-root metadata files.
#[must_use]
pub fn detect_declared_dependencies(workspace_root: &Path) -> Vec<DeclaredDependency> {
    let mut dependencies = Vec::new();

    collect_from_file(
        &mut dependencies,
        &workspace_root.join("cpanfile"),
        extract_cpanfile_requirements,
    );
    collect_from_file(
        &mut dependencies,
        &workspace_root.join("Makefile.PL"),
        extract_makefile_pl_requirements,
    );
    collect_from_file(
        &mut dependencies,
        &workspace_root.join("Build.PL"),
        extract_build_pl_requirements,
    );
    collect_from_file(
        &mut dependencies,
        &workspace_root.join("dist.ini"),
        extract_dist_ini_requirements,
    );
    collect_from_file(
        &mut dependencies,
        &workspace_root.join("META.json"),
        extract_meta_json_requirements,
    );
    collect_from_file(
        &mut dependencies,
        &workspace_root.join("META.yml"),
        extract_meta_yml_requirements,
    );

    dependencies
}

/// Extract literal dependencies from a `cpanfile`.
#[must_use]
pub fn extract_cpanfile_requirements(source: &str) -> Vec<DeclaredDependency> {
    let source = strip_comments_preserving_strings(source);
    let mut dependencies = Vec::new();

    for statement in source.split(';') {
        let statement = statement.trim();
        for key in CPANFILE_KEYS {
            if !starts_with_keyword(statement, key) {
                continue;
            }
            let args = quoted_strings(statement);
            let Some(module) = args.first().and_then(|value| normalize_module_name(value)) else {
                continue;
            };
            let version = args.get(1).and_then(|value| normalize_version(value));
            push_unique(
                &mut dependencies,
                DeclaredDependency::new(
                    module,
                    version.as_deref(),
                    *key,
                    DeclaredDependencySource::Cpanfile,
                ),
            );
        }
    }

    dependencies
}

/// Extract literal dependencies from `Makefile.PL`.
#[must_use]
pub fn extract_makefile_pl_requirements(source: &str) -> Vec<DeclaredDependency> {
    extract_hash_requirements(source, MAKEFILE_KEYS, DeclaredDependencySource::MakefilePl)
}

/// Extract literal dependencies from `Build.PL`.
#[must_use]
pub fn extract_build_pl_requirements(source: &str) -> Vec<DeclaredDependency> {
    extract_hash_requirements(source, BUILD_PL_KEYS, DeclaredDependencySource::BuildPl)
}

/// Extract literal dependencies from Dist::Zilla `dist.ini` `[Prereqs]` sections.
#[must_use]
pub fn extract_dist_ini_requirements(source: &str) -> Vec<DeclaredDependency> {
    let mut dependencies = Vec::new();
    let mut active_section: Option<String> = None;

    for raw_line in source.lines() {
        let line = raw_line.split_once(';').map_or(raw_line, |(before, _)| before).trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let section = line.trim_matches(['[', ']']).trim();
            let lower = section.to_ascii_lowercase();
            active_section = (lower.contains("prereqs") && !lower.contains("fromcpanfile"))
                .then(|| section.to_string());
            continue;
        }

        let Some(kind) = active_section.as_deref() else {
            continue;
        };
        let Some((module, version)) = line.split_once('=') else {
            continue;
        };
        let Some(module) = normalize_module_name(module) else {
            continue;
        };
        push_unique(
            &mut dependencies,
            DeclaredDependency::new(
                module,
                normalize_version(version).as_deref(),
                kind,
                DeclaredDependencySource::DistIni,
            ),
        );
    }

    dependencies
}

/// Extract dependencies from CPAN `META.json` prerequisite maps.
#[must_use]
pub fn extract_meta_json_requirements(source: &str) -> Vec<DeclaredDependency> {
    let Ok(value) = serde_json::from_str::<Value>(source) else {
        return Vec::new();
    };

    let mut dependencies = Vec::new();
    collect_meta_json_requirements(&value, &mut dependencies);
    dependencies
}

/// Extract dependencies from simple CPAN `META.yml` prerequisite maps.
#[must_use]
pub fn extract_meta_yml_requirements(source: &str) -> Vec<DeclaredDependency> {
    let mut dependencies = Vec::new();
    let mut active_key: Option<(usize, String)> = None;

    for raw_line in source.lines() {
        let without_comment = raw_line.split_once('#').map_or(raw_line, |(before, _)| before);
        let indent = without_comment.chars().take_while(|ch| ch.is_whitespace()).count();
        let line = without_comment.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((active_indent, _)) = active_key.as_ref()
            && indent <= *active_indent
        {
            active_key = None;
        }

        if line.ends_with(':') {
            let key = line.trim_end_matches(':').trim().trim_matches(['"', '\'']);
            if META_RELATION_KEYS.contains(&key) {
                active_key = Some((indent, key.to_string()));
            }
            continue;
        }

        let Some((_, kind)) = active_key.as_ref() else {
            continue;
        };

        let Some((module, version)) = line.rsplit_once(':') else {
            continue;
        };
        let Some(module) = normalize_module_name(module) else {
            continue;
        };
        push_unique(
            &mut dependencies,
            DeclaredDependency::new(
                module,
                normalize_version(version).as_deref(),
                kind.as_str(),
                DeclaredDependencySource::MetaYml,
            ),
        );
    }

    dependencies
}

fn collect_from_file(
    into: &mut Vec<DeclaredDependency>,
    path: &Path,
    extractor: fn(&str) -> Vec<DeclaredDependency>,
) {
    let Ok(source) = fs::read_to_string(path) else {
        return;
    };
    for dependency in extractor(&source) {
        push_unique(into, dependency);
    }
}

fn push_unique(into: &mut Vec<DeclaredDependency>, dependency: DeclaredDependency) {
    if into.iter().any(|existing| existing.module == dependency.module) {
        return;
    }
    into.push(dependency);
}

fn extract_hash_requirements(
    source: &str,
    keys: &[&str],
    dependency_source: DeclaredDependencySource,
) -> Vec<DeclaredDependency> {
    let source = strip_comments_preserving_strings(source);
    let mut dependencies = Vec::new();

    for key in keys {
        let mut search_from = 0;
        while let Some(value_start) = find_key_arrow_value(&source, key, search_from) {
            let bytes = source.as_bytes();
            let mut idx = value_start;
            skip_ws(bytes, &mut idx);
            if bytes.get(idx) != Some(&b'{') {
                search_from = value_start.saturating_add(1);
                continue;
            }
            let Some(end) = matching_brace(&source, idx) else {
                search_from = value_start.saturating_add(1);
                continue;
            };
            parse_hash_dependency_pairs(
                &source[idx + 1..end],
                key,
                dependency_source,
                &mut dependencies,
            );
            search_from = end.saturating_add(1);
        }
    }

    dependencies
}

fn parse_hash_dependency_pairs(
    body: &str,
    kind: &str,
    dependency_source: DeclaredDependencySource,
    dependencies: &mut Vec<DeclaredDependency>,
) {
    let bytes = body.as_bytes();
    let mut idx = 0;

    while idx < bytes.len() {
        skip_ws_and_commas(bytes, &mut idx);
        let Some((module_raw, consumed)) = parse_quoted_string(body, idx) else {
            idx += 1;
            continue;
        };
        idx += consumed;

        let Some(module) = normalize_module_name(&module_raw) else {
            continue;
        };

        skip_ws(bytes, &mut idx);
        let version = if bytes.get(idx) == Some(&b'=') && bytes.get(idx + 1) == Some(&b'>') {
            idx += 2;
            skip_ws(bytes, &mut idx);
            parse_literal_or_bare_value(body, &mut idx).and_then(|value| normalize_version(&value))
        } else {
            None
        };

        push_unique(
            dependencies,
            DeclaredDependency::new(module, version.as_deref(), kind, dependency_source),
        );
    }
}

fn collect_meta_json_requirements(value: &Value, dependencies: &mut Vec<DeclaredDependency>) {
    if let Some(prereqs) = value.get("prereqs").and_then(Value::as_object) {
        for (phase_name, phase) in prereqs
            .iter()
            .filter_map(|(key, value)| value.as_object().map(|object| (key.as_str(), object)))
        {
            for key in META_RELATION_KEYS {
                let Some(map) = phase.get(*key).and_then(Value::as_object) else {
                    continue;
                };
                for (module, version) in map {
                    let Some(module) = normalize_module_name(module) else {
                        continue;
                    };
                    push_unique(
                        dependencies,
                        DeclaredDependency::new(
                            module,
                            meta_json_version(version).as_deref(),
                            format!("{phase_name}.{key}"),
                            DeclaredDependencySource::MetaJson,
                        ),
                    );
                }
            }
        }
        return;
    }

    for key in META_RELATION_KEYS {
        let Some(map) = value.get(*key).and_then(Value::as_object) else {
            continue;
        };
        for (module, version) in map {
            let Some(module) = normalize_module_name(module) else {
                continue;
            };
            push_unique(
                dependencies,
                DeclaredDependency::new(
                    module,
                    meta_json_version(version).as_deref(),
                    *key,
                    DeclaredDependencySource::MetaJson,
                ),
            );
        }
    }
}

fn meta_json_version(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => normalize_version(value),
        Value::Number(value) => normalize_version(&value.to_string()),
        _ => None,
    }
}

fn starts_with_keyword(statement: &str, key: &str) -> bool {
    let Some(rest) = statement.strip_prefix(key) else {
        return false;
    };
    rest.chars().next().is_none_or(|ch| ch.is_whitespace() || matches!(ch, '(' | '\'' | '"'))
}

fn quoted_strings(source: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut idx = 0;

    while idx < source.len() {
        if let Some((value, consumed)) = parse_quoted_string(source, idx) {
            values.push(value);
            idx += consumed;
        } else {
            idx += source[idx..].chars().next().map_or(1, char::len_utf8);
        }
    }

    values
}

fn parse_literal_or_bare_value(source: &str, idx: &mut usize) -> Option<String> {
    if let Some((value, consumed)) = parse_quoted_string(source, *idx) {
        *idx += consumed;
        return Some(value);
    }

    let start = *idx;
    while *idx < source.len() {
        let ch = source[*idx..].chars().next()?;
        if matches!(ch, ',' | '}' | ')' | '\n') {
            break;
        }
        *idx += ch.len_utf8();
    }

    Some(source[start..*idx].trim().to_string())
}

fn find_key_arrow_value(source: &str, key: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let key_bytes = key.as_bytes();
    let mut idx = start;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while idx < bytes.len() {
        let byte = bytes[idx];
        if in_single_quote {
            idx += 1;
            if byte == b'\\' && idx < bytes.len() {
                idx += 1;
                continue;
            }
            if byte == b'\'' {
                in_single_quote = false;
            }
            continue;
        }
        if in_double_quote {
            idx += 1;
            if byte == b'\\' && idx < bytes.len() {
                idx += 1;
                continue;
            }
            if byte == b'"' {
                in_double_quote = false;
            }
            continue;
        }

        match byte {
            b'\'' => {
                in_single_quote = true;
                idx += 1;
                continue;
            }
            b'"' => {
                in_double_quote = true;
                idx += 1;
                continue;
            }
            _ => {}
        }

        if !bytes[idx..].starts_with(key_bytes) || !is_key_boundary(bytes, idx, key_bytes.len()) {
            idx += 1;
            continue;
        }

        let mut value_idx = idx + key_bytes.len();
        skip_ws(bytes, &mut value_idx);
        if bytes.get(value_idx) == Some(&b'=') && bytes.get(value_idx + 1) == Some(&b'>') {
            value_idx += 2;
            return Some(value_idx);
        }

        idx = value_idx.saturating_add(1);
    }

    None
}

fn matching_brace(source: &str, open_idx: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open_idx) != Some(&b'{') {
        return None;
    }

    let mut depth = 0usize;
    let mut idx = open_idx;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while idx < bytes.len() {
        let byte = bytes[idx];
        if in_single_quote {
            idx += 1;
            if byte == b'\\' && idx < bytes.len() {
                idx += 1;
                continue;
            }
            if byte == b'\'' {
                in_single_quote = false;
            }
            continue;
        }
        if in_double_quote {
            idx += 1;
            if byte == b'\\' && idx < bytes.len() {
                idx += 1;
                continue;
            }
            if byte == b'"' {
                in_double_quote = false;
            }
            continue;
        }

        match byte {
            b'\'' => in_single_quote = true,
            b'"' => in_double_quote = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
        idx += 1;
    }

    None
}

fn parse_quoted_string(source: &str, start: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let quote = *bytes.get(start)?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }

    let mut value = String::new();
    let mut idx = start + 1;
    let mut escaped = false;

    while idx < bytes.len() {
        let ch = source[idx..].chars().next()?;
        idx += ch.len_utf8();

        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch as u8 == quote {
            return Some((value, idx - start));
        }
        if ch == '\n' {
            return None;
        }
        value.push(ch);
    }

    None
}

fn strip_comments_preserving_strings(source: &str) -> String {
    let mut stripped = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(ch) = chars.next() {
        if in_single_quote {
            stripped.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    stripped.push(next);
                }
                continue;
            }
            if ch == '\'' {
                in_single_quote = false;
            }
            continue;
        }

        if in_double_quote {
            stripped.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    stripped.push(next);
                }
                continue;
            }
            if ch == '"' {
                in_double_quote = false;
            }
            continue;
        }

        match ch {
            '\'' => {
                in_single_quote = true;
                stripped.push(ch);
            }
            '"' => {
                in_double_quote = true;
                stripped.push(ch);
            }
            '#' => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        stripped.push('\n');
                        break;
                    }
                }
            }
            _ => stripped.push(ch),
        }
    }

    stripped
}

fn normalize_module_name(value: &str) -> Option<String> {
    let module = value.trim().trim_matches(['"', '\'']);
    if module.is_empty() || module == "perl" {
        return None;
    }
    if module.chars().any(|ch| ch.is_whitespace() || matches!(ch, '/' | '\\' | '$' | '@' | '%')) {
        return None;
    }
    if module.split("::").any(|part| {
        part.is_empty()
            || !part.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '\''))
    }) {
        return None;
    }

    Some(module.to_string())
}

fn normalize_version(value: &str) -> Option<String> {
    let version = value.trim().trim_matches(['"', '\'']);
    if version.is_empty() || version == "undef" { None } else { Some(version.to_string()) }
}

fn is_key_boundary(bytes: &[u8], key_pos: usize, key_len: usize) -> bool {
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let before_ok =
        key_pos.checked_sub(1).and_then(|idx| bytes.get(idx)).is_none_or(|b| !is_ident(*b));
    let after_ok = bytes.get(key_pos + key_len).is_none_or(|b| !is_ident(*b));
    before_ok && after_ok
}

fn skip_ws(bytes: &[u8], idx: &mut usize) {
    while let Some(byte) = bytes.get(*idx) {
        if !byte.is_ascii_whitespace() {
            break;
        }
        *idx += 1;
    }
}

fn skip_ws_and_commas(bytes: &[u8], idx: &mut usize) {
    while let Some(byte) = bytes.get(*idx) {
        if !byte.is_ascii_whitespace() && *byte != b',' {
            break;
        }
        *idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn missing(message: &'static str) -> std::io::Error {
        std::io::Error::other(message)
    }

    #[test]
    fn hash_requirement_edges_have_local_oracles() {
        let source = r#"
            my $ignored = 'PREREQ_PM\\not_a_key';
            WriteMakefile(
                PREREQ_PM => 'Not::A::Hash',
                BUILD_REQUIRES => { 'Broken::Hash' => '1.0',
                TEST_REQUIRES => {
                    'Bad Module' => '1.00',
                    'No::Version',
                    'Valid::Test' => '1.00',
                },
            );
        "#;

        assert_eq!(
            extract_makefile_pl_requirements(source),
            vec![
                DeclaredDependency::new(
                    "No::Version",
                    None,
                    "TEST_REQUIRES",
                    DeclaredDependencySource::MakefilePl,
                ),
                DeclaredDependency::new(
                    "Valid::Test",
                    Some("1.00"),
                    "TEST_REQUIRES",
                    DeclaredDependencySource::MakefilePl,
                ),
            ],
        );
    }

    #[test]
    fn meta_json_edge_versions_have_local_oracles() {
        let nested = r#"
            {
              "prereqs": {
                "runtime": {
                  "requires": {
                    "Bad Module": "1.0",
                    "Nested::Number": 2,
                    "Nested::Unspecified": null
                  }
                }
              }
            }
        "#;
        let top_level = r#"
            {
              "requires": {
                "Bad Module": "1.0",
                "Top::Number": 3,
                "Top::Unspecified": null
              }
            }
        "#;

        assert_eq!(
            extract_meta_json_requirements(nested),
            vec![
                DeclaredDependency::new(
                    "Nested::Number",
                    Some("2"),
                    "runtime.requires",
                    DeclaredDependencySource::MetaJson,
                ),
                DeclaredDependency::new(
                    "Nested::Unspecified",
                    None,
                    "runtime.requires",
                    DeclaredDependencySource::MetaJson,
                ),
            ],
        );
        assert_eq!(
            extract_meta_json_requirements(top_level),
            vec![
                DeclaredDependency::new(
                    "Top::Number",
                    Some("3"),
                    "requires",
                    DeclaredDependencySource::MetaJson,
                ),
                DeclaredDependency::new(
                    "Top::Unspecified",
                    None,
                    "requires",
                    DeclaredDependencySource::MetaJson,
                ),
            ],
        );
    }

    #[test]
    fn key_scanner_ignores_quoted_and_embedded_keys() -> TestResult {
        let source = r#"'PREREQ_PM\\not_a_key' "PREREQ_PM" NOT_PREREQ_PM => {} PREREQ_PM => {}"#;

        let value_start = find_key_arrow_value(source, "PREREQ_PM", 0)
            .ok_or_else(|| missing("expected unquoted key after quoted and embedded keys"))?;

        assert!(source[value_start..].trim_start().starts_with('{'));
        assert!(find_key_arrow_value("NOT_PREREQ_PM => {}", "PREREQ_PM", 0).is_none());
        assert!(find_key_arrow_value(r#""PREREQ_PM" => {}"#, "PREREQ_PM", 0).is_none());
        Ok(())
    }

    #[test]
    fn key_arrow_scanner_boundary_discriminators() -> TestResult {
        assert_eq!(
            find_key_arrow_value("", "PREREQ_PM", 0),
            None,
            "input that hits the boundary: while idx == bytes.len()"
        );
        assert_eq!(
            find_key_arrow_value("PREREQ_PM", "PREREQ_PM", 0),
            None,
            "input that hits the boundary: bytes.get(value_idx) != Some(&b'=')"
        );

        let source =
            r#"'PREREQ_PM\' => ignored' "PREREQ_PM\" => ignored" PREREQ_PM => 'Real::Module'"#;
        let value_start = find_key_arrow_value(source, "PREREQ_PM", 0)
            .ok_or_else(|| missing("expected unquoted key after escaped quoted keys"))?;

        assert_eq!(
            source[value_start..].trim_start(),
            "'Real::Module'",
            "input that hits the boundary: bytes.get(value_idx) == Some(&b'=') && bytes.get(value_idx + 1) == Some(&b'>')"
        );
        assert_eq!(
            source[value_start..].trim_start(),
            "'Real::Module'",
            "input that hits the boundary: byte == b'\\\\' && idx < bytes.len()"
        );
        assert_eq!(
            source[value_start..].trim_start(),
            "'Real::Module'",
            "input that hits the boundary: byte == b'\\''"
        );
        assert_eq!(
            source[value_start..].trim_start(),
            "'Real::Module'",
            "input that hits the boundary: byte == b'\"'"
        );
        Ok(())
    }

    #[test]
    fn matching_brace_boundary_discriminators() -> TestResult {
        assert_eq!(
            matching_brace("", 0),
            None,
            "input that hits the boundary: bytes.get(open_idx) != Some(&b'{{')"
        );
        assert_eq!(
            matching_brace("not a hash", 0),
            None,
            "input that hits the boundary: bytes.get(open_idx) != Some(&b'{{')"
        );

        let braced = r##"{ single => '\}', double => "\"}", nested => { value => 1 } } tail"##;
        let close = matching_brace(braced, 0)
            .ok_or_else(|| missing("expected matching brace after quoted braces"))?;

        assert_eq!(&braced[close..=close], "}", "input that hits the boundary: depth == 0");
        assert_eq!(
            &braced[close + 1..],
            " tail",
            "input that hits the boundary: byte == b'\\\\' && idx < bytes.len()"
        );
        assert_eq!(&braced[close + 1..], " tail", "input that hits the boundary: byte == b'\\''");
        assert_eq!(&braced[close + 1..], " tail", "input that hits the boundary: byte == b'\"'");
        assert_eq!(
            matching_brace("{ nested => { value => 1 }", 0),
            None,
            "input that hits the boundary: while idx == bytes.len()"
        );
        Ok(())
    }

    #[test]
    fn quoted_string_parser_boundary_discriminators() {
        assert_eq!(
            parse_quoted_string("", 0),
            None,
            "input that hits the boundary: quote != b'\\'' && quote != b'\"'"
        );
        assert_eq!(
            parse_quoted_string("not quoted", 0),
            None,
            "input that hits the boundary: quote != b'\\'' && quote != b'\"'"
        );
        assert_eq!(
            parse_quoted_string("'line\nbreak'", 0),
            None,
            "input that hits the boundary: ch == '\\n'"
        );
        assert_eq!(
            parse_quoted_string("'unterminated", 0),
            None,
            "input that hits the boundary: while idx == bytes.len()"
        );

        assert_eq!(
            parse_quoted_string(r#""Escaped\"Quote""#, 0),
            Some(("Escaped\"Quote".to_string(), r#""Escaped\"Quote""#.len())),
            "input that hits the boundary: ch == '\\\\'"
        );
        assert_eq!(
            parse_quoted_string(r#""Escaped\"Quote""#, 0),
            Some(("Escaped\"Quote".to_string(), r#""Escaped\"Quote""#.len())),
            "input that hits the boundary: ch as u8 == quote"
        );
        assert_eq!(
            parse_quoted_string(r#"'Escaped\'Quote'"#, 0),
            Some(("Escaped'Quote".to_string(), r#"'Escaped\'Quote'"#.len())),
            "input that hits the boundary: ch == '\\\\'"
        );
        assert_eq!(
            parse_quoted_string(r#"'Escaped\'Quote'"#, 0),
            Some(("Escaped'Quote".to_string(), r#"'Escaped\'Quote'"#.len())),
            "input that hits the boundary: ch as u8 == quote"
        );
    }

    #[test]
    fn brace_string_and_comment_helpers_have_local_oracles() -> TestResult {
        let braced = r#"{ '{' => "}", nested => { value => 1 } } tail"#;
        let close = matching_brace(braced, 0).ok_or_else(|| missing("expected matching brace"))?;
        assert_eq!(&braced[close..=close], "}");
        assert!(matching_brace("{ 'unterminated", 0).is_none());

        let (value, consumed) = parse_quoted_string(r#"'Module\::Name'"#, 0)
            .ok_or_else(|| missing("expected string"))?;
        assert_eq!(value, "Module::Name");
        assert_eq!(consumed, r#"'Module\::Name'"#.len());
        assert!(parse_quoted_string("'unterminated", 0).is_none());
        assert!(parse_quoted_string("'line\nbreak'", 0).is_none());

        let stripped = strip_comments_preserving_strings(
            r##"'Escaped\'#Tag' # drop
"Escaped\"#Tag" # drop
requires 'Kept#Tag'; # drop
"##,
        );
        assert!(stripped.contains("'Escaped\\'#Tag'"));
        assert!(stripped.contains(r##""Escaped\"#Tag""##));
        assert!(stripped.contains("'Kept#Tag'"));
        assert!(!stripped.contains("drop"));
        Ok(())
    }
}
