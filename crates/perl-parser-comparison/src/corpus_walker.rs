//! Corpus walker for differential parser testing.
//!
//! Walks the project's real-world Perl corpora (test_corpus/, tree-sitter
//! highlight fixtures) and runs all three parsers on each file, collecting
//! per-file [`FileRecord`]s that classify the outcome.
//!
//! # Disagreement kinds
//!
//! | Kind | Meaning |
//! |------|---------|
//! | `AllAgree` | All three parsers gave the same outcome category |
//! | `RecoveryDisagreement` | Some parsers recovered, others errored |
//! | `V3OnlyClean` | v3 succeeded; v1 and v2 both failed or produced errors |
//! | `V2OnlyClean` | v2 succeeded; v1 and v3 both failed (rare) |
//! | `V1OnlyClean` | v1 succeeded; v2 and v3 both failed (should be ~never) |
//! | `EachDisagrees` | Three different outcomes |

use std::fmt;
use std::path::PathBuf;

use crate::harness::{parse_v1, parse_v2, parse_v3};
use crate::outcomes::Verdict;

/// Maximum file size in bytes to process (1 MiB).
const MAX_FILE_BYTES: u64 = 1_024 * 1_024;

/// The outcome of running all three parsers on a single corpus file.
#[derive(Debug, Clone)]
pub struct FileRecord {
    /// Path to the source file.
    pub path: PathBuf,
    /// Verdict from v1 (tree-sitter-c).
    pub v1: Verdict,
    /// Verdict from v2 (pest).
    pub v2: Verdict,
    /// Verdict from v3 (recursive-descent production parser).
    pub v3: Verdict,
    /// Classification of how (or whether) the three parsers disagreed.
    pub disagreement: DisagreementKind,
}

/// Categories of parser disagreement for a single file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DisagreementKind {
    /// All three produced the same outcome category (boring).
    AllAgree,
    /// Some parsers recovered (Errors/WrongButPlausible), others errored cleanly.
    RecoveryDisagreement,
    /// v3 parsed cleanly; v1 and v2 both errored or recovered.
    V3OnlyClean,
    /// v2 parsed cleanly; v1 and v3 both errored or recovered (rare).
    V2OnlyClean,
    /// v1 parsed cleanly; v2 and v3 both errored or recovered (rare).
    V1OnlyClean,
    /// All three produced different outcomes.
    EachDisagrees,
}

impl fmt::Display for DisagreementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllAgree => write!(f, "all_agree"),
            Self::RecoveryDisagreement => write!(f, "recovery_disagreement"),
            Self::V3OnlyClean => write!(f, "v3_only_clean"),
            Self::V2OnlyClean => write!(f, "v2_only_clean"),
            Self::V1OnlyClean => write!(f, "v1_only_clean"),
            Self::EachDisagrees => write!(f, "each_disagrees"),
        }
    }
}

/// Returns `true` if the verdict indicates the parser produced a clean result
/// with no error nodes or diagnostics.
fn is_clean(v: &Verdict) -> bool {
    matches!(v, Verdict::Correct)
}

/// Classify the disagreement among the three verdicts.
pub fn classify(v1: &Verdict, v2: &Verdict, v3: &Verdict) -> DisagreementKind {
    let c1 = is_clean(v1);
    let c2 = is_clean(v2);
    let c3 = is_clean(v3);

    match (c1, c2, c3) {
        // All clean — always all_agree
        (true, true, true) => DisagreementKind::AllAgree,
        // All non-clean — check if they're the same kind of failure
        (false, false, false) if v1 == v2 && v2 == v3 => DisagreementKind::AllAgree,
        (false, false, false) => DisagreementKind::RecoveryDisagreement,

        // Only one parser is clean
        (false, false, true) => DisagreementKind::V3OnlyClean,
        (false, true, false) => DisagreementKind::V2OnlyClean,
        (true, false, false) => DisagreementKind::V1OnlyClean,

        // Two parsers clean, one not — recovery disagreement
        (true, true, false) | (true, false, true) | (false, true, true) => {
            DisagreementKind::RecoveryDisagreement
        }
    }
}

