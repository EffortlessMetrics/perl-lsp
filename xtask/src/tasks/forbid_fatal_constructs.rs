//! Wrapper task for forbidden-fatal-construct checks.

use color_eyre::eyre::Result;

use crate::tasks::ci_hygiene;

pub fn run(args: Vec<String>) -> Result<()> {
    ci_hygiene::run("forbid-fatal-constructs".to_string(), args)
}
