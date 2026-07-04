//! `cargo xtask check-native-product-surface` — native-stack product-surface guard.
//!
//! The product ships the native stack (`perllsp`, `perl-dap`, native formatter,
//! native critic). The first-mile product surfaces users actually read must not
//! tell them the product *requires* external Perl tooling (`perltidy`,
//! `perlcritic`) or a legacy `Perl::LanguageServer` bridge. This check greps
//! those surfaces for the misleading product-surface phrasings and fails if any
//! reappear.
//!
//! It bans the *requirement / legacy-as-product* framings, not benign
//! native-first negations — e.g. "the native path does not require
//! `Perl::LanguageServer`" is allowed and must stay allowed. Historical
//! design/architecture docs and reference/legacy/compatibility/conformance docs
//! are out of scope by design; legacy details are expected to live in
//! `docs/reference/DAP_LEGACY_BRIDGE_COMPAT.md`.
//!
//! ## Strict mode (`--strict`)
//!
//! The default pass only catches the known bad *phrasings* in [`DISALLOWED`].
//! Strict mode adds a second, stronger pass: on first-mile **prose** surfaces
//! (`.md`), any bare external-tool name ([`STRICT_BARE_MARKERS`]) that appears
//! on a line **without** a native-first qualifier ([`NATIVE_FIRST_QUALIFIERS`])
//! is a violation. This catches *new* leaks a fixed phrase list would miss —
//! e.g. a future "install perlcritic first" that isn't spelled exactly like an
//! existing banned marker.
//!
//! Strict mode scans `.md` surfaces line-by-line and `.json` manifests by their
//! user-facing **prose values** only. `package.json` legitimately contains the
//! tool names inside setting keys (`perl-lsp.perlcritic.enabled`) and command
//! ids, so a naive bare-name rule over the whole file would be all false
//! positives; instead the JSON scan walks only [`STRICT_PROSE_KEYS`] values
//! (`description` / `markdownDescription` / `title` / `detail` /
//! `enumDescriptions` / `deprecationMessage` and their markdown variants) — the
//! copy a user actually reads in the Settings UI — while leaving keys and ids
//! untouched. Their
//! product-surface risk in keys/ids is still covered by the default
//! [`DISALLOWED`] pass. Reference/compatibility/conformance/archive material and
//! tests are exempt via [`STRICT_PATH_ALLOWLIST`] — that is where legacy detail
//! is meant to live.

use color_eyre::eyre::{Result, bail};
use std::fs;
use std::path::Path;

use crate::utils::project_root;

/// First-mile product surfaces scanned for disallowed native-stack leaks.
const SURFACES: &[&str] = &[
    "vscode-extension/package.json",
    "vscode-extension/README.md",
    "crates/perl-dap/README.md",
    "docs/project/status/dap.md",
    "docs/tutorials/DAP_USER_GUIDE.md",
];

/// Misleading product-surface phrasings that must not appear on a first-mile
/// surface. Chosen to catch the "install external tools" / "legacy bridge is the
/// product" framings without flagging correct native-first negations such as
/// "does not require `Perl::LanguageServer`".
const DISALLOWED: &[&str] = &[
    "BridgeAdapter",
    "cpanm Perl::LanguageServer",
    "cpan Perl::LanguageServer",
    "Perl::LanguageServer requirement",
    "Bridge path documents",
    "requires perltidy",
    "requires perlcritic",
    "external Perl::Critic diagnostics",
    "Use bridge mode",
    "--bridge",
];

/// Bare external-tool tokens that must not appear on a first-mile prose surface
/// (`.md`) without a native-first qualifier on the same line. Strict mode only.
const STRICT_BARE_MARKERS: &[&str] = &[
    "perltidy",
    "perlcritic",
    "perl::critic",
    "perl::tidy",
    "perl::languageserver",
    "devel::tsperldap",
];

