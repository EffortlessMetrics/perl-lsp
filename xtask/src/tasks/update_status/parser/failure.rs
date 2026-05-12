//! Parser corpus failure clustering for status output.

use std::collections::BTreeMap;

/// Failure cluster categories used to group parser corpus error buckets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FailureCluster {
    TransliterationQuote,
    DeclarationPackage,
    HeredocDelimiter,
    RecoveryOnly,
    EncodingMultibyte,
    Other,
}

impl FailureCluster {
    fn display_name(self) -> &'static str {
        match self {
            FailureCluster::TransliterationQuote => "transliteration / quote parsing",
            FailureCluster::DeclarationPackage => "declaration / package parsing",
            FailureCluster::HeredocDelimiter => "heredoc / delimiter handling",
            FailureCluster::RecoveryOnly => "recovery-only failures",
            FailureCluster::EncodingMultibyte => "encoding / multibyte",
            FailureCluster::Other => "other",
        }
    }
}

fn ordered_clusters() -> [FailureCluster; 6] {
    [
        FailureCluster::TransliterationQuote,
        FailureCluster::DeclarationPackage,
        FailureCluster::HeredocDelimiter,
        FailureCluster::RecoveryOnly,
        FailureCluster::EncodingMultibyte,
        FailureCluster::Other,
    ]
}

/// Classify a single error bucket name into a [`FailureCluster`].
///
/// Priority order (highest first):
/// 1. `RecoveryOnly` — catch-all token/expression recovery buckets
/// 2. `HeredocDelimiter` — delimiter and bracket mismatch errors
/// 3. `DeclarationPackage` — identifier, variable, and declaration errors
/// 4. `EncodingMultibyte` — wide-character and Unicode errors
/// 5. `TransliterationQuote` — transliteration, quote, and string errors
/// 6. `Other` — anything else
pub(crate) fn classify_failure_bucket(bucket: &str) -> FailureCluster {
    let lower = bucket.to_ascii_lowercase();

    // RecoveryOnly: generic expression-level recovery buckets
    if lower.contains("unexpected_token") || lower.contains("incomplete") {
        return FailureCluster::RecoveryOnly;
    }

    // HeredocDelimiter: bracket/brace/paren/delimiter mismatches and unclosed_ errors
    if lower.contains("brace")
        || lower.contains("bracket")
        || lower.contains("paren")
        || lower.contains("delimiter")
        || lower.starts_with("unclosed_")
    {
        return FailureCluster::HeredocDelimiter;
    }

    // DeclarationPackage: identifier, variable, and named-block declarations
    if lower.contains("identifier")
        || lower.contains("variable")
        || lower.contains("check must")
        || lower.contains("signature")
        || lower.contains("declaration")
        || lower.contains("package")
    {
        return FailureCluster::DeclarationPackage;
    }

    // EncodingMultibyte: wide-character and Unicode errors
    if lower.contains("wide character")
        || lower.contains("unicode")
        || lower.contains("utf")
        || lower.contains("multibyte")
        || lower.contains("encoding")
    {
        return FailureCluster::EncodingMultibyte;
    }

    // TransliterationQuote: transliteration operators and string/quote errors
    if lower.contains("tr/")
        || lower.contains("translit")
        || lower.contains("string")
        || lower.contains("quote")
    {
        return FailureCluster::TransliterationQuote;
    }

    FailureCluster::Other
}

/// Build a markdown table summarising parser corpus failures grouped by cluster.
///
/// Returns a multi-line string with one row per [`FailureCluster`] variant.
/// The table has **no** header row — callers embed it inside an existing table.
pub(crate) fn build_failure_worklist(
    report: &super::super::super::parser_corpus_sweep::SweepReport,
) -> String {
    let mut counts: BTreeMap<FailureCluster, usize> = BTreeMap::new();
    for (bucket_name, &count) in &report.first_error_buckets {
        let cluster = classify_failure_bucket(bucket_name);
        *counts.entry(cluster).or_insert(0) += count;
    }

    ordered_clusters()
        .iter()
        .map(|cluster| {
            let count = counts.get(cluster).copied().unwrap_or(0);
            format!("| {} | {} |", cluster.display_name(), count)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build a markdown table that preserves the raw first-error buckets inside
/// each generated parser failure cluster.
///
/// The rows are ordered by cluster, then by descending count and bucket name.
pub(crate) fn build_failure_bucket_details(
    report: &super::super::super::parser_corpus_sweep::SweepReport,
) -> String {
    if report.first_error_buckets.is_empty() {
        return "| none | n/a | 0 |".to_string();
    }

    let mut rows: Vec<_> = report
        .first_error_buckets
        .iter()
        .map(|(bucket_name, &count)| (classify_failure_bucket(bucket_name), bucket_name, count))
        .collect();
    rows.sort_by(|(cluster_a, bucket_a, count_a), (cluster_b, bucket_b, count_b)| {
        cluster_a
            .cmp(cluster_b)
            .then_with(|| count_b.cmp(count_a))
            .then_with(|| bucket_a.cmp(bucket_b))
    });

    rows.iter()
        .map(|(cluster, bucket, count)| {
            format!("| {} | `{}` | {} |", cluster.display_name(), bucket, count)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
