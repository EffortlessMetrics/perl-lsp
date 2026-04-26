# Perl corpus concept floors

`cargo xtask parser-ratchet concept-floors` enforces absolute parser behavior floors for repo-owned fixtures in `tests/perl-corpus/`.

## Command

```bash
cargo xtask parser-ratchet concept-floors \
  --manifest target/parser-ratchet/corpus-manifest.json \
  --receipt target/receipts/parser-concept-floor.json
```

## Fixture metadata

Each `*.pl` fixture has a sidecar `*.meta.toml`:

```toml
concepts = ["regex", "interpolation", "capture"]
profile = ["pr"]
expected = "parse_clean"
```

### Required concept buckets

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

### Expected values

- `parse_clean`
- `recover_without_panic`
- `allow_errors`
- `ast_shape` (checked when an adjacent `.ast.snap` exists)

## Gate rules

- Missing required concept bucket => harness failure.
- `parse_clean` failure => hard failure.
- `recover_without_panic` panic or timeout => hard failure.
- `allow_errors` may report parser diagnostics, but panic and timeout are failures.

## Receipt

The receipt records:

- `concepts_required`
- `concepts_hit`
- `missing_concepts`
- `violations`

Schema: `.ci/receipts/schemas/parser-concept-floor.schema.json`.
