/**
 * Startup error diagnosis for the Perl LSP binary (#3280).
 *
 * Pure functions — no vscode or child_process imports — so tests can run
 * without a full extension host.
 */

/** Categories of binary startup failure with actionable remediation. */
export const enum StartupErrorKind {
    GlibcMismatch = 'GlibcMismatch',
    MissingSharedLibrary = 'MissingSharedLibrary',
    ExecFormatError = 'ExecFormatError',
    PermissionDenied = 'PermissionDenied',
    WindowsBinaryError = 'WindowsBinaryError',
    MacOsDylibError = 'MacOsDylibError',
    Unknown = 'Unknown',
}

export interface StartupErrorDiagnosis {
    kind: StartupErrorKind;
    /** Short user-visible hint (≤200 chars), fit for a VS Code notification. */
    hint: string;
    /** Concrete remediation step. */
    remediation: string;
}

/**
 * Classify a binary startup failure from its stderr/stdout output.
 *
 * Pure function — no I/O, safe to test without a real binary.
 */
export function classifyStartupError(output: string): StartupErrorDiagnosis {
    // glibc version mismatch — common on Alpine/musl or older distros
    const glibcMatch = output.match(/version [`']?(GLIBC_[\d.]+)[`']?\s+not found/i);
    if (glibcMatch) {
        const version = glibcMatch[1];
        return {
            kind: StartupErrorKind.GlibcMismatch,
            hint: `glibc version mismatch: binary requires ${version} but your system has an older version. This typically happens on Alpine/musl or older Linux distros.`,
            remediation: 'Install from source with: cargo install perl-lsp-rs\nOr set perl-lsp.serverPath to a locally-built binary.',
        };
    }

    // missing shared library — libssl, libgcc_s, etc.
    const missingLibMatch = output.match(
        /(?:error while loading shared libraries|cannot open shared object file)[:\s]+([^\s:]+\.so[\d.]*)(?:\s|:|$)/i
    );
    if (missingLibMatch) {
        const lib = missingLibMatch[1];
        return {
            kind: StartupErrorKind.MissingSharedLibrary,
            hint: `Missing shared library: ${lib}. The pre-built binary depends on system libraries not present on your machine.`,
            remediation: `Install the missing library (e.g. apt install ${lib.replace(/\.so.*/, '')} or equivalent), or install from source: cargo install perl-lsp-rs`,
        };
    }

    // wrong architecture / exec format error
    if (/Exec format error/i.test(output) || /cannot execute binary file/i.test(output)) {
        return {
            kind: StartupErrorKind.ExecFormatError,
            hint: 'Architecture mismatch: the pre-built binary is for a different CPU architecture than your system.',
            remediation: 'Reinstall the extension to get a binary for your architecture, or build from source: cargo install perl-lsp-rs',
        };
    }

    // permission denied
    if (/[Pp]ermission denied/i.test(output)) {
        return {
            kind: StartupErrorKind.PermissionDenied,
            hint: 'The binary does not have execute permission.',
            remediation: 'Fix with: chmod +x <path-to-perllsp>\nOr check that your filesystem allows execute permissions.',
        };
    }

    // Windows: DLL load failure or wrong PE architecture
    if (
        /DLL initialization routine failed/i.test(output) ||
        /not a valid Win32 application/i.test(output) ||
        /The specified module could not be found/i.test(output) ||
        /0xc000007b/i.test(output)
    ) {
        return {
            kind: StartupErrorKind.WindowsBinaryError,
            hint: 'Windows could not load the LSP binary. The binary may be corrupt, for a different Windows architecture (x86 vs x64), or missing a required DLL.',
            remediation: 'Reinstall the extension to get a matching binary, or build from source: cargo install perl-lsp-rs',
        };
    }

    // macOS: dylib load failure or code signature issue
    if (
        /dyld[: ]+Library not loaded/i.test(output) ||
        /code signature invalid/i.test(output) ||
        /dyld[: ]+could not load/i.test(output)
    ) {
        return {
            kind: StartupErrorKind.MacOsDylibError,
            hint: 'macOS could not load the LSP binary. A required dylib is missing, or macOS Gatekeeper blocked execution due to a code signature issue.',
            remediation: 'If Gatekeeper blocked the binary, run: xattr -d com.apple.quarantine <path-to-perllsp>\nOr reinstall the extension to fetch a fresh signed binary.',
        };
    }

    // fallback for anything else
    return {
        kind: StartupErrorKind.Unknown,
        hint: 'The LSP binary failed to start. Check the Output panel for details.',
        remediation: 'Try "Run Health Check" to diagnose, or "Reinstall" to fetch a fresh binary.',
    };
}

/**
 * Choose the most informative startup diagnosis to surface to the user.
 *
 * Strategy (#3329): OS-level probe results are preferred when they classify a
 * specific error (glibc mismatch, wrong arch, permission error, etc.).  When
 * the probe returns `Unknown` — meaning execFile gave no useful output — fall
 * back to the health-check string from `runStartupDiagnostics`, which can
 * detect environment issues like a missing Perl interpreter.
 *
 * @param probe      Result of `probeStartupFailure`.
 * @param healthMsg  Result of `onboardingManager.runStartupDiagnostics`, or
 *                   `undefined` if that call was skipped / not yet awaited.
 * @returns          The best available diagnosis, with `hint` and `remediation`
 *                   ready for display.
 */
export function selectBestDiagnosis(
    probe: StartupErrorDiagnosis,
    healthMsg: string | undefined,
): StartupErrorDiagnosis {
    if (probe.kind !== StartupErrorKind.Unknown) {
        // Probe classified the error specifically — trust it.
        return probe;
    }
    if (!healthMsg) {
        // Nothing better available — return the probe as-is.
        return probe;
    }
    // Probe was inconclusive; promote the health-check string as the hint so
    // the user sees "Perl interpreter not found" instead of the generic message.
    return {
        kind: StartupErrorKind.Unknown,
        hint: healthMsg,
        remediation: probe.remediation,
    };
}

/**
 * Format the startup failure dialog shown to the user.
 *
 * When the health-check fallback returns a specific onboarding message, we
 * surface that verbatim so the user sees the actionable Perl-missing guidance
 * immediately instead of a generic wrapper.
 */
export function formatStartupFailureDialog(
    probe: StartupErrorDiagnosis,
    healthMsg: string | undefined,
): string {
    if (probe.kind === StartupErrorKind.Unknown && healthMsg) {
        return healthMsg;
    }

    return (
        `Perl Language Server failed to start.\n\n${probe.hint}\n\n` +
        `Suggestion: ${probe.remediation}`
    );
}
