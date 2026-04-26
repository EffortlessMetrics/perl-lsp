# Parser Corpus Manifest (PR profile)

`cargo xtask parser-corpus manifest` discovers the parser ratchet corpus used in PR mode.

## Command

```bash
cargo xtask parser-corpus manifest \
  --profile pr \
  --out target/parser-ratchet/corpus-manifest.json \
  --receipt target/receipts/parser-corpus-manifest.json
```

## Discovery rules

PR profile includes two source families:

1. Repo-owned corpus files (when present):
   - `tests/perl-corpus/**/*.pl`
   - `tests/perl-corpus/**/*.pm`
   - `tests/parser/**/*.pl`
   - `tests/parser/**/*.pm`
2. Ambient system Perl from `perl -MConfig` roots:
   - `privlib`
   - `archlib`
   - `vendorlib`
   - `vendorarch`

Only readable `.pm` / `.pl` files are included. No CPAN install step is required.

## Determinism contract

- Stable file ordering by `path`, then `source`, then `sha256`.
- Per-file SHA-256 and byte-size are included in the manifest.
- `fingerprint` is deterministically computed from schema/profile/runner/file tuples.
- Base and candidate comparisons must use the exact same manifest fingerprint.

## Failure policy

- PR profile treats system Perl discovery failure as `advisory` infrastructure metadata.
- Profiles that explicitly require system Perl can fail hard.

Generated manifests are runtime artifacts and should remain under `target/parser-ratchet/`.