/// Native-first qualifiers. In strict mode a line that names a bare external
/// tool is allowed when it also carries one of these signals — i.e. the mention
/// is framed as optional / legacy / compatibility, a negation, or a conflict
/// warning, rather than as a product requirement. Matched as a lowercase
/// substring of the backtick-stripped line.
///
/// These are deliberately *narrow*. Broad single words like a bare "native" or
/// "default" are intentionally NOT here: they would exempt genuine leaks such as
/// "install perltidy for native support" or "our native formatter requires
/// perltidy". Every qualifier below itself signals *optionality / negation /
/// legacy*, not mere co-occurrence with a native mention. (A requirement framing
/// such as "requires perltidy" is additionally caught by the default
/// [`DISALLOWED`] pass regardless of any qualifier on the line.)
const NATIVE_FIRST_QUALIFIERS: &[&str] = &[
    "not require",
    "not required",
    "not needed",
    "does not",
    "doesn't",
    "no external",
    "avoids", // "native path avoids Perl::LanguageServer dependency" (plural form only)
    "optional",
    "compatibility",
    "conformance",
    "legacy",
    "opt-in",
    "migration",
    "overlap", // "Perl::Critic ... can overlap with perl-lsp features"
    "conflict",
];

/// Path fragments whose surfaces are exempt from strict bare-name scanning.
/// Reference / compatibility / conformance / archive material and tests are
/// where legacy external-tool detail is meant to live, so a bare name there is
/// expected, not a leak.
const STRICT_PATH_ALLOWLIST: &[&str] = &[
    "reference/",
    "compatibility/",
    "conformance/",
    "archive/",
    "/tests/",
    "tests/",
    "/test/",
    ".spec",
];

/// JSON object keys whose values are user-facing prose on a manifest surface
/// (`package.json`): setting descriptions, dropdown-option descriptions,
/// deprecation tooltips, command titles, command-palette categories, walkthrough
/// copy. Strict mode scans these values for bare external-tool names while
/// leaving keys, setting ids, and command ids untouched — those legitimately
/// contain the tool names (`perl-lsp.perlcritic.enabled`) and are covered by the
/// default [`DISALLOWED`] pass, not the strict bare-name rule.
///
/// Both scalar-string values (`description`, `category`) and array-of-string
/// values (`enumDescriptions`, one entry per dropdown option) are covered — see
/// [`collect_json_prose`]. All are rendered in the same VS Code Settings UI or
/// command palette a user reads, so a leak in any of them is as user-visible as
/// one in `description` (a future `"category": "Perl::Critic"` prefix would show
/// on every command in the palette).
const STRICT_PROSE_KEYS: &[&str] = &[
    "description",
    "markdowndescription",
    "title",
    "detail",
    "category",
    "enumdescriptions",
    "markdownenumdescriptions",
    "deprecationmessage",
    "markdowndeprecationmessage",
];

/// Entry point for `cargo xtask check-native-product-surface`, honoring the
/// `--strict` flag.
pub fn run_with(strict: bool) -> Result<()> {
    run_at(&project_root()?, strict)
}

/// Scan the first-mile surfaces under `root` and report. Split from [`run`] so
/// both the clean and the regressed paths are unit-testable against a fixture
/// tree without touching the live repository.
fn run_at(root: &Path, strict: bool) -> Result<()> {
    let mut violations = scan(root)?;
    if strict {
        violations.extend(scan_strict(root)?);
    }

    if violations.is_empty() {
        let mode = if strict { " (strict)" } else { "" };
        println!(
            "Native product-surface check passed{mode}: {} first-mile surface(s) are free of legacy/external-tool product framing.",
            SURFACES.len()
        );
        return Ok(());
    }

    eprintln!("NATIVE PRODUCT-SURFACE VIOLATIONS:");
    eprintln!("{}", "=".repeat(60));
    for v in &violations {
        eprintln!("  {v}");
    }
    eprintln!("{}", "=".repeat(60));
    eprintln!(
        "These phrasings belong only in reference/legacy/compatibility docs \
         (e.g. docs/reference/DAP_LEGACY_BRIDGE_COMPAT.md), not on first-mile \
         product surfaces."
    );
    bail!("native product-surface check failed with {} violation(s)", violations.len())
}

