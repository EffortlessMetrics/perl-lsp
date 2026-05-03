# 2026-05-03 — Windows Defender Source-Side EBUSY

**Lens**: `EBUSY` on `fs.copyFileSync` is usually assumed to be destination-side. It can also be source-side, and the source-side case is invisible on hosted CI.

## What we observed

When the VS Code extension extracts a release archive into `%TEMP%` and then copies the binary into `globalStorage/...`, `fs.copyFileSync` can fail with:

```
EBUSY: resource busy or locked, copyfile
  'C:\...\Temp\perl-lsp-XXXXXX\extracted\perllsp-0.13.1-...\perllsp.exe'
  -> 'C:\...\globalStorage\effortlessmetrics.perl-lsp-rs\bin\win32-x64\perllsp.exe'
```

The lock is on the **source** path — the freshly extracted `perllsp.exe` in `%TEMP%`. The destination doesn't exist yet on a clean install. The first install fails before any prior version exists.

## Cause

Windows Defender (and other AV) scans new executables on file creation. While the scan runs, the file is opened with restricted sharing — `fs.copyFileSync` (which calls `CopyFileExW` underneath) tries to open the source for read and gets denied. Node surfaces this as `EBUSY`.

The scan window is variable:

- **Warm cache** (binary signature already classified): milliseconds.
- **Cold cache** (first time Defender sees this binary): 5-15 seconds typical, 30+ seconds occasionally.

Hosted GitHub Actions runners (`windows-latest`) have aggressive Defender exclusions on temp dirs and pre-warmed signature caches. Real user machines don't. This is why the failure is invisible on CI but reproduces consistently on a real Windows 11 + Defender machine.

## What was assumed

The original retry-on-`EBUSY` hardening (`#7862`) was sized for the *destination*-side lock case (a running `perllsp.exe` being overwritten). The retry budget was `[100, 250, 500, 1000, 2000]ms`, ~4 seconds total. That's not enough to outlast a cold Defender first-time scan.

## What's true

Both source-side and destination-side locks are real, and they need different fixes:

- **Source-side lock**: extend the retry budget to cover Defender's scan window. Total wait should be at least 30 seconds.
- **Destination-side lock**: don't try to overwrite a running binary; install to a fresh path (versioned managed install dirs).

The combined fix that shipped in v0.13.3 addresses both. See `2026-05-03-v0.13.3-windows-install-dual-lock.md` for the full incident.

## Detection signals

When triaging an `EBUSY` from `fs.copyFileSync` on Windows, check the error message — Node includes both source and destination paths. If the lock is on the *source* path (not the destination), this is the AV-scan case, and the right fix is retry-budget length, not destination-path strategy.

If both paths exist and the destination is a known-running binary location, both failure modes may be in play.

## Lock the regression

The retry budget is unit-tested in `vscode-extension/src/test/downloader.test.ts`:

```typescript
test('long-tail retry budget allows up to 8 transient failures before succeeding', ...)
test('all four transient codes are retried (EBUSY/EPERM/EACCES/ETXTBSY)', ...)
```

These tests don't depend on Defender — they mock `copyFileSync` to throw `EBUSY` for the first 8 calls and assert the budget tolerates it. The mock is deterministic; the test runs in milliseconds; the regression is locked independently of any environmental factor.

## Related

- Forensics: `2026-05-03-v0.13.3-windows-install-dual-lock.md`
- Articles: `../articles/CI_VS_REAL_USER_PARITY.md` (broader hosted-CI vs. real-user calibration)
- Constant in code: `MANAGED_INSTALL_RETRY_DELAYS_MS` in `vscode-extension/src/downloader.ts`
