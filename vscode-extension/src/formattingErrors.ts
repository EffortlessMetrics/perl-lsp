/**
 * Formatting error feedback helpers (issue #2111).
 *
 * Extracted into a standalone module so the pure notification logic is
 * unit-testable without pulling in the vscode-languageclient runtime,
 * which requires the full VS Code host to initialise.
 */

import * as vscode from 'vscode';

let lastFormatErrorTime = 0;
const FORMAT_ERROR_COOLDOWN_MS = 30_000;

/**
 * Reset the format-error cooldown timer. Exported for test isolation.
 */
export function resetFormatErrorCooldown(): void {
    lastFormatErrorTime = 0;
}

/**
 * Show a formatting error notification if outside the cooldown window.
 * Returns true if the notification was shown.
 */
export function handleFormattingError(
    message: string,
    outputCh: vscode.OutputChannel,
): boolean {
    const now = Date.now();
    if (now - lastFormatErrorTime < FORMAT_ERROR_COOLDOWN_MS) {
        return false; // suppress: still in cooldown
    }
    lastFormatErrorTime = now;

    const firstLine = (message.split('\n').find(l => l.trim().length > 0) ?? message).trimEnd();
    const truncated = firstLine.length > 120 ? firstLine.slice(0, 117) + '...' : firstLine;

    const isNotFound = message.includes('perltidy not found');
    const label = isNotFound ? 'Run Health Check' : 'Show Output';
    const msg = isNotFound
        ? `Perl formatting requires perltidy, which was not found on PATH. ` +
          `Install it via: cpan Perl::Tidy  (or set perl-lsp.perltidyConfig to your config path)`
        : `Perl formatting failed: ${truncated}`;

    vscode.window.showErrorMessage(msg, label).then(sel => {
        if (sel === 'Show Output') { outputCh.show(); }
        if (sel === 'Run Health Check') {
            void vscode.commands.executeCommand('perl-lsp.runHealthCheck');
        }
    });
    return true;
}
