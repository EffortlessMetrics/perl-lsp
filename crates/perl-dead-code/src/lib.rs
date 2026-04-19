//! Dead code detection for Perl codebases (stub implementation)
//!
//! This module identifies unused code including unreachable code and unused symbols.
//! Currently a stub implementation to demonstrate the architecture.

use perl_workspace::workspace_index::{SymbolKind, WorkspaceIndex, fs_path_to_uri, uri_to_fs_path};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Types of dead code detected during Perl script analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeadCodeType {
    /// Subroutine defined but never called
    UnusedSubroutine,
    /// Variable declared but never used
    UnusedVariable,
    /// Constant defined but never referenced
    UnusedConstant,
    /// Package declared but never used
    UnusedPackage,
    /// Code that can never be executed
    UnreachableCode,
    /// Conditional branch that is never taken
    DeadBranch,
    /// Module imported but never used
    UnusedImport,
    /// Function exported but never used externally
    UnusedExport,
}

/// A piece of dead code detected during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadCode {
    /// Type of dead code (subroutine, variable, etc.)
    pub code_type: DeadCodeType,
    /// Name of the dead code element if available
    pub name: Option<String>,
    /// File path where the dead code is located
    pub file_path: PathBuf,
    /// Starting line number (1-based)
    pub start_line: usize,
    /// Ending line number (1-based)
    pub end_line: usize,
    /// Human-readable explanation of why this is considered dead code
    pub reason: String,
    /// Confidence level (0.0-1.0) in the detection accuracy
    pub confidence: f32,
    /// Optional suggestion for fixing the dead code
    pub suggestion: Option<String>,
}

/// Dead code analysis result for a Perl workspace
#[derive(Debug, Serialize, Deserialize)]
pub struct DeadCodeAnalysis {
    /// List of all dead code instances found
    pub dead_code: Vec<DeadCode>,
    /// Statistical summary of dead code analysis
    pub stats: DeadCodeStats,
    /// Number of files analyzed in the workspace
    pub files_analyzed: usize,
    /// Total lines of code analyzed
    pub total_lines: usize,
}

/// Statistical summary of dead code analysis results
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeadCodeStats {
    /// Number of unused subroutines detected
    pub unused_subroutines: usize,
    /// Number of unused variables detected
    pub unused_variables: usize,
    /// Number of unused constants detected
    pub unused_constants: usize,
    /// Number of unused packages detected
    pub unused_packages: usize,
    /// Number of unreachable code statements
    pub unreachable_statements: usize,
    /// Number of dead conditional branches
    pub dead_branches: usize,
    /// Total lines of dead code identified
    pub total_dead_lines: usize,
}

/// Dead code detector
pub struct DeadCodeDetector {
    workspace_index: WorkspaceIndex,
    entry_points: HashSet<PathBuf>,
}

impl DeadCodeDetector {
    /// Create a new dead code detector with the given workspace index
    ///
    /// # Arguments
    /// * `workspace_index` - Indexed workspace containing symbol definitions and references
    pub fn new(workspace_index: WorkspaceIndex) -> Self {
        Self {
            workspace_index,
            entry_points: HashSet::new(),
        }
    }

    /// Add an entry point (main script)
    pub fn add_entry_point(&mut self, path: PathBuf) {
        self.entry_points.insert(path);
    }

    /// Analyze a single file for dead code
    pub fn analyze_file(&self, file_path: &Path) -> Result<Vec<DeadCode>, String> {
        let uri = fs_path_to_uri(file_path).map_err(|e| e.to_string())?;
        let text = self
            .workspace_index
            .document_store()
            .get_text(&uri)
            .ok_or_else(|| "file not indexed".to_string())?;

        let mut dead = Vec::new();
        let mut terminator: Option<(usize, String)> = None;

        for (i, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if let Some((term_line, term_kw)) = &terminator {
                if !trimmed.is_empty() {
                    dead.push(DeadCode {
                        code_type: DeadCodeType::UnreachableCode,
                        name: None,
                        file_path: file_path.to_path_buf(),
                        start_line: i + 1,
                        end_line: i + 1,
                        reason: format!(
                            "Code is unreachable after `{}` on line {}",
                            term_kw, term_line
                        ),
                        confidence: 0.5,
                        suggestion: Some("Remove or restructure this code".to_string()),
                    });
                    break;
                }
            }

            if ["return", "die", "exit"]
                .iter()
                .any(|kw| trimmed.starts_with(kw))
            {
                if let Some(first_word) = trimmed.split_whitespace().next() {
                    terminator = Some((i + 1, first_word.to_string()));
                }
            }
        }

        // Dead branch detection: scan for constant-condition patterns.
        detect_dead_branches(file_path, &text, &mut dead);

        Ok(dead)
    }

