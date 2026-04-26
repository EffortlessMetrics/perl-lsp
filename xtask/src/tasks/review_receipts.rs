use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub next_routes: Vec<String>,
    pub supersedes: Option<String>,
    pub diff_classification: Option<DiffClassification>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Clean,
    NeedsBuilderFix,
    NeedsDiffFix,
    NeedsHuman,
    BlockedUnknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DiffClassification {
    Trivial,
    NonTrivial,
}

pub fn validate_review_receipt(value: &Value) -> Result<(), Vec<String>> {
    let receipt = match serde_json::from_value::<ReviewReceipt>(value.clone()) {
        Ok(receipt) => receipt,
        Err(error) => {
            return Err(vec![format!("schema violation: {error}")]);
        }
    };

    let mut errors = Vec::new();

    if receipt.kind != "review" {
        errors.push("kind must be 'review'".to_string());
    }
    if receipt.producer.trim().is_empty() {
        errors.push("producer must be non-empty".to_string());
    }
    if receipt.pr == 0 {
        errors.push("pr must be >= 1".to_string());
    }
    if !is_hex_sha(&receipt.head_sha) {
        errors.push("head_sha must be a 7-40 char lowercase hex sha".to_string());
    }
    if !is_hex_sha(&receipt.base_sha) {
        errors.push("base_sha must be a 7-40 char lowercase hex sha".to_string());
    }

    if receipt.verdict == ReviewVerdict::Clean {
        if receipt.negative_checks.is_empty() {
            errors.push("clean verdict must include at least one negative_check".to_string());
        }

        if receipt.diff_classification == Some(DiffClassification::NonTrivial)
            && receipt.material_observations.is_empty()
        {
            errors.push(
                "clean verdict on a non-trivial diff must include material_observations"
                    .to_string(),
            );
        }
    }

    if receipt.verdict != ReviewVerdict::Clean
        && receipt.next_routes.iter().any(|route| is_clean_signoff_route(route))
    {
        errors.push("needs-fix verdicts cannot emit clean sign-off route intent".to_string());
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn is_hex_sha(value: &str) -> bool {
    let len = value.len();
    (7..=40).contains(&len)
        && value.chars().all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
}

fn is_clean_signoff_route(route: &str) -> bool {
    let normalized = route.trim().to_ascii_lowercase();
    normalized == "signoff:clean" || normalized == "signoff_clean" || normalized == "clean-signoff"
}

#[cfg(test)]
mod tests {
    use super::validate_review_receipt;
    use color_eyre::eyre::Result;

    fn fixture(path: &str) -> Result<serde_json::Value> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    #[test]
    fn clean_receipt_with_observations_passes() -> Result<()> {
        let receipt = fixture("tests/fixtures/review-receipts/clean-with-observations.json")?;
        assert!(validate_review_receipt(&receipt).is_ok());
        Ok(())
    }

    #[test]
    fn clean_receipt_without_observations_fails() -> Result<()> {
        let receipt = fixture("tests/fixtures/review-receipts/clean-without-observations.json")?;
        let errors = validate_review_receipt(&receipt)
            .expect_err("clean receipt on non-trivial diff must fail without observations");
        assert!(
            errors.iter().any(|error| error.contains("material_observations")),
            "errors must mention missing material observations: {errors:?}"
        );
        Ok(())
    }

    #[test]
    fn needs_builder_fix_with_clean_signoff_intent_fails() -> Result<()> {
        let receipt = fixture(
            "tests/fixtures/review-receipts/needs-builder-fix-with-clean-signoff-intent.json",
        )?;
        let errors = validate_review_receipt(&receipt)
            .expect_err("needs_builder_fix must not carry clean signoff route intent");
        assert!(
            errors.iter().any(|error| error.contains("sign-off")),
            "errors must mention clean sign-off intent conflict: {errors:?}"
        );
        Ok(())
    }
}
