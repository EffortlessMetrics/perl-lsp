//! Parser corpus sweep task
//!
//! Scans system-installed Perl `.pm` files, parses each with the v3 recursive
//! descent parser, and reports clean-parse rates plus error buckets.
//!
//! Produces a JSON report suitable for baseline comparison and regression gating.

use color_eyre::eyre::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use perl_parser::{Node, NodeKind, Parser};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Instant;
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// Lazy-compiled regexes for error normalization
// ---------------------------------------------------------------------------

static RE_SYNTAX_POS: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^Invalid syntax at position \d+: (.+)$").ok());
static RE_TRAILING_AT: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r" at \d+$").ok());

// ---------------------------------------------------------------------------
// Semantic bucket mapping table (first match wins)
// ---------------------------------------------------------------------------

const SEMANTIC_BUCKETS: &[(&str, &str)] = &[
    ("catastrophic backtracking", "catastrophic_backtracking"),
    ("Expected variable, found", "expected_variable"),
    ("Expected string or identifier in import", "expected_import_item"),
    ("Expected comma or closing parenthesis in signature", "signature_param"),
    ("Expected comma or closing parenthesis", "expected_comma_or_close_paren"),
    ("Expected module name or version", "expected_module_name"),
    ("Expected '>' to close angle", "unclosed_angle"),
    ("Substitution operator should be", "substitution_misparse"),
    ("expected expression, found FatArrow", "unexpected_fat_arrow_expr"),
    ("expected expression, found Arrow", "unexpected_arrow_expr"),
    ("expected expression, found Slash", "unexpected_slash_expr"),
    ("expected expression, found Question", "unexpected_question_expr"),
    ("expected expression, found Return", "unexpected_return_expr"),
    ("expected expression, found", "unexpected_token_in_expr"),
    ("expected RightBrace, found Semicolon", "unclosed_brace_semicolon"),
    ("expected RightBrace, found Eof", "unclosed_brace_eof"),
    ("expected RightBrace, found", "unclosed_brace"),
    ("expected RightParen, found Identifier", "unclosed_paren_identifier"),
    ("expected RightParen, found", "unclosed_paren"),
    ("expected RightBracket, found", "unclosed_bracket"),
    ("expected LeftParen, found", "expected_left_paren"),
    ("expected LeftBrace, found", "expected_left_brace"),
    ("expected Semicolon, found", "expected_semicolon"),
    ("expected Colon, found", "expected_colon"),
    ("expected Identifier, found", "expected_identifier"),
    ("expected Comma, found", "expected_comma"),
];

/// Configuration for the corpus sweep
#[derive(Debug, Clone)]
pub struct SweepConfig {
    /// High-level roots for report metadata (e.g., `/usr/share/perl`)
    pub base_roots: Vec<PathBuf>,
    /// Expanded directories to actually scan (includes versioned subdirs)
    pub corpus_roots: Vec<PathBuf>,
    /// Optional manifest file listing module names to resolve via `perl`
    pub manifest_path: Option<PathBuf>,
    /// Optional JSON output file
    pub output_path: Option<PathBuf>,
    /// Compare against baseline JSON file
    pub baseline_path: Option<PathBuf>,
    /// Return nonzero if regression detected
    pub enforce: bool,
    /// Include per-file details in output
    pub verbose: bool,
    /// Write receipt JSON to target/receipts/corpus-sweep.json
    pub receipt: bool,
}

/// Overall sweep report (serialized to JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepReport {
    pub schema_version: String,
    pub commit: String,
    pub timestamp: String,
    /// Corpus profile identifier (always "system" for now)
    #[serde(default = "default_corpus_profile")]
    pub corpus_profile: String,
    /// High-level base roots (e.g., 3 base directories)
    pub corpus_roots: Vec<String>,
    /// Number of actual directories scanned after expansion
    #[serde(default)]
    pub resolved_roots_count: usize,
    /// Perl version from `perl -e 'print $]'`
    #[serde(default = "default_perl_version")]
    pub perl_version: String,
    pub total_files: usize,
    pub files_unreadable: usize,
    pub clean_files: usize,
    pub files_with_errors: usize,
    pub total_error_nodes: usize,
    pub first_error_buckets: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub file_results: Vec<FileResult>,
    pub elapsed_secs: f64,
}

