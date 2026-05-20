//! Scenario 42 - Catalyst semantic-token false-exact and freshness receipt.
//!
//! This receipt exercises `textDocument/semanticTokens/full` over the committed
//! Catalyst skeleton workspace. It keeps the semantic-token lane black-box and
//! user-facing: generated/dynamic-looking Catalyst source shapes must not become
//! exact symbol tokens, and an edit must refresh token text.

use anyhow::{Context, Result, anyhow};
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_42_catalyst_semantic_tokens_false_exact_freshness.rs";
const FALSE_EXACT_FILE: &str = "lib/Catalyst/Log.pm";
const FRESHNESS_FILE: &str = "lib/Catalyst/Dispatcher.pm";

const TOKEN_TYPE_NAMES: [&str; 23] = [
    "namespace",
    "type",
    "class",
    "interface",
    "enum",
    "enumMember",
    "typeParameter",
    "function",
    "method",
    "property",
    "macro",
    "variable",
    "parameter",
    "keyword",
    "modifier",
    "comment",
    "string",
    "number",
    "regexp",
    "operator",
    "sql_string",
    "sql_heredoc_keyword",
    "json_heredoc_key",
];

const TOKEN_MODIFIER_COUNT: u32 = 13;
const GENERATED_METHOD_CANDIDATES: [&str; 10] = [
    "is_debug", "is_info", "is_warn", "is_error", "is_fatal", "debug", "info", "warn", "error",
    "fatal",
];
const DYNAMIC_BOUNDARY_TEXTS: [&str; 3] = ["is_$level", "*{$level}", "${\\\"is_$level\"}"];

#[derive(Debug)]
struct FixtureFile {
    relative_path: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct DecodedToken {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
    token_type_name: &'static str,
    modifiers: u32,
    text: String,
}

#[derive(Debug, Clone)]
struct SourceRange {
    line: u32,
    start: u32,
    end: u32,
}

#[derive(Debug, Serialize)]
struct FalseExactReport {
    file: &'static str,
    token_count: usize,
    dynamic_boundary_source_hit_count: usize,
    dynamic_boundary_symbol_misclassifications: Vec<String>,
    generated_method_candidate_count: usize,
    generated_method_symbol_hits: Vec<String>,
    source_backed_hits: Vec<String>,
    invalid_tuple_count: usize,
    invalid_type_count: usize,
    invalid_modifier_count: usize,
    overlap_count: usize,
    token_type_counts: BTreeMap<&'static str, usize>,
}

#[derive(Debug, Serialize)]
struct FreshnessReport {
    file: &'static str,
    before_token_count: usize,
    after_token_count: usize,
    stale_token_absent: bool,
    fresh_token_present: bool,
    before_hits: Vec<String>,
    after_hits: Vec<String>,
    after_invalid_tuple_count: usize,
    after_overlap_count: usize,
}

fn workspace_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("CARGO_MANIFEST_DIR must be nested under the workspace root")
}

fn catalyst_fixture_root() -> Result<PathBuf> {
    Ok(workspace_root()?.join("test_corpus").join("real_projects").join("catalyst_skeleton"))
}

fn is_perl_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "pm" | "pl" | "t"))
}

fn collect_perl_files(root: &Path, dir: &Path, files: &mut Vec<FixtureFile>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry.with_context(|| format!("reading an entry under {}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_perl_files(root, &path, files)?;
        } else if is_perl_source(&path) {
            let relative_path = path
                .strip_prefix(root)
                .with_context(|| format!("stripping fixture root from {}", path.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            let content =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            files.push(FixtureFile { relative_path, content });
        }
    }
    Ok(())
}

fn load_catalyst_fixture_files() -> Result<Vec<FixtureFile>> {
    let root = catalyst_fixture_root()?;
    let mut files = Vec::new();
    collect_perl_files(&root, &root, &mut files)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

fn create_catalyst_harness(files: &[FixtureFile]) -> Result<UxHarness> {
    let mut config = ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
        .env("PERL_LSP_WORKSPACE", "1");

    for file in files {
        config = config.with_file(&file.relative_path, &file.content);
    }

    UxHarness::new(config)
}

fn open_all_fixture_files(harness: &UxHarness, files: &[FixtureFile]) -> Result<()> {
    for file in files {
        harness.open_file(&file.relative_path, &file.content)?;
    }
    Ok(())
}

fn fixture_content<'a>(files: &'a [FixtureFile], relative_path: &str) -> Result<&'a str> {
    files
        .iter()
        .find(|file| file.relative_path == relative_path)
        .map(|file| file.content.as_str())
        .with_context(|| format!("missing fixture file {relative_path}"))
}

fn semantic_tokens(harness: &UxHarness, relative_path: &str) -> Result<Value> {
    let uri = harness.workspace.uri(relative_path);
    let response = harness.client.request(
        "textDocument/semanticTokens/full",
        json!({
            "textDocument": { "uri": uri }
        }),
        Duration::from_secs(20),
    )?;
    if response.get("error").is_some() {
        return Err(anyhow!("semanticTokens returned error: {}", response["error"]));
    }
    Ok(response["result"].clone())
}

fn token_type_name(index: u32) -> &'static str {
    TOKEN_TYPE_NAMES.get(index as usize).copied().unwrap_or("invalid")
}

