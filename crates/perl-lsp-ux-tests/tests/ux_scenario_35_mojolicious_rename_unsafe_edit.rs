//! Scenario 35 - Mojolicious rename unsafe-edit receipt.
//!
//! This receipt exercises `textDocument/rename` over the committed
//! Mojolicious skeleton workspace. It records real-workspace rename safety
//! boundaries without broadening live rename behavior.
//!
//! Receipt signals:
//! - exact local lexical rename returns same-file scoped edits
//! - generated accessor and typeglob/dynamic-boundary probes do not produce
//!   unsafe edits
//! - after an open-document edit, rename does not act on stale source text

use anyhow::{Context, Result, anyhow};
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{
    ProjectFixtureFile as FixtureFile, UxCiTier, UxComponent, UxHarness, create_fixture_harness,
    fixture_content, load_mojolicious_fixture_files, open_all_fixture_files, run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_35_mojolicious_rename_unsafe_edit.rs";
const STATIC_FILE: &str = "lib/Mojolicious/Static.pm";
const ROUTES_FILE: &str = "lib/Mojolicious/Routes.pm";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RenameExpectation {
    ExactLocalEdits,
    NoUnsafeEdits,
}

#[derive(Debug)]
struct RenameProbe {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    zero_based_line: usize,
    needle: &'static str,
    cursor_offset: usize,
    new_name: &'static str,
    expectation: RenameExpectation,
}

#[derive(Debug)]
struct ProbePosition {
    line: u32,
    character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RenameResponseState {
    Edits,
    EmptyEdit,
    Null,
    Error,
}

#[derive(Debug, Clone, Serialize)]
struct RenameEdit {
    uri: String,
    relative_path: String,
    start_line: u64,
    start_character: u64,
    end_line: u64,
    end_character: u64,
    source_text: String,
    new_text: String,
}

#[derive(Debug, Serialize)]
struct RenameProbeReport {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    zero_based_line: u32,
    character: u32,
    expectation: RenameExpectation,
    response_state: RenameResponseState,
    edit_count: usize,
    edit_start_lines: Vec<u64>,
    touched_uris: Vec<String>,
    touched_relative_paths: Vec<String>,
    touched_source_texts: Vec<String>,
    new_texts: Vec<String>,
    error_message: Option<String>,
    blocked_or_empty: bool,
}

#[derive(Debug, Serialize)]
struct FreshnessReport {
    file: &'static str,
    old_name: &'static str,
    fresh_name: &'static str,
    requested_name: &'static str,
    response_state: RenameResponseState,
    edit_count: usize,
    stale_source_edit_count: usize,
    fresh_source_edit_count: usize,
    touched_source_texts: Vec<String>,
    new_texts: Vec<String>,
    error_message: Option<String>,
}

fn resolve_probe_position(files: &[FixtureFile], probe: &RenameProbe) -> Result<ProbePosition> {
    let content = fixture_content(files, probe.file)?;
    resolve_position_in_source(
        content,
        probe.file,
        probe.zero_based_line,
        probe.needle,
        probe.cursor_offset,
    )
}

fn resolve_position_in_source(
    source: &str,
    relative_path: &str,
    zero_based_line: usize,
    needle: &str,
    cursor_offset: usize,
) -> Result<ProbePosition> {
    let line_text = source
        .lines()
        .nth(zero_based_line)
        .with_context(|| format!("missing line {zero_based_line} in {relative_path}"))?;
    let needle_start = line_text.find(needle).with_context(|| {
        format!("missing needle `{needle}` on {relative_path}:{zero_based_line}")
    })?;
    let character =
        needle_start.checked_add(cursor_offset).context("probe cursor offset overflow")?;
    Ok(ProbePosition {
        line: u32::try_from(zero_based_line).context("probe line does not fit in u32")?,
        character: u32::try_from(character).context("probe character does not fit in u32")?,
    })
}

fn rename_request(
    harness: &UxHarness,
    relative_path: &str,
    position: &ProbePosition,
    new_name: &str,
) -> Result<Value> {
    let uri = harness.workspace.uri(relative_path);
    harness.client.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": position.line, "character": position.character },
            "newName": new_name
        }),
        Duration::from_secs(20),
    )
}

