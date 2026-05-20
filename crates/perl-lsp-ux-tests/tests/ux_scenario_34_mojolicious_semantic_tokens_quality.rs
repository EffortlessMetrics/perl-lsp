//! Scenario 34 - Mojolicious semantic-token quality receipt.
//!
//! This receipt exercises `textDocument/semanticTokens/full` over the committed
//! Mojolicious skeleton workspace. It records project-shaped live token quality
//! without changing provider behavior or claiming compiler-backed token cutover.
//!
//! Receipt signals:
//! - valid LSP 5-tuple semantic-token encoding
//! - non-overlapping token spans for selected real-workspace files
//! - expected source-backed package, subroutine, keyword, and variable tokens
//! - dynamic-boundary-shaped strings are not promoted as exact code symbols
//! - freshness after editing a document so stale token text disappears

use anyhow::{Context, Result, anyhow};
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{
    ProjectFixtureFile as FixtureFile, UxCiTier, UxComponent, UxHarness, create_fixture_harness,
    fixture_content, load_mojolicious_fixture_files, open_all_fixture_files, run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_34_mojolicious_semantic_tokens_quality.rs";
const FRESHNESS_FILE: &str = "lib/Mojolicious/Static.pm";

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

#[derive(Debug)]
struct SemanticTokenProbe {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    expected_tokens: &'static [ExpectedToken],
    dynamic_boundary_texts: &'static [&'static str],
}

