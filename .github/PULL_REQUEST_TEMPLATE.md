## Summary
<!-- What changed and why. Link the issue: Fixes #NNN -->

## Changes
<!-- List changed files and what each change does -->

## Test
<!-- What test was added? Does it fail before the fix and pass after? -->

## Verification
- [ ] `cargo fmt --all` — clean
- [ ] I used a narrow orthogonal pass first (freshness check, truth-check, or targeted repro) before the broader gate.
- [ ] `cargo clippy -p <crate> --tests` — clean
- [ ] `cargo test -p <crate>` — pass
- [ ] This PR introduces UX-visible changes. I have verified that error messages are actionable and the UX test harness still passes.

## What I considered but didn't do
<!-- Alternative approaches, related issues found, scope decisions -->

## What's next
<!-- Follow-up work, edge cases to address, related issues to file -->

## Agent
<!-- If created by swarm: agent type, issue number, model tier -->