fn error_message(response: &Value) -> Option<String> {
    response
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn response_state(response: &Value, edit_count: usize) -> RenameResponseState {
    if response.get("error").is_some() {
        return RenameResponseState::Error;
    }

    match response.get("result") {
        Some(result) if result.is_null() => RenameResponseState::Null,
        Some(_) if edit_count > 0 => RenameResponseState::Edits,
        _ => RenameResponseState::EmptyEdit,
    }
}

fn relative_path_for_uri(files: &[FixtureFile], uri: &str) -> String {
    files
        .iter()
        .find(|file| uri.ends_with(&file.relative_path))
        .map(|file| file.relative_path.clone())
        .unwrap_or_else(|| uri.to_string())
}

fn source_for_edit<'a>(
    files: &'a [FixtureFile],
    relative_path: &str,
    override_file: Option<&str>,
    override_content: Option<&'a str>,
) -> Result<&'a str> {
    if override_file == Some(relative_path) {
        if let Some(content) = override_content {
            return Ok(content);
        }
    }
    fixture_content(files, relative_path)
}

fn edit_range_u64(edit: &Value, path: &[&str]) -> Result<u64> {
    let mut current = edit;
    for key in path {
        current =
            current.get(*key).with_context(|| format!("rename edit missing {}", path.join(".")))?;
    }
    current
        .as_u64()
        .with_context(|| format!("rename edit {} must be an unsigned integer", path.join(".")))
}

fn edit_new_text(edit: &Value) -> Result<String> {
    edit.get("newText")
        .and_then(Value::as_str)
        .map(str::to_string)
        .context("rename edit missing newText")
}

fn utf16_byte_index(line: &str, target_units: u64) -> Option<usize> {
    let mut units = 0_u64;
    if target_units == 0 {
        return Some(0);
    }

    for (idx, ch) in line.char_indices() {
        units = units.checked_add(ch.len_utf16() as u64)?;
        if units == target_units {
            return Some(idx + ch.len_utf8());
        }
    }

    if units == target_units { Some(line.len()) } else { None }
}

fn source_slice_by_lsp_range(
    source: &str,
    start_line: u64,
    start_character: u64,
    end_line: u64,
    end_character: u64,
) -> Result<String> {
    if start_line != end_line {
        return Ok("<multi-line-edit>".to_string());
    }
    let line_index = usize::try_from(start_line).context("rename edit line does not fit usize")?;
    let line = source
        .lines()
        .nth(line_index)
        .with_context(|| format!("rename edit line {start_line} missing from source"))?;
    let start = utf16_byte_index(line, start_character)
        .with_context(|| format!("rename edit start character {start_character} is invalid"))?;
    let end = utf16_byte_index(line, end_character)
        .with_context(|| format!("rename edit end character {end_character} is invalid"))?;
    if start > end || end > line.len() {
        return Err(anyhow!("rename edit range is outside source line bounds"));
    }
    Ok(line[start..end].to_string())
}

fn collect_one_edit(
    files: &[FixtureFile],
    uri: &str,
    edit: &Value,
    override_file: Option<&str>,
    override_content: Option<&str>,
) -> Result<RenameEdit> {
    let relative_path = relative_path_for_uri(files, uri);
    let start_line = edit_range_u64(edit, &["range", "start", "line"])?;
    let start_character = edit_range_u64(edit, &["range", "start", "character"])?;
    let end_line = edit_range_u64(edit, &["range", "end", "line"])?;
    let end_character = edit_range_u64(edit, &["range", "end", "character"])?;
    let source = source_for_edit(files, &relative_path, override_file, override_content)?;
    Ok(RenameEdit {
        uri: uri.to_string(),
        relative_path,
        start_line,
        start_character,
        end_line,
        end_character,
        source_text: source_slice_by_lsp_range(
            source,
            start_line,
            start_character,
            end_line,
            end_character,
        )?,
        new_text: edit_new_text(edit)?,
    })
}

fn collect_rename_edits(
    files: &[FixtureFile],
    response: &Value,
    override_file: Option<&str>,
    override_content: Option<&str>,
) -> Result<Vec<RenameEdit>> {
    let Some(result) = response.get("result") else {
        return Ok(Vec::new());
    };
    if result.is_null() {
        return Ok(Vec::new());
    }

    let mut edits = Vec::new();
    if let Some(changes) = result.get("changes").and_then(Value::as_object) {
        for (uri, uri_edits) in changes {
            if let Some(uri_edits) = uri_edits.as_array() {
                for edit in uri_edits {
                    edits.push(collect_one_edit(
                        files,
                        uri,
                        edit,
                        override_file,
                        override_content,
                    )?);
                }
            }
        }
    }

    if let Some(document_changes) = result.get("documentChanges").and_then(Value::as_array) {
        for change in document_changes {
            let uri = change
                .get("textDocument")
                .and_then(|text_document| text_document.get("uri"))
                .and_then(Value::as_str)
                .context("rename documentChanges entry missing textDocument.uri")?;
            let change_edits = change
                .get("edits")
                .and_then(Value::as_array)
                .context("rename documentChanges entry missing edits")?;
            for edit in change_edits {
                edits.push(collect_one_edit(files, uri, edit, override_file, override_content)?);
            }
        }
    }

    Ok(edits)
}