fn valid_modifier_mask() -> u32 {
    (1_u32 << TOKEN_MODIFIER_COUNT) - 1
}

fn as_u32(value: &Value, field: &str) -> Result<u32> {
    let raw = value.as_u64().with_context(|| format!("{field} must be an unsigned integer"))?;
    u32::try_from(raw).with_context(|| format!("{field} value {raw} does not fit in u32"))
}

fn decode_tokens(result: &Value, source: &str) -> Result<(Vec<DecodedToken>, usize)> {
    let Some(data) = result.get("data").and_then(Value::as_array) else {
        if result.is_null() {
            return Ok((Vec::new(), 0));
        }
        return Err(anyhow!("semanticTokens result must contain a data array or null: {result:?}"));
    };

    let invalid_tuple_count = usize::from(data.len() % 5 != 0);
    let mut tokens = Vec::new();
    let mut line = 0_u32;
    let mut start = 0_u32;
    let lines = source.lines().collect::<Vec<_>>();

    for chunk in data.chunks(5) {
        if chunk.len() != 5 {
            continue;
        }

        let delta_line = as_u32(&chunk[0], "deltaLine")?;
        let delta_start = as_u32(&chunk[1], "deltaStart")?;
        let length = as_u32(&chunk[2], "length")?;
        let token_type = as_u32(&chunk[3], "tokenType")?;
        let modifiers = as_u32(&chunk[4], "tokenModifiers")?;

        line = line.saturating_add(delta_line);
        start = if delta_line == 0 { start.saturating_add(delta_start) } else { delta_start };

        let text = lines
            .get(line as usize)
            .and_then(|source_line| slice_by_utf16(source_line, start, length))
            .unwrap_or_default();

        tokens.push(DecodedToken {
            line,
            start,
            length,
            token_type,
            token_type_name: token_type_name(token_type),
            modifiers,
            text,
        });
    }

    Ok((tokens, invalid_tuple_count))
}

fn slice_by_utf16(line: &str, start: u32, length: u32) -> Option<String> {
    let end = start.checked_add(length)?;
    let mut units = 0_u32;
    let mut start_byte = None;
    let mut end_byte = None;

    if start == 0 {
        start_byte = Some(0);
    }

    for (idx, ch) in line.char_indices() {
        if units == start && start_byte.is_none() {
            start_byte = Some(idx);
        }
        units = units.checked_add(ch.len_utf16() as u32)?;
        if units == end {
            end_byte = Some(idx + ch.len_utf8());
            break;
        }
    }

    if end == units && end_byte.is_none() {
        end_byte = Some(line.len());
    }

    match (start_byte, end_byte) {
        (Some(start_idx), Some(end_idx)) if start_idx <= end_idx => {
            Some(line[start_idx..end_idx].to_string())
        }
        _ => None,
    }
}

fn source_ranges(source: &str, texts: &[&str]) -> Vec<SourceRange> {
    let mut ranges = Vec::new();

    for text in texts {
        for (line_index, line) in source.lines().enumerate() {
            let mut search_start = 0_usize;
            while let Some(relative_start) = line[search_start..].find(text) {
                let start_byte = search_start + relative_start;
                let end_byte = start_byte + text.len();
                let start = line[..start_byte].encode_utf16().count() as u32;
                let end = start + line[start_byte..end_byte].encode_utf16().count() as u32;
                ranges.push(SourceRange { line: line_index as u32, start, end });
                search_start = end_byte;
            }
        }
    }

    ranges
}

fn token_inside_any_range(token: &DecodedToken, ranges: &[SourceRange]) -> bool {
    ranges.iter().any(|range| {
        token.line == range.line
            && token.start >= range.start
            && token.start.saturating_add(token.length) <= range.end
    })
}

fn overlap_count(tokens: &[DecodedToken]) -> usize {
    let mut last_end_by_line = BTreeMap::new();
    let mut overlaps = 0_usize;

    for token in tokens {
        let end = token.start.saturating_add(token.length);
        if let Some(previous_end) = last_end_by_line.get(&token.line) {
            if token.start < *previous_end {
                overlaps += 1;
            }
        }
        last_end_by_line.insert(token.line, end);
    }

    overlaps
}

