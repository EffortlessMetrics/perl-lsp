use color_eyre::eyre::{Result, WrapErr, bail};
use std::fs;
use std::path::PathBuf;

use super::aggregate_receipts::AggregatorReceipt;

pub fn run(receipt: PathBuf) -> Result<()> {
    let raw = fs::read_to_string(&receipt)
        .with_context(|| format!("reading aggregator receipt {}", receipt.display()))?;
    let parsed: AggregatorReceipt =
        serde_json::from_str(&raw).context("parsing aggregator receipt JSON")?;

    match parsed.verdict.as_str() {
        "pass" | "warn" => {
            println!("{}: {} ({})", parsed.check, parsed.verdict, parsed.classification);
            Ok(())
        }
        "fail" => bail!(
            "{} failed: classification={}, missing={:?}",
            parsed.check,
            parsed.classification,
            parsed.missing_receipts
        ),
        other => bail!("unsupported verdict in receipt: {other}"),
    }
}