fn summarize_touched_values(
    edits: &[RenameEdit],
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let touched_uris = edits.iter().map(|edit| edit.uri.clone()).collect::<BTreeSet<_>>();
    let touched_relative_paths =
        edits.iter().map(|edit| edit.relative_path.clone()).collect::<BTreeSet<_>>();
    let source_texts = edits.iter().map(|edit| edit.source_text.clone()).collect::<BTreeSet<_>>();
    let new_texts = edits.iter().map(|edit| edit.new_text.clone()).collect::<BTreeSet<_>>();

    (
        touched_uris.into_iter().collect(),
        touched_relative_paths.into_iter().collect(),
        source_texts.into_iter().collect(),
        new_texts.into_iter().collect(),
    )
}

fn run_probe(
    harness: &UxHarness,
    files: &[FixtureFile],
    probe: &RenameProbe,
) -> Result<RenameProbeReport> {
    let position = resolve_probe_position(files, probe)?;
    let response = rename_request(harness, probe.file, &position, probe.new_name)?;
    let edits = collect_rename_edits(files, &response, None, None)?;
    let edit_start_lines = edits.iter().map(|edit| edit.start_line).collect::<BTreeSet<_>>();
    let (touched_uris, touched_relative_paths, touched_source_texts, new_texts) =
        summarize_touched_values(&edits);
    let state = response_state(&response, edits.len());

    Ok(RenameProbeReport {
        name: probe.name,
        category: probe.category,
        file: probe.file,
        zero_based_line: position.line,
        character: position.character,
        expectation: probe.expectation,
        response_state: state,
        edit_count: edits.len(),
        edit_start_lines: edit_start_lines.into_iter().collect(),
        touched_uris,
        touched_relative_paths,
        touched_source_texts,
        new_texts,
        error_message: error_message(&response),
        blocked_or_empty: matches!(
            state,
            RenameResponseState::EmptyEdit | RenameResponseState::Null | RenameResponseState::Error
        ),
    })
}

fn rename_probes() -> Vec<RenameProbe> {
    vec![
        RenameProbe {
            name: "exact_local_lexical_asset",
            category: "exact_local_lexical",
            file: STATIC_FILE,
            zero_based_line: 42,
            needle: "$asset",
            cursor_offset: 2,
            new_name: "renamed_asset",
            expectation: RenameExpectation::ExactLocalEdits,
        },
        RenameProbe {
            name: "generated_accessor_paths",
            category: "generated_accessor_boundary",
            file: STATIC_FILE,
            zero_based_line: 11,
            needle: "paths",
            cursor_offset: 1,
            new_name: "asset_paths",
            expectation: RenameExpectation::NoUnsafeEdits,
        },
        RenameProbe {
            name: "dynamic_typeglob_route_method",
            category: "dynamic_typeglob_boundary",
            file: ROUTES_FILE,
            zero_based_line: 24,
            needle: "Mojolicious::Routes::Route",
            cursor_offset: 24,
            new_name: "RenamedRoute",
            expectation: RenameExpectation::NoUnsafeEdits,
        },
    ]
}

fn freshness_report(harness: &UxHarness, files: &[FixtureFile]) -> Result<FreshnessReport> {
    let original = fixture_content(files, STATIC_FILE)?;
    let updated =
        original.replace("my ($self, $c, $asset) = @_;", "my ($self, $c, $fresh_asset) = @_;");
    if updated == original {
        return Err(anyhow!("freshness fixture must rename parameter asset to fresh_asset"));
    }

    harness.change_file_full(STATIC_FILE, &updated)?;
    std::thread::sleep(Duration::from_millis(300));

    let position = resolve_position_in_source(&updated, STATIC_FILE, 38, "$fresh_asset", 2)?;
    let response = rename_request(harness, STATIC_FILE, &position, "renamed_fresh_asset")?;
    let edits = collect_rename_edits(files, &response, Some(STATIC_FILE), Some(&updated))?;
    let state = response_state(&response, edits.len());
    let stale_source_edit_count = edits.iter().filter(|edit| edit.source_text == "$asset").count();
    let fresh_source_edit_count =
        edits.iter().filter(|edit| edit.source_text == "$fresh_asset").count();
    let (_, _, touched_source_texts, new_texts) = summarize_touched_values(&edits);

    Ok(FreshnessReport {
        file: STATIC_FILE,
        old_name: "$asset",
        fresh_name: "$fresh_asset",
        requested_name: "$renamed_fresh_asset",
        response_state: state,
        edit_count: edits.len(),
        stale_source_edit_count,
        fresh_source_edit_count,
        touched_source_texts,
        new_texts,
        error_message: error_message(&response),
    })
}

