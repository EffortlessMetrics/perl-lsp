/**
 * POD Preview Panel (issue #2062)
 *
 * Provides a VSCode webview panel that renders POD documentation from the
 * currently active Perl file as HTML. Auto-updates on file save.
 *
 * The POD-to-HTML conversion is implemented in pure TypeScript so the feature
 * has zero runtime dependencies (no Perl executable required).
 */

import * as vscode from 'vscode';

// ---------------------------------------------------------------------------
// POD → HTML conversion
// ---------------------------------------------------------------------------

/**
 * Convert a POD source string (or a Perl source file containing POD sections)
 * to an HTML fragment. Non-POD lines are skipped. Returns a complete HTML
 * document body, not a full <html> skeleton — the caller wraps it.
 */
export function podToHtml(source: string): string {
    const podLines = extractPodLines(source);
    if (podLines.length === 0) {
        return '<p class="no-pod">No POD documentation found in this file.</p>';
    }

    const blocks = parseBlocks(podLines);
    return renderBlocks(blocks);
}

// ---------------------------------------------------------------------------
// Step 1: extract POD lines from mixed Perl+POD source
// ---------------------------------------------------------------------------

/**
 * POD begins at a =command that starts at column 0 and ends at =cut.
 * Multiple POD sections per file are supported.
 */
function extractPodLines(source: string): string[] {
    const result: string[] = [];
    let inPod = false;

    for (const line of source.split('\n')) {
        if (!inPod) {
            // Any =word at start of line begins a POD section
            if (/^=[a-zA-Z]/.test(line)) {
                inPod = true;
                // =cut at the very start before any other pod would be odd,
                // but handle it: don't add the line, stay out of pod
                if (/^=cut\b/.test(line)) {
                    inPod = false;
                    continue;
                }
                result.push(line);
            }
        } else {
            if (/^=cut\b/.test(line)) {
                inPod = false;
                // Don't include the =cut line itself
            } else {
                result.push(line);
            }
        }
    }

    return result;
}

// ---------------------------------------------------------------------------
// Step 2: parse POD lines into logical blocks
// ---------------------------------------------------------------------------

type Block =
    | { kind: 'command'; cmd: string; text: string }
    | { kind: 'verbatim'; lines: string[] }
    | { kind: 'para'; text: string };

function parseBlocks(lines: string[]): Block[] {
    const blocks: Block[] = [];
    let i = 0;

    while (i < lines.length) {
        const line = lines[i];

        // Command paragraph: line starting with =
        if (/^=[a-zA-Z]/.test(line)) {
            const m = line.match(/^=(\S+)\s*(.*)/);
            const cmd = m ? m[1] : '';
            const text = m ? m[2].trim() : '';
            blocks.push({ kind: 'command', cmd, text });
            i++;
            continue;
        }

        // Verbatim paragraph: indented line
        if (/^\s+\S/.test(line)) {
            const verbLines: string[] = [];
            while (i < lines.length && (/^\s/.test(lines[i]) || lines[i] === '')) {
                // Stop collecting if we hit a blank line followed by non-indented
                if (lines[i] === '') {
                    // Peek: is the next non-empty line indented?
                    let j = i + 1;
                    while (j < lines.length && lines[j] === '') { j++; }
                    if (j >= lines.length || !/^\s/.test(lines[j])) {
                        break;
                    }
                }
                verbLines.push(lines[i]);
                i++;
            }
            // Skip trailing blank lines inside the verbatim block
            while (verbLines.length > 0 && verbLines[verbLines.length - 1] === '') {
                verbLines.pop();
            }
            if (verbLines.length > 0) {
                blocks.push({ kind: 'verbatim', lines: verbLines });
            }
            continue;
        }

        // Blank line: paragraph separator — skip
        if (line.trim() === '') {
            i++;
            continue;
        }

        // Ordinary paragraph: collect until blank line or command
        const paraLines: string[] = [];
        while (i < lines.length && lines[i].trim() !== '' && !/^=[a-zA-Z]/.test(lines[i]) && !/^\s+\S/.test(lines[i])) {
            paraLines.push(lines[i]);
            i++;
        }
        if (paraLines.length > 0) {
            blocks.push({ kind: 'para', text: paraLines.join(' ') });
        }
    }

    return blocks;
}