    /// Analyze entire workspace for dead code
    pub fn analyze_workspace(&self) -> DeadCodeAnalysis {
        let docs = self.workspace_index.document_store().all_documents();
        let mut dead_code = Vec::new();
        let mut total_lines = 0;

        // Per-file unreachable code
        for doc in &docs {
            total_lines += doc.text.lines().count();
            if let Some(path) = uri_to_fs_path(&doc.uri) {
                if let Ok(mut file_dead) = self.analyze_file(&path) {
                    dead_code.append(&mut file_dead);
                }
            }
        }

        // Unused symbols across workspace
        for sym in self.workspace_index.find_unused_symbols() {
            let code_type = match sym.kind {
                SymbolKind::Subroutine => DeadCodeType::UnusedSubroutine,
                SymbolKind::Variable(_) => DeadCodeType::UnusedVariable,
                SymbolKind::Constant => DeadCodeType::UnusedConstant,
                SymbolKind::Package => DeadCodeType::UnusedPackage,
                _ => continue,
            };

            let file_path = uri_to_fs_path(&sym.uri).unwrap_or_else(|| PathBuf::from(&sym.uri));

            dead_code.push(DeadCode {
                code_type,
                name: Some(sym.name.clone()),
                file_path,
                start_line: sym.range.start.line as usize + 1,
                end_line: sym.range.end.line as usize + 1,
                reason: "Symbol is never used".to_string(),
                confidence: 0.9,
                suggestion: Some("Remove or use this symbol".to_string()),
            });
        }

        // Compute stats
        let mut stats = DeadCodeStats::default();
        for item in &dead_code {
            let lines = item.end_line.saturating_sub(item.start_line) + 1;
            stats.total_dead_lines += lines;
            match item.code_type {
                DeadCodeType::UnusedSubroutine => stats.unused_subroutines += 1,
                DeadCodeType::UnusedVariable => stats.unused_variables += 1,
                DeadCodeType::UnusedConstant => stats.unused_constants += 1,
                DeadCodeType::UnusedPackage => stats.unused_packages += 1,
                DeadCodeType::UnreachableCode => stats.unreachable_statements += 1,
                DeadCodeType::DeadBranch => stats.dead_branches += 1,
                _ => {}
            }
        }

        DeadCodeAnalysis {
            dead_code,
            stats,
            files_analyzed: docs.len(),
            total_lines,
        }
    }
}

/// Returns `true` if `condition` is a trivially-false constant expression.
///
/// Matches: `0`, `""`, `''`, `undef`, `(0)`, `( 0 )` — the standard Perl idioms
/// used to write permanently-dead `if`/`while`/`for` blocks.
fn is_always_false(condition: &str) -> bool {
    let c = condition.trim();
    matches!(c, "0" | "\"\"" | "''" | "undef")
        || (c.starts_with('(') && c.ends_with(')') && is_always_false(&c[1..c.len() - 1]))
}

/// Returns `true` if `condition` is a trivially-true constant expression.
///
/// Matches: `1`, `"1"`, `'1'`, any non-zero integer literal, `(1)` etc.
fn is_always_true(condition: &str) -> bool {
    let c = condition.trim();
    // Non-zero integer literal
    if c.parse::<i64>().is_ok_and(|n| n != 0) {
        return true;
    }
    // Non-zero float literal
    if c.parse::<f64>().is_ok_and(|n| n != 0.0) {
        return true;
    }
    // Quoted non-empty string that is not "0"
    if (c.starts_with('"') && c.ends_with('"') || c.starts_with('\'') && c.ends_with('\''))
        && c.len() > 2
    {
        let inner = &c[1..c.len() - 1];
        return inner != "0";
    }
    // Parenthesised
    c.starts_with('(') && c.ends_with(')') && is_always_true(&c[1..c.len() - 1])
}

