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

    // fallback for anything else
    return {
        kind: StartupErrorKind.Unknown,
        hint: 'The LSP binary failed to start. Check the Output panel for details.',
        remediation: 'Try "Run Health Check" to diagnose, or "Reinstall" to fetch a fresh binary.',
    };
}