// ---------------------------------------------------------------------------
// Step 3: render blocks to HTML
// ---------------------------------------------------------------------------

function renderBlocks(blocks: Block[]): string {
    const parts: string[] = [];
    let listStack: Array<'ul' | 'ol'> = [];

    function closeAllLists(): void {
        while (listStack.length > 0) {
            parts.push(`</${listStack.pop()}>`);
        }
    }

    for (const block of blocks) {
        if (block.kind === 'command') {
            const { cmd, text } = block;

            // Heading commands
            if (/^head[1-4]$/.test(cmd)) {
                closeAllLists();
                const level = cmd[4];
                parts.push(`<h${level}>${escapeHtml(text)}</h${level}>`);
                continue;
            }

            // =pod: no-op (marks start of pod, already extracted)
            if (cmd === 'pod') {
                continue;
            }

            // =over: begin a list
            if (cmd === 'over') {
                // defer list type until we see the first =item
                listStack.push('ul'); // placeholder; corrected on first =item
                parts.push('<!-- over -->'); // placeholder index for replacement
                continue;
            }

            // =item: list item
            if (cmd === 'item') {
                // Determine / correct list type from item marker
                const isOrdered = /^\d+[.)]\s/.test(text);
                const isUnordered = /^[*\-]\s/.test(text) || text === '*' || text === '-';

                // If we are not inside any list context, open one
                if (listStack.length === 0) {
                    const listTag: 'ul' | 'ol' = isOrdered ? 'ol' : 'ul';
                    listStack.push(listTag);
                    parts.push(`<${listTag}>`);
                } else {
                    // Fix the placeholder if this is the first item
                    const expectedTag: 'ul' | 'ol' = isOrdered ? 'ol' : 'ul';
                    const placeholderIdx = parts.lastIndexOf('<!-- over -->');
                    if (placeholderIdx !== -1) {
                        parts[placeholderIdx] = `<${expectedTag}>`;
                        listStack[listStack.length - 1] = expectedTag;
                    }
                }

                // Strip leading marker
                let itemText = text;
                if (isOrdered) {
                    itemText = text.replace(/^\d+[.)]\s*/, '').trim();
                } else if (isUnordered) {
                    itemText = text.replace(/^[*\-]\s*/, '').trim();
                }

                parts.push(`<li>${renderInline(itemText)}</li>`);
                continue;
            }

            // =back: end a list
            if (cmd === 'back') {
                if (listStack.length > 0) {
                    const tag = listStack.pop();
                    // Remove the placeholder if it was never replaced
                    const placeholderIdx = parts.lastIndexOf('<!-- over -->');
                    if (placeholderIdx !== -1) {
                        parts[placeholderIdx] = `<${tag ?? 'ul'}>`;
                    }
                    parts.push(`</${tag ?? 'ul'}>`);
                }
                continue;
            }

            // =begin / =end / =for / =encoding: skip
            if (['begin', 'end', 'for', 'encoding'].includes(cmd)) {
                continue;
            }

            // Unknown command: treat as paragraph
            if (text) {
                parts.push(`<p>${renderInline(text)}</p>`);
            }
            continue;
        }

        if (block.kind === 'verbatim') {
            closeAllLists();
            // Compute common indentation to strip
            const stripped = stripCommonIndent(block.lines);
            parts.push(`<pre><code>${escapeHtml(stripped.join('\n'))}</code></pre>`);
            continue;
        }

        if (block.kind === 'para') {
            closeAllLists();
            parts.push(`<p>${renderInline(block.text)}</p>`);
            continue;
        }
    }

    closeAllLists();

    // Remove any remaining placeholder comments (e.g. =over with no =item)
    return parts
        .filter(p => p !== '<!-- over -->')
        .join('\n');
}

// ---------------------------------------------------------------------------
// Inline formatting: B<>, I<>, C<>, L<>, E<>, S<>, X<>
// ---------------------------------------------------------------------------

