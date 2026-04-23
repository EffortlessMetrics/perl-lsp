/**
 * Unit tests for HealthWidget.
 *
 * The widget is fully pure-TypeScript — it receives a mocked StatusBarItem
 * and we assert on text / tooltip / backgroundColor mutations.
 */

import { HealthWidget, ClientState } from '../healthWidget';
import { ThemeColor } from 'vscode';

// ---------------------------------------------------------------------------
// Minimal StatusBarItem stub
// ---------------------------------------------------------------------------

function makeStatusBarItem() {
    return {
        text: '',
        tooltip: '' as string | undefined,
        command: '',
        backgroundColor: undefined as ThemeColor | undefined,
        show: jest.fn(),
        hide: jest.fn(),
        dispose: jest.fn(),
    };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeWidget() {
    const item = makeStatusBarItem();
    const widget = new HealthWidget(item as any);
    return { item, widget };
}

// ---------------------------------------------------------------------------
// Initial state
// ---------------------------------------------------------------------------

describe('HealthWidget — initial state', () => {
    test('starts in "starting" mode with spinner text', () => {
        const { item, widget } = makeWidget();
        expect(widget.mode).toBe('starting');
        expect(item.text).toBe('$(sync~spin) Perl LSP');
        expect(item.backgroundColor).toBeUndefined();
    });
});

// ---------------------------------------------------------------------------
// State transitions via onStateChange
// ---------------------------------------------------------------------------

describe('HealthWidget — onStateChange', () => {
    test('Running → shows check icon without counts when no data provided', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        expect(widget.mode).toBe('running');
        expect(item.text).toBe('$(check) perl-lsp');
        expect(item.backgroundColor).toBeUndefined();
    });

    test('Running → shows file count when provided before state change', () => {
        const { item, widget } = makeWidget();
        widget.setFileCount(200);
        widget.onStateChange(ClientState.Running);
        expect(item.text).toBe('$(check) perl-lsp: 200 files');
    });

    test('Running → shows file and error counts', () => {
        const { item, widget } = makeWidget();
        widget.setFileCount(847);
        widget.setErrorCount(12);
        widget.onStateChange(ClientState.Running);
        expect(item.text).toBe('$(check) perl-lsp: 847 files | 12 errors');
    });

    test('Running → shows only error count when file count unknown', () => {
        const { item, widget } = makeWidget();
        widget.setErrorCount(3);
        widget.onStateChange(ClientState.Running);
        expect(item.text).toBe('$(check) perl-lsp: 3 errors');
    });

    test('Stopped → shows error icon and red background', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Stopped);
        expect(widget.mode).toBe('stopped');
        expect(item.text).toBe('$(error) perl-lsp: stopped');
        expect(item.backgroundColor).toBeInstanceOf(ThemeColor);
        expect((item.backgroundColor as ThemeColor).id).toBe(
            'statusBarItem.errorBackground'
        );
    });

    test('Starting → shows spinner', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        widget.onStateChange(ClientState.Starting);
        expect(widget.mode).toBe('starting');
        expect(item.text).toBe('$(sync~spin) Perl LSP');
    });
});

// ---------------------------------------------------------------------------
// $/progress notifications
// ---------------------------------------------------------------------------

describe('HealthWidget — $/progress', () => {
    test('begin → enters indexing mode with spinner', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        widget.onProgress('token-1', { kind: 'begin', title: 'Indexing', message: 'Scanning files' });
        expect(widget.mode).toBe('indexing');
        expect(item.text).toContain('$(sync~spin)');
        expect(item.text).toContain('Scanning files');
    });

    test('begin with only title uses title as message', () => {
        const { item, widget } = makeWidget();
        widget.onProgress('token-1', { kind: 'begin', title: 'Building index' });
        expect(item.text).toContain('Building index');
    });

    test('report updates message', () => {
        const { item, widget } = makeWidget();
        widget.onProgress('token-1', { kind: 'begin', title: 'Indexing', message: 'Step 1' });
        widget.onProgress('token-1', { kind: 'report', message: 'Step 2' });
        expect(item.text).toContain('Step 2');
    });

    test('report without message does not clear existing message', () => {
        const { item, widget } = makeWidget();
        widget.onProgress('token-1', { kind: 'begin', title: 'Indexing', message: 'Step 1' });
        widget.onProgress('token-1', { kind: 'report', percentage: 50 });
        expect(item.text).toContain('Step 1');
    });

    test('end with single token → returns to running', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        widget.onProgress('token-1', { kind: 'begin', title: 'Indexing' });
        widget.onProgress('token-1', { kind: 'end' });
        expect(widget.mode).toBe('running');
        expect(item.text).toBe('$(check) perl-lsp');
    });

    test('overlapping tokens stay in indexing until all end', () => {
        const { item, widget } = makeWidget();
        widget.onProgress('token-1', { kind: 'begin', title: 'Task A' });
        widget.onProgress('token-2', { kind: 'begin', title: 'Task B' });
        widget.onProgress('token-1', { kind: 'end' });
        expect(widget.mode).toBe('indexing');
        widget.onProgress('token-2', { kind: 'end' });
        expect(widget.mode).toBe('running');
    });

    test('end for unknown token is a no-op', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        widget.onProgress('ghost-token', { kind: 'end' });
        expect(widget.mode).toBe('running');
    });

    test('report for unknown token is a no-op', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        const before = item.text;
        widget.onProgress('ghost-token', { kind: 'report', message: 'ignored' });
        expect(item.text).toBe(before);
    });

    test('Stopped clears all active tokens', () => {
        const { widget } = makeWidget();
        widget.onProgress('token-1', { kind: 'begin', title: 'Indexing' });
        widget.onStateChange(ClientState.Stopped);
        expect(widget.mode).toBe('stopped');
        // After restart, a Running state should render cleanly.
        widget.onStateChange(ClientState.Running);
        expect(widget.mode).toBe('running');
    });

    test('Running during active tokens stays indexing', () => {
        const { widget } = makeWidget();
        widget.onProgress('token-1', { kind: 'begin', title: 'Indexing' });
        widget.onStateChange(ClientState.Running);
        expect(widget.mode).toBe('indexing');
    });
});