fn default_corpus_profile() -> String {
    "system".to_string()
}

fn default_perl_version() -> String {
    "unknown".to_string()
}

/// Per-file result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResult {
    pub path: String,
    pub status: String,
    pub error_node_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_error: Option<String>,
}

/// A single ratchet violation
#[derive(Debug, Clone)]
pub struct RatchetViolation {
    pub metric: String,
    pub baseline_value: String,
    pub current_value: String,
}

/// Default high-level corpus root directories for system Perl
pub fn default_base_roots() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/share/perl"),
        PathBuf::from("/usr/lib/x86_64-linux-gnu/perl"),
        PathBuf::from("/usr/share/perl5"),
    ]
}

/// Expand base roots into all scannable directories (including versioned subdirs)
pub fn resolve_corpus_roots(base_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for base in base_roots {
        if base.exists() {
            roots.push(base.clone());
        }
        // Expand versioned subdirectories like /usr/share/perl/5.38
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    roots.push(p);
                }
            }
        }
    }
    if roots.is_empty() { base_roots.to_vec() } else { roots }
}

/// Get the Perl version from the system `perl` binary
fn get_perl_version() -> String {
    std::process::Command::new("perl")
        .args(["-e", "print $]"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Summary of Error nodes found in an AST.
struct ErrorSummary {
    /// Total count of NodeKind::Error nodes in the tree
    count: usize,
    /// The message from the Error node with the smallest location.start
    first_message: Option<String>,
}

/// Walk the AST and summarize error nodes.
///
/// Counts all `NodeKind::Error` nodes and captures the raw message from
/// the earliest error by byte offset. Uses `for_each_child` to traverse
/// the full tree including `partial` subtrees of Error nodes.
fn collect_error_summary(root: &Node) -> ErrorSummary {
    let mut count = 0usize;
    let mut first_start = usize::MAX;
    let mut first_message: Option<String> = None;
    walk_errors(root, &mut count, &mut first_start, &mut first_message);
    ErrorSummary { count, first_message }
}

fn walk_errors(
    node: &Node,
    count: &mut usize,
    first_start: &mut usize,
    first_message: &mut Option<String>,
) {
    if let NodeKind::Error { message, .. } = &node.kind {
        *count += 1;
        if node.location.start < *first_start {
            *first_start = node.location.start;
            *first_message = Some(message.clone());
        }
    }
    node.for_each_child(|child| {
        walk_errors(child, count, first_start, first_message);
    });
}

/// Parse a manifest file into module names (skipping comments and empty lines).
pub fn parse_manifest(manifest_path: &Path) -> Result<Vec<String>> {
    let file = fs::File::open(manifest_path)
        .with_context(|| format!("Failed to open manifest: {}", manifest_path.display()))?;
    let reader = std::io::BufReader::new(file);
    let mut modules = Vec::new();
    for line in reader.lines() {
        let line = line.context("Failed to read manifest line")?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        modules.push(trimmed.to_string());
    }
    Ok(modules)
}

/// Resolve module names to file paths via a single `perl` invocation.
///
/// Returns an error if fewer than `min_resolved` modules resolve successfully.
pub fn resolve_manifest_modules(manifest_path: &Path, min_resolved: usize) -> Result<Vec<PathBuf>> {
    let modules = parse_manifest(manifest_path)?;
    if modules.is_empty() {
        return Err(color_eyre::eyre::eyre!("Manifest is empty: {}", manifest_path.display()));
    }

    // Build a single perl command to resolve all modules at once
    let module_list = modules.iter().map(|m| m.as_str()).collect::<Vec<_>>().join(" ");
    let perl_script = format!(
        r#"for (qw({})) {{ eval "require $_"; (my $f = "$_.pm") =~ s|::|/|g; print "$f=$INC{{$f}}\n" if $INC{{$f}} }}"#,
        module_list
    );

    let output = std::process::Command::new("perl")
        .args(["-e", &perl_script])
        .output()
        .context("Failed to run perl for module resolution")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Perl may emit warnings for missing modules but still succeed partially
        eprintln!("perl warnings: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout).context("Invalid UTF-8 in perl output")?;
    let mut resolved = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((_module_file, path)) = line.split_once('=') {
            let p = PathBuf::from(path);
            if p.exists() {
                resolved.push(p);
            }
        }
    }

    if resolved.len() < min_resolved {
        return Err(color_eyre::eyre::eyre!(
            "Only {} of {} modules resolved (minimum: {}). Resolved: {:?}",
            resolved.len(),
            modules.len(),
            min_resolved,
            resolved,
        ));
    }

    resolved.sort();
    Ok(resolved)
}

/// Enforce strict zero-error policy for common corpus.
///
/// Returns violations if any files are unreadable or contain errors.
pub fn enforce_strict_clean(report: &SweepReport) -> Vec<RatchetViolation> {
    let mut violations = Vec::new();

    if report.files_unreadable > 0 {
        violations.push(RatchetViolation {
            metric: "files_unreadable".to_string(),
            baseline_value: "0".to_string(),
            current_value: report.files_unreadable.to_string(),
        });
    }

    if report.files_with_errors > 0 {
        violations.push(RatchetViolation {
            metric: "files_with_errors".to_string(),
            baseline_value: "0".to_string(),
            current_value: report.files_with_errors.to_string(),
        });
    }

    if report.total_error_nodes > 0 {
        violations.push(RatchetViolation {
            metric: "total_error_nodes".to_string(),
            baseline_value: "0".to_string(),
            current_value: report.total_error_nodes.to_string(),
        });
    }

    violations
}

/// Run the corpus sweep with the given configuration
pub fn run(config: SweepConfig) -> Result<()> {
    let start_time = Instant::now();

    // Determine corpus profile and file list
    let (corpus_profile, pm_files) = if let Some(ref manifest) = config.manifest_path {
        let files = resolve_manifest_modules(manifest, 6)?;
        ("common".to_string(), files)
    } else {
        ("system".to_string(), discover_pm_files(&config.corpus_roots))
    };

    let use_manifest = config.manifest_path.is_some();
    if pm_files.is_empty() {
        if use_manifest {
            println!("No modules resolved from manifest.");
        } else {
            println!("No .pm files found in specified roots.");
            println!("Searched: {:?}", config.corpus_roots);
        }
        return Ok(());
    }

    if use_manifest {
        println!("Resolved {} modules from manifest", pm_files.len());
    } else {
        println!("Found {} .pm files across {} roots", pm_files.len(), config.corpus_roots.len());
    }

    // Parse each file
    let progress = ProgressBar::new(pm_files.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("=>-"),
    );

    let mut total_files = 0usize;
    let mut files_unreadable = 0usize;
    let mut clean_files = 0usize;
    let mut files_with_errors = 0usize;
    let mut total_error_nodes = 0usize;
    let mut first_error_buckets: BTreeMap<String, usize> = BTreeMap::new();
    let mut file_results: Vec<FileResult> = Vec::new();

    for path in &pm_files {
        total_files += 1;
        progress.set_message(
            path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        );

        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                files_unreadable += 1;
                if config.verbose {
                    file_results.push(FileResult {
                        path: path.display().to_string(),
                        status: "unreadable".to_string(),
                        error_node_count: 0,
                        first_error: None,
                    });
                }
                progress.inc(1);
                continue;
            }
        };

        // Parse
        let mut parser = Parser::new(&source);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(_) => {
                // Catastrophic failure (recursion limit etc.) — count as error
                files_with_errors += 1;
                total_error_nodes += 1;
                let bucket = "catastrophic_parse_failure".to_string();
                *first_error_buckets.entry(bucket.clone()).or_default() += 1;
                if config.verbose {
                    file_results.push(FileResult {
                        path: path.display().to_string(),
                        status: "errors".to_string(),
                        error_node_count: 1,
                        first_error: Some(bucket),
                    });
                }
                progress.inc(1);
                continue;
            }
        };

        // Count ERROR nodes via AST walk
        let summary = collect_error_summary(&ast);

        if summary.count == 0 {
            clean_files += 1;
            if config.verbose {
                file_results.push(FileResult {
                    path: path.display().to_string(),
                    status: "clean".to_string(),
                    error_node_count: 0,
                    first_error: None,
                });
            }
        } else {
            files_with_errors += 1;
            total_error_nodes += summary.count;
            let first = summary.first_message.as_deref().unwrap_or("unknown");
            let bucket = normalize_error_bucket(first);
            *first_error_buckets.entry(bucket.clone()).or_default() += 1;
            if config.verbose {
                file_results.push(FileResult {
                    path: path.display().to_string(),
                    status: "errors".to_string(),
                    error_node_count: summary.count,
                    first_error: Some(bucket),
                });
            }
        }

        progress.inc(1);
    }

    progress.finish_and_clear();

    let elapsed = start_time.elapsed();
    let commit = get_git_commit();

    let report = SweepReport {
        schema_version: "1.1.0".to_string(),
        commit,
        timestamp: chrono::Utc::now().to_rfc3339(),
        corpus_profile: corpus_profile.clone(),
        corpus_roots: if use_manifest {
            vec![config.manifest_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default()]
        } else {
            config.base_roots.iter().map(|p| p.display().to_string()).collect()
        },
        resolved_roots_count: if use_manifest { pm_files.len() } else { config.corpus_roots.len() },
        perl_version: get_perl_version(),
        total_files,
        files_unreadable,
        clean_files,
        files_with_errors,
        total_error_nodes,
        first_error_buckets,
        file_results: if config.verbose { file_results } else { Vec::new() },
        elapsed_secs: elapsed.as_secs_f64(),
    };

    // Print summary
    print_summary(&report);

    // Write output if requested
    if let Some(ref output_path) = config.output_path {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).context("Failed to create output directory")?;
        }
        let json = serde_json::to_string_pretty(&report).context("Failed to serialize report")?;
        fs::write(output_path, json).context("Failed to write report file")?;
        println!("\nReport written to: {}", output_path.display());
    }

    // Write receipt if requested
    if config.receipt {
        let receipt_path =
            PathBuf::from(format!("target/receipts/{}-corpus-sweep.json", corpus_profile));
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent).context("Failed to create receipt directory")?;
        }
        let json = serde_json::to_string_pretty(&report).context("Failed to serialize receipt")?;
        fs::write(&receipt_path, json).context("Failed to write receipt file")?;
        eprintln!("Receipt written to: {}", receipt_path.display());
    }

    // Enforcement: strict clean for manifest mode, ratchet for system mode
    if use_manifest && config.enforce {
        let violations = enforce_strict_clean(&report);
        if !violations.is_empty() {
            println!("\n--- Strict clean violations ---");
            for v in &violations {
                println!(
                    "  VIOLATION: {} — expected: {}, actual: {}",
                    v.metric, v.baseline_value, v.current_value
                );
            }
            return Err(color_eyre::eyre::eyre!(
                "Common corpus strict enforcement failed: {} violation(s) detected",
                violations.len(),
            ));
        }
        println!("Strict clean: all {} files parse without errors", report.total_files);
    }

    // Baseline comparison and ratchet enforcement (system mode)
    if let Some(ref baseline_path) = config.baseline_path {
        let baseline_json =
            fs::read_to_string(baseline_path).context("Failed to read baseline file")?;
        let baseline: SweepReport =
            serde_json::from_str(&baseline_json).context("Failed to parse baseline JSON")?;

        println!("\n--- Baseline comparison ---");
        println!(
            "Baseline: {} clean / {} total ({:.1}%)",
            baseline.clean_files,
            baseline.total_files,
            100.0 * baseline.clean_files as f64 / baseline.total_files.max(1) as f64,
        );
        println!(
            "Current:  {} clean / {} total ({:.1}%)",
            report.clean_files,
            report.total_files,
            100.0 * report.clean_files as f64 / report.total_files.max(1) as f64,
        );

        let delta = report.clean_files as i64 - baseline.clean_files as i64;
        if delta > 0 {
            println!("Result: +{delta} clean files (improvement)");
        } else if delta < 0 {
            println!("Result: {delta} clean files (REGRESSION)");
        } else {
            println!("Result: no change");
        }

        if config.enforce {
            let violations = enforce_ratchet(&report, &baseline);
            if !violations.is_empty() {
                println!("\n--- Ratchet violations ---");
                for v in &violations {
                    println!(
                        "  VIOLATION: {} — baseline: {}, current: {}",
                        v.metric, v.baseline_value, v.current_value
                    );
                }
                return Err(color_eyre::eyre::eyre!(
                    "Ratchet enforcement failed: {} violation(s) detected",
                    violations.len(),
                ));
            }
            println!("Ratchet: all checks passed");
        }
    }

    Ok(())
}

