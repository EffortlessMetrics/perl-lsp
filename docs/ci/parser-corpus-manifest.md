# Parser corpus manifest (PR profile)

`cargo xtask parser-corpus manifest` discovers a deterministic parser corpus input set for parser-ratchet checks.

## Command

```bash
cargo xtask parser-corpus manifest \
  --profile pr \
  --out target/parser-ratchet/corpus-manifest.json \
  --receipt target/receipts/parser-corpus-manifest.json
```

## Discovery sources

PR profile uses two source families:

1. **Repo-owned corpus roots** (only when paths exist):
   - `tests/perl-corpus/**/*.pl`
   - `tests/perl-corpus/**/*.pm`
   - `tests/parser/**/*.pl`
   - `tests/parser/**/*.pm`
2. **Ambient system Perl** roots from `Config` (`privlib`, `archlib`, `vendorlib`, `vendorarch`).

This mode intentionally avoids CPAN installation and does not use a committed runtime corpus list.

## Guarantees

- Stable file ordering.
- Deterministic `fingerprint` from profile + runner metadata + file tuples.
- Manifest emitted under `target/parser-ratchet/`.
- Base and candidate comparisons can use the exact same manifest fingerprint when the discovered file set is identical.

## Failure policy

- In `--profile pr`, system Perl discovery failures are emitted as **advisory infra** in the receipt.
- In `--profile system-required`, system Perl discovery is a hard error.
- Missing/unreadable roots are recorded in `sources` with status metadata.
