/**
 * Unit tests for formatting error feedback (issue #2111).
 *
 * Tests the handleFormattingError helper which surfaces LSP formatting
 * errors as VS Code toast notifications with debouncing.
 *
 * handleFormattingError is exported for direct unit testability without
 * requiring the full extension activation path.
 */

import * as vscode from 'vscode';
import { handleFormattingError, resetFormatErrorCooldown } from '../formattingErrors';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeOutputChannel(): { show: jest.Mock; appendLine: jest.Mock } {
    return { show: jest.fn(), appendLine: jest.fn() };
}

// ---------------------------------------------------------------------------
// handleFormattingError
// ---------------------------------------------------------------------------

describe('handleFormattingError', () => {
    beforeEach(() => {
        jest.useFakeTimers();
        resetFormatErrorCooldown();
        (vscode.window.showErrorMessage as jest.Mock).mockClear();
        (vscode.commands.executeCommand as jest.Mock).mockClear();
    });

    afterEach(() => {
        jest.useRealTimers();
    });

    test('shows toast for perltidy syntax error', () => {
        const ch = makeOutputChannel();
        handleFormattingError('perltidy error: syntax error at line 5', ch as any);
        expect(vscode.window.showErrorMessage).toHaveBeenCalledWith(
            expect.stringContaining('Perl formatting failed:'),
            'Show Output'
        );
    });

    test('shows Run Health Check button when perltidy is not found', () => {
        const ch = makeOutputChannel();
        handleFormattingError('perltidy not found: /usr/bin/perltidy', ch as any);
        expect(vscode.window.showErrorMessage).toHaveBeenCalledWith(
            expect.stringContaining('perltidy, which was not found on PATH'),
            'Run Health Check'
        );
    });

    test('does not show toast again within 30s cooldown', () => {
        const ch = makeOutputChannel();
        handleFormattingError('perltidy error: line 1', ch as any);
        handleFormattingError('perltidy error: line 2', ch as any);
        expect(vscode.window.showErrorMessage).toHaveBeenCalledTimes(1);
    });

    test('shows toast again after 30s cooldown expires', () => {
        const ch = makeOutputChannel();
        handleFormattingError('perltidy error: line 1', ch as any);
        jest.advanceTimersByTime(31_000);
        handleFormattingError('perltidy error: line 2', ch as any);
        expect(vscode.window.showErrorMessage).toHaveBeenCalledTimes(2);
    });

    test('truncates multi-line perltidy error to first non-empty line', () => {
        const ch = makeOutputChannel();
        handleFormattingError('line one\nline two\nline three', ch as any);
        const call = (vscode.window.showErrorMessage as jest.Mock).mock.calls[0];
        expect(call[0]).toContain('line one');
        expect(call[0]).not.toContain('line two');
    });

    test('truncates very long single-line error to 120 chars with ellipsis', () => {
        const ch = makeOutputChannel();
        const longMsg = 'x'.repeat(200);
        handleFormattingError(longMsg, ch as any);
        const call = (vscode.window.showErrorMessage as jest.Mock).mock.calls[0];
        // The toast message contains "Perl formatting failed: " prefix plus truncated content
        expect(call[0]).toContain('...');
        // The truncated portion (firstLine capped at 120 chars) should not exceed 120 chars.
        // The implementation truncates the raw error line to 120 chars before composing the message.
        const prefix = 'Perl formatting failed: ';
        const content = call[0].slice(prefix.length);
        expect(content.length).toBeLessThanOrEqual(120);
    });

    test('returns true when notification is shown', () => {
        const ch = makeOutputChannel();
        const shown = handleFormattingError('perltidy error: syntax error', ch as any);
        expect(shown).toBe(true);
    });

    test('returns false when suppressed by cooldown', () => {
        const ch = makeOutputChannel();
        handleFormattingError('perltidy error: line 1', ch as any);
        const shown = handleFormattingError('perltidy error: line 2', ch as any);
        expect(shown).toBe(false);
    });

    test('Show Output button calls outputCh.show()', async () => {
        const ch = makeOutputChannel();
        (vscode.window.showErrorMessage as jest.Mock).mockResolvedValueOnce('Show Output');
        handleFormattingError('perltidy error: syntax error', ch as any);
        // Flush the .then() microtask on the showErrorMessage promise
        await Promise.resolve();
        expect(ch.show).toHaveBeenCalled();
    });

    test('Run Health Check button calls perl-lsp.runHealthCheck command', async () => {
        const ch = makeOutputChannel();
        (vscode.window.showErrorMessage as jest.Mock).mockResolvedValueOnce('Run Health Check');
        handleFormattingError('perltidy not found: /usr/bin/perltidy', ch as any);
        await Promise.resolve();
        expect(vscode.commands.executeCommand).toHaveBeenCalledWith('perl-lsp.runHealthCheck');
    });

    test('skips leading empty lines when extracting first line', () => {
        const ch = makeOutputChannel();
        handleFormattingError('\n\n  \nactual error line\nmore info', ch as any);
        const call = (vscode.window.showErrorMessage as jest.Mock).mock.calls[0];
        expect(call[0]).toContain('actual error line');
        expect(call[0]).not.toContain('more info');
    });

    test('strips trailing carriage return from CRLF error messages', () => {
        const ch = makeOutputChannel();
        handleFormattingError('\r\nperltidy error: line 5\r\nmore info', ch as any);
        const call = (vscode.window.showErrorMessage as jest.Mock).mock.calls[0];
        // The toast message should not contain a bare \r character
        expect(call[0]).not.toContain('\r');
        expect(call[0]).toContain('perltidy error: line 5');
    });
});