/// Enforce multi-metric ratchet between current report and baseline.
///
/// Returns a list of violations (empty means all checks passed).
pub fn enforce_ratchet(report: &SweepReport, baseline: &SweepReport) -> Vec<RatchetViolation> {
    let mut violations = Vec::new();

    // 1. Crash count must be 0
    let crash_count =
        report.first_error_buckets.get("catastrophic_parse_failure").copied().unwrap_or(0);
    if crash_count > 0 {
        violations.push(RatchetViolation {
            metric: "crash_count".to_string(),
            baseline_value: "0".to_string(),
            current_value: crash_count.to_string(),
        });
    }

    // 2. Unreadable count must not increase
    if report.files_unreadable > baseline.files_unreadable {
        violations.push(RatchetViolation {
            metric: "files_unreadable".to_string(),
            baseline_value: baseline.files_unreadable.to_string(),
            current_value: report.files_unreadable.to_string(),
        });
    }

    // 3. Clean-file count must not decrease
    if report.clean_files < baseline.clean_files {
        violations.push(RatchetViolation {
            metric: "clean_files".to_string(),
            baseline_value: baseline.clean_files.to_string(),
            current_value: report.clean_files.to_string(),
        });
    }

    // 4. Total ERROR nodes must not increase
    if report.total_error_nodes > baseline.total_error_nodes {
        violations.push(RatchetViolation {
            metric: "total_error_nodes".to_string(),
            baseline_value: baseline.total_error_nodes.to_string(),
            current_value: report.total_error_nodes.to_string(),
        });
    }

    // 5. Per-bucket: existing bucket counts must not increase (new buckets allowed)
    for (bucket, &baseline_count) in &baseline.first_error_buckets {
        let current_count = report.first_error_buckets.get(bucket).copied().unwrap_or(0);
        if current_count > baseline_count {
            violations.push(RatchetViolation {
                metric: format!("bucket:{bucket}"),
                baseline_value: baseline_count.to_string(),
                current_value: current_count.to_string(),
            });
        }
    }

    violations
}