#[test]
fn scenario_35_mojolicious_rename_unsafe_edit_receipt() {
    run_ux_scenario(
        "mojolicious_rename_unsafe_edit",
        SCENARIO_FILE,
        "scenario_35_mojolicious_rename_unsafe_edit_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Rename),
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

            let probes = rename_probes();
            let mut reports = Vec::new();

            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = run_probe(&harness, &fixture_files, probe)?;
                recorder.mark_first_useful_result(probe.name);
                eprintln!(
                    "rename_probe={} category={} state={:?} edits={} paths={:?} source={:?} new={:?} error={:?}",
                    report.name,
                    report.category,
                    report.response_state,
                    report.edit_count,
                    report.touched_relative_paths,
                    report.touched_source_texts,
                    report.new_texts,
                    report.error_message
                );
                reports.push(report);
            }

            recorder.mark_request_start("freshness_after_edit");
            let freshness = freshness_report(&harness, &fixture_files)?;
            recorder.mark_first_useful_result("freshness_after_edit");
            eprintln!(
                "rename_freshness state={:?} edits={} stale_source_edits={} fresh_source_edits={} source={:?} new={:?} error={:?}",
                freshness.response_state,
                freshness.edit_count,
                freshness.stale_source_edit_count,
                freshness.fresh_source_edit_count,
                freshness.touched_source_texts,
                freshness.new_texts,
                freshness.error_message
            );

            let categories = reports.iter().map(|report| report.category).collect::<BTreeSet<_>>();
            let exact_local_report = reports
                .iter()
                .find(|report| report.name == "exact_local_lexical_asset")
                .context("missing exact local rename report")?;
            let generated_report = reports
                .iter()
                .find(|report| report.name == "generated_accessor_paths")
                .context("missing generated accessor report")?;
            let dynamic_report = reports
                .iter()
                .find(|report| report.name == "dynamic_typeglob_route_method")
                .context("missing dynamic typeglob report")?;
            let exact_local_lines =
                exact_local_report.edit_start_lines.iter().copied().collect::<BTreeSet<_>>();

            let receipt = json!({
                "schema_version": 1,
                "project": "mojolicious",
                "surface": "rename",
                "claim_boundary": "real-workspace rename unsafe-edit receipt only; no live rename behavior promoted",
                "fixture_file_count": fixture_files.len(),
                "probe_count": reports.len(),
                "reports": &reports,
                "freshness": &freshness,
                "exact_local_lines": &exact_local_lines,
            });
            eprintln!(
                "mojolicious_rename_unsafe_edit_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check("all rename probes produced reports", reports.len() == probes.len())?;
            recorder.check(
                "rename probes covered intended receipt categories",
                categories
                    == BTreeSet::from([
                        "dynamic_typeglob_boundary",
                        "exact_local_lexical",
                        "generated_accessor_boundary",
                    ]),
            )?;
            recorder.check(
                "exact local lexical rename returned same-file scoped edits",
                exact_local_report.response_state == RenameResponseState::Edits
                    && exact_local_report.edit_count >= 3
                    && exact_local_report.touched_relative_paths == vec![STATIC_FILE.to_string()]
                    && exact_local_report.touched_source_texts == vec!["$asset".to_string()]
                    && exact_local_report.new_texts == vec!["$renamed_asset".to_string()]
                    && exact_local_lines == BTreeSet::from([38, 40, 42]),
            )?;
            recorder.check(
                "generated accessor rename did not produce unsafe edits",
                generated_report.edit_count == 0 && generated_report.blocked_or_empty,
            )?;
            recorder.check(
                "dynamic typeglob rename did not produce unsafe edits",
                dynamic_report.edit_count == 0 && dynamic_report.blocked_or_empty,
            )?;
            recorder.check(
                "rename after open-document edit did not act on stale source text",
                freshness.stale_source_edit_count == 0
                    && (freshness.edit_count == 0 || freshness.fresh_source_edit_count > 0),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
