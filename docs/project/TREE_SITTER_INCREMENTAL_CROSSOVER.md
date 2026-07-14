# Tree-Sitter Incremental Crossover Evidence

This document records the first broader measurement pass for the token-replay
contract exposed by `tree-sitter-perl-rs`. It is evidence for routing design,
not a universal performance promise and not evidence for AST-subtree reuse.

## Receipt identity

All three profiles were run from commit `085c09bb8c8b9950264d9e8322ca263228860daf`
with `rustc 1.95.0 (59807616e 2026-04-14)`:

```text
rtk cargo xtask tree-sitter-incremental-proof --profile pr
rtk cargo xtask tree-sitter-incremental-proof --profile nightly
rtk cargo xtask tree-sitter-incremental-proof --profile release
```

The machine-readable receipts are written to
`target/receipts/tree-sitter-incremental-proof-{pr,nightly,release}.json` and
are identified by the profile-specific `input_hash` recorded in each receipt.

```text
pr      fa6523905635d4a16a21f59b73f2c66e76330c742460153029a653ec346e659c
nightly 8cd53b407f0b3bf964af2d1b7f56160690d1fda22b8365cb1ebb15fccaab055b
release 3086329efa38a0a94e160590e320e34a295f47a226223727d33de2afbc7e7052
```

| Profile | Rows | Operations | Fallback | Equivalence failures | Replay p95 faster |
| --- | ---: | ---: | ---: | ---: | ---: |
| PR | 54 | 270 | 29.3% | 0 | 6 (11.1%) |
| Nightly | 72 | 1,080 | 29.7% | 0 | 14 (19.4%) |
| Release | 90 | 2,250 | 29.9% | 0 | 14 (15.6%) |

The measurements establish mechanical fresh-equivalence for this matrix. They
do not establish that replay is faster in general; p95 results are from one
machine and vary between runs.

## Release-profile crossover map

The release profile groups rows by document size. `Ratio` is the median of
`replay_p95 / fresh_p95`; values below 100% favor replay. Work columns are
averages over the rows in each band.

| Size band | Rows | Faster rows | Faster rate | Ratio | Fallback | Reprocessed bytes | Tokens reused | Tokens re-lexed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Small (<=1.2 KB) | 46 | 3 | 7% | 148% | 33% | 238 | 52 | 57 |
| Medium (1.2-10 KB) | 22 | 3 | 14% | 160% | 41% | 2,234 | 572 | 565 |
| Large (10-50 KB) | 18 | 5 | 28% | 108% | 14% | 5,183 | 2,944 | 479 |
| Very large (>50 KB) | 4 | 3 | 75% | 80% | 12% | 41,918 | 6,512 | 1,149 |

The current evidence suggests a provisional shape:

- small and medium documents should remain on the ordinary fresh path;
- large documents are mixed and need repeated measurements by edit class;
- very-large documents show the clearest replay benefit, but the sample is only
  four rows;
- recovery, incomplete, quote-like, heredoc, and other context-sensitive edits
  retain substantial fallback behavior and must remain fail-closed.

The data is not sufficient to hard-code a byte threshold or enable replay
globally. The next measurement pass must repeat each matrix cell and report
confidence or run dispersion, then add phase timing for edit validation,
checkpoint selection, re-lexing, token assembly, parser execution, AST
reconstruction, and cache rebuilding.

## AST-reuse decision

AST-subtree reuse remains deferred. Token replay is fresh-equivalent and shows
benefit in the largest measured band, but the current receipt does not measure
phase costs or prove that AST reconstruction is the dominant remaining cost.
Nonzero token reuse is therefore evidence to continue measurement, not a reason
to retain AST subtrees or change replay routing yet.
