use crate::tasks::aggregate_receipts::{
    AggregatorReceipt, FailureClassification, SubreceiptVerdict,
};
use color_eyre::eyre::{Result, bail};
use std::fs;
use std::path::PathBuf;

pub fn run(receipt: PathBuf) -> Result<()> {
    let raw = fs::read_to_string(&receipt)?;
    let parsed: AggregatorReceipt = serde_json::from_str(&raw)?;

    match parsed.verdict {
        SubreceiptVerdict::Pass => {
            println!("Finalized check '{}' as PASS", parsed.check);
            Ok(())
        }
        SubreceiptVerdict::Warn => {
            println!("Finalized check '{}' as WARN (advisory-only failures)", parsed.check);
            Ok(())
        }
        SubreceiptVerdict::Fail => bail!(
            "Finalized check '{}' as FAIL (classification: {:?}, missing: {:?})",
            parsed.check,
            parsed.classification,
            parsed.missing_receipts
        ),
        SubreceiptVerdict::Skipped => {
            if parsed.classification == FailureClassification::Skipped {
                println!("Finalized check '{}' as PASS (allowed no-op)", parsed.check);
                Ok(())
            } else {
                bail!(
                    "Finalized check '{}' as SKIPPED without allow-noop classification",
                    parsed.check
                )
            }
        }
        SubreceiptVerdict::Unknown => bail!("Finalized check '{}' as UNKNOWN", parsed.check),
    }
}