fn invalid_type_count(tokens: &[DecodedToken]) -> usize {
    tokens.iter().filter(|token| token.token_type_name == "invalid").count()
}

fn invalid_modifier_count(tokens: &[DecodedToken]) -> usize {
    let mask = valid_modifier_mask();
    tokens.iter().filter(|token| token.modifiers & !mask != 0).count()
}

fn token_type_counts(tokens: &[DecodedToken]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for token in tokens {
        *counts.entry(token.token_type_name).or_insert(0) += 1;
    }
    counts
}

fn symbol_token_hits(tokens: &[DecodedToken], candidates: &[&str]) -> Vec<String> {
    let symbol_types = ["namespace", "class", "function", "method", "property"];
    let candidate_set = candidates.iter().copied().collect::<BTreeSet<_>>();
    let mut hits = tokens
        .iter()
        .filter(|token| candidate_set.contains(token.text.as_str()))
        .filter(|token| symbol_types.contains(&token.token_type_name))
        .map(|token| format!("{}:{}:{}", token.text, token.token_type_name, token.line))
        .collect::<Vec<_>>();
    hits.sort();
    hits.dedup();
    hits
}

fn token_text_hits(tokens: &[DecodedToken], names: &[&str]) -> Vec<String> {
    names
        .iter()
        .filter(|name| tokens.iter().any(|token| token.text == **name))
        .map(|name| (*name).to_string())
        .collect()
}

fn false_exact_report(harness: &UxHarness, files: &[FixtureFile]) -> Result<FalseExactReport> {
    let source = fixture_content(files, FALSE_EXACT_FILE)?;
    let result = semantic_tokens(harness, FALSE_EXACT_FILE)?;
    let (tokens, invalid_tuple_count) = decode_tokens(&result, source)?;
    let dynamic_ranges = source_ranges(source, &DYNAMIC_BOUNDARY_TEXTS);
    let dynamic_boundary_symbol_misclassifications =
        symbol_token_hits_inside_ranges(&tokens, &dynamic_ranges);
    let generated_method_symbol_hits = symbol_token_hits(&tokens, &GENERATED_METHOD_CANDIDATES);
    let source_backed_hits = token_text_hits(&tokens, &["Catalyst::Log", "_build__level_num"]);

    Ok(FalseExactReport {
        file: FALSE_EXACT_FILE,
        token_count: tokens.len(),
        dynamic_boundary_source_hit_count: dynamic_ranges.len(),
        dynamic_boundary_symbol_misclassifications,
        generated_method_candidate_count: GENERATED_METHOD_CANDIDATES.len(),
        generated_method_symbol_hits,
        source_backed_hits,
        invalid_tuple_count,
        invalid_type_count: invalid_type_count(&tokens),
        invalid_modifier_count: invalid_modifier_count(&tokens),
        overlap_count: overlap_count(&tokens),
        token_type_counts: token_type_counts(&tokens),
    })
}

fn symbol_token_hits_inside_ranges(tokens: &[DecodedToken], ranges: &[SourceRange]) -> Vec<String> {
    let symbol_types = ["namespace", "class", "function", "method", "property"];
    tokens
        .iter()
        .filter(|token| token_inside_any_range(token, ranges))
        .filter(|token| symbol_types.contains(&token.token_type_name))
        .map(|token| format!("{}:{}:{}", token.text, token.token_type_name, token.line))
        .collect()
}

fn freshness_report(harness: &UxHarness, files: &[FixtureFile]) -> Result<FreshnessReport> {
    let original = fixture_content(files, FRESHNESS_FILE)?;
    let before = semantic_tokens(harness, FRESHNESS_FILE)?;
    let (before_tokens, before_invalid_tuple_count) = decode_tokens(&before, original)?;
    if before_invalid_tuple_count > 0 {
        return Err(anyhow!("freshness before-edit token data contained invalid tuples"));
    }

    let updated = original.replace("sub get_action", "sub get_registered_action");
    if updated == original {
        return Err(anyhow!("freshness fixture must rename get_action to get_registered_action"));
    }

    harness.change_file_full(FRESHNESS_FILE, &updated)?;
    std::thread::sleep(Duration::from_millis(500));

    let after = semantic_tokens(harness, FRESHNESS_FILE)?;
    let (after_tokens, after_invalid_tuple_count) = decode_tokens(&after, &updated)?;
    let before_hits = token_text_hits(&before_tokens, &["get_action"]);
    let after_hits = token_text_hits(&after_tokens, &["get_action", "get_registered_action"]);
    let stale_token_absent = !after_tokens.iter().any(|token| token.text == "get_action");
    let fresh_token_present =
        after_tokens.iter().any(|token| token.text == "get_registered_action");

    Ok(FreshnessReport {
        file: FRESHNESS_FILE,
        before_token_count: before_tokens.len(),
        after_token_count: after_tokens.len(),
        stale_token_absent,
        fresh_token_present,
        before_hits,
        after_hits,
        after_invalid_tuple_count,
        after_overlap_count: overlap_count(&after_tokens),
    })
}