#[derive(Debug)]
struct ExpectedToken {
    text: &'static str,
    allowed_types: &'static [&'static str],
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
struct SemanticTokenProbeReport {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    token_count: usize,
    invalid_tuple_count: usize,
    invalid_type_count: usize,
    invalid_modifier_count: usize,
    overlap_count: usize,
    source_slice_miss_count: usize,
    expected_hits: Vec<String>,
    missing_expected_tokens: Vec<String>,
    dynamic_boundary_candidate_count: usize,
    dynamic_boundary_source_hit_count: usize,
    dynamic_boundary_symbol_misclassifications: Vec<String>,
    token_type_counts: BTreeMap<&'static str, usize>,
    token_sample: Vec<String>,
    fallback_or_empty: bool,
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

fn decode_tokens(result: &Value, source: &str) -> Result<(Vec<DecodedToken>, usize, usize)> {
    let Some(data) = result.get("data").and_then(Value::as_array) else {
        if result.is_null() {
            return Ok((Vec::new(), 0, 0));
        }
        return Err(anyhow!("semanticTokens result must contain a data array or null: {result:?}"));
    };

    let invalid_tuple_count = usize::from(data.len() % 5 != 0);
    let mut tokens = Vec::new();
    let mut line = 0_u32;
    let mut start = 0_u32;
    let mut source_slice_miss_count = 0_usize;
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
            .unwrap_or_else(|| {
                source_slice_miss_count += 1;
                String::new()
            });

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

    Ok((tokens, invalid_tuple_count, source_slice_miss_count))
}

fn as_u32(value: &Value, field: &str) -> Result<u32> {
    let raw = value.as_u64().with_context(|| format!("{field} must be an unsigned integer"))?;
    u32::try_from(raw).with_context(|| format!("{field} value {raw} does not fit in u32"))
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

fn token_type_counts(tokens: &[DecodedToken]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for token in tokens {
        *counts.entry(token.token_type_name).or_insert(0) += 1;
    }
    counts
}

fn matching_expected_tokens(
    tokens: &[DecodedToken],
    expected: &[ExpectedToken],
) -> (Vec<String>, Vec<String>) {
    let mut hits = Vec::new();
    let mut misses = Vec::new();

    for item in expected {
        let matched = tokens.iter().any(|token| {
            token.text == item.text && item.allowed_types.contains(&token.token_type_name)
        });
        if matched {
            hits.push(format!("{}:{:?}", item.text, item.allowed_types));
        } else {
            misses.push(format!("{}:{:?}", item.text, item.allowed_types));
        }
    }

    (hits, misses)
}

fn dynamic_boundary_symbol_misclassifications(
    tokens: &[DecodedToken],
    dynamic_boundary_ranges: &[SourceRange],
) -> Vec<String> {
    let symbol_types = ["namespace", "class", "function", "method", "property"];
    tokens
        .iter()
        .filter(|token| token_inside_any_range(token, dynamic_boundary_ranges))
        .filter(|token| symbol_types.contains(&token.token_type_name))
        .map(|token| format!("{}:{}:{}", token.text, token.token_type_name, token.line))
        .collect()
}

fn dynamic_boundary_source_ranges(source: &str, texts: &[&str]) -> Vec<SourceRange> {
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

fn invalid_type_count(tokens: &[DecodedToken]) -> usize {
    tokens.iter().filter(|token| token.token_type_name == "invalid").count()
}

fn invalid_modifier_count(tokens: &[DecodedToken]) -> usize {
    let mask = valid_modifier_mask();
    tokens.iter().filter(|token| token.modifiers & !mask != 0).count()
}

fn token_sample(tokens: &[DecodedToken]) -> Vec<String> {
    tokens
        .iter()
        .filter(|token| !token.text.is_empty())
        .take(20)
        .map(|token| {
            format!("{}:{}:{}:{}", token.line, token.start, token.token_type_name, token.text)
        })
        .collect()
}

fn semantic_token_probes() -> Vec<SemanticTokenProbe> {
    vec![
        SemanticTokenProbe {
            name: "app_tokens",
            category: "source_backed_package_sub_and_vars",
            file: "lib/Mojolicious.pm",
            expected_tokens: &[
                ExpectedToken { text: "package", allowed_types: &["keyword"] },
                ExpectedToken { text: "Mojolicious", allowed_types: &["namespace", "class"] },
                ExpectedToken { text: "sub", allowed_types: &["keyword"] },
                ExpectedToken { text: "new", allowed_types: &["function", "method"] },
                ExpectedToken { text: "$self", allowed_types: &["variable", "parameter"] },
            ],
            dynamic_boundary_texts: &[],
        },
        SemanticTokenProbe {
            name: "controller_tokens",
            category: "source_backed_controller_symbols",
            file: "lib/Mojolicious/Controller.pm",
            expected_tokens: &[
                ExpectedToken {
                    text: "Mojolicious::Controller",
                    allowed_types: &["namespace", "class"],
                },
                ExpectedToken { text: "render", allowed_types: &["function", "method"] },
                ExpectedToken { text: "rendered", allowed_types: &["function", "method"] },
                ExpectedToken { text: "$self", allowed_types: &["variable", "parameter"] },
            ],
            dynamic_boundary_texts: &[],
        },
        SemanticTokenProbe {
            name: "routes_dynamic_boundary_tokens",
            category: "dynamic_boundary_shape",
            file: "lib/Mojolicious/Routes.pm",
            expected_tokens: &[
                ExpectedToken {
                    text: "Mojolicious::Routes",
                    allowed_types: &["namespace", "class"],
                },
                ExpectedToken { text: "add_shortcut", allowed_types: &["function", "method"] },
                ExpectedToken { text: "get", allowed_types: &["function", "method"] },
                ExpectedToken { text: "$name", allowed_types: &["variable", "parameter"] },
            ],
            dynamic_boundary_texts: &["\"Mojolicious::Routes::Route::$name\""],
        },
        SemanticTokenProbe {
            name: "mojo_base_typeglob_tokens",
            category: "typeglob_generated_boundary",
            file: "lib/Mojo/Base.pm",
            expected_tokens: &[
                ExpectedToken { text: "Mojo::Base", allowed_types: &["namespace", "class"] },
                ExpectedToken { text: "import", allowed_types: &["function", "method"] },
                ExpectedToken { text: "_attr", allowed_types: &["function", "method"] },
                ExpectedToken { text: "$class", allowed_types: &["variable", "parameter"] },
            ],
            dynamic_boundary_texts: &["\"${class}::${name}\""],
        },
    ]
}

fn probe_report(
    harness: &UxHarness,
    files: &[FixtureFile],
    probe: &SemanticTokenProbe,
) -> Result<SemanticTokenProbeReport> {
    let source = fixture_content(files, probe.file)?;
    let result = semantic_tokens(harness, probe.file)?;
    let (tokens, invalid_tuple_count, source_slice_miss_count) = decode_tokens(&result, source)?;
    let (expected_hits, missing_expected_tokens) =
        matching_expected_tokens(&tokens, probe.expected_tokens);
    let dynamic_boundary_source_ranges =
        dynamic_boundary_source_ranges(source, probe.dynamic_boundary_texts);
    let dynamic_boundary_source_hit_count = dynamic_boundary_source_ranges.len();
    let dynamic_boundary_symbol_misclassifications =
        dynamic_boundary_symbol_misclassifications(&tokens, &dynamic_boundary_source_ranges);

    Ok(SemanticTokenProbeReport {
        name: probe.name,
        category: probe.category,
        file: probe.file,
        token_count: tokens.len(),
        invalid_tuple_count,
        invalid_type_count: invalid_type_count(&tokens),
        invalid_modifier_count: invalid_modifier_count(&tokens),
        overlap_count: overlap_count(&tokens),
        source_slice_miss_count,
        expected_hits,
        missing_expected_tokens,
        dynamic_boundary_candidate_count: probe.dynamic_boundary_texts.len(),
        dynamic_boundary_source_hit_count,
        dynamic_boundary_symbol_misclassifications,
        token_type_counts: token_type_counts(&tokens),
        token_sample: token_sample(&tokens),
        fallback_or_empty: tokens.is_empty(),
    })
}

fn token_text_hits(tokens: &[DecodedToken], names: &[&str]) -> Vec<String> {
    names
        .iter()
        .filter(|name| tokens.iter().any(|token| token.text == **name))
        .map(|name| (*name).to_string())
        .collect()
}

fn freshness_report(harness: &UxHarness, files: &[FixtureFile]) -> Result<FreshnessReport> {
    let original = fixture_content(files, FRESHNESS_FILE)?;
    let before = semantic_tokens(harness, FRESHNESS_FILE)?;
    let (before_tokens, before_invalid_tuple_count, _) = decode_tokens(&before, original)?;
    if before_invalid_tuple_count > 0 {
        return Err(anyhow!("freshness before-edit token data contained invalid tuples"));
    }

    let updated = original.replace("sub serve_asset", "sub serve_blob");
    if updated == original {
        return Err(anyhow!("freshness fixture must rename serve_asset to serve_blob"));
    }

    harness.change_file_full(FRESHNESS_FILE, &updated)?;
    std::thread::sleep(Duration::from_millis(300));

    let after = semantic_tokens(harness, FRESHNESS_FILE)?;
    let (after_tokens, after_invalid_tuple_count, _) = decode_tokens(&after, &updated)?;
    let before_hits = token_text_hits(&before_tokens, &["serve_asset"]);
    let after_hits = token_text_hits(&after_tokens, &["serve_asset", "serve_blob"]);
    let stale_token_absent = !after_tokens.iter().any(|token| token.text == "serve_asset");
    let fresh_token_present = after_tokens.iter().any(|token| token.text == "serve_blob");

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
fn scenario_34_mojolicious_semantic_tokens_quality_receipt() {
    run_ux_scenario(
        "mojolicious_semantic_tokens_quality",
        SCENARIO_FILE,
        "scenario_34_mojolicious_semantic_tokens_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::SemanticTokens),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let fixture_files = load_mojolicious_fixture_files()?;
            recorder
                .check("mojolicious fixture has committed Perl files", !fixture_files.is_empty())?;

            let harness = create_fixture_harness(&fixture_files)?;
            open_all_fixture_files(&harness, &fixture_files)?;
            std::thread::sleep(Duration::from_millis(500));

            let probes = semantic_token_probes();
            let mut reports = Vec::new();

            for probe in &probes {
                let request_name = format!("semantic_tokens_{}", probe.name);
                harness.assert_no_crash();
                recorder.mark_request_start(&request_name);

                let report = probe_report(&harness, &fixture_files, probe)?;
                if !report.fallback_or_empty {
                    recorder.mark_first_useful_result(&request_name);
                }
                eprintln!(
                    "semantic_token_probe={} category={} tokens={} expected_hits={:?} missing={:?} dynamic_misclassified={:?}",
                    report.name,
                    report.category,
                    report.token_count,
                    report.expected_hits,
                    report.missing_expected_tokens,
                    report.dynamic_boundary_symbol_misclassifications
                );
                reports.push(report);
            }

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

            let categories = reports.iter().map(|report| report.category).collect::<BTreeSet<_>>();
            let token_total: usize = reports.iter().map(|report| report.token_count).sum();
            let invalid_tuple_total: usize =
                reports.iter().map(|report| report.invalid_tuple_count).sum();
            let invalid_type_total: usize =
                reports.iter().map(|report| report.invalid_type_count).sum();
            let invalid_modifier_total: usize =
                reports.iter().map(|report| report.invalid_modifier_count).sum();
            let overlap_total: usize = reports.iter().map(|report| report.overlap_count).sum();
            let source_slice_miss_total: usize =
                reports.iter().map(|report| report.source_slice_miss_count).sum();
            let missing_expected_total: usize =
                reports.iter().map(|report| report.missing_expected_tokens.len()).sum();
            let dynamic_boundary_candidate_total: usize =
                reports.iter().map(|report| report.dynamic_boundary_candidate_count).sum();
            let dynamic_boundary_source_hit_total: usize =
                reports.iter().map(|report| report.dynamic_boundary_source_hit_count).sum();
            let dynamic_boundary_misclassification_total: usize = reports
                .iter()
                .map(|report| report.dynamic_boundary_symbol_misclassifications.len())
                .sum();
            let fallback_or_empty_count =
                reports.iter().filter(|report| report.fallback_or_empty).count();

            let receipt = json!({
                "schema_version": 1,
                "project": "mojolicious",
                "surface": "semantic_tokens",
                "claim_boundary": "real-workspace semantic-token quality receipt only; no compiler-backed semantic-token live cutover",
                "fixture_file_count": fixture_files.len(),
                "probe_count": reports.len(),
                "token_total": token_total,
                "invalid_tuple_total": invalid_tuple_total,
                "invalid_type_total": invalid_type_total,
                "invalid_modifier_total": invalid_modifier_total,
                "overlap_total": overlap_total,
                "source_slice_miss_total": source_slice_miss_total,
                "missing_expected_total": missing_expected_total,
                "dynamic_boundary_candidate_total": dynamic_boundary_candidate_total,
                "dynamic_boundary_source_hit_total": dynamic_boundary_source_hit_total,
                "dynamic_boundary_misclassification_total": dynamic_boundary_misclassification_total,
                "fallback_or_empty_count": fallback_or_empty_count,
                "freshness": freshness,
                "reports": reports,
            });
            eprintln!(
                "mojolicious_semantic_tokens_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check(
                "all semantic-token probes produced reports",
                reports.len() == probes.len(),
            )?;
            recorder.check(
                "semantic-token probes covered intended receipt categories",
                categories
                    == BTreeSet::from([
                        "dynamic_boundary_shape",
                        "source_backed_controller_symbols",
                        "source_backed_package_sub_and_vars",
                        "typeglob_generated_boundary",
                    ]),
            )?;
            recorder.check("semantic-token probes returned live tokens", token_total > 0)?;
            recorder.check(
                "semantic-token data used valid 5-tuple encoding",
                invalid_tuple_total == 0,
            )?;
            recorder.check(
                "semantic-token type indices were in the advertised legend bounds",
                invalid_type_total == 0,
            )?;
            recorder.check(
                "semantic-token modifier bits stayed in the advertised legend bounds",
                invalid_modifier_total == 0,
            )?;
            recorder
                .check("semantic-token spans did not overlap within a line", overlap_total == 0)?;
            recorder.check(
                "semantic-token spans mapped back to source text",
                source_slice_miss_total == 0,
            )?;
            recorder.check(
                "expected real-workspace package, subroutine, keyword, and variable tokens were present",
                missing_expected_total == 0,
            )?;
            recorder.check(
                "dynamic-boundary-shaped strings were not promoted as exact code symbols",
                dynamic_boundary_candidate_total > 0
                    && dynamic_boundary_source_hit_total >= dynamic_boundary_candidate_total
                    && dynamic_boundary_misclassification_total == 0,
            )?;
            recorder.check(
                "semantic-token freshness after edit removed stale name and surfaced fresh name",
                freshness.stale_token_absent
                    && freshness.fresh_token_present
                    && freshness.after_invalid_tuple_count == 0
                    && freshness.after_overlap_count == 0,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
