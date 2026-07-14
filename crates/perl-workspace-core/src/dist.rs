//! Distribution-metadata facts.
//!
//! Reads a Perl distribution's metadata files — `META.json` (CPAN Meta Spec
//! v2, via `serde_json`), `cpanfile` (a Perl DSL, via a dependency-light
//! statement scan) — into typed facts: name, version, abstract, licenses, and
//! prerequisites. The extraction mirrors the proven, std+serde_json approach in
//! `perl-lsp-rs-core::config::metadata_dependencies` (which the substrate may
//! not depend on, being above the leaf line), ported here so dist facts sit in
//! the substrate for Kwalitee and other consumers (PLSP-ADR-0006 PR 7).

use serde::{Deserialize, Serialize};

use crate::id::FileId;

/// Which metadata file a dist fact came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistMetadataSource {
    /// `META.json` (CPAN Meta Spec v2).
    MetaJson,
    /// A `cpanfile` (Perl DSL).
    Cpanfile,
}

/// One declared prerequisite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prereq {
    /// The required module.
    pub module: String,
    /// The version requirement, if any (`"0"` means "any").
    pub version: Option<String>,
    /// Phase: `configure` / `build` / `test` / `runtime` / `develop`.
    pub phase: String,
    /// Relation: `requires` / `recommends` / `suggests` / `conflicts`.
    pub relation: String,
}

/// Distribution-metadata facts extracted from one metadata file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistMetadataFacts {
    /// The metadata file these facts came from.
    pub file_id: FileId,
    /// Which metadata format.
    pub source: DistMetadataSource,
    /// Distribution name (e.g. `Foo-Bar`), if declared.
    pub name: Option<String>,
    /// Distribution version, if declared.
    pub version: Option<String>,
    /// The `abstract` one-line description, if declared.
    pub summary: Option<String>,
    /// Declared licenses (SPDX-ish tokens like `perl_5`).
    pub licenses: Vec<String>,
    /// Declared prerequisites.
    pub prereqs: Vec<Prereq>,
}

/// The prereq relations recognized in cpanfile / META.json.
const RELATIONS: &[&str] = &["requires", "recommends", "suggests", "conflicts"];
/// cpanfile statement keywords → (relation, phase).
const CPANFILE_KEYWORDS: &[(&str, &str, &str)] = &[
    ("configure_requires", "requires", "configure"),
    ("build_requires", "requires", "build"),
    ("test_requires", "requires", "test"),
    ("author_requires", "requires", "develop"),
    ("requires", "requires", "runtime"),
    ("recommends", "recommends", "runtime"),
    ("suggests", "suggests", "runtime"),
    ("conflicts", "conflicts", "runtime"),
];

/// Parse a `META.json` (CPAN Meta Spec v2, with a v1.4 flat fallback).
///
/// Returns `None` when the content is not valid JSON.
#[must_use]
pub fn parse_meta_json(file_id: FileId, content: &str) -> Option<DistMetadataFacts> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;

    let name = value.get("name").and_then(json_string);
    let version = value.get("version").and_then(json_scalar_string);
    let summary = value.get("abstract").and_then(json_string);
    let licenses = match value.get("license") {
        // v2: an array of license strings.
        Some(serde_json::Value::Array(items)) => {
            items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()
        }
        // v1.4 / META.yml: a single string.
        Some(serde_json::Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    };

    let mut prereqs = Vec::new();
    // v2: prereqs[phase][relation] = { module: version }.
    if let Some(serde_json::Value::Object(phases)) = value.get("prereqs") {
        for (phase, relations) in phases {
            let serde_json::Value::Object(relations) = relations else { continue };
            for (relation, modules) in relations {
                collect_modules(modules, phase, relation, &mut prereqs);
            }
        }
    }
    // v1.4 flat fallback: top-level requires/recommends/... = { module: version }.
    if prereqs.is_empty() {
        for relation in RELATIONS {
            if let Some(modules) = value.get(relation) {
                collect_modules(modules, "runtime", relation, &mut prereqs);
            }
        }
    }
    prereqs.sort_by(|a, b| {
        (&a.phase, &a.relation, &a.module).cmp(&(&b.phase, &b.relation, &b.module))
    });

    Some(DistMetadataFacts {
        file_id,
        source: DistMetadataSource::MetaJson,
        name,
        version,
        summary,
        licenses,
        prereqs,
    })
}

