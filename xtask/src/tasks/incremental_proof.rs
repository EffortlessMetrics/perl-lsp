//! Deterministic fresh-vs-token-replay measurements for the Rust tree-sitter facade.
//!
//! This command deliberately measures the shipped token-replay contract only:
//! replay reuses cached parser tokens and reconstructs the AST. It does not
//! measure or claim AST subtree reuse. The lower-tier parser-core test suite
//! separately verifies the complete replayed token stream against fresh lexing.

use crate::tasks::git_context::git_stdout_with_worktree_fallback;
use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail, eyre};
use perl_position_tracking::Position;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tree_sitter_perl_rs::{InputEdit, Node, Parser, ReparseMode, Tree};

/// Receipt profile controlling fixture breadth and measurement iterations.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Profile {
    /// Small deterministic slice suitable for pull requests.
    Pr,
    /// Broader edit and size matrix for scheduled validation.
    Nightly,
    /// Release-sized deterministic matrix.
    Release,
}

impl Profile {
    fn slug(self) -> &'static str {
        match self {
            Self::Pr => "pr",
            Self::Nightly => "nightly",
            Self::Release => "release",
        }
    }

    fn multipliers(self) -> &'static [usize] {
        match self {
            Self::Pr => &[1, 8, 32],
            Self::Nightly => &[1, 8, 32, 128],
            Self::Release => &[1, 8, 32, 128, 512],
        }
    }

    fn iterations(self) -> usize {
        match self {
            Self::Pr => 5,
            Self::Nightly => 15,
            Self::Release => 25,
        }
    }
}

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_OUTPUT_PREFIX: &str = "target/receipts/tree-sitter-incremental-proof-";

#[derive(Debug, Clone)]
struct EditCase {
    name: &'static str,
    class: &'static str,
    old_text: &'static str,
    new_text: &'static str,
    fixed_start: Option<usize>,
}

#[derive(Debug, Clone)]
struct Fixture {
    name: &'static str,
    base_source: String,
    cases: Vec<EditCase>,
    scale_strategy: ScaleStrategy,
}

#[derive(Debug, Clone, Copy)]
enum ScaleStrategy {
    RepeatDocument,
    RepeatIncompleteBody,
}