/// Scan every configured surface under `root`. A surface that does not exist is
/// skipped (not a violation) so the check stays robust to file moves.
fn scan(root: &Path) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for surface in SURFACES {
        let path = root.join(surface);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        collect_violations(surface, &text, &mut violations);
    }
    Ok(violations)
}

/// Pure per-surface scan, separated so it is unit-testable without touching the
/// repository.
///
/// Each line is normalized before matching — Markdown backticks stripped and
/// lowercased — so a banned phrasing cannot slip past by wrapping it in
/// `` `code` `` or changing its capitalization (e.g. `` requires `perltidy` ``
/// or `Requires perltidy`).
fn collect_violations(surface: &str, text: &str, violations: &mut Vec<String>) {
    for (idx, line) in text.lines().enumerate() {
        let normalized = line.replace('`', "").to_ascii_lowercase();
        for marker in DISALLOWED {
            if normalized.contains(&marker.to_ascii_lowercase()) {
                violations.push(format!(
                    "{surface}:{}: disallowed native-stack marker `{marker}`",
                    idx + 1
                ));
            }
        }
    }
}

/// Strict pass: scan first-mile surfaces for bare external-tool names lacking a
/// native-first qualifier. `.md` surfaces are scanned line-by-line; `.json`
/// manifests are scanned by their user-facing *prose values* only (setting
/// descriptions, command titles) so keys and setting/command ids are left
/// untouched. Allowlisted paths are skipped (see the module docs for why).
fn scan_strict(root: &Path) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for surface in SURFACES {
        if is_strict_allowlisted(surface) {
            continue;
        }
        let path = root.join(surface);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if surface.ends_with(".md") {
            collect_strict_violations(surface, &text, &mut violations);
        } else if surface.ends_with(".json") {
            collect_strict_json_prose_violations(surface, &text, &mut violations);
        }
    }
    Ok(violations)
}

/// True when a surface path is exempt from strict bare-name scanning.
fn is_strict_allowlisted(surface: &str) -> bool {
    let lowered = surface.to_ascii_lowercase();
    STRICT_PATH_ALLOWLIST.iter().any(|frag| lowered.contains(frag))
}

/// Whole-word containment check: `needle` (already lowercase) must appear in
/// `haystack` bounded by non-alphanumeric characters (or the string edges) on
/// both sides. This lets `perltidy` match the standalone word while never
/// matching `perltidyconfig` or `.perltidyrc`, so setting names and config
/// filenames are not mistaken for a product-requirement mention.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(pos) = haystack[from..].find(needle) {
        let start = from + pos;
        let end = start + needle.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        from = start + needle.len();
    }
    false
}

/// Return the bare external-tool markers present in `text` that lack a
/// native-first qualifier. Shared by the `.md` line scan and the package.json
/// prose scan.
///
/// The text is normalized first — Markdown code/emphasis markers stripped and
/// lowercased — so a qualifier is still detected when bolded or wrapped (e.g.
/// "does **not** require" reads as "does not require"). Both the qualifier and
/// the marker are matched with [`contains_word`] (whole-word, non-alphanumeric
/// boundaries): substring matching would let an unrelated word that merely
/// *embeds* a qualifier wrongly exempt a real leak — "incompatibility" embeds
/// "compatibility", "immigration" embeds "migration".
fn unqualified_markers(text: &str) -> Vec<&'static str> {
    let normalized = text.replace(['`', '*'], "").to_ascii_lowercase();
    if NATIVE_FIRST_QUALIFIERS.iter().any(|q| contains_word(&normalized, q)) {
        return Vec::new();
    }
    STRICT_BARE_MARKERS
        .iter()
        .copied()
        .filter(|marker| contains_word(&normalized, marker))
        .collect()
}

