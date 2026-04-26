# Perl corpus concept floors

`parser-ratchet concept-floors` enforces repo-owned fixture floors for `tests/perl-corpus`.

## Metadata sidecars

Each fixture has a `.meta.toml` sidecar:

```toml
concepts = ["regex", "interpolation", "capture"]
profile = ["pr"]
expected = "parse_clean"
```

Supported `expected` values:

- `parse_clean`
- `recover_without_panic`
- `allow_errors`
- `ast_shape` (parse must succeed; shape assertions can be layered later)

## Required concept buckets

PR profile currently requires all of these buckets to be represented:

- regex
- interpolation
- heredoc
- package
- subroutine
- lexical
- references
- hash_array_deref
- pod
- data_section
- recovery

## Command

```bash
cargo xtask parser-ratchet concept-floors \
  --manifest target/parser-ratchet/corpus-manifest.json \
  --receipt target/receipts/parser-concept-floor.json
```

Harness behavior:

- missing required concept bucket => failure
- `parse_clean` fixture with parser errors or catastrophic parse => failure
- `recover_without_panic` fixture panic => failure
- `allow_errors` may include parser errors, but panic => failure
- receipt captures `concepts_required`, `concepts_hit`, `missing_concepts`, and `violations`
