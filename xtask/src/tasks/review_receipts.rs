use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

const CLEAN_VERDICT: &str = "clean";
const SIGNOFF_CLEAN_ROUTE: &str = "signoff_clean";

/// Validate review receipt policy constraints that cannot be expressed
/// (or are intentionally duplicated) from JSON Schema alone.
pub fn validate_review_receipt_value(receipt: &Value) -> Result<()> {
    let verdict = receipt
        .get("verdict")
        .and_then(Value::as_str)
        .context("review receipt is missing verdict")?;

    let material_observations = receipt
        .get("material_observations")
        .and_then(Value::as_array)
        .context("review receipt is missing material_observations array")?;

    let negative_checks = receipt
        .get("negative_checks")
        .and_then(Value::as_array)
        .context("review receipt is missing negative_checks array")?;

    let next_routes = receipt
        .get("next_routes")
        .and_then(Value::as_array)
        .context("review receipt is missing next_routes array")?;

    if verdict == CLEAN_VERDICT && material_observations.is_empty() {
        bail!(
            "clean review receipts must include at least one material observation for non-trivial diffs"
        );
    }

    if verdict == CLEAN_VERDICT && negative_checks.is_empty() {
        bail!("clean review receipts must include negative checks");
    }

    if verdict.starts_with("needs_")
        && next_routes.iter().any(|route| route.as_str() == Some(SIGNOFF_CLEAN_ROUTE))
    {
        bail!("needs-fix verdicts must not emit clean sign-off intent routes");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_review_receipt_value;
    use anyhow::{Context, Result};
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("review-receipts")
            .join(name)
    }

    fn load_fixture(name: &str) -> Result<serde_json::Value> {
        let path = fixture_path(name);
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read fixture {}", path.display()))?;
        let value = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse fixture {}", path.display()))?;
        Ok(value)
    }

    #[test]
    fn clean_receipt_with_observations_passes() -> Result<()> {
        let fixture = load_fixture("clean-with-observations.json")?;
        validate_review_receipt_value(&fixture)
    }

    #[test]
    fn clean_receipt_without_observations_fails() -> Result<()> {
        let fixture = load_fixture("clean-without-observations.json")?;
        let error = validate_review_receipt_value(&fixture)
            .expect_err("fixture should fail without material observations");
        let message = format!("{error:#}");
        assert!(
            message.contains("material observation"),
            "error should mention missing material observations: {message}"
        );
        Ok(())
    }

    #[test]
    fn needs_builder_fix_with_clean_signoff_intent_fails() -> Result<()> {
        let fixture = load_fixture("needs-builder-fix-with-clean-signoff-intent.json")?;
        let error = validate_review_receipt_value(&fixture)
            .expect_err("fixture should fail when needs-fix includes clean sign-off intent");
        let message = format!("{error:#}");
        assert!(
            message.contains("must not emit clean sign-off intent"),
            "error should mention forbidden clean sign-off intent: {message}"
        );
        Ok(())
    }
}

pub fn validate_review_receipt_file(path: &Path) -> Result<()> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read review receipt {}", path.display()))?;
    let receipt: Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse review receipt {}", path.display()))?;
    validate_review_receipt_value(&receipt)
}