/// Pure per-surface strict scan. A line naming a bare external tool passes when
/// it carries any native-first qualifier; otherwise every bare marker on it is a
/// violation.
fn collect_strict_violations(surface: &str, text: &str, violations: &mut Vec<String>) {
    for (idx, line) in text.lines().enumerate() {
        for marker in unqualified_markers(line) {
            violations.push(format!(
                "{surface}:{}: unqualified external-tool name `{marker}` on a first-mile surface (strict) — add a native-first qualifier (optional/legacy/compatibility/not required) or move the detail to reference/compatibility",
                idx + 1
            ));
        }
    }
}

/// Strict scan of a JSON manifest's user-facing prose values. Parses the JSON,
/// walks every [`STRICT_PROSE_KEYS`] string value, and flags any bare
/// external-tool marker lacking a native-first qualifier. Keys, setting ids, and
/// command ids are never inspected — only the human-readable copy. An unparseable
/// manifest is skipped silently (the default [`DISALLOWED`] pass still greps its
/// raw text).
fn collect_strict_json_prose_violations(surface: &str, text: &str, violations: &mut Vec<String>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return;
    };
    let mut prose = Vec::new();
    collect_json_prose(&value, &mut prose);
    for field in prose {
        // Scope the qualifier check to each sentence/clause, not the whole
        // value. `unqualified_markers` exempts a segment as soon as any
        // native-first qualifier appears in it, so checking a whole
        // multi-sentence value at once would let a qualifier in one sentence
        // excuse an unqualified requirement in another — e.g. "Legacy mode is
        // documented below. Install perltidy to enable formatting." must still
        // flag the second sentence.
        for segment in prose_segments(&field) {
            for marker in unqualified_markers(segment) {
                let snippet: String = segment.trim().chars().take(60).collect();
                violations.push(format!(
                    "{surface}: unqualified external-tool name `{marker}` in a user-facing prose field (strict): \"{snippet}\" — reword native-first (optional/legacy/compatibility) or move the detail to reference/compatibility"
                ));
            }
        }
    }
}

/// Split a prose value into sentence/clause units for per-segment qualifier
/// scoping. Breaks on `;` and newlines unconditionally, and on a sentence
/// terminator (`.` / `!` / `?`) only when it is followed by whitespace or ends
/// the value. The whitespace condition is deliberate: an intra-token dot in a
/// config path or filename (`.perl-lsp.toml`, `Foo/Bar.pm`) is *not* followed by
/// whitespace, so it never fragments a sentence and separates a native-first
/// qualifier from the marker it qualifies.
fn prose_segments(field: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut chars = field.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        let boundary = match ch {
            ';' | '\n' => true,
            '.' | '!' | '?' => chars.peek().is_none_or(|(_, next)| next.is_whitespace()),
            _ => false,
        };
        if boundary {
            segments.push(&field[start..i]);
            start = i + ch.len_utf8();
        }
    }
    if start < field.len() {
        segments.push(&field[start..]);
    }
    segments
}