// ---------------------------------------------------------------------------
// Counts
// ---------------------------------------------------------------------------

describe('HealthWidget — counts', () => {
    test('setFileCount updates display immediately in running mode', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        widget.setFileCount(500);
        expect(item.text).toBe('$(check) perl-lsp: 500 files');
    });

    test('setErrorCount to zero omits error from display', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        widget.setErrorCount(5);
        widget.setErrorCount(0);
        expect(item.text).toBe('$(check) perl-lsp');
    });

    test('singular "error" for exactly 1 error', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        widget.setErrorCount(1);
        expect(item.text).toContain('1 error');
        expect(item.text).not.toContain('errors');
    });

    test('plural "errors" for 2+ errors', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        widget.setErrorCount(2);
        expect(item.text).toContain('2 errors');
    });

    test('counts do not affect stopped display', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Stopped);
        widget.setFileCount(100);
        widget.setErrorCount(5);
        expect(item.text).toBe('$(error) perl-lsp: stopped');
    });

    test('counts do not affect indexing display', () => {
        const { item, widget } = makeWidget();
        widget.onProgress('t', { kind: 'begin', title: 'Indexing' });
        widget.setFileCount(100);
        expect(item.text).not.toContain('100 files');
    });

    test('fileCount accessor returns current value', () => {
        const { widget } = makeWidget();
        expect(widget.fileCount).toBeUndefined();
        widget.setFileCount(42);
        expect(widget.fileCount).toBe(42);
    });

    test('errorCount accessor returns current value', () => {
        const { widget } = makeWidget();
        expect(widget.errorCount).toBe(0);
        widget.setErrorCount(7);
        expect(widget.errorCount).toBe(7);
    });
});

// ---------------------------------------------------------------------------
// Version display (issue #2340)
// ---------------------------------------------------------------------------

describe('HealthWidget — version display', () => {
    test('version accessor is undefined before setVersion', () => {
        const { widget } = makeWidget();
        expect(widget.version).toBeUndefined();
    });

    test('setVersion updates the accessor', () => {
        const { widget } = makeWidget();
        widget.setVersion('0.12.0');
        expect(widget.version).toBe('0.12.0');
    });

    test('running state without version shows plain "perl-lsp"', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        expect(item.text).toBe('$(check) perl-lsp');
    });

    test('running state with version shows "perl-lsp v{version}"', () => {
        const { item, widget } = makeWidget();
        widget.setVersion('0.12.0');
        widget.onStateChange(ClientState.Running);
        expect(item.text).toBe('$(check) perl-lsp v0.12.0');
    });

    test('setVersion while running updates text immediately', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        widget.setVersion('0.13.0');
        expect(item.text).toBe('$(check) perl-lsp v0.13.0');
    });

    test('version is shown with file count', () => {
        const { item, widget } = makeWidget();
        widget.setVersion('0.12.0');
        widget.setFileCount(100);
        widget.onStateChange(ClientState.Running);
        expect(item.text).toBe('$(check) perl-lsp v0.12.0: 100 files');
    });

    test('version is shown with error count', () => {
        const { item, widget } = makeWidget();
        widget.setVersion('0.12.0');
        widget.setErrorCount(3);
        widget.onStateChange(ClientState.Running);
        expect(item.text).toBe('$(check) perl-lsp v0.12.0: 3 errors');
    });

    test('tooltip includes version when set', () => {
        const { item, widget } = makeWidget();
        widget.setVersion('0.12.0');
        widget.onStateChange(ClientState.Running);
        expect(item.tooltip).toContain('v0.12.0');
    });

    test('tooltip does not include version string when not set', () => {
        const { item, widget } = makeWidget();
        widget.onStateChange(ClientState.Running);
        // The tooltip should not contain a version like "v0.12.0"
        expect(item.tooltip).not.toMatch(/v\d+\.\d+/);
    });

    test('version does not affect stopped display', () => {
        const { item, widget } = makeWidget();
        widget.setVersion('0.12.0');
        widget.onStateChange(ClientState.Stopped);
        expect(item.text).toBe('$(error) perl-lsp: stopped');
    });

    test('version does not affect starting display', () => {
        const { item, widget } = makeWidget();
        widget.setVersion('0.12.0');
        // widget is already in starting mode
        expect(item.text).toBe('$(sync~spin) Perl LSP');
    });

    test('version does not affect indexing display', () => {
        const { item, widget } = makeWidget();
        widget.setVersion('0.12.0');
        widget.onProgress('t', { kind: 'begin', title: 'Indexing' });
        expect(item.text).not.toContain('v0.12.0');
    });

    test('after progress ends, running shows version again', () => {
        const { item, widget } = makeWidget();
        widget.setVersion('0.12.0');
        widget.onStateChange(ClientState.Running);
        widget.onProgress('t', { kind: 'begin', title: 'Indexing' });
        widget.onProgress('t', { kind: 'end' });
        expect(widget.mode).toBe('running');
        expect(item.text).toBe('$(check) perl-lsp v0.12.0');
    });
});