function renderInline(text: string): string {
    // Process inline codes. POD allows extended delimiters (B<< >> etc.) but
    // we handle the common single-delimiter case for practicality.
    return text
        // Extended double-angle delimiters first
        .replace(/([BICLESFZX])<<\s+(.*?)\s+>>/g, (_m, code, inner) => renderInlineCode(code, inner))
        // Single-angle delimiters
        .replace(/([BICLESFZX])<([^>]*)>/g, (_m, code, inner) => renderInlineCode(code, inner))
        // Finally escape any remaining bare HTML
        .replace(/&(?!(?:[a-zA-Z]+|#\d+|#x[0-9a-fA-F]+);)/g, '&amp;')
        .replace(/<(?![a-zA-Z/])/g, '&lt;')
        .replace(/(?<![a-zA-Z"])>/g, '&gt;');
}

function renderInlineCode(code: string, inner: string): string {
    switch (code) {
        case 'B': return `<strong>${escapeHtml(inner)}</strong>`;
        case 'I': return `<em>${escapeHtml(inner)}</em>`;
        case 'C': return `<code>${escapeHtml(inner)}</code>`;
        case 'F': return `<code class="filename">${escapeHtml(inner)}</code>`;
        case 'L': return renderLinkCode(inner);
        case 'E': return renderEscapeCode(inner);
        case 'S': return `<span class="no-break">${escapeHtml(inner)}</span>`;
        case 'Z': return ''; // zero-width
        case 'X': return ''; // index entry, invisible
        default:  return escapeHtml(inner);
    }
}

function renderLinkCode(inner: string): string {
    // L<text|url> or L<name> or L<name/section>
    const pipeIdx = inner.indexOf('|');
    if (pipeIdx !== -1) {
        const label = inner.slice(0, pipeIdx).trim();
        const target = inner.slice(pipeIdx + 1).trim();
        if (/^https?:\/\//.test(target)) {
            return `<a href="${escapeAttr(target)}">${escapeHtml(label)}</a>`;
        }
        return `<a href="#${escapeAttr(target.replace(/\s+/g, '-').toLowerCase())}">${escapeHtml(label)}</a>`;
    }

    if (/^https?:\/\//.test(inner)) {
        return `<a href="${escapeAttr(inner)}">${escapeHtml(inner)}</a>`;
    }

    // Module/manpage reference
    const slashIdx = inner.indexOf('/');
    if (slashIdx !== -1) {
        const mod = inner.slice(0, slashIdx);
        const section = inner.slice(slashIdx + 1);
        return `<a href="https://perldoc.perl.org/${escapeAttr(mod)}">${escapeHtml(mod)}/${escapeHtml(section)}</a>`;
    }

    return `<a href="https://perldoc.perl.org/${escapeAttr(inner)}">${escapeHtml(inner)}</a>`;
}

function renderEscapeCode(name: string): string {
    const namedEntities: Record<string, string> = {
        lt: '&lt;',
        gt: '&gt;',
        sol: '/',
        verbar: '|',
        amp: '&amp;',
        apos: "'",
        quot: '&quot;',
    };
    if (name in namedEntities) {
        return namedEntities[name];
    }
    if (/^\d+$/.test(name)) {
        return `&#${name};`;
    }
    if (/^0x[0-9a-fA-F]+$/i.test(name)) {
        return `&#x${name.slice(2)};`;
    }
    // Unicode name (U+xxxx) — best effort
    const unicodeMatch = name.match(/^U\+([0-9A-Fa-f]+)$/);
    if (unicodeMatch) {
        return `&#x${unicodeMatch[1]};`;
    }
    return `&amp;${escapeHtml(name)};`;
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

function escapeHtml(text: string): string {
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

function escapeAttr(text: string): string {
    return text
        .replace(/&/g, '&amp;')
        .replace(/"/g, '&quot;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
}

function stripCommonIndent(lines: string[]): string[] {
    const nonEmptyLines = lines.filter(l => l.trim() !== '');
    if (nonEmptyLines.length === 0) { return lines; }

    const minIndent = nonEmptyLines.reduce((min, l) => {
        const m = l.match(/^(\s*)/);
        const indent = m ? m[1].length : 0;
        return Math.min(min, indent);
    }, Infinity);

    if (minIndent === 0 || !isFinite(minIndent)) { return lines; }
    return lines.map(l => (l.trim() === '' ? '' : l.slice(minIndent)));
}

// ---------------------------------------------------------------------------
// Webview panel
// ---------------------------------------------------------------------------

let podPreviewPanel: vscode.WebviewPanel | undefined;

/**
 * Open (or reveal) the POD preview panel for the given document.
 */
export function showPodPreview(
    context: vscode.ExtensionContext,
    document: vscode.TextDocument
): void {
    const column = vscode.ViewColumn.Beside;

    if (podPreviewPanel) {
        podPreviewPanel.reveal(column);
    } else {
        podPreviewPanel = vscode.window.createWebviewPanel(
            'perlPodPreview',
            'POD Preview',
            column,
            {
                enableScripts: false,
                retainContextWhenHidden: true,
            }
        );

        podPreviewPanel.onDidDispose(
            () => { podPreviewPanel = undefined; },
            null,
            context.subscriptions
        );
    }

    updatePodPreviewContent(document);
}

function updatePodPreviewContent(document: vscode.TextDocument): void {
    if (!podPreviewPanel) { return; }

    const source = document.getText();
    const bodyHtml = podToHtml(source);
    const fileName = document.fileName.split(/[\\/]/).pop() ?? document.fileName;

    podPreviewPanel.title = `POD: ${fileName}`;
    podPreviewPanel.webview.html = buildWebviewHtml(fileName, bodyHtml);
}

function buildWebviewHtml(title: string, bodyHtml: string): string {
    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
  <title>${escapeHtml(title)}</title>
  <style>
    body {
      font-family: var(--vscode-editor-font-family, 'Segoe UI', Tahoma, sans-serif);
      font-size: var(--vscode-editor-font-size, 14px);
      line-height: 1.6;
      color: var(--vscode-editor-foreground, #ccc);
      background: var(--vscode-editor-background, #1e1e1e);
      padding: 1.5em 2em;
      max-width: 860px;
    }
    h1, h2, h3, h4 {
      color: var(--vscode-textLink-foreground, #4ec9b0);
      border-bottom: 1px solid var(--vscode-panel-border, #333);
      padding-bottom: 0.2em;
      margin-top: 1.5em;
    }
    h1 { font-size: 1.8em; }
    h2 { font-size: 1.4em; }
    h3 { font-size: 1.2em; }
    h4 { font-size: 1.05em; }
    pre {
      background: var(--vscode-textCodeBlock-background, #252526);
      border: 1px solid var(--vscode-panel-border, #333);
      border-radius: 4px;
      padding: 0.8em 1em;
      overflow-x: auto;
    }
    code {
      font-family: var(--vscode-editor-font-family, 'Courier New', monospace);
      font-size: 0.92em;
      background: var(--vscode-textCodeBlock-background, #252526);
      padding: 0.1em 0.3em;
      border-radius: 2px;
    }
    pre code {
      background: transparent;
      padding: 0;
    }
    a {
      color: var(--vscode-textLink-foreground, #4ec9b0);
      text-decoration: none;
    }
    a:hover { text-decoration: underline; }
    ul, ol { padding-left: 1.5em; }
    li { margin: 0.3em 0; }
    p { margin: 0.6em 0; }
    .no-pod {
      color: var(--vscode-disabledForeground, #888);
      font-style: italic;
    }
    .no-break { white-space: nowrap; }
  </style>
</head>
<body>
${bodyHtml}
</body>
</html>`;
}

/**
 * Register the preview command and the on-save auto-update watcher.
 * Call this from extension.ts activate().
 */
export function registerPodPreview(context: vscode.ExtensionContext): vscode.Disposable[] {
    const previewCommand = vscode.commands.registerCommand('perl-lsp.previewPod', () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'perl') {
            vscode.window.showErrorMessage('No active Perl file to preview POD documentation');
            return;
        }
        showPodPreview(context, editor.document);
    });

    const saveWatcher = vscode.workspace.onDidSaveTextDocument((document) => {
        if (document.languageId !== 'perl') { return; }
        if (!podPreviewPanel) { return; }
        updatePodPreviewContent(document);
    });

    return [previewCommand, saveWatcher];
}
