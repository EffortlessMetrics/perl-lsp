# Parser Ratchet Comparator (PR mode)

`cargo xtask parser-ratchet` now compares **live base vs candidate** metrics in PR/merge-group mode.

## Commands

```bash
cargo xtask parser-ratchet \
  --profile pr \
  --base <sha> \
  --head <sha> \
  --manifest target/parser-ratchet/corpus-manifest.json \
  --receipt target/receipts/parser-ratchet.json

cargo xtask parser-ratchet compare \
  --base-metrics <json> \
  --head-metrics <json> \
  --receipt <json>
```

## Policy summary

- No committed PR baseline metrics file is used.
- One manifest is selected and fingerprinted, then used for both base and candidate.
- `perl-corpus` scope enforces strict panic/timeout zero, floor checks, and regression controls.
- `system-perl` scope is differential only (unchanged base failures do not block).
- `corpus_runtime_ms` is advisory (`warn`) only.

## Receipt fields

The comparator writes:

- `check`, `profile`, `selected`, `selection_reason`
- `manifest_fingerprint`
- `base_sha`, `candidate_sha`
- `metrics.base`, `metrics.head`
- `violations`, `ratchet_opportunity`, `verdict`
- `repro.command`
