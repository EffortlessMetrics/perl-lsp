use color_eyre::eyre::{Context, Result, bail};
use std::fs;
use std::path::PathBuf;

use crate::tasks::aggregate_receipts::{
    AggregatorReceipt, Classification, Verdict, evaluate_receipt,
};

#[derive(Debug, Clone)]
pub struct FinalizeCheckConfig {
    pub receipt: PathBuf,
    pub allow_noop: bool,
    pub fail_on_advisory: bool,
}

pub fn run(config: FinalizeCheckConfig) -> Result<()> {
    let body = fs::read_to_string(&config.receipt)
        .with_context(|| format!("failed to read receipt {}", config.receipt.display()))?;
    let receipt: AggregatorReceipt =
        serde_json::from_str(&body).context("failed to parse aggregator receipt JSON")?;

    let (mut verdict, mut classification) =
        evaluate_receipt(&receipt.subreceipts, &receipt.missing_receipts, config.allow_noop);

    if verdict == Verdict::Pass && config.fail_on_advisory {
        let advisory_failed = receipt
            .subreceipts
            .iter()
            .any(|sub| !sub.required && matches!(sub.verdict, Verdict::Fail | Verdict::Warn));
        if advisory_failed {
            verdict = Verdict::Fail;
            classification = Classification::Unknown;
        }
    }

    println!(
        "finalized check={} verdict={:?} classification={:?}",
        receipt.check, verdict, classification
    );

    if verdict == Verdict::Fail {
        bail!("finalize-check failed");
    }

    Ok(())
}
