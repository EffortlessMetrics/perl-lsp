//! Dead code detection for Perl codebases (stub implementation)
//!
//! This module identifies unused code including unreachable code and unused symbols.
//! Currently a stub implementation to demonstrate the architecture.

use perl_workspace::workspace_index::{SymbolKind, WorkspaceIndex, fs_path_to_uri, uri_to_fs_path};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

mod dead_branches;
mod report;

pub use report::generate_report;

use crate::dead_branches::detect_dead_branches;

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
        Self { workspace_index, entry_points: HashSet::new() }
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
        let mut terminator: Option<(usize, String, usize)> = None;
        let mut block_depth = 0usize;

        for (i, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            let line_number = i + 1;
            let structural_close_only = trimmed.chars().all(|ch| ch == '}');

            if let Some((term_line, term_kw, term_depth)) = &terminator {
                if block_depth == *term_depth {
                    if is_executable_line(trimmed) {
                        dead.push(DeadCode {
                            code_type: DeadCodeType::UnreachableCode,
                            name: None,
                            file_path: file_path.to_path_buf(),
                            start_line: line_number,
                            end_line: line_number,
                            reason: format!(
                                "Code is unreachable after `{}` on line {}",
                                term_kw, term_line
                            ),
                            confidence: 0.9,
                            suggestion: Some("Remove or restructure this code".to_string()),
                        });
                        break;
                    }
                } else if block_depth < *term_depth {
                    terminator = None;
                }
            }

            if structural_close_only {
                block_depth = block_depth.saturating_sub(trimmed.chars().count());
                if let Some((_, _, term_depth)) = &terminator {
                    if block_depth < *term_depth {
                        terminator = None;
                    }
                }
                continue;
            }

            if terminator.is_none()
                && let Some(term_kw) = detect_unconditional_terminator(trimmed)
            {
                terminator = Some((line_number, term_kw.to_string(), block_depth));
            }

            let opens = line.chars().filter(|ch| *ch == '{').count();
            let closes = line.chars().filter(|ch| *ch == '}').count();
            block_depth += opens;
            block_depth = block_depth.saturating_sub(closes);
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

        DeadCodeAnalysis { dead_code, stats, files_analyzed: docs.len(), total_lines }
    }
}

fn is_executable_line(trimmed: &str) -> bool {
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return false;
    }

    !trimmed.chars().all(|ch| ch == '{' || ch == '}')
}

fn detect_unconditional_terminator(trimmed: &str) -> Option<&'static str> {
    let code = strip_inline_comment(trimmed).trim();
    if code.is_empty() {
        return None;
    }

    for (raw, canonical) in [
        ("return", "return"),
        ("die", "die"),
        ("exit", "exit"),
        ("CORE::exit", "CORE::exit"),
    ] {
        if starts_with_keyword(code, raw) {
            let rest = code[raw.len()..].trim_start();
            if starts_with_postfix_conditional(rest) {
                return None;
            }
            return Some(canonical);
        }
    }

    None
}

fn starts_with_keyword(code: &str, keyword: &str) -> bool {
    code.starts_with(keyword)
        && code
            .chars()
            .nth(keyword.len())
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
}

fn starts_with_postfix_conditional(rest: &str) -> bool {
    ["if", "unless", "while", "until", "for", "foreach", "when"]
        .iter()
        .any(|kw| starts_with_keyword(rest, kw))
}

fn strip_inline_comment(line: &str) -> &str {
    if let Some((code, _)) = line.split_once('#') {
        code
    } else {
        line
    }
}