/// Walk the corpora and return one [`FileRecord`] per file.
///
/// Skips files larger than [`MAX_FILE_BYTES`] and skips `target/` and `.git/`
/// trees.
///
/// `corpus_roots` is a list of directories to walk.  Pass the workspace-relative
/// paths (`test_corpus/`, `tree-sitter-perl/test/highlight/`) converted to
/// absolute paths using the workspace root.
pub fn walk_corpora(corpus_roots: &[PathBuf]) -> Vec<FileRecord> {
    let mut records = Vec::new();

    for root in corpus_roots {
        if !root.exists() {
            continue;
        }
        let walker =
            walkdir::WalkDir::new(root).follow_links(false).into_iter().filter_entry(|e| {
                // Skip hidden dirs and build artifacts
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "target"
            });

        for entry in walker.flatten() {
            let path = entry.path().to_owned();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "pl" | "pm" | "t") {
                continue;
            }
            // Skip large files
            if let Ok(meta) = path.metadata() {
                if meta.len() > MAX_FILE_BYTES {
                    continue;
                }
            }
            let source = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => continue, // Skip unreadable files (binary, encoding issues)
            };

            let r1 = parse_v1(&source);
            let r2 = parse_v2(&source);
            let r3 = parse_v3(&source);

            let disagreement = classify(&r1.verdict, &r2.verdict, &r3.verdict);

            records.push(FileRecord {
                path,
                v1: r1.verdict,
                v2: r2.verdict,
                v3: r3.verdict,
                disagreement,
            });
        }
    }

    records
}

/// Aggregate statistics over a set of [`FileRecord`]s.
#[derive(Debug, Default)]
pub struct AggregateStats {
    /// Total files processed.
    pub total: usize,
    /// Files where all parsers agreed on the same outcome.
    pub all_agree: usize,
    /// Files where parsers had mixed error/recovery outcomes.
    pub recovery_disagreement: usize,
    /// Files where only v3 parsed cleanly.
    pub v3_only_clean: usize,
    /// Files where only v2 parsed cleanly.
    pub v2_only_clean: usize,
    /// Files where only v1 parsed cleanly.
    pub v1_only_clean: usize,
    /// Files where all three parsers gave different outcomes.
    pub each_disagrees: usize,

    /// Number of files v1 parsed cleanly (Correct verdict).
    pub v1_clean: usize,
    /// Number of files v1 produced errors on.
    pub v1_errors: usize,
    /// Number of files v1 crashed on.
    pub v1_crashes: usize,
    /// Number of files v1 gave WrongButPlausible or SilentlyEmpty verdict.
    pub v1_other: usize,

    /// Number of files v2 parsed cleanly (Correct verdict).
    pub v2_clean: usize,
    /// Number of files v2 produced errors on.
    pub v2_errors: usize,
    /// Number of files v2 crashed on.
    pub v2_crashes: usize,
    /// Number of files v2 gave WrongButPlausible or SilentlyEmpty verdict.
    pub v2_other: usize,

    /// Number of files v3 parsed cleanly (Correct verdict).
    pub v3_clean: usize,
    /// Number of files v3 produced errors on.
    pub v3_errors: usize,
    /// Number of files v3 crashed on.
    pub v3_crashes: usize,
    /// Number of files v3 gave WrongButPlausible or SilentlyEmpty verdict.
    pub v3_other: usize,
}

impl AggregateStats {
    /// Build aggregated statistics from a slice of file records.
    pub fn from_records(records: &[FileRecord]) -> Self {
        let mut s = Self { total: records.len(), ..Self::default() };

        for r in records {
            match r.disagreement {
                DisagreementKind::AllAgree => s.all_agree += 1,
                DisagreementKind::RecoveryDisagreement => s.recovery_disagreement += 1,
                DisagreementKind::V3OnlyClean => s.v3_only_clean += 1,
                DisagreementKind::V2OnlyClean => s.v2_only_clean += 1,
                DisagreementKind::V1OnlyClean => s.v1_only_clean += 1,
                DisagreementKind::EachDisagrees => s.each_disagrees += 1,
            }
            count_verdict(
                &r.v1,
                &mut s.v1_clean,
                &mut s.v1_errors,
                &mut s.v1_crashes,
                &mut s.v1_other,
            );
            count_verdict(
                &r.v2,
                &mut s.v2_clean,
                &mut s.v2_errors,
                &mut s.v2_crashes,
                &mut s.v2_other,
            );
            count_verdict(
                &r.v3,
                &mut s.v3_clean,
                &mut s.v3_errors,
                &mut s.v3_crashes,
                &mut s.v3_other,
            );
        }
        s
    }

    /// Total files where parsers disagree (not all_agree).
    pub fn total_disagreements(&self) -> usize {
        self.recovery_disagreement
            + self.v3_only_clean
            + self.v2_only_clean
            + self.v1_only_clean
            + self.each_disagrees
    }
}

fn count_verdict(
    v: &Verdict,
    clean: &mut usize,
    errors: &mut usize,
    crashes: &mut usize,
    other: &mut usize,
) {
    match v {
        Verdict::Correct => *clean += 1,
        Verdict::Errors => *errors += 1,
        Verdict::Crashes => *crashes += 1,
        Verdict::WrongButPlausible | Verdict::SilentlyEmpty => *other += 1,
    }
}

