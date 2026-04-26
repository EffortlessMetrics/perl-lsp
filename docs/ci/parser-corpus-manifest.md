# Parser corpus manifest (PR profile)

`cargo xtask parser-corpus manifest` discovers the parser-ratchet corpus for PR mode without installing CPAN.

## Command

```bash
cargo xtask parser-corpus manifest \
  --profile pr \
  --out target/parser-ratchet/corpus-manifest.json \
  --receipt target/receipts/parser-corpus-manifest.json
```

## Sources included

1. Repo-owned corpus (when present):
   - `tests/perl-corpus/**/*.pl`
   - `tests/perl-corpus/**/*.pm`
   - `tests/parser/**/*.pl`
   - `tests/parser/**/*.pm`
2. Ambient system Perl from `perl -MConfig` library roots:
   - `privlib`
   - `archlib`
   - `vendorlib`
   - `vendorarch`

Only readable `.pm`/`.pl` files are included.

## Determinism and fingerprinting

- Files are ordered stably by path then source.
- Each file entry includes `bytes` and `sha256`.
- The manifest `fingerprint` hashes schema/profile/runner info plus ordered file metadata.
- Base and candidate should run against the same runner image to get the same fingerprint for the same file set.

## Failure posture

- System Perl discovery failures are emitted as advisories for PR profile.
- Missing/unreadable system paths are handled gracefully and recorded in `sources`.
- Runtime manifest artifacts stay under `target/parser-ratchet/` and are not committed.
