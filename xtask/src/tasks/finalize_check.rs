use color_eyre::eyre::{Context, Result, bail};
use std::fs;

use super::aggregate_receipts::{AdvisoryMode, AggregatorReceipt};

pub fn run(receipt: std::path::PathBuf) -> Result<()> {
    let content = fs::read_to_string(&receipt)
        .with_context(|| format!("Failed to read {}", receipt.display()))?;
    let parsed: AggregatorReceipt =
        serde_json::from_str(&content).context("Failed to parse aggregator receipt")?;
    let finalized = finalize_receipt(parsed);

    let json = serde_json::to_string_pretty(&finalized).context("Failed to serialize receipt")?;
    fs::write(&receipt, json).with_context(|| format!("Failed to write {}", receipt.display()))?;

    if finalized.verdict == "pass" {
        println!("{}: pass ({})", finalized.check, finalized.classification);
        Ok(())
    } else {
        bail!("{}: fail ({})", finalized.check, finalized.classification)
    }
}

pub fn finalize_receipt(mut receipt: AggregatorReceipt) -> AggregatorReceipt {
    let required: Vec<_> = receipt.subreceipts.iter().filter(|item| item.required).collect();

    if !receipt.missing_receipts.is_empty() {
        receipt.verdict = "fail".to_string();
        receipt.classification = "stale_base".to_string();
        return receipt;
    }

    if required.is_empty()
        && receipt.allow_noop
        && receipt
            .subreceipts
            .iter()
            .all(|item| !item.selected || item.verdict.as_str() == "skipped")
    {
        receipt.verdict = "pass".to_string();
        receipt.classification = "skipped".to_string();
        return receipt;
    }

    if let Some(failed_required) = required
        .into_iter()
        .find(|item| item.verdict.as_str() != "pass" && item.verdict.as_str() != "skipped")
    {
        receipt.verdict = "fail".to_string();
        receipt.classification = classify_failure(failed_required.classification.as_deref());
        return receipt;
    }

    let advisory_failures = receipt.subreceipts.iter().any(|item| {
        !item.required && item.verdict.as_str() != "pass" && item.verdict.as_str() != "skipped"
    });

    if advisory_failures && receipt.advisory_mode == AdvisoryMode::Fail {
        receipt.verdict = "fail".to_string();
        receipt.classification = "infra_failure".to_string();
        return receipt;
    }

    receipt.verdict = "pass".to_string();
    receipt.classification =
        if advisory_failures { "infra_failure" } else { "unknown" }.to_string();
    receipt
}

fn classify_failure(classification: Option<&str>) -> String {
    match classification {
        Some("code_regression") => "code_regression".to_string(),
        Some("infra_failure") => "infra_failure".to_string(),
        Some("stale_base") => "stale_base".to_string(),
        Some("skipped") => "skipped".to_string(),
        _ => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::aggregate_receipts::{Repro, Subreceipt};
    use color_eyre::Result;

    #[test]
    fn finalizes_failing_fixture() -> Result<()> {
        let receipt = AggregatorReceipt {
            check: "Test Gate".to_string(),
            schema_version: "1".to_string(),
            event: "pull_request".to_string(),
            verdict: "unknown".to_string(),
            classification: "unknown".to_string(),
            subreceipts: vec![Subreceipt {
                check: "unit-tests".to_string(),
                required: true,
                selected: true,
                verdict: "fail".to_string(),
                classification: Some("code_regression".to_string()),
            }],
            missing_receipts: Vec::new(),
            repro: Repro { command: "cargo xtask aggregate-receipts".to_string() },
            allow_noop: true,
            advisory_mode: AdvisoryMode::Warn,
        };

        let final_receipt = finalize_receipt(receipt);
        assert_eq!(final_receipt.verdict, "fail");
        Ok(())
    }
}
