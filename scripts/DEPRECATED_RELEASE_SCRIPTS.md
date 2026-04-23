# Non-Authoritative Legacy Release Scripts

## Authoritative path for RC orchestration

Use `just release-turnkey` (equivalent `cargo xtask release-turnkey`) for the
supported release orchestration flow. This is the canonical entrypoint for
RC-style releases.

## Deprecated / removed legacy scripts

The following scripts are not authoritative and should not be used for
current release operations:

- `scripts/release.sh` (removed, legacy pre-flows)
- `scripts/release-ga.sh` (removed, legacy GA helper)
- `scripts/publish-v0.8.3.sh` (removed, historical one-off v0.8.3 helper)
- `scripts/release-turnkey-pr.sh` (legacy orchestration wrapper; retained for compatibility)