/// Recursively collect the prose values of every [`STRICT_PROSE_KEYS`] object
/// key into `out`. Keys are matched case-insensitively so `markdownDescription`
/// and `markdowndescription` both count.
///
/// A prose key's value may be a scalar string (`description`) or an array of
/// strings (`enumDescriptions` — one entry per dropdown option); both are
/// flattened into `out`. Values that are neither (numbers, nested objects) are
/// ignored at the prose-key level but still recursed into, so prose keys nested
/// deeper in the tree are still found.
fn collect_json_prose(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, v) in map {
                if STRICT_PROSE_KEYS.contains(&key.to_ascii_lowercase().as_str()) {
                    match v {
                        serde_json::Value::String(s) => out.push(s.clone()),
                        serde_json::Value::Array(arr) => {
                            out.extend(arr.iter().filter_map(|el| el.as_str().map(String::from)));
                        }
                        _ => {}
                    }
                }
                collect_json_prose(v, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_json_prose(v, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_each_disallowed_marker() {
        for marker in DISALLOWED {
            let text = format!("intro line\nsome text with {marker} in it\ntrailing\n");
            let mut violations = Vec::new();
            collect_violations("surface.md", &text, &mut violations);
            assert!(
                violations.iter().any(|v| v.contains(marker)),
                "marker `{marker}` should be flagged"
            );
        }
    }

    #[test]
    fn allows_native_first_negations() {
        // These are correct native-first statements that must NOT be flagged.
        let text = "\
The native path does **not** require `Perl::LanguageServer`.\n\
Native path avoids `Perl::LanguageServer` dependency.\n\
Enable native Perl document formatting.\n\
Enable external `perlcritic` diagnostics; native critic is always on by default.\n\
`perltidy` is not required unless you select an external compatibility mode.\n";
        let mut violations = Vec::new();
        collect_violations("surface.md", text, &mut violations);
        assert!(violations.is_empty(), "native-first negations must pass: {violations:?}");
    }

    #[test]
    fn reports_line_numbers() {
        let text = "clean\nclean\ncpanm Perl::LanguageServer\n";
        let mut violations = Vec::new();
        collect_violations("guide.md", text, &mut violations);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].starts_with("guide.md:3:"), "got: {}", violations[0]);
    }

    #[test]
    fn flags_markdown_wrapped_and_capitalized_variants() {
        // The just-removed style must not be able to sneak back in wrapped in
        // Markdown backticks or with different casing.
        for text in [
            "This requires `perltidy` to be installed.\n",
            "Requires perltidy on PATH.\n",
            "Enable external `Perl::Critic` diagnostics from the extension.\n",
        ] {
            let mut violations = Vec::new();
            collect_violations("surface.md", text, &mut violations);
            assert!(!violations.is_empty(), "variant must be flagged: {text:?}");
        }
    }

    /// The live repository's first-mile surfaces must be clean, exercised
    /// through the real entry point. This is the enforcement that makes the
    /// check meaningful: if a future edit reintroduces a legacy/external-tool
    /// product framing on a first-mile surface, the default pass bails and this
    /// fails.
    #[test]
    fn live_product_surface_is_clean() -> Result<()> {
        run_with(false)
    }

    #[test]
    fn run_at_is_ok_on_a_clean_tree() -> Result<()> {
        // A tree missing every surface file scans clean (missing surfaces are
        // skipped), so run_at returns Ok and prints the pass message.
        let dir = tempfile::tempdir()?;
        run_at(dir.path(), false)
    }

    #[test]
    fn run_at_errors_when_a_surface_regresses() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let pkg_dir = dir.path().join("vscode-extension");
        std::fs::create_dir_all(&pkg_dir)?;
        std::fs::write(pkg_dir.join("package.json"), "\"desc\": \"requires perltidy\"\n")?;
        assert!(run_at(dir.path(), false).is_err(), "a regressed surface must make run_at bail");
        Ok(())
    }

    #[test]
    fn contains_word_respects_boundaries() {
        assert!(contains_word("install perltidy first", "perltidy"));
        assert!(contains_word("perltidy", "perltidy"));
        assert!(contains_word("run perlcritic.", "perlcritic"));
        // Setting keys and config filenames must NOT match the bare word.
        assert!(!contains_word("perl-lsp.perltidyconfig setting", "perltidy"));
        assert!(!contains_word("path to .perltidyrc file", "perltidy"));
        assert!(!contains_word("perltidyrc", "perltidy"));
        // `::`-qualified markers.
        assert!(contains_word("uses perl::critic policy", "perl::critic"));
    }

    #[test]
    fn strict_flags_unqualified_bare_names() {
        let text = "Install perltidy to enable formatting.\nRun perlcritic on your code.\n";
        let mut violations = Vec::new();
        collect_strict_violations("guide.md", text, &mut violations);
        assert_eq!(violations.len(), 2, "both unqualified names flagged: {violations:?}");
        assert!(violations[0].contains("perltidy"));
        assert!(violations[1].contains("perlcritic"));
    }

    #[test]
    fn strict_does_not_exempt_on_bare_native_or_default() {
        // Regression for the qualifier-too-broad loophole (PR #3315 review): a
        // bare "native"/"default"/"only for" must NOT exempt a real leak.
        for leak in [
            "Install perltidy for native support.",
            "Our native formatter requires perltidy.",
            "perlcritic is the default linter — install it first.",
            "Install perltidy only for full formatting.",
            "Use perltidy instead of the built-in formatter.",
        ] {
            let mut violations = Vec::new();
            collect_strict_violations("surface.md", &format!("{leak}\n"), &mut violations);
            assert!(!violations.is_empty(), "leak must still flag under strict: {leak:?}");
        }
    }

    #[test]
    fn strict_allows_native_first_qualified_lines() {
        // Every line names a tool but is framed native-first — none should flag.
        let text = "\
The native path does not require `Perl::LanguageServer`.\n\
Native document formatting works; `perltidy` is not required.\n\
`perlcritic` is only used for explicit legacy compatibility.\n\
Perl::Critic and PerlTidy extensions can overlap with perl-lsp features.\n\
Native path avoids `Perl::LanguageServer` dependency.\n\
Install `perltidy` only if you selected the external compatibility engine.\n";
        let mut violations = Vec::new();
        collect_strict_violations("surface.md", text, &mut violations);
        assert!(violations.is_empty(), "native-first lines must pass strict: {violations:?}");
    }

    #[test]
    fn strict_qualifier_requires_word_boundary() {
        // "incompatibility" embeds "compatibility" and "immigration" embeds
        // "migration" as raw substrings. A plain `.contains(q)` qualifier check
        // would wrongly treat these as native-first framing and let a real
        // requirement leak through. The qualifier match must respect word
        // boundaries the same way the bare-marker match already does.
        for leak in [
            "There is a known incompatibility; perlcritic must be installed for diagnostics to appear.",
            "As part of the immigration to v2 tooling, perltidy must be installed first.",
        ] {
            let mut violations = Vec::new();
            collect_strict_violations("surface.md", &format!("{leak}\n"), &mut violations);
            assert!(
                !violations.is_empty(),
                "embedded-substring qualifier collision must not exempt a real leak: {leak:?}"
            );
        }
    }

    #[test]
    fn strict_qualifier_survives_markdown_emphasis() {
        // A bolded/italicized negation must still count as a native-first
        // qualifier (mirrors the live README line 87).
        let text = "The native path does **not** require `Perl::LanguageServer`.\n";
        let mut violations = Vec::new();
        collect_strict_violations("readme.md", text, &mut violations);
        assert!(violations.is_empty(), "bolded negation must qualify: {violations:?}");
    }

    #[test]
    fn strict_ignores_setting_keys_and_config_filenames() {
        // Lines that only reference the setting id / config filename (not the
        // bare tool as a requirement) must not flag under strict.
        let text = "\
| `perl-lsp.perltidyConfig` | `\"\"` | Path to `.perltidyrc` (auto-detected if empty) |\n";
        let mut violations = Vec::new();
        collect_strict_violations("readme.md", text, &mut violations);
        assert!(violations.is_empty(), "setting-key/filename lines must pass: {violations:?}");
    }

    #[test]
    fn strict_path_allowlist_matches_reference_and_tests() {
        assert!(is_strict_allowlisted("docs/reference/DAP_LEGACY_BRIDGE_COMPAT.md"));
        assert!(is_strict_allowlisted("docs/reference/compatibility/perlcritic.md"));
        assert!(is_strict_allowlisted("docs/reference/conformance/perltidy.md"));
        assert!(is_strict_allowlisted("docs/reference/archive/old.md"));
        assert!(is_strict_allowlisted("crates/perl-dap/tests/legacy.md"));
        assert!(!is_strict_allowlisted("vscode-extension/README.md"));
        assert!(!is_strict_allowlisted("docs/tutorials/DAP_USER_GUIDE.md"));
    }

    #[test]
    fn json_prose_flags_unqualified_marker_in_description() {
        // A bare external-tool requirement in a user-facing description value
        // must flag, even though the surrounding JSON keys/ids are clean.
        let json = r#"{
            "contributes": {
                "configuration": {
                    "properties": {
                        "perl-lsp.perlcritic.enabled": {
                            "type": "boolean",
                            "description": "Install perltidy to enable formatting."
                        }
                    }
                }
            }
        }"#;
        let mut violations = Vec::new();
        collect_strict_json_prose_violations("package.json", json, &mut violations);
        assert_eq!(violations.len(), 1, "unqualified prose value must flag: {violations:?}");
        assert!(violations[0].contains("perltidy"));
        assert!(violations[0].contains("prose field"));
    }

    #[test]
    fn json_prose_ignores_keys_and_ids() {
        // Setting ids and command ids legitimately contain the tool name; only
        // prose VALUES are scanned, so a manifest whose descriptions are all
        // native-first must pass even with `perlcritic` all over its keys/ids.
        let json = r#"{
            "contributes": {
                "commands": [
                    { "command": "perl-lsp.runPerlCritic", "title": "Perl: Run Critic" }
                ],
                "configuration": {
                    "properties": {
                        "perl-lsp.perlcritic.enabled": {
                            "description": "Enable the native critic (external perlcritic is optional).",
                            "markdownDescription": "Native critic runs by default; `perlcritic` is not required."
                        }
                    }
                }
            }
        }"#;
        let mut violations = Vec::new();
        collect_strict_json_prose_violations("package.json", json, &mut violations);
        assert!(violations.is_empty(), "clean prose + tool-named ids must pass: {violations:?}");
    }

    #[test]
    fn json_prose_scans_title_and_detail_and_nested_values() {
        // All four prose keys are scanned, at any nesting depth (walkthrough
        // steps, nested config), and each unqualified marker flags once.
        let json = r#"{
            "contributes": {
                "walkthroughs": [{
                    "steps": [{
                        "title": "Set up perltidy",
                        "description": "You must install perlcritic before starting."
                    }]
                }],
                "menus": {
                    "commandPalette": [{ "detail": "Runs perltidy on save." }]
                }
            }
        }"#;
        let mut violations = Vec::new();
        collect_strict_json_prose_violations("package.json", json, &mut violations);
        // title:perltidy, description:perlcritic, detail:perltidy => 3
        assert_eq!(violations.len(), 3, "title/description/detail all scanned: {violations:?}");
    }

    #[test]
    fn json_prose_scans_enum_descriptions_and_deprecation_messages() {
        // `enumDescriptions` (array of per-option strings) and
        // `deprecationMessage` (string) are user-facing Settings-UI copy. A bare
        // external-tool requirement in either must flag — this is the leak class
        // a `description`-only key list missed (the live `perl-lsp.critic.engine`
        // enumDescription names Perl::Critic and is saved only by an incidental
        // "legacy"; without a qualifier it must be caught).
        let json = r#"{
            "contributes": {
                "configuration": {
                    "properties": {
                        "perl-lsp.critic.engine": {
                            "enum": ["native", "legacy"],
                            "enumDescriptions": [
                                "Native built-in critic (no external tools)",
                                "Requires the perlcritic binary on PATH"
                            ],
                            "deprecationMessage": "Install perltidy to format."
                        }
                    }
                }
            }
        }"#;
        let mut violations = Vec::new();
        collect_strict_json_prose_violations("package.json", json, &mut violations);
        // enumDescriptions[1]:perlcritic (unqualified), deprecationMessage:perltidy => 2.
        // enumDescriptions[0] carries no marker; a native-first qualifier is absent
        // on the two leaking entries, so both flag.
        assert_eq!(violations.len(), 2, "enum + deprecation prose scanned: {violations:?}");
        assert!(violations.iter().any(|v| v.contains("perlcritic")));
        assert!(violations.iter().any(|v| v.contains("perltidy")));
    }

    #[test]
    fn json_prose_enum_description_with_legacy_qualifier_passes() {
        // Mirrors the live manifest: an enumDescription that names the external
        // tool but frames it as `legacy` is native-first and must NOT flag.
        let json = r#"{
            "contributes": {
                "configuration": {
                    "properties": {
                        "perl-lsp.critic.engine": {
                            "enumDescriptions": [
                                "Native built-in critic (default, no external tools required)",
                                "Legacy external Perl::Critic (requires the perlcritic binary on PATH)"
                            ]
                        }
                    }
                }
            }
        }"#;
        let mut violations = Vec::new();
        collect_strict_json_prose_violations("package.json", json, &mut violations);
        assert!(violations.is_empty(), "legacy-qualified enum prose must pass: {violations:?}");
    }

    #[test]
    fn json_prose_qualifier_does_not_cross_sentences() {
        // A native-first qualifier in one sentence must NOT exempt an
        // unqualified external-tool requirement in another sentence of the same
        // prose value. The qualifier check is scoped per sentence/clause.
        let json = r#"{
            "contributes": {
                "configuration": {
                    "properties": {
                        "perl-lsp.critic.engine": {
                            "description": "Legacy mode is documented below. Install perltidy to enable formatting."
                        }
                    }
                }
            }
        }"#;
        let mut violations = Vec::new();
        collect_strict_json_prose_violations("package.json", json, &mut violations);
        assert_eq!(violations.len(), 1, "cross-sentence leak must flag: {violations:?}");
        assert!(violations[0].contains("perltidy"));
        // The snippet must be the offending sentence, not the whole value.
        assert!(
            violations[0].contains("Install perltidy"),
            "snippet is the leaking sentence: {violations:?}"
        );
    }

    #[test]
    fn json_prose_qualifier_in_same_clause_passes() {
        // A marker framed native-first within its own clause must still pass,
        // even when other clauses exist.
        let json = r#"{
            "contributes": {
                "configuration": {
                    "properties": {
                        "perl-lsp.critic.engine": {
                            "description": "Native critic is the default; the legacy engine shells out to an external perlcritic binary for compatibility."
                        }
                    }
                }
            }
        }"#;
        let mut violations = Vec::new();
        collect_strict_json_prose_violations("package.json", json, &mut violations);
        assert!(violations.is_empty(), "clause-local qualifier must pass: {violations:?}");
    }

    #[test]
    fn json_prose_skips_unparseable_manifest() {
        // A manifest that does not parse is skipped silently (the default
        // DISALLOWED pass still greps its raw text); no panic, no violations.
        let mut violations = Vec::new();
        collect_strict_json_prose_violations("package.json", "{ not valid json", &mut violations);
        assert!(violations.is_empty());
    }

    /// The live repository's first-mile `.md` surfaces must pass strict mode,
    /// and the `package.json` manifest must be prose-clean. This is the
    /// enforcement that gives `--strict` teeth: a future edit that drops an
    /// unqualified `perltidy`/`perlcritic`/`Perl::LanguageServer` onto a
    /// first-mile prose surface — a `.md` line or a manifest description/title —
    /// makes `run_with(true)` bail and this fails.
    #[test]
    fn live_strict_surface_is_clean() -> Result<()> {
        run_with(true)
    }
}
