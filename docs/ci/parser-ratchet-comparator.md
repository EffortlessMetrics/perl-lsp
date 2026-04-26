# Parser Ratchet Comparator (PR mode)

`cargo xtask parser-ratchet` in PR/merge-group mode compares **live base-vs-candidate** metrics using the **same manifest fingerprint**.

## Commands

```bash
cargo xtask parser-ratchet --profile pr --base <sha> --head <sha> --manifest target/parser-ratchet/corpus-manifest.json --receipt target/receipts/parser-ratchet.json
cargo xtask parser-ratchet compare --profile pr --selected perl-corpus --base-metrics <json> --head-metrics <json> --receipt <json>
```

## Rules

- `perl-corpus`
  - `panic_count == 0`
  - `timeout_count == 0`
  - concept floors pass
  - `clean_parse_rate` must not regress beyond epsilon
  - `error_node_count` must not materially increase
  - `node_kind_seen_count` must not drop unexpectedly
- `system-perl` (differential only)
  - fail on new/worsened `panic_count`
  - fail on new/worsened `timeout_count`
  - fail on `clean_parse_rate` regression beyond epsilon
  - unchanged existing base failures do **not** block unrelated PRs
- `corpus_runtime_ms` is advisory (warn/pass)

## Notes

- No committed PR baseline file is used.
- No automatic baseline updates are performed.
- Base and candidate must use the same manifest.
- Current implementation includes explicit integration hooks for live base/head metric acquisition.
