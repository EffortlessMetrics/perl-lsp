// Allow dead_code because this helper is currently exercised by tests and schema fixtures before CLI wiring lands.
#![allow(dead_code)]

use color_eyre::eyre::{Result, bail};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReviewVerdict {
    Clean,
    NeedsBuilderFix,
    NeedsDiffFix,
    NeedsHuman,
    BlockedUnknown,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewReceipt {
    kind: String,
    producer: String,
    pr: u64,
    head_sha: String,
    base_sha: String,
    verdict: ReviewVerdict,
    material_observations: Vec<String>,
    negative_checks: Vec<String>,
    blockers: Vec<String>,
    next_routes: Vec<String>,
    supersedes: Option<String>,
}

/// Validate review receipt invariants that are easy to keep in sync with tests.
pub fn validate_review_receipt(value: &Value) -> Result<()> {
    let receipt: ReviewReceipt = serde_json::from_value(value.clone())?;

    if receipt.kind != "review" {
        bail!("kind must be 'review'");
    }

    if receipt.producer.is_empty() {
        bail!("producer must be non-empty");
    }

    if receipt.pr == 0 {
        bail!("pr must be >= 1");
    }

    if !is_sha1_hex(&receipt.head_sha) {
        bail!("head_sha must be a 40-char lowercase hex commit sha");
    }

    if !is_sha1_hex(&receipt.base_sha) {
        bail!("base_sha must be a 40-char lowercase hex commit sha");
    }

    if receipt.next_routes.is_empty() {
        bail!("next_routes must not be empty");
    }

    let has_signoff_intent = receipt.next_routes.iter().any(|route| route == "signoff_clean");

    if matches!(receipt.verdict, ReviewVerdict::Clean) {
        if receipt.material_observations.is_empty() {
            bail!("clean verdict requires at least one material observation");
        }
        if receipt.negative_checks.is_empty() {
            bail!("clean verdict requires at least one negative check");
        }
        if !has_signoff_intent {
            bail!("clean verdict must include signoff_clean in next_routes");
        }
    }

    if matches!(
        receipt.verdict,
        ReviewVerdict::NeedsBuilderFix
            | ReviewVerdict::NeedsDiffFix
            | ReviewVerdict::NeedsHuman
            | ReviewVerdict::BlockedUnknown
    ) && has_signoff_intent
    {
        bail!("needs-fix/human/blocked verdicts must not emit clean sign-off intent");
    }

    // Evidence-only receipt: route hints are allowed, mutation commands are not.
    if receipt.next_routes.iter().any(|route| route.starts_with("label:")) {
        bail!("review receipts are evidence-only and may not include label mutations");
    }

    let _ = receipt.blockers;
    let _ = receipt.supersedes;

    Ok(())
}

fn is_sha1_hex(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::validate_review_receipt;
    use color_eyre::eyre::{Result, eyre};
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> Result<PathBuf> {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = base.join("tests").join("fixtures").join("review-receipts").join(name);
        if !path.exists() {
            return Err(eyre!("missing fixture: {}", path.display()));
        }
        Ok(path)
    }

    fn load_fixture(name: &str) -> Result<Value> {
        let raw = fs::read_to_string(fixture_path(name)?)?;
        let parsed: Value = serde_json::from_str(&raw)?;
        Ok(parsed)
    }

    #[test]
    fn clean_receipt_with_observations_passes() -> Result<()> {
        let value = load_fixture("clean-with-observations.json")?;
        validate_review_receipt(&value)
    }

    #[test]
    fn clean_receipt_without_observations_fails() -> Result<()> {
        let value = load_fixture("clean-without-observations.json")?;
        let err = validate_review_receipt(&value)
            .err()
            .ok_or_else(|| eyre!("fixture should fail validation"))?;
        let message = err.to_string();
        if !message.contains("material observation") {
            return Err(eyre!("expected material observation failure, got: {message}"));
        }
        Ok(())
    }

    #[test]
    fn needs_builder_fix_with_clean_signoff_intent_fails() -> Result<()> {
        let value = load_fixture("needs-builder-fix-with-clean-signoff-intent.json")?;
        let err = validate_review_receipt(&value)
            .err()
            .ok_or_else(|| eyre!("fixture should fail validation"))?;
        let message = err.to_string();
        if !message.contains("must not emit clean sign-off intent") {
            return Err(eyre!("expected signoff intent failure, got: {message}"));
        }
        Ok(())
    }
}