#[test]
fn scenario_42_catalyst_semantic_tokens_false_exact_freshness_receipt() {
    run_ux_scenario(
        "catalyst_semantic_tokens_false_exact_freshness",
        SCENARIO_FILE,
        "scenario_42_catalyst_semantic_tokens_false_exact_freshness_receipt",
        UxCiTier::Pr,
        Some(UxComponent::SemanticTokens),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let fixture_files = load_catalyst_fixture_files()?;
            recorder
                .check("Catalyst fixture has committed Perl files", !fixture_files.is_empty())?;

            let harness = create_catalyst_harness(&fixture_files)?;
            open_all_fixture_files(&harness, &fixture_files)?;
            std::thread::sleep(Duration::from_millis(500));

            recorder.mark_request_start("false_exact_dynamic_generated_shapes");
            let false_exact = false_exact_report(&harness, &fixture_files)?;
            if false_exact.token_count > 0 {
                recorder.mark_first_useful_result("false_exact_dynamic_generated_shapes");
            }
            eprintln!(
                "semantic_token_false_exact file={} tokens={} dynamic_hits={} dynamic_misclassified={:?} generated_symbol_hits={:?}",
                false_exact.file,
                false_exact.token_count,
                false_exact.dynamic_boundary_source_hit_count,
                false_exact.dynamic_boundary_symbol_misclassifications,
                false_exact.generated_method_symbol_hits
            );

            recorder.check(
                "Catalyst false-exact probe produced semantic tokens",
                false_exact.token_count > 0,
            )?;
            recorder.check(
                "Catalyst false-exact token tuples are valid",
                false_exact.invalid_tuple_count == 0,
            )?;
            recorder.check(
                "Catalyst false-exact token types are valid",
                false_exact.invalid_type_count == 0,
            )?;
            recorder.check(
                "Catalyst false-exact token modifiers are valid",
                false_exact.invalid_modifier_count == 0,
            )?;
            recorder.check(
                "Catalyst false-exact token spans do not overlap",
                false_exact.overlap_count == 0,
            )?;
            recorder.check(
                "Catalyst source-backed symbols still tokenize",
                false_exact.source_backed_hits.len() >= 2,
            )?;
            recorder.check(
                "Catalyst dynamic boundary source shapes were present",
                false_exact.dynamic_boundary_source_hit_count >= DYNAMIC_BOUNDARY_TEXTS.len(),
            )?;
            recorder.check(
                "Catalyst dynamic boundary shapes are not exact symbol tokens",
                false_exact.dynamic_boundary_symbol_misclassifications.is_empty(),
            )?;
            recorder.check(
                "Catalyst generated method candidates stay false-exact",
                false_exact.generated_method_symbol_hits.is_empty(),
            )?;

            recorder.mark_request_start("freshness_after_edit");
            let freshness = freshness_report(&harness, &fixture_files)?;
            if freshness.stale_token_absent && freshness.fresh_token_present {
                recorder.mark_first_useful_result("freshness_after_edit");
            }
            eprintln!(
                "semantic_token_freshness file={} stale_absent={} fresh_present={} before_hits={:?} after_hits={:?}",
                freshness.file,
                freshness.stale_token_absent,
                freshness.fresh_token_present,
                freshness.before_hits,
                freshness.after_hits
            );

            recorder.check(
                "Catalyst freshness before edit had live tokens",
                freshness.before_token_count > 0,
            )?;
            recorder.check(
                "Catalyst freshness after edit had live tokens",
                freshness.after_token_count > 0,
            )?;
            recorder.check(
                "Catalyst stale semantic-token text disappears after edit",
                freshness.stale_token_absent,
            )?;
            recorder.check(
                "Catalyst fresh semantic-token text appears after edit",
                freshness.fresh_token_present,
            )?;
            recorder.check(
                "Catalyst after-edit token tuples are valid",
                freshness.after_invalid_tuple_count == 0,
            )?;
            recorder.check(
                "Catalyst after-edit semantic-token spans do not overlap",
                freshness.after_overlap_count == 0,
            )?;

            eprintln!(
                "semantic_token_claim_boundary=Catalyst project-shaped semantic-token receipt only: generated/dynamic-looking source shapes are false-exact, edit freshness is observed, and no broader compiler-backed token cutover is claimed."
            );
            Ok(())
        },
    );
}