/// Discover all .pm files under the given roots
fn discover_pm_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root).follow_links(true).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "pm") {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    files
}

/// Normalize error messages into semantic buckets.
///
/// Two-pass approach:
/// 1. Strip position info (both `"Invalid syntax at position N: msg"` and `"msg at N"` formats)
/// 2. Map to semantic bucket names via substring lookup table
pub fn normalize_error_bucket(error: &str) -> String {
    // Pass 1: strip position info
    let stripped = if let Some(ref re) = *RE_SYNTAX_POS {
        if let Some(caps) = re.captures(error) {
            caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| error.to_string())
        } else if let Some(ref re_at) = *RE_TRAILING_AT {
            re_at.replace(error, "").to_string()
        } else {
            error.to_string()
        }
    } else if let Some(ref re_at) = *RE_TRAILING_AT {
        re_at.replace(error, "").to_string()
    } else {
        error.to_string()
    };

    // Pass 2: map to semantic bucket via first-match substring lookup
    for &(substring, bucket_name) in SEMANTIC_BUCKETS {
        if stripped.contains(substring) {
            return bucket_name.to_string();
        }
    }

    // No match — use position-stripped string as-is
    stripped
}

/// Get the current git commit hash
fn get_git_commit() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Print a summary of the sweep results
fn print_summary(report: &SweepReport) {
    let clean_pct = 100.0 * report.clean_files as f64 / report.total_files.max(1) as f64;
    println!("\n=== Parser Corpus Sweep Results ===");
    println!("Total files:       {}", report.total_files);
    println!("Unreadable:        {}", report.files_unreadable);
    println!("Clean (no errors): {} ({:.1}%)", report.clean_files, clean_pct);
    println!("With errors:       {}", report.files_with_errors);
    println!("Total ERROR nodes: {}", report.total_error_nodes);
    println!("Elapsed:           {:.1}s", report.elapsed_secs);

    if !report.first_error_buckets.is_empty() {
        println!("\n--- First-error buckets (top 20) ---");
        let mut sorted: Vec<_> = report.first_error_buckets.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (i, (bucket, count)) in sorted.iter().enumerate() {
            if i >= 20 {
                let remaining: usize = sorted[20..].iter().map(|(_, c)| *c).sum();
                println!("  ... and {} more buckets ({} files)", sorted.len() - 20, remaining);
                break;
            }
            println!("  {:>4} {}", count, bucket);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test report with sensible defaults
    fn test_report(
        clean_files: usize,
        files_with_errors: usize,
        total_error_nodes: usize,
        files_unreadable: usize,
        first_error_buckets: BTreeMap<String, usize>,
    ) -> SweepReport {
        SweepReport {
            schema_version: "1.1.0".to_string(),
            commit: "abc".to_string(),
            timestamp: "now".to_string(),
            corpus_profile: "system".to_string(),
            corpus_roots: vec![],
            resolved_roots_count: 0,
            perl_version: "unknown".to_string(),
            total_files: clean_files + files_with_errors + files_unreadable,
            files_unreadable,
            clean_files,
            files_with_errors,
            total_error_nodes,
            first_error_buckets,
            file_results: vec![],
            elapsed_secs: 1.0,
        }
    }

    use perl_parser::SourceLocation;

    /// Helper to create a Node with the given kind at the given byte offset.
    fn node_at(kind: NodeKind, start: usize, end: usize) -> Node {
        Node::new(kind, SourceLocation { start, end })
    }

    #[test]
    fn test_collect_error_summary_no_errors() {
        let root = node_at(
            NodeKind::Program {
                statements: vec![node_at(NodeKind::Number { value: "42".to_string() }, 0, 2)],
            },
            0,
            2,
        );
        let summary = collect_error_summary(&root);
        assert_eq!(summary.count, 0);
        assert!(summary.first_message.is_none());
    }

    #[test]
    fn test_collect_error_summary_single_error() {
        let root = node_at(
            NodeKind::Program {
                statements: vec![node_at(
                    NodeKind::Error {
                        message: "expected semicolon".to_string(),
                        expected: vec![],
                        found: None,
                        partial: None,
                    },
                    5,
                    10,
                )],
            },
            0,
            10,
        );
        let summary = collect_error_summary(&root);
        assert_eq!(summary.count, 1);
        assert_eq!(summary.first_message.as_deref(), Some("expected semicolon"));
    }

    #[test]
    fn test_collect_error_summary_multiple_errors_picks_earliest() {
        let root = node_at(
            NodeKind::Program {
                statements: vec![
                    node_at(
                        NodeKind::Error {
                            message: "later error".to_string(),
                            expected: vec![],
                            found: None,
                            partial: None,
                        },
                        20,
                        30,
                    ),
                    node_at(
                        NodeKind::Error {
                            message: "earlier error".to_string(),
                            expected: vec![],
                            found: None,
                            partial: None,
                        },
                        5,
                        15,
                    ),
                ],
            },
            0,
            30,
        );
        let summary = collect_error_summary(&root);
        assert_eq!(summary.count, 2);
        assert_eq!(summary.first_message.as_deref(), Some("earlier error"));
    }

    #[test]
    fn test_collect_error_summary_error_in_partial() {
        // Error node with a partial subtree containing another Error
        let inner_error = node_at(
            NodeKind::Error {
                message: "inner error".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            15,
            20,
        );
        let outer_error = node_at(
            NodeKind::Error {
                message: "outer error".to_string(),
                expected: vec![],
                found: None,
                partial: Some(Box::new(inner_error)),
            },
            5,
            20,
        );
        let root = node_at(NodeKind::Program { statements: vec![outer_error] }, 0, 20);
        let summary = collect_error_summary(&root);
        assert_eq!(summary.count, 2);
        // Outer error at offset 5 wins over inner at offset 15
        assert_eq!(summary.first_message.as_deref(), Some("outer error"));
    }

    #[test]
    fn test_normalize_error_bucket_trailing_at() {
        // Position suffix stripped, then mapped to semantic bucket
        assert_eq!(
            normalize_error_bucket("expected RightBracket, found Eof at 42"),
            "unclosed_bracket",
        );
        // Just position stripping, no semantic match
        assert_eq!(normalize_error_bucket("some rare error at 99"), "some rare error",);
    }

    #[test]
    fn test_normalize_error_bucket_passthrough() {
        assert_eq!(
            normalize_error_bucket("some new error we haven't seen"),
            "some new error we haven't seen",
        );
    }

    #[test]
    fn test_normalize_error_bucket_syntax_position() {
        assert_eq!(
            normalize_error_bucket(
                "Invalid syntax at position 1006: Potential catastrophic backtracking detected"
            ),
            "catastrophic_backtracking",
        );
    }

    #[test]
    fn test_normalize_error_bucket_unclosed_brace_semicolon() {
        assert_eq!(
            normalize_error_bucket("Unexpected token: expected RightBrace, found Semicolon at 42"),
            "unclosed_brace_semicolon",
        );
    }

    #[test]
    fn test_semantic_bucket_mapping() {
        let cases = [
            ("expected expression, found FatArrow at 10", "unexpected_fat_arrow_expr"),
            ("expected expression, found Arrow", "unexpected_arrow_expr"),
            ("expected expression, found Slash at 5", "unexpected_slash_expr"),
            ("expected expression, found Question", "unexpected_question_expr"),
            ("expected expression, found Return at 99", "unexpected_return_expr"),
            ("expected expression, found SomeOtherToken", "unexpected_token_in_expr"),
            ("expected RightBrace, found Eof", "unclosed_brace_eof"),
            ("expected RightBrace, found Semicolon", "unclosed_brace_semicolon"),
            ("expected RightBrace, found Something", "unclosed_brace"),
            ("expected RightParen, found Identifier", "unclosed_paren_identifier"),
            ("expected RightParen, found Eof", "unclosed_paren"),
            ("expected RightBracket, found Eof", "unclosed_bracket"),
            ("expected LeftParen, found Semicolon", "expected_left_paren"),
            ("expected LeftBrace, found Semicolon", "expected_left_brace"),
            ("expected Semicolon, found RightBrace", "expected_semicolon"),
            ("expected Colon, found Semicolon", "expected_colon"),
            ("expected Identifier, found Number", "expected_identifier"),
            ("expected Comma, found Semicolon", "expected_comma"),
            ("Expected variable, found something", "expected_variable"),
            ("Expected string or identifier in import list", "expected_import_item"),
            ("Expected comma or closing parenthesis in signature", "signature_param"),
            ("Expected comma or closing parenthesis", "expected_comma_or_close_paren"),
            ("Expected module name or version string", "expected_module_name"),
            ("Expected '>' to close angle bracket", "unclosed_angle"),
            ("Substitution operator should be s///", "substitution_misparse"),
            ("Invalid syntax at position 42: Expected variable, found X", "expected_variable"),
        ];

        for (input, expected) in cases {
            assert_eq!(normalize_error_bucket(input), expected, "Failed for input: {input}",);
        }
    }

    #[test]
    fn test_enforce_ratchet_no_violations() {
        let report = test_report(
            80,
            18,
            25,
            2,
            BTreeMap::from([("unclosed_brace".to_string(), 10), ("unclosed_paren".to_string(), 8)]),
        );

        let baseline = report.clone();
        let violations = enforce_ratchet(&report, &baseline);
        assert!(violations.is_empty(), "Expected no violations when report equals baseline");
    }

    #[test]
    fn test_enforce_ratchet_regression() {
        let baseline =
            test_report(80, 18, 25, 2, BTreeMap::from([("unclosed_brace".to_string(), 10)]));

        let report = SweepReport {
            clean_files: 75,       // decreased (violation)
            total_error_nodes: 30, // increased (violation)
            files_unreadable: 3,   // increased (violation)
            ..baseline.clone()
        };

        let violations = enforce_ratchet(&report, &baseline);
        assert_eq!(violations.len(), 3, "Expected 3 violations");

        let metrics: Vec<&str> = violations.iter().map(|v| v.metric.as_str()).collect();
        assert!(metrics.contains(&"files_unreadable"));
        assert!(metrics.contains(&"clean_files"));
        assert!(metrics.contains(&"total_error_nodes"));
    }

    #[test]
    fn test_enforce_ratchet_per_bucket() {
        let baseline = test_report(
            80,
            20,
            20,
            0,
            BTreeMap::from([("unclosed_brace".to_string(), 10), ("unclosed_paren".to_string(), 5)]),
        );

        let report = SweepReport {
            first_error_buckets: BTreeMap::from([
                ("unclosed_brace".to_string(), 12), // increased (violation)
                ("unclosed_paren".to_string(), 3),  // decreased (ok)
                ("new_bucket".to_string(), 5),      // new bucket (ok)
            ]),
            ..baseline.clone()
        };

        let violations = enforce_ratchet(&report, &baseline);
        assert_eq!(violations.len(), 1, "Expected 1 per-bucket violation");
        assert_eq!(violations[0].metric, "bucket:unclosed_brace");
        assert_eq!(violations[0].baseline_value, "10");
        assert_eq!(violations[0].current_value, "12");
    }

    #[test]
    fn test_enforce_ratchet_crash_count() {
        let baseline = test_report(80, 20, 20, 0, BTreeMap::new());

        let report = SweepReport {
            first_error_buckets: BTreeMap::from([("catastrophic_parse_failure".to_string(), 2)]),
            ..baseline.clone()
        };

        let violations = enforce_ratchet(&report, &baseline);
        let crash_violation = violations.iter().find(|v| v.metric == "crash_count");
        assert!(crash_violation.is_some(), "Expected crash_count violation");
    }

    #[test]
    fn test_discover_nonexistent_root() {
        let files = discover_pm_files(&[PathBuf::from("/nonexistent/path")]);
        assert!(files.is_empty());
    }

    #[test]
    fn test_backward_compatible_deserialization() {
        // Old 1.0.0 schema without the new fields should deserialize cleanly
        let old_json = r#"{
            "schema_version": "1.0.0",
            "commit": "abc",
            "timestamp": "now",
            "corpus_roots": ["/usr/share/perl"],
            "total_files": 100,
            "files_unreadable": 0,
            "clean_files": 80,
            "files_with_errors": 20,
            "total_error_nodes": 30,
            "first_error_buckets": {},
            "elapsed_secs": 1.0
        }"#;
        let report: SweepReport = serde_json::from_str(old_json).expect("should deserialize");
        assert_eq!(report.corpus_profile, "system");
        assert_eq!(report.resolved_roots_count, 0);
        assert_eq!(report.perl_version, "unknown");
    }

    #[test]
    fn test_enforce_strict_clean_all_clean() {
        let report = test_report(10, 0, 0, 0, BTreeMap::new());
        let violations = enforce_strict_clean(&report);
        assert!(violations.is_empty(), "Expected no violations for all-clean report");
    }

    #[test]
    fn test_enforce_strict_clean_with_errors() {
        let report = test_report(8, 2, 5, 0, BTreeMap::from([("unclosed_brace".to_string(), 2)]));
        let violations = enforce_strict_clean(&report);
        assert_eq!(
            violations.len(),
            2,
            "Expected 2 violations (files_with_errors + total_error_nodes)"
        );
        let metrics: Vec<&str> = violations.iter().map(|v| v.metric.as_str()).collect();
        assert!(metrics.contains(&"files_with_errors"));
        assert!(metrics.contains(&"total_error_nodes"));
    }

    #[test]
    fn test_enforce_strict_clean_with_unreadable() {
        let report = test_report(9, 0, 0, 1, BTreeMap::new());
        let violations = enforce_strict_clean(&report);
        assert_eq!(violations.len(), 1, "Expected 1 violation for unreadable files");
        assert_eq!(violations[0].metric, "files_unreadable");
    }

    #[test]
    fn test_parse_manifest() {
        let dir = std::env::temp_dir().join("test_parse_manifest");
        let _ = fs::create_dir_all(&dir);
        let manifest = dir.join("test-manifest.txt");
        fs::write(&manifest, "# Comment line\n\nExporter\nCarp\n# Another comment\nFile::Find\n")
            .expect("write manifest");
        let modules = parse_manifest(&manifest).expect("parse manifest");
        assert_eq!(modules, vec!["Exporter", "Carp", "File::Find"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_manifest_empty() {
        let dir = std::env::temp_dir().join("test_parse_manifest_empty");
        let _ = fs::create_dir_all(&dir);
        let manifest = dir.join("empty-manifest.txt");
        fs::write(&manifest, "# Only comments\n\n# Nothing here\n").expect("write manifest");
        let modules = parse_manifest(&manifest).expect("parse manifest");
        assert!(modules.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_corpus_roots_nonexistent() {
        let roots = resolve_corpus_roots(&[PathBuf::from("/nonexistent/base")]);
        // Falls back to the base roots themselves
        assert_eq!(roots, vec![PathBuf::from("/nonexistent/base")]);
    }

    #[test]
    fn test_default_base_roots() {
        let roots = default_base_roots();
        assert_eq!(roots.len(), 3);
    }
}
