# Perl corpus concept floors

`cargo xtask parser-ratchet concept-floors` enforces repo-owned Perl corpus concept floors.

## Command

```bash
cargo xtask parser-ratchet concept-floors \
  --manifest target/parser-ratchet/corpus-manifest.json \
  --receipt target/receipts/parser-concept-floor.json
```

## Fixture format

Each `tests/perl-corpus/**/*.pl` fixture has a sidecar `*.meta.toml`:

```toml
concepts = ["regex", "interpolation", "capture"]
profile = ["pr"]
expected = "parse_clean"
```

Supported `expected` values:

- `parse_clean`
- `recover_without_panic`
- `allow_errors`
- `ast_shape` (reserved when AST snapshots are present)

## Rules

- Missing required concept bucket fails the harness.
- `parse_clean` failures are hard failures.
- `recover_without_panic` panics are hard failures.
- `allow_errors` may report parser errors but must not panic.

## Receipt

The receipt includes:

- `concepts_required`
- `concepts_hit`
- `missing_concepts`
- `violations`
