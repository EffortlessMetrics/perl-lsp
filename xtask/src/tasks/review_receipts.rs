use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct ReviewReceipt {
    pub kind: String,
    pub producer: String,
    pub pr: u64,
    pub head_sha: String,
    pub base_sha: String,
    pub verdict: ReviewVerdict,
    pub material_observations: Vec<String>,
    pub negative_checks: Vec<String>,
    pub blockers: Vec<String>,
    pub next_routes: Vec<ReviewRoute>,
    pub supersedes: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Clean,
    NeedsBuilderFix,
    NeedsDiffFix,
    NeedsHuman,
    BlockedUnknown,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewRoute {
    BuilderFix,
    DiffFix,
    HumanReview,
    AwaitContext,
    SignoffClean,
}

pub fn run_validate(path: PathBuf) -> Result<()> {
    let payload = fs::read_to_string(&path)
        .with_context(|| format!("failed to read receipt {}", path.display()))?;
    let receipt: ReviewReceipt = serde_json::from_str(&payload)
        .with_context(|| format!("failed to parse receipt {}", path.display()))?;
    validate_review_receipt(&receipt)
}

pub fn validate_review_receipt(receipt: &ReviewReceipt) -> Result<()> {
    if receipt.kind != "review" {
        bail!("review receipt kind must be 'review'");
    }

    let _evidence_fields_are_present = (
        &receipt.producer,
        receipt.pr,
        &receipt.head_sha,
        &receipt.base_sha,
        &receipt.blockers,
        &receipt.supersedes,
    );

    if receipt.verdict == ReviewVerdict::Clean {
        if receipt.material_observations.is_empty() {
            bail!("clean verdict requires at least one material observation");
        }

        if receipt.negative_checks.is_empty() {
            bail!("clean verdict requires at least one negative check");
        }
    }

    if matches!(receipt.verdict, ReviewVerdict::NeedsBuilderFix | ReviewVerdict::NeedsDiffFix)
        && receipt.next_routes.contains(&ReviewRoute::SignoffClean)
    {
        bail!("needs-fix verdict cannot include signoff_clean route");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::{Context, Result};

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("review-receipts")
            .join(name)
    }

    fn load_fixture(name: &str) -> Result<ReviewReceipt> {
        let path = fixture_path(name);
        let data = fs::read_to_string(&path)
            .with_context(|| format!("failed to read fixture {}", path.display()))?;
        serde_json::from_str(&data)
            .with_context(|| format!("failed to parse fixture {}", path.display()))
    }

    #[test]
    fn clean_receipt_with_observations_passes() -> Result<()> {
        let receipt = load_fixture("clean-with-observations.json")?;
        validate_review_receipt(&receipt)
    }

    #[test]
    fn clean_receipt_without_observations_fails() -> Result<()> {
        let receipt = load_fixture("clean-without-observations.json")?;
        let result = validate_review_receipt(&receipt);
        if result.is_ok() {
            bail!("expected validation to fail for clean receipt without observations");
        }
        Ok(())
    }

    #[test]
    fn needs_builder_fix_with_signoff_intent_fails() -> Result<()> {
        let receipt = load_fixture("needs-builder-fix-with-signoff-intent.json")?;
        let result = validate_review_receipt(&receipt);
        if result.is_ok() {
            bail!(
                "expected validation to fail when needs_builder_fix includes signoff_clean route"
            );
        }
        Ok(())
    }
}