/// Scan `text` for constant-condition dead branches and append `DeadBranch`
/// entries to `out`.
///
/// Detects:
/// - `if (0) { ... }`  — body is never executed
/// - `while (0) { ... }` — loop body is never executed
/// - `unless (1) { ... }` — equivalent to `if (0)`
/// - `until (1) { ... }` — equivalent to `while (0)`
/// - `else` block following an always-true `if (1)` — dead else branch
///
/// Uses a simple brace-counting heuristic to locate the block extent.
/// Only fires for single-line condition + opening brace patterns (the most
/// common idiom); multi-line conditions are skipped to avoid false positives.
fn detect_dead_branches(file_path: &Path, text: &str, out: &mut Vec<DeadCode>) {
    let lines: Vec<&str> = text.lines().collect();
    let n = lines.len();
    let mut i = 0;

    while i < n {
        let trimmed = lines[i].trim();

        // Determine if this line opens a dead branch.
        // We look for: KEYWORD WHITESPACE? ( CONDITION ) WHITESPACE? {
        let dead_reason_and_keyword: Option<(String, &str)> = 'detect: {
            for kw in &["if", "while", "elsif", "unless", "until", "for", "foreach"] {
                let rest = match trimmed.strip_prefix(kw) {
                    Some(r)
                        if r.is_empty()
                            || r.starts_with(|c: char| c.is_whitespace() || c == '(') =>
                    {
                        r.trim_start()
                    }
                    _ => continue,
                };
                // Extract balanced parentheses for the condition.
                if !rest.starts_with('(') {
                    continue;
                }
                let condition = extract_balanced_parens(rest);
                let condition = match condition {
                    Some(c) => c,
                    None => continue,
                };
                let after_cond = rest[condition.len() + 2..].trim(); // skip '(' ... ')'
                // Only fire if opening brace is on the same line.
                if !after_cond.starts_with('{') && !after_cond.is_empty() {
                    continue;
                }
                let inner = condition.trim();

                let reason = if matches!(*kw, "unless" | "until") {
                    // unless/until: body is dead when condition is always-true
                    if is_always_true(inner) {
                        Some(format!(
                            "`{kw}` condition `{inner}` is always true — block is never executed"
                        ))
                    } else {
                        None
                    }
                } else {
                    // if/while/for/foreach: body is dead when condition is always-false
                    if is_always_false(inner) {
                        Some(format!(
                            "`{kw}` condition `{inner}` is always false — block is never executed"
                        ))
                    } else {
                        None
                    }
                };

                if let Some(r) = reason {
                    break 'detect Some((r, *kw));
                }
            }

            // Also check `else` block following always-true `if`.
            // We handle this by looking back at the previously emitted entry
            // or by a simple heuristic: `} else {` on its own line after an
            // always-true if that we tracked.  This is deferred to a follow-up;
            // for now focus on always-false/always-true keyword conditions.
            None
        };

        if let Some((reason, _kw)) = dead_reason_and_keyword {
            // Find the closing brace of this block by counting brace depth.
            let block_start = i + 1; // 1-based
            let end_line = find_block_end(&lines, i);
            out.push(DeadCode {
                code_type: DeadCodeType::DeadBranch,
                name: None,
                file_path: file_path.to_path_buf(),
                start_line: block_start,
                end_line,
                reason,
                confidence: 0.9,
                suggestion: Some("Remove this dead branch or fix the condition".to_string()),
            });
            // Skip to after the block to avoid nested false positives.
            i = end_line;
            continue;
        }

        i += 1;
    }
}

/// Extract the content of the first balanced `(...)` starting at the
/// beginning of `s`.  Returns the inner content (without the outer parens),
/// or `None` if the parens are unbalanced or `s` doesn't start with `(`.
fn extract_balanced_parens(s: &str) -> Option<&str> {
    if !s.starts_with('(') {
        return None;
    }
    let mut depth = 0usize;
    for (idx, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[1..idx]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Find the 1-based line number of the closing `}` for the block that opens
/// on line `open_line` (0-based index).  Uses simple brace counting.
/// Returns `open_line + 1` (1-based same line) if the block closes on the
/// same line, or the last line of the file if braces are unbalanced.
fn find_block_end(lines: &[&str], open_line: usize) -> usize {
    let mut depth = 0i32;
    for (i, line) in lines.iter().enumerate().skip(open_line) {
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return i + 1; // 1-based
                    }
                }
                _ => {}
            }
        }
    }
    lines.len() // fallback: end of file
}

/// Generate a report from dead code analysis
pub fn generate_report(analysis: &DeadCodeAnalysis) -> String {
    let mut report = String::new();

    report.push_str("=== Dead Code Analysis Report ===\n\n");

    report.push_str(&format!("Files analyzed: {}\n", analysis.files_analyzed));
    report.push_str(&format!("Total lines: {}\n", analysis.total_lines));
    report.push_str(&format!(
        "Dead code items: {}\n\n",
        analysis.dead_code.len()
    ));

    report.push_str("Statistics:\n");
    report.push_str(&format!(
        "  Unused subroutines: {}\n",
        analysis.stats.unused_subroutines
    ));
    report.push_str(&format!(
        "  Unused variables: {}\n",
        analysis.stats.unused_variables
    ));
    report.push_str(&format!(
        "  Unused constants: {}\n",
        analysis.stats.unused_constants
    ));
    report.push_str(&format!(
        "  Unused packages: {}\n",
        analysis.stats.unused_packages
    ));
    report.push_str(&format!(
        "  Unreachable statements: {}\n",
        analysis.stats.unreachable_statements
    ));
    report.push_str(&format!(
        "  Dead branches: {}\n",
        analysis.stats.dead_branches
    ));
    report.push_str(&format!(
        "  Total dead lines: {}\n",
        analysis.stats.total_dead_lines
    ));

    report
}
