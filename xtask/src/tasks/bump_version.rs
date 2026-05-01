//! bump-version task wrapper.
//!
//! Delegates to the generic `ci-hygiene` passthrough runner so command
//! execution policy (prefer fresh local binary, fallback to cargo run)
//! stays centralized in one place.

use color_eyre::eyre::Result;

pub fn run(version: String) -> Result<()> {
    super::ci_hygiene::run("bump-version".to_string(), vec![version])
}
