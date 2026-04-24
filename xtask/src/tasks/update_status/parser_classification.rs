use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Default)]
pub(super) struct StatusClassification {
    pub(super) valid_parser_gap: usize,
    pub(super) expected_recovery_only: usize,
    pub(super) known_invalid: usize,
    pub(super) unreadable: usize,
}

fn parse_path_manifest(path: &Path) -> BTreeSet<String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToString::to_string)
        .collect()
}

pub(super) fn classify_for_status(
    root: &Path,
    profile: &str,
    report: &super::super::parser_corpus_sweep::SweepReport,
) -> StatusClassification {
    let prefix = match profile {
        "system" => "parser",
        "cpan" => "cpan",
        _ => return StatusClassification::default(),
    };
    let valid =
        parse_path_manifest(&root.join(format!(".ci/{prefix}-valid-parser-gap-manifest.txt")));
    let recovery =
        parse_path_manifest(&root.join(format!(".ci/{prefix}-known-recovery-manifest.txt")));
    let invalid =
        parse_path_manifest(&root.join(format!(".ci/{prefix}-known-invalid-manifest.txt")));
    let unreadable_manifest =
        parse_path_manifest(&root.join(format!(".ci/{prefix}-known-unreadable-manifest.txt")));

    let dirty_errors: BTreeSet<String> =
        report.files_by_bucket.values().flat_map(|v| v.iter().cloned()).collect();
    let unreadable: BTreeSet<String> = report.unreadable_files.iter().cloned().collect();

    StatusClassification {
        valid_parser_gap: dirty_errors.intersection(&valid).count(),
        expected_recovery_only: dirty_errors.intersection(&recovery).count(),
        known_invalid: dirty_errors.intersection(&invalid).count(),
        unreadable: unreadable.intersection(&unreadable_manifest).count(),
    }
}
