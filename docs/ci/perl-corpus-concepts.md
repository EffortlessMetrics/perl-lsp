# Perl corpus concept floors

`cargo xtask parser-ratchet concept-floors` enforces absolute parser behavior floors for
repo-owned fixtures under `tests/perl-corpus/`.

## Fixture format

Each `*.pl` fixture has a `*.meta.toml` sidecar:

```toml
concepts = ["regex", "interpolation", "capture"]
profile = ["pr"]
expected = "parse_clean"
```

## Supported expected values

- `parse_clean`
- `recover_without_panic`
- `allow_errors`
- `ast_shape` (reserved; requires snapshot integration)

## Required concept buckets

- `regex`
- `interpolation`
- `heredoc`
- `package`
- `subroutine`
- `lexical`
- `references`
- `hash_array_deref`
- `pod`
- `data_section`
- `recovery`

Missing required buckets fail the harness and are listed in the receipt.

## Command

```bash
cargo xtask parser-ratchet concept-floors \
  --manifest target/parser-ratchet/corpus-manifest.json \
  --receipt target/receipts/parser-concept-floor.json
```

## Receipt

Receipt schema: `.ci/receipts/schemas/parser-concept-floor.schema.json`.

The receipt records:

- `concepts_required`
- `concepts_hit`
- `missing_concepts`
- `violations`