/// Parse a `cpanfile` for its prerequisites (heuristic statement scan — no Perl
/// parser). Name/version/abstract are not declared in a cpanfile.
///
/// Handles both the flat form (`requires`, `test_requires`, …) and the
/// block form (`on 'test' => sub { requires '...' }`): statements are split on
/// `;` / `{` / `}`, and an `on 'phase' => sub { ... }` block sets the phase for
/// the plain `requires`/`recommends`/`suggests` inside it.
#[must_use]
pub fn parse_cpanfile(file_id: FileId, content: &str) -> DistMetadataFacts {
    let cleaned = strip_comments(content);
    let mut prereqs = Vec::new();
    let mut phase_stack: Vec<String> = Vec::new();
    let mut buf = String::new();
    for ch in cleaned.chars() {
        match ch {
            ';' => {
                handle_cpanfile_statement(&buf, phase_stack.last(), &mut prereqs);
                buf.clear();
            }
            '{' => {
                // `on 'phase' => sub {` opens a phase block; other blocks inherit.
                let phase = parse_on_phase(&buf)
                    .or_else(|| phase_stack.last().cloned())
                    .unwrap_or_else(|| "runtime".to_string());
                phase_stack.push(phase);
                buf.clear();
            }
            '}' => {
                handle_cpanfile_statement(&buf, phase_stack.last(), &mut prereqs);
                buf.clear();
                phase_stack.pop();
            }
            _ => buf.push(ch),
        }
    }
    handle_cpanfile_statement(&buf, phase_stack.last(), &mut prereqs);

    prereqs.sort_by(|a, b| {
        (&a.phase, &a.relation, &a.module).cmp(&(&b.phase, &b.relation, &b.module))
    });
    DistMetadataFacts {
        file_id,
        source: DistMetadataSource::Cpanfile,
        name: None,
        version: None,
        summary: None,
        licenses: Vec::new(),
        prereqs,
    }
}

/// Recognize a prereq statement and push it, resolving its phase.
///
/// A prefixed keyword (`configure_requires`/`build_requires`/`test_requires`)
/// carries its own phase; a plain `requires`/`recommends`/`suggests` uses the
/// enclosing `on 'phase'` block's phase, defaulting to `runtime`.
fn handle_cpanfile_statement(buf: &str, block_phase: Option<&String>, out: &mut Vec<Prereq>) {
    let statement = buf.trim();
    // Longest keyword first so `configure_requires` isn't matched by `requires`.
    let Some((_kw, relation, kw_phase)) =
        CPANFILE_KEYWORDS.iter().find(|(kw, _, _)| statement.starts_with(kw))
    else {
        return;
    };
    let phase = if *kw_phase == "runtime" {
        block_phase.map_or("runtime", String::as_str)
    } else {
        kw_phase
    };
    let quoted = quoted_strings(statement);
    if let Some(module) = quoted.first() {
        out.push(Prereq {
            module: module.clone(),
            version: quoted.get(1).cloned(),
            phase: phase.to_string(),
            relation: (*relation).to_string(),
        });
    }
}

/// Extract the phase from an `on 'phase' => sub` block header, if `buf` is one.
fn parse_on_phase(buf: &str) -> Option<String> {
    let rest = buf.trim().strip_prefix("on")?;
    // `on` must be followed by whitespace or a quote, not be part of a longer word.
    if !rest.starts_with(|c: char| c.is_whitespace() || c == '\'' || c == '"') {
        return None;
    }
    // Prefer a quoted phase (`on 'test'`); fall back to a bareword (`on test`).
    quoted_strings(buf)
        .into_iter()
        .next()
        .or_else(|| rest.split_whitespace().next().map(str::to_string))
}

/// Collect `{ module: version }` object entries into prereqs.
fn collect_modules(
    modules: &serde_json::Value,
    phase: &str,
    relation: &str,
    out: &mut Vec<Prereq>,
) {
    let serde_json::Value::Object(map) = modules else { return };
    for (module, version) in map {
        out.push(Prereq {
            module: module.clone(),
            version: json_scalar_string(version),
            phase: phase.to_string(),
            relation: relation.to_string(),
        });
    }
}

