# Parser Ratchet Comparator (PR Mode)

`cargo xtask parser-ratchet` now compares **live base vs candidate** metrics using one manifest.

## Commands

- `cargo xtask parser-ratchet --profile pr --base <sha> --head <sha> --manifest target/parser-ratchet/corpus-manifest.json --receipt target/receipts/parser-ratchet.json`
- `cargo xtask parser-ratchet compare --base-metrics <json> --head-metrics <json> --receipt <json>`

## Rules

### `perl-corpus`

- `panic_count == 0`
- `timeout_count == 0`
- concept floors must pass (when available)
- `clean_parse_rate` may not regress beyond epsilon
- `error_node_count` may not materially increase
- `node_kind_seen_count` may not unexpectedly drop

### `system-perl`

Differential-only:

- fail on new/worsened `panic_count`
- fail on new/worsened `timeout_count`
- fail on `clean_parse_rate` regression beyond epsilon
- unchanged existing base failures do not block

### runtime

`corpus_runtime_ms` is advisory: runtime-only regression emits a warning and still passes.

## Receipt fields

- `check`, `profile`, `selected`, `selection_reason`
- `manifest_fingerprint`
- `base_sha`, `head_sha`, `candidate_sha`
- `metrics.base`, `metrics.head`
- `violations`, `ratchet_opportunity`, `verdict`
- `repro.command`

## Validation matrix

- equal metrics -> pass
- improvement -> pass with `ratchet_opportunity=true`
- perl-corpus panic in head -> fail
- system Perl existing base failure unchanged -> pass
- system Perl worsened failure -> fail
- runtime-only regression -> warn/pass
