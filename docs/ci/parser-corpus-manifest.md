# Parser corpus manifest (PR profile)

`cargo xtask parser-corpus manifest` discovers the parser ratchet input set for pull request mode.

## Command

```bash
cargo xtask parser-corpus manifest \
  --profile pr \
  --out target/parser-ratchet/corpus-manifest.json \
  --receipt target/receipts/parser-corpus-manifest.json
```

## Discovery policy

PR profile includes:

1. Repo-owned fixtures (if directories exist):
   - `tests/perl-corpus/**/*.pl`
   - `tests/perl-corpus/**/*.pm`
   - `tests/parser/**/*.pl`
   - `tests/parser/**/*.pm`
2. Ambient system Perl libraries from `perl -MConfig` paths (`privlib`, `archlib`, `vendorlib`, `vendorarch`).

PR profile does **not** install CPAN and does not require ad-hoc CPAN paths.

## Determinism

- Manifest entries are stably ordered by normalized path then source.
- Every file records byte size and SHA-256 digest.
- Manifest fingerprint is deterministic for the same discovered file set.
- Base and candidate comparisons should run against the same generated manifest fingerprint.

## Failure handling

- Missing/unreadable Perl-configured directories are advisory in PR profile.
- If system Perl discovery itself fails, PR profile emits advisory receipt data instead of hard-failing.
- Runtime manifests are generated under `target/parser-ratchet/` and should not be committed.