/// A JSON value as a string, only if it *is* a string.
fn json_string(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

/// A JSON scalar (string or number) coerced to a string.
///
/// Note: a bare JSON *number* version like `1.20` round-trips through `f64` and
/// serializes back as `1.2` (trailing zeros lost). CPAN Meta Spec recommends
/// versions be strings for exactly this reason; string versions are preserved
/// verbatim.
fn json_scalar_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Strip `#` comments from cpanfile source, preserving `#` inside quotes.
fn strip_comments(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    for line in content.lines() {
        let mut in_single = false;
        let mut in_double = false;
        for ch in line.chars() {
            match ch {
                '\'' if !in_double => in_single = !in_single,
                '"' if !in_single => in_double = !in_double,
                '#' if !in_single && !in_double => break,
                _ => {}
            }
            out.push(ch);
        }
        out.push('\n');
    }
    out
}

/// Extract single- or double-quoted string literals from a statement.
fn quoted_strings(statement: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = statement.chars();
    while let Some(ch) = chars.next() {
        if ch == '\'' || ch == '"' {
            let quote = ch;
            let mut literal = String::new();
            for c in chars.by_ref() {
                if c == quote {
                    break;
                }
                literal.push(c);
            }
            out.push(literal);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Digest;

    fn fid() -> FileId {
        FileId::new("META.json", &Digest::of("x"))
    }

    #[test]
    fn parses_meta_json_v2() {
        let content = r#"{
            "name": "Foo-Bar",
            "version": "1.23",
            "abstract": "does foo to bar",
            "license": ["perl_5"],
            "prereqs": {
                "runtime": { "requires": { "strict": "0", "Moo": "2.0" } },
                "test":    { "requires": { "Test::More": "0.98" } }
            }
        }"#;
        let facts = parse_meta_json(fid(), content).unwrap();
        assert_eq!(facts.name.as_deref(), Some("Foo-Bar"));
        assert_eq!(facts.version.as_deref(), Some("1.23"));
        assert_eq!(facts.summary.as_deref(), Some("does foo to bar"));
        assert_eq!(facts.licenses, vec!["perl_5"]);
        assert!(facts.prereqs.iter().any(|p| p.module == "Moo" && p.phase == "runtime"));
        assert!(facts.prereqs.iter().any(|p| p.module == "Test::More" && p.phase == "test"));
    }

    #[test]
    fn coerces_numeric_version() {
        let facts = parse_meta_json(fid(), r#"{"name":"X","version":1.5}"#).unwrap();
        assert_eq!(facts.version.as_deref(), Some("1.5"), "numeric version coerced to string");
    }

    #[test]
    fn v1_4_flat_prereqs_and_string_license() {
        let content = r#"{"name":"X","license":"perl","requires":{"Carp":"0"}}"#;
        let facts = parse_meta_json(fid(), content).unwrap();
        assert_eq!(facts.licenses, vec!["perl"], "v1.4 single-string license");
        assert!(
            facts.prereqs.iter().any(|p| p.module == "Carp" && p.relation == "requires"),
            "flat top-level requires read as fallback"
        );
    }

    #[test]
    fn invalid_json_returns_none() {
        assert!(parse_meta_json(fid(), "{not json").is_none());
    }

    #[test]
    fn parses_cpanfile() {
        let content = "requires 'Moo', '2.0';\n# a comment with 'quotes'\ntest_requires 'Test::More';\nrequires 'Path::Tiny';\n";
        let facts = parse_cpanfile(FileId::new("cpanfile", &Digest::of("x")), content);
        assert!(facts.prereqs.iter().any(|p| p.module == "Moo"
            && p.version.as_deref() == Some("2.0")
            && p.phase == "runtime"));
        assert!(
            facts.prereqs.iter().any(|p| p.module == "Test::More" && p.phase == "test"),
            "test_requires → test phase; prereqs={:?}",
            facts.prereqs
        );
        assert!(facts.prereqs.iter().any(|p| p.module == "Path::Tiny"));
        // The comment's quoted text must not leak in as a module.
        assert!(!facts.prereqs.iter().any(|p| p.module == "quotes"));
    }

    #[test]
    fn cpanfile_block_form_phase_deps() {
        // Module::CPANfile block syntax: `on 'phase' => sub { requires ... }`.
        let content = "requires 'Moo';\non 'test' => sub {\n    requires 'Test::More', '0.88';\n};\non 'develop' => sub {\n    requires 'Perl::Critic';\n};\n";
        let facts = parse_cpanfile(FileId::new("cpanfile", &Digest::of("x")), content);
        assert!(
            facts.prereqs.iter().any(|p| p.module == "Moo" && p.phase == "runtime"),
            "flat requires stays runtime"
        );
        assert!(
            facts.prereqs.iter().any(|p| p.module == "Test::More" && p.phase == "test"),
            "block-form requires picks up the on-phase; prereqs={:?}",
            facts.prereqs
        );
        assert!(
            facts.prereqs.iter().any(|p| p.module == "Perl::Critic" && p.phase == "develop"),
            "develop block phase"
        );
    }

    #[test]
    fn cpanfile_longest_keyword_wins() {
        // `configure_requires` must not be captured by the `requires` prefix.
        let facts = parse_cpanfile(
            FileId::new("cpanfile", &Digest::of("x")),
            "configure_requires 'ExtUtils::MakeMaker';\n",
        );
        let p = facts.prereqs.iter().find(|p| p.module == "ExtUtils::MakeMaker").unwrap();
        assert_eq!(p.phase, "configure");
    }

    #[test]
    fn cpanfile_conflicts_is_recognized() {
        // Regression: `conflicts` is a documented cpanfile/META relation
        // (RELATIONS includes it), but CPANFILE_KEYWORDS previously had no
        // entry for it, so `conflicts 'Foo';` was silently dropped.
        let facts = parse_cpanfile(
            FileId::new("cpanfile", &Digest::of("x")),
            "conflicts 'Some::Broken::Module';\n",
        );
        assert!(
            facts.prereqs.iter().any(|p| p.module == "Some::Broken::Module"),
            "conflicts statement must produce a prereq entry; prereqs={:?}",
            facts.prereqs
        );
        let p = facts.prereqs.iter().find(|p| p.module == "Some::Broken::Module").unwrap();
        assert_eq!(p.relation, "conflicts");
        assert_eq!(p.phase, "runtime");
    }
}
