# Linux System Corpus Refresh

**Date**: 2026-05-18
**Commit**: f52dd0065
**System**: WSL2 Linux
**Profile**: system

## Claim Boundary

This is a measurement-only refresh note. It records a fresh Linux system corpus
sweep, but it does not update `.ci/parser-corpus-baseline.json`, does not
regenerate generated parser status, and does not claim parser support
promotion. The sweep improved the headline clean count but failed the existing
per-bucket ratchet, so replacing the committed baseline would hide regressions.

## Command

```bash
cargo xtask parser-corpus-sweep --baseline .ci/parser-corpus-baseline.json --enforce --receipt
```

Run through WSL from the Windows worktree:

```bash
cd /mnt/h/pcorpus
cargo xtask parser-corpus-sweep --baseline .ci/parser-corpus-baseline.json --enforce --receipt
```

Result:

```text
exit code: 1
receipt: target/receipts/system-corpus-sweep.json
reason: ratchet enforcement failed
```

## Substrate Versions

| Component | Version |
| --- | --- |
| Rust | rustc 1.95.0 (59807616e 2026-04-14) |
| Cargo | cargo 1.95.0 (f2d3ce0bd 2026-03-21) |
| Perl | v5.38.2 |
| OS | WSL2 Linux 6.6.87.2-microsoft-standard-WSL2 |

## Headline Delta

| Metric | Committed baseline | Fresh WSL receipt | Delta |
| --- | ---: | ---: | ---: |
| total files | 7095 | 7095 | 0 |
| unreadable files | 48 | 48 | 0 |
| clean files | 6871 | 6935 | +64 |
| dirty files | 176 | 112 | -64 |
| structured-recovery-only files | 36 | 50 | +14 |
| files with ERROR nodes | 140 | 62 | -78 |
| catastrophic parse failures | 0 | 0 | 0 |
| total ERROR nodes | 536 | 228 | -308 |
| recovered nodes | 94 | 116 | +22 |
| recovery salvage rate | 20.5% | 44.6% | +24.1 pp |

## First-Error Buckets

| Bucket | Committed baseline | Fresh WSL receipt | Ratchet result |
| --- | ---: | ---: | --- |
| `unexpected_rparen_expr` | 8 | 18 | violation |
| `unexpected_token_in_expr` | 38 | 12 | improved |
| `unexpected_rbrace_expr` | 6 | 8 | violation |
| `expected_left_brace` | 6 | 6 | unchanged |
| `invalid_substitution_modifier` | 0 | 4 | new bucket |
| `unexpected_assign_expr` | 12 | 4 | improved |
| `unexpected_word_op_or` | 4 | 4 | unchanged |
| `expected_colon` | 4 | 2 | improved |
| `unclosed_brace` | 10 | 2 | improved |
| `unexpected_word_op_and` | 0 | 2 | new bucket |
| `unclosed_paren_identifier` | 22 | 0 | absent from fresh first-error buckets |

## Ratchet Violations

```text
bucket:unexpected_rbrace_expr baseline=6 current=8
bucket:unexpected_rparen_expr baseline=8 current=18
```

The existing ratchet did its job: the broad clean-rate improvement is visible,
but the per-bucket regressions remain blocking evidence. Do not promote the
baseline or generated parser status until those regressions are explained or
fixed.

## Routing Decision

The stale `unclosed_paren_identifier` note should no longer be used as a
current runtime-fix starting point without a focused failing fixture. The fresh
receipt routes the next parser capability investigation toward
`unexpected_rparen_expr`, with `unexpected_rbrace_expr` as the second blocking
bucket. Any follow-up parser PR should stay separate from this measurement note.

Valid follow-up shapes:

- `test(parser): lock source-backed unexpected_rparen_expr fixture`
- `fix(parser): repair one unexpected_rparen_expr boundary`
- `test(parser): lock source-backed unexpected_rbrace_expr fixture`

Invalid follow-up shapes:

- updating `.ci/parser-corpus-baseline.json` to the failed receipt
- claiming raw bucket movement in generated parser status
- combining this measurement with parser runtime behavior changes
- weakening per-bucket ratchet semantics

## Verification

```bash
cargo xtask parser-corpus-sweep --baseline .ci/parser-corpus-baseline.json --enforce --receipt
cargo xtask metrics parser-accuracy --check
```
