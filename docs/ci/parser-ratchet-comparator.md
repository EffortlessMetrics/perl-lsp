# Parser Ratchet Comparator

`cargo xtask parser-ratchet` compares parser metrics from base vs candidate against one shared manifest fingerprint.

## Commands

- PR mode (integration hook for live collection):
  - `cargo xtask parser-ratchet --profile pr --base <sha> --head <sha> --manifest <path> --receipt <path>`
- Direct comparator mode:
  - `cargo xtask parser-ratchet compare --base-metrics <json> --head-metrics <json> --receipt <json>`

## Rule summary

### `selected = perl-corpus`
- `panic_count` must be zero.
- `timeout_count` must be zero.
- `concept_floors_pass` must pass when supplied.
- `clean_parse_rate` must not regress beyond profile epsilon.
- `error_node_count` must not materially increase.
- `node_kind_seen_count` must not unexpectedly drop.

### `selected = system-perl`
- Differential-only evaluation.
- Fail on worsened `panic_count` or `timeout_count`.
- Fail on `clean_parse_rate` regression beyond epsilon.
- Existing base failures that do not worsen are allowed.

### Runtime
- `corpus_runtime_ms` is advisory only (warning), not a hard fail.

## Receipt fields

Receipt includes:
- `check`, `profile`, `selected`, `selection_reason`, `manifest_fingerprint`
- `base_sha`, `head_sha`
- `metrics.base`, `metrics.head`
- `violations`, `ratchet_opportunity`, `verdict`
- `repro.command`
