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
            expect.stringContaining('perltidy is not installed'),
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
        // The truncated content portion should not exceed 120 chars
        const prefix = 'Perl formatting failed: ';
        const content = call[0].slice(prefix.length);
        expect(content.length).toBeLessThanOrEqual(120);
    });
});