impl ScaleStrategy {
    fn slug(self) -> &'static str {
        match self {
            Self::RepeatDocument => "repeat_document",
            Self::RepeatIncompleteBody => "repeat_incomplete_body",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct Receipt {
    schema_version: u32,
    kind: &'static str,
    captured_at: String,
    profile: &'static str,
    git_sha: String,
    toolchain: String,
    feature_set: &'static str,
    input_hash: String,
    token_equivalence_proof: &'static str,
    facade_equivalence: FacadeEquivalence,
    rows: Vec<MeasurementRow>,
    summary: Summary,
}

#[derive(Debug, Clone, Serialize)]
struct FacadeEquivalence {
    compared: &'static str,
    failures: usize,
}

#[derive(Debug, Clone, Serialize)]
struct MeasurementRow {
    fixture: &'static str,
    edit: &'static str,
    edit_class: &'static str,
    document_size_bytes: usize,
    edit_position_byte: usize,
    iterations: usize,
    fresh_p50_ns: u128,
    fresh_p95_ns: u128,
    replay_p50_ns: u128,
    replay_p95_ns: u128,
    average_reprocessed_bytes: usize,
    average_tokens_reused: usize,
    average_tokens_relexed: usize,
    fallback_count: usize,
    fallback_rate: f64,
    fallback_reasons: BTreeMap<String, usize>,
    operation_modes: BTreeMap<String, usize>,
    reprocessed_ranges: Vec<ByteRange>,
    equivalence_failures: usize,
    first_equivalence_failure: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ByteRange {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Serialize)]
struct Summary {
    rows: usize,
    iterations: usize,
    fallback_count: usize,
    fallback_rate: f64,
    equivalence_failures: usize,
    replay_p95_faster_rows: usize,
}

/// Run the deterministic incremental proof and write its JSON receipt.
pub fn run(profile: Profile, output: Option<PathBuf>) -> Result<()> {
    let root = project_root()?;
    let fixtures = fixtures();
    let input_hash = hash_inputs(profile, &fixtures);
    let git_sha = git_stdout_with_worktree_fallback(&root, &["rev-parse", "HEAD"])
        .context("resolving the exact proof head SHA")?;
    let toolchain = rustc_version()?;
    let iterations = profile.iterations();

    let mut rows = Vec::new();
    for multiplier in profile.multipliers() {
        for fixture in &fixtures {
            let source = scaled_source(fixture, *multiplier)?;
            for case in &fixture.cases {
                rows.push(measure_case(fixture, &source, case, iterations)?);
            }
        }
    }

    let summary = summarize(&rows, iterations);
    let receipt = Receipt {
        schema_version: SCHEMA_VERSION,
        kind: "tree_sitter_incremental_proof",
        captured_at: chrono::Utc::now().to_rfc3339(),
        profile: profile.slug(),
        git_sha,
        toolchain,
        feature_set: "tree-sitter-perl-rs:default;xtask:default",
        input_hash,
        token_equivalence_proof: "perl-parser-core incremental library suite",
        facade_equivalence: FacadeEquivalence {
            compared: "S-expression, node kind/field/span/point/text tree, diagnostics, and error status",
            failures: summary.equivalence_failures,
        },
        rows,
        summary,
    };

    let output = output_path(&root, profile, output);
    write_receipt(&output, &receipt)?;
    println!(
        "tree-sitter incremental proof: {} rows, {} equivalence failures, {:.1}% fallback; receipt {}",
        receipt.summary.rows,
        receipt.summary.equivalence_failures,
        receipt.summary.fallback_rate * 100.0,
        output.display()
    );

    if receipt.summary.equivalence_failures != 0 {
        bail!(
            "incremental facade equivalence failed {} time(s); see {}",
            receipt.summary.equivalence_failures,
            output.display()
        );
    }
    Ok(())
}

fn scaled_source(fixture: &Fixture, multiplier: usize) -> Result<String> {
    match fixture.scale_strategy {
        ScaleStrategy::RepeatDocument => Ok(fixture.base_source.repeat(multiplier)),
        ScaleStrategy::RepeatIncompleteBody => {
            let prefix = "sub foo {";
            let body = fixture.base_source.strip_prefix(prefix).ok_or_else(|| {
                eyre!("fixture {} has an invalid incomplete prefix", fixture.name)
            })?;
            Ok(format!("{prefix}{}", body.repeat(multiplier)))
        }
    }
}

fn measure_case(
    fixture: &Fixture,
    base_source: &str,
    case: &EditCase,
    iterations: usize,
) -> Result<MeasurementRow> {
    let mut parser = Parser::new();
    let mut current_source = base_source.to_owned();
    let mut current_tree = parser
        .parse(&current_source)
        .ok_or_else(|| eyre!("fixture {} did not produce an initial tree", fixture.name))?;

    // Warm the lazy token cache and validate the first direction before timing.
    let (target_source, edit) = make_edit(&current_source, case, true)?;
    let mut warmed = current_tree.clone();
    warmed.edit(&edit);
    let warm_replayed = parser
        .parse_with_old_tree(&target_source, &warmed)
        .ok_or_else(|| eyre!("warm replay failed for {} / {}", fixture.name, case.name))?;
    let warm_fresh = parser
        .parse(&target_source)
        .ok_or_else(|| eyre!("warm fresh parse failed for {} / {}", fixture.name, case.name))?;
    if let Some(reason) = compare_trees(&warm_replayed, &warm_fresh)? {
        bail!("warm replay equivalence failed for {} / {}: {reason}", fixture.name, case.name);
    }
    current_source = target_source;
    current_tree = warm_replayed;

    let mut fresh_samples = Vec::with_capacity(iterations);
    let mut replay_samples = Vec::with_capacity(iterations);
    let mut reprocessed_bytes = Vec::with_capacity(iterations);
    let mut tokens_reused = Vec::with_capacity(iterations);
    let mut tokens_relexed = Vec::with_capacity(iterations);
    let mut fallback_reasons = BTreeMap::new();
    let mut operation_modes = BTreeMap::new();
    let mut ranges = BTreeSet::new();
    let mut fallback_count = 0;
    let mut equivalence_failures = 0;
    let mut first_equivalence_failure = None;
    let mut edit_position = 0;

    for _ in 0..iterations {
        let forward = current_source == base_source;
        let (new_source, edit) = make_edit(&current_source, case, forward)?;
        edit_position = edit.start_byte;
        let mut edited_tree = current_tree.clone();
        edited_tree.edit(&edit);

        let replay_started = Instant::now();
        let replayed = parser
            .parse_with_old_tree(&new_source, &edited_tree)
            .ok_or_else(|| eyre!("replay failed for {} / {}", fixture.name, case.name))?;
        replay_samples.push(replay_started.elapsed().as_nanos());

        let fresh_started = Instant::now();
        let fresh = parser
            .parse(&new_source)
            .ok_or_else(|| eyre!("fresh parse failed for {} / {}", fixture.name, case.name))?;
        fresh_samples.push(fresh_started.elapsed().as_nanos());

        if let Some(metrics) = replayed.incremental_metrics() {
            reprocessed_bytes.push(metrics.reparsed_bytes);
            tokens_reused.push(metrics.tokens_reused);
            tokens_relexed.push(metrics.tokens_relexed);
        }
        let mode = mode_name(replayed.reparse_mode());
        *operation_modes.entry(mode.to_owned()).or_insert(0) += 1;
        match replayed.reparse_mode() {
            Some(ReparseMode::FullParseFallback(reason)) => {
                fallback_count += 1;
                *fallback_reasons.entry(format!("{reason:?}")).or_insert(0) += 1;
            }
            Some(ReparseMode::Unchanged | ReparseMode::TokenReplay) | None => {}
            Some(_) => {
                fallback_count += 1;
                *fallback_reasons.entry("unclassified".to_owned()).or_insert(0) += 1;
            }
        }
        for range in replayed.reprocessed_ranges() {
            ranges.insert((range.start, range.end));
        }

        if let Some(reason) = compare_trees(&replayed, &fresh)? {
            equivalence_failures += 1;
            if first_equivalence_failure.is_none() {
                first_equivalence_failure = Some(reason);
            }
        }

        current_source = new_source;
        current_tree = replayed;
    }

    let range_list = ranges.into_iter().map(|(start, end)| ByteRange { start, end }).collect();
    Ok(MeasurementRow {
        fixture: fixture.name,
        edit: case.name,
        edit_class: case.class,
        document_size_bytes: base_source.len(),
        edit_position_byte: edit_position,
        iterations,
        fresh_p50_ns: nearest_rank_percentile(&fresh_samples, 50),
        fresh_p95_ns: nearest_rank_percentile(&fresh_samples, 95),
        replay_p50_ns: nearest_rank_percentile(&replay_samples, 50),
        replay_p95_ns: nearest_rank_percentile(&replay_samples, 95),
        average_reprocessed_bytes: average(&reprocessed_bytes),
        average_tokens_reused: average(&tokens_reused),
        average_tokens_relexed: average(&tokens_relexed),
        fallback_count,
        fallback_rate: fallback_count as f64 / iterations as f64,
        fallback_reasons,
        operation_modes,
        reprocessed_ranges: range_list,
        equivalence_failures,
        first_equivalence_failure,
    })
}

fn make_edit(source: &str, case: &EditCase, forward: bool) -> Result<(String, InputEdit)> {
    let (old_text, new_text) =
        if forward { (case.old_text, case.new_text) } else { (case.new_text, case.old_text) };
    let start = match case.fixed_start {
        Some(start) => start,
        None => source
            .find(old_text)
            .ok_or_else(|| eyre!("edit text {:?} is absent from case source", old_text))?,
    };
    let old_end = start
        .checked_add(old_text.len())
        .ok_or_else(|| eyre!("edit range overflow for {:?}", case.name))?;
    if source.get(start..old_end) != Some(old_text) {
        bail!("edit anchor mismatch for {}: expected {:?} at {}", case.name, old_text, start);
    }
    let new_end = start
        .checked_add(new_text.len())
        .ok_or_else(|| eyre!("new edit range overflow for {:?}", case.name))?;
    let mut new_source = String::with_capacity(source.len() - old_text.len() + new_text.len());
    new_source.push_str(&source[..start]);
    new_source.push_str(new_text);
    new_source.push_str(&source[old_end..]);

    let edit = InputEdit::new(
        start,
        old_end,
        new_end,
        position_at(source, start)?,
        position_at(source, old_end)?,
        position_at(&new_source, new_end)?,
    );
    Ok((new_source, edit))
}

fn position_at(source: &str, byte: usize) -> Result<Position> {
    if !source.is_char_boundary(byte) {
        bail!("byte offset {byte} is not a UTF-8 boundary");
    }
    let prefix = source
        .get(..byte)
        .ok_or_else(|| eyre!("byte offset {byte} is outside source length {}", source.len()))?;
    let line = prefix
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        .checked_add(1)
        .ok_or_else(|| eyre!("line number exceeds usize"))?;
    let line = u32::try_from(line).map_err(|_| eyre!("line number exceeds u32"))?;
    let column = prefix
        .rsplit('\n')
        .next()
        .map_or(prefix.len(), str::len)
        .checked_add(1)
        .ok_or_else(|| eyre!("column exceeds usize"))?;
    let column = u32::try_from(column).map_err(|_| eyre!("column exceeds u32"))?;
    Ok(Position::new(byte, line, column))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeSnapshot {
    kind: String,
    field_name: Option<&'static str>,
    start_byte: usize,
    end_byte: usize,
    start_point: (usize, usize),
    end_point: (usize, usize),
    text: String,
    children: Vec<NodeSnapshot>,
}

fn snapshot(node: Node<'_>) -> Result<NodeSnapshot> {
    snapshot_node(node, None)
}

fn snapshot_node(node: Node<'_>, field_name: Option<&'static str>) -> Result<NodeSnapshot> {
    let text = node
        .utf8_text(node.tree_source().as_bytes())
        .map_err(|error| eyre!("node text was not valid UTF-8: {error}"))?
        .to_owned();
    let start = node.start_position();
    let end = node.end_position();
    let children = snapshot_children(node)?;
    Ok(NodeSnapshot {
        kind: node.kind(),
        field_name,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_point: (start.row, start.column),
        end_point: (end.row, end.column),
        text,
        children,
    })
}

fn snapshot_children(node: Node<'_>) -> Result<Vec<NodeSnapshot>> {
    let mut children = Vec::with_capacity(node.child_count());
    for index in 0..node.child_count() {
        let child =
            node.child(index).ok_or_else(|| eyre!("child {index} disappeared during snapshot"))?;
        children.push(snapshot_node(child, node.field_name_for_child(index))?);
    }
    Ok(children)
}

fn compare_trees(incremental: &Tree, fresh: &Tree) -> Result<Option<String>> {
    if incremental.root_node().to_sexp() != fresh.root_node().to_sexp() {
        return Ok(Some("S-expression mismatch".to_owned()));
    }
    if incremental.diagnostics() != fresh.diagnostics() {
        return Ok(Some("diagnostic mismatch".to_owned()));
    }
    if incremental.has_error() != fresh.has_error() {
        return Ok(Some("error-status mismatch".to_owned()));
    }
    if snapshot(incremental.root_node())? != snapshot(fresh.root_node())? {
        return Ok(Some("node kind/field/span/point/text mismatch".to_owned()));
    }
    Ok(None)
}

fn mode_name(mode: Option<ReparseMode>) -> &'static str {
    match mode {
        Some(ReparseMode::Unchanged) => "unchanged",
        Some(ReparseMode::TokenReplay) => "token_replay",
        Some(ReparseMode::FullParseFallback(_)) => "full_parse_fallback",
        None => "initial_or_unclassified",
        Some(_) => "unclassified",
    }
}

fn summarize(rows: &[MeasurementRow], iterations: usize) -> Summary {
    let fallback_count = rows.iter().map(|row| row.fallback_count).sum::<usize>();
    let total_iterations = rows.len() * iterations;
    Summary {
        rows: rows.len(),
        iterations: total_iterations,
        fallback_count,
        fallback_rate: if total_iterations == 0 {
            0.0
        } else {
            fallback_count as f64 / total_iterations as f64
        },
        equivalence_failures: rows.iter().map(|row| row.equivalence_failures).sum(),
        replay_p95_faster_rows: rows
            .iter()
            .filter(|row| row.replay_p95_ns < row.fresh_p95_ns)
            .count(),
    }
}

/// Compute a nearest-rank percentile while retaining nanosecond precision.
///
/// The shared parser helper uses `u64` samples; this receipt keeps `u128`
/// timings so the measurement cannot truncate a future long-running sample.
fn nearest_rank_percentile(values: &[u128], pct: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = sorted.len().saturating_mul(pct.min(100)).saturating_add(99) / 100;
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted[index]
}

fn average(values: &[usize]) -> usize {
    if values.is_empty() { 0 } else { values.iter().sum::<usize>() / values.len() }
}

fn hash_inputs(profile: Profile, fixtures: &[Fixture]) -> String {
    let mut hasher = Sha256::new();
    hash_u64(&mut hasher, u64::from(SCHEMA_VERSION));
    hash_bytes(&mut hasher, b"tree_sitter_incremental_proof");
    hash_bytes(&mut hasher, profile.slug().as_bytes());
    hash_u64(&mut hasher, profile.iterations() as u64);
    hash_u64(&mut hasher, profile.multipliers().len() as u64);
    for multiplier in profile.multipliers() {
        hash_u64(&mut hasher, *multiplier as u64);
    }
    hash_u64(&mut hasher, fixtures.len() as u64);
    for fixture in fixtures {
        hash_bytes(&mut hasher, fixture.name.as_bytes());
        hash_bytes(&mut hasher, fixture.scale_strategy.slug().as_bytes());
        hash_bytes(&mut hasher, fixture.base_source.as_bytes());
        hash_u64(&mut hasher, fixture.cases.len() as u64);
        for case in &fixture.cases {
            hash_bytes(&mut hasher, case.name.as_bytes());
            hash_bytes(&mut hasher, case.class.as_bytes());
            hash_bytes(&mut hasher, case.old_text.as_bytes());
            hash_bytes(&mut hasher, case.new_text.as_bytes());
            match case.fixed_start {
                Some(start) => {
                    hasher.update([1]);
                    hash_u64(&mut hasher, start as u64);
                }
                None => hasher.update([0]),
            }
        }
    }
    hex_lower(&hasher.finalize())
}

fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hash_u64(hasher, bytes.len() as u64);
    hasher.update(bytes);
}

fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        output.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    output
}