/// Format a Markdown differential report from the collected records.
pub fn format_report(records: &[FileRecord], stats: &AggregateStats) -> String {
    let mut out = String::new();

    out.push_str("# Parser Corpus Differential Report\n\n");
    out.push_str(
        "Three-parser (v1/v2/v3) differential run across the project's real-world Perl corpora.\n\n",
    );

    out.push_str("## Summary\n\n");
    out.push_str("| Disagreement Kind | Count | % of Total |\n");
    out.push_str("|-------------------|------:|----------:|\n");
    let pct = |n: usize| {
        if stats.total == 0 { 0.0_f64 } else { n as f64 / stats.total as f64 * 100.0 }
    };
    out.push_str(&format!(
        "| `all_agree`               | {:5} | {:5.1}% |\n",
        stats.all_agree,
        pct(stats.all_agree)
    ));
    out.push_str(&format!(
        "| `recovery_disagreement`   | {:5} | {:5.1}% |\n",
        stats.recovery_disagreement,
        pct(stats.recovery_disagreement)
    ));
    out.push_str(&format!(
        "| `v3_only_clean`           | {:5} | {:5.1}% |\n",
        stats.v3_only_clean,
        pct(stats.v3_only_clean)
    ));
    out.push_str(&format!(
        "| `v2_only_clean`           | {:5} | {:5.1}% |\n",
        stats.v2_only_clean,
        pct(stats.v2_only_clean)
    ));
    out.push_str(&format!(
        "| `v1_only_clean`           | {:5} | {:5.1}% |\n",
        stats.v1_only_clean,
        pct(stats.v1_only_clean)
    ));
    out.push_str(&format!(
        "| `each_disagrees`          | {:5} | {:5.1}% |\n",
        stats.each_disagrees,
        pct(stats.each_disagrees)
    ));
    out.push_str(&format!("| **Total files**           | {:5} |         |\n", stats.total));
    out.push_str(&format!(
        "| **Total disagreements**   | {:5} | {:5.1}% |\n\n",
        stats.total_disagreements(),
        pct(stats.total_disagreements())
    ));

    out.push_str("## Per-Parser Totals\n\n");
    out.push_str(
        "| Parser | Clean | Errors | Crashes | Other (WrongButPlausible/SilentlyEmpty) |\n",
    );
    out.push_str(
        "|--------|------:|-------:|--------:|---------------------------------------:|\n",
    );
    out.push_str(&format!(
        "| v1 (tree-sitter-c)       | {} | {} | {} | {} |\n",
        stats.v1_clean, stats.v1_errors, stats.v1_crashes, stats.v1_other
    ));
    out.push_str(&format!(
        "| v2 (pest)                | {} | {} | {} | {} |\n",
        stats.v2_clean, stats.v2_errors, stats.v2_crashes, stats.v2_other
    ));
    out.push_str(&format!(
        "| v3 (recursive-descent)   | {} | {} | {} | {} |\n\n",
        stats.v3_clean, stats.v3_errors, stats.v3_crashes, stats.v3_other
    ));

    // Top disagreements (exclude all_agree)
    let mut disagreements: Vec<&FileRecord> =
        records.iter().filter(|r| r.disagreement != DisagreementKind::AllAgree).collect();
    // Sort by disagreement kind (most interesting first)
    disagreements.sort_by_key(|r| match r.disagreement {
        DisagreementKind::EachDisagrees => 0,
        DisagreementKind::V3OnlyClean => 1,
        DisagreementKind::V1OnlyClean => 2,
        DisagreementKind::V2OnlyClean => 3,
        DisagreementKind::RecoveryDisagreement => 4,
        DisagreementKind::AllAgree => 5,
    });

    out.push_str("## Top Disagreements (up to 20)\n\n");
    out.push_str("| File | v1 | v2 | v3 | Kind |\n");
    out.push_str("|------|----|----|----|---------|\n");
    for r in disagreements.iter().take(20) {
        let short_path = r.path.display().to_string();
        // Try to shorten the path for display by finding a recognizable prefix
        let display_path = short_path
            .find("test_corpus")
            .or_else(|| short_path.find("tree-sitter-perl"))
            .or_else(|| short_path.find("perl-corpus"))
            .map(|i| &short_path[i..])
            .unwrap_or(short_path.as_str());
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | `{}` |\n",
            display_path, r.v1, r.v2, r.v3, r.disagreement
        ));
    }

    out
}
