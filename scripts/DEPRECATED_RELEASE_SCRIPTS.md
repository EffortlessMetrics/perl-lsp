# Non-Authoritative Legacy Release Scripts

## Authoritative path for RC orchestration

Use `scripts/release-turnkey-pr.sh` for the supported release orchestration flow
(or the equivalent `just release-turnkey`/xtask flow). This is the canonical
entrypoint for RC-style releases.

## Deprecated / removed legacy scripts

The following scripts are not authoritative and should not be used for
current release operations:

- `scripts/release.sh` (removed, legacy pre-flows)
- `scripts/release-ga.sh` (removed, legacy GA helper)
- `scripts/publish-v0.8.3.sh` (removed, historical one-off v0.8.3 helper)
- `scripts/prepare-release.sh` (compatibility wrapper; forwards to turnkey flow)