fn rustc_version() -> Result<String> {
    let output =
        Command::new("rustc").arg("--version").output().context("running rustc --version")?;
    if !output.status.success() {
        bail!("rustc --version failed with status {}", output.status);
    }
    String::from_utf8(output.stdout)
        .map(|version| version.trim().to_owned())
        .context("rustc --version returned non-UTF-8 output")
}

fn output_path(root: &Path, profile: Profile, output: Option<PathBuf>) -> PathBuf {
    let output = output.unwrap_or_else(|| {
        PathBuf::from(format!("{DEFAULT_OUTPUT_PREFIX}{}.json", profile.slug()))
    });
    if output.is_absolute() { output } else { root.join(output) }
}

fn write_receipt(path: &Path, receipt: &Receipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(receipt).context("serializing incremental proof")?;
    fs::write(path, format!("{json}\n")).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn fixtures() -> Vec<Fixture> {
    let long_identifier = "x".repeat(254);
    let lexical_prefix = "my $identifier = ";
    let lexical_source = format!(
        "{lexical_prefix}{long_identifier};\nmy $regex = $left / $right;\nmy $quoted = q{{value}};\n"
    );
    let checkpoint = lexical_prefix.len() + long_identifier.len();
    vec![
        Fixture {
            name: "syntax",
            base_source: "my $value = 1;\nmy $sum = $value + 2;\n".to_owned(),
            scale_strategy: ScaleStrategy::RepeatDocument,
            cases: vec![
                EditCase {
                    name: "identifier",
                    class: "identifier",
                    old_text: "value",
                    new_text: "value_long",
                    fixed_start: None,
                },
                EditCase {
                    name: "literal",
                    class: "literal",
                    old_text: "1",
                    new_text: "42",
                    fixed_start: None,
                },
                EditCase {
                    name: "whitespace",
                    class: "whitespace",
                    old_text: " = ",
                    new_text: "  =  ",
                    fixed_start: None,
                },
                EditCase {
                    name: "operator",
                    class: "operator",
                    old_text: " + ",
                    new_text: " - ",
                    fixed_start: None,
                },
                EditCase {
                    name: "newline",
                    class: "newline",
                    old_text: ";\n",
                    new_text: "; ",
                    fixed_start: None,
                },
                EditCase {
                    name: "comment",
                    class: "comment",
                    old_text: ";\n",
                    new_text: "; # note\n",
                    fixed_start: None,
                },
                EditCase {
                    name: "unicode",
                    class: "unicode",
                    old_text: "value",
                    new_text: "café",
                    fixed_start: None,
                },
            ],
        },
        Fixture {
            name: "lexical-boundaries",
            base_source: lexical_source,
            scale_strategy: ScaleStrategy::RepeatDocument,
            cases: vec![
                EditCase {
                    name: "quote-delimiter",
                    class: "quote-delimiter",
                    old_text: "q{",
                    new_text: "q[",
                    fixed_start: None,
                },
                EditCase {
                    name: "regex-operator",
                    class: "operator",
                    old_text: " / ",
                    new_text: " // ",
                    fixed_start: None,
                },
                EditCase {
                    name: "special-variable",
                    class: "special-variable",
                    old_text: "$left",
                    new_text: "$^A",
                    fixed_start: None,
                },
                EditCase {
                    name: "checkpoint-insertion",
                    class: "checkpoint-boundary",
                    old_text: "",
                    new_text: "z",
                    fixed_start: Some(checkpoint),
                },
            ],
        },
        Fixture {
            name: "heredoc",
            base_source: "my $text = <<'EOF';\nbody line\nEOF\n".to_owned(),
            scale_strategy: ScaleStrategy::RepeatDocument,
            cases: vec![
                EditCase {
                    name: "heredoc-delimiter",
                    class: "heredoc-delimiter",
                    old_text: "EOF",
                    new_text: "DATA",
                    fixed_start: None,
                },
                EditCase {
                    name: "heredoc-body",
                    class: "heredoc-body",
                    old_text: "body line",
                    new_text: "changed body",
                    fixed_start: None,
                },
            ],
        },
        Fixture {
            name: "recovered",
            base_source: "my $value = ;\n".to_owned(),
            scale_strategy: ScaleStrategy::RepeatDocument,
            cases: vec![
                EditCase {
                    name: "recovered-literal",
                    class: "recovery",
                    old_text: "= ;",
                    new_text: "= 1;",
                    fixed_start: None,
                },
                EditCase {
                    name: "recovered-insertion",
                    class: "recovery",
                    old_text: "",
                    new_text: "# note\n",
                    fixed_start: Some(0),
                },
            ],
        },
        Fixture {
            name: "incomplete",
            base_source: "sub foo { my $value = 1;\n".to_owned(),
            scale_strategy: ScaleStrategy::RepeatIncompleteBody,
            cases: vec![
                EditCase {
                    name: "incomplete-body",
                    class: "incomplete",
                    old_text: "1",
                    new_text: "42",
                    fixed_start: None,
                },
                EditCase {
                    name: "incomplete-close",
                    class: "incomplete",
                    old_text: "\n",
                    new_text: "}\n",
                    fixed_start: None,
                },
            ],
        },
        Fixture {
            name: "format",
            base_source: "format REPORT =\nName: @<<<\n$value\n.\n".to_owned(),
            scale_strategy: ScaleStrategy::RepeatDocument,
            cases: vec![EditCase {
                name: "format-value",
                class: "format",
                old_text: "$value",
                new_text: "$other",
                fixed_start: None,
            }],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_is_deterministic_and_bounded() -> Result<()> {
        let median = nearest_rank_percentile(&[9, 1, 5, 3, 7], 50);
        if median != 5 {
            return Err(eyre!("expected nearest-rank p50 of 5, got {median}"));
        }
        let p95 = nearest_rank_percentile(&[9, 1, 5, 3, 7], 95);
        if p95 != 9 {
            return Err(eyre!("expected nearest-rank p95 of 9, got {p95}"));
        }
        if nearest_rank_percentile(&[], 95) != 0 {
            return Err(eyre!("empty percentile sample must return zero"));
        }
        Ok(())
    }

    #[test]
    fn scaled_profiles_have_initial_trees() -> Result<()> {
        let fixtures = fixtures();
        for profile in [Profile::Pr, Profile::Nightly, Profile::Release] {
            for multiplier in profile.multipliers() {
                for fixture in &fixtures {
                    let source = scaled_source(fixture, *multiplier)?;
                    let mut parser = Parser::new();
                    if parser.parse(&source).is_none() {
                        return Err(eyre!(
                            "fixture {} did not produce an initial tree at multiplier {} for {}",
                            fixture.name,
                            multiplier,
                            profile.slug()
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn fixture_hash_changes_when_a_case_changes() -> Result<()> {
        let fixtures = fixtures();
        let first = hash_inputs(Profile::Pr, &fixtures);
        let mut changed = fixtures;
        changed[0].cases[0].new_text = "other";
        let second = hash_inputs(Profile::Pr, &changed);
        if first == second {
            return Err(eyre!("changing a fixture case must change the input hash"));
        }
        Ok(())
    }

    #[test]
    fn edit_builder_preserves_exact_source_splice() -> Result<()> {
        let case = EditCase {
            name: "literal",
            class: "literal",
            old_text: "1",
            new_text: "42",
            fixed_start: None,
        };
        let (new_source, edit) = make_edit("my $x = 1;", &case, true)?;
        if new_source != "my $x = 42;" {
            return Err(eyre!("unexpected edited source: {new_source:?}"));
        }
        if edit.start_byte != 8 || edit.old_end_byte != 9 || edit.new_end_byte != 10 {
            return Err(eyre!(
                "unexpected edit range: {}..{} -> {}",
                edit.start_byte,
                edit.old_end_byte,
                edit.new_end_byte
            ));
        }
        Ok(())
    }

    #[test]
    fn position_at_uses_one_based_lines_and_columns() -> Result<()> {
        let position = position_at("a\nβ", "a\nβ".len())?;
        if position != Position::new(4, 2, 3) {
            return Err(eyre!("unexpected position: {position:?}"));
        }
        Ok(())
    }

    #[test]
    fn position_at_rejects_non_utf8_boundaries() -> Result<()> {
        if position_at("β", 1).is_ok() {
            return Err(eyre!("position_at must reject a non-UTF-8 boundary"));
        }
        Ok(())
    }
}
