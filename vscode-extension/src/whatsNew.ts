/**
 * WhatsNewManager — show a "What's New" webview panel on extension update.
 *
 * Responsibilities:
 * - Detect extension version changes by comparing the stored version in
 *   `context.globalState` (`perl-lsp.lastVersion`) against the version
 *   declared in `package.json`.
 * - Render a styled HTML webview panel from the CHANGELOG entries for the
 *   current version.
 * - Expose `showWhatsNew()` so the command `perl-lsp.showWhatsNew` can open
 *   the panel at any time.
 * - Never block extension startup; the panel is shown fire-and-forget.
 */

import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export class WhatsNewManager {
    private readonly context: vscode.ExtensionContext;
    private readonly outputChannel: vscode.OutputChannel;

    constructor(
        context: vscode.ExtensionContext,
        outputChannel: vscode.OutputChannel,
    ) {
        this.context = context;
        this.outputChannel = outputChannel;
    }

    // -----------------------------------------------------------------------
    // Version tracking
    // -----------------------------------------------------------------------

    /**
     * Returns `true` when the extension version stored in global state differs
     * from the currently installed extension version.
     *
     * On a fresh install there is no stored version, so this returns `true` to
     * trigger the panel — but `OnboardingManager.shouldShowWelcome()` handles
     * the truly first-run case.  The caller is responsible for coordinating
     * which panel to show.
     */
    shouldShowWhatsNew(): boolean {
        const currentVersion = this.currentVersion();
        if (!currentVersion) {
            return false;
        }
        const storedVersion = this.context.globalState.get<string>(
            'perl-lsp.lastVersion',
        );
        return storedVersion !== currentVersion;
    }

    /**
     * Persist the current version so `shouldShowWhatsNew()` returns `false`
     * until the next update.
     */
    async markVersionSeen(): Promise<void> {
        const currentVersion = this.currentVersion();
        if (currentVersion) {
            await this.context.globalState.update(
                'perl-lsp.lastVersion',
                currentVersion,
            );
        }
    }

    // -----------------------------------------------------------------------
    // Webview panel
    // -----------------------------------------------------------------------

    /**
     * Open (or reveal) the "What's New" webview panel.
     *
     * Reads CHANGELOG.md from the extension root and extracts the section for
     * the current version.  Falls back to showing the full CHANGELOG on
     * parse failure so the user always sees something useful.
     */
    async showWhatsNew(): Promise<void> {
        const version = this.currentVersion() ?? 'Unknown';
        const changelogSection = this.extractChangelogSection(version);

        const panel = vscode.window.createWebviewPanel(
            'perlLspWhatsNew',
            `What's New in Perl LSP v${version}`,
            vscode.ViewColumn.One,
            {
                enableScripts: false,
                localResourceRoots: [],
            },
        );

        panel.webview.html = this.buildHtml(version, changelogSection);
        this.outputChannel.appendLine(
            `[whats-new] Opened What's New panel for v${version}`,
        );
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /**
     * Read the version from the extension's `package.json`.
     *
     * Returns `undefined` when the extension cannot be located (e.g., in
     * unit tests that do not supply a real extension context).
     */
    currentVersion(): string | undefined {
        try {
            const pkgPath = path.join(
                this.context.extensionPath,
                'package.json',
            );
            const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8')) as {
                version?: string;
            };
            return pkg.version ?? undefined;
        } catch {
            return undefined;
        }
    }

    /**
     * Extract the changelog section for the given version from CHANGELOG.md.
     *
     * Returns the raw Markdown text for that version heading and its content,
     * or an empty string when the version is not found.
     */
    extractChangelogSection(version: string): string {
        try {
            const changelogPath = path.join(
                this.context.extensionPath,
                'CHANGELOG.md',
            );
            const full = fs.readFileSync(changelogPath, 'utf8');
            return extractVersionSection(full, version);
        } catch {
            return '';
        }
    }

    /**
     * Build the HTML page for the webview.
     *
     * The content is static — no scripts are loaded.  Markdown is converted
     * to HTML via a minimal inline renderer so the webview does not need an
     * external Markdown library.
     */
    buildHtml(version: string, markdownContent: string): string {
        const htmlBody =
            markdownContent.trim().length > 0
                ? markdownToHtml(markdownContent)
                : `<p>See the full <a href="https://github.com/EffortlessMetrics/perl-lsp/blob/master/vscode-extension/CHANGELOG.md">CHANGELOG</a> for details.</p>`;

        return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
<title>What's New in Perl LSP v${escapeHtml(version)}</title>
<style>
  body {
    font-family: var(--vscode-font-family, sans-serif);
    font-size: var(--vscode-font-size, 13px);
    color: var(--vscode-foreground);
    background: var(--vscode-editor-background);
    padding: 24px 32px;
    max-width: 760px;
    margin: 0 auto;
    line-height: 1.6;
  }
  h1 { font-size: 1.6em; margin-bottom: 0.25em; }
  h2 { font-size: 1.2em; margin-top: 1.5em; border-bottom: 1px solid var(--vscode-panel-border, #444); padding-bottom: 4px; }
  h3 { font-size: 1em; margin-top: 1.2em; }
  ul { padding-left: 1.4em; }
  li { margin-bottom: 0.3em; }
  code { font-family: var(--vscode-editor-font-family, monospace); background: var(--vscode-textCodeBlock-background, #1e1e1e); padding: 1px 4px; border-radius: 3px; }
  strong { font-weight: 600; }
  a { color: var(--vscode-textLink-foreground, #4daafc); }
  .version-badge {
    display: inline-block;
    background: var(--vscode-badge-background, #4d4d4d);
    color: var(--vscode-badge-foreground, #fff);
    border-radius: 10px;
    padding: 2px 10px;
    font-size: 0.85em;
    margin-left: 8px;
    vertical-align: middle;
  }
</style>
</head>
<body>
<h1>What's New <span class="version-badge">v${escapeHtml(version)}</span></h1>
${htmlBody}
</body>
</html>`;
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions (exported for testing)
// ---------------------------------------------------------------------------

/**
 * Extract the changelog section for a given version from the full CHANGELOG.md
 * text.
 *
 * Looks for a heading of the form `## [x.y.z]` or `## x.y.z` and returns
 * everything up to (but not including) the next `##`-level heading.
 *
 * Returns an empty string when the version is not found.
 */
export function extractVersionSection(
    changelog: string,
    version: string,
): string {
    // Match headings like: ## [0.12.0] - 2026-03-19  or  ## 0.12.0
    const escaped = version.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const headingRe = new RegExp(
        `^##\\s+(?:\\[${escaped}\\]|${escaped})(?:\\s+[^\\n]*)?$`,
        'm',
    );
    const match = headingRe.exec(changelog);
    if (!match) {
        return '';
    }

    const start = match.index;
    // Find the next ## heading after this one
    const rest = changelog.slice(start + match[0].length);
    const nextHeading = /^## /m.exec(rest);
    const end =
        nextHeading !== null
            ? start + match[0].length + nextHeading.index
            : changelog.length;

    return changelog.slice(start, end).trim();
}

/**
 * Minimal Markdown-to-HTML converter for CHANGELOG content.
 *
 * Handles the subset used in CHANGELOG.md:
 *   - `##` and `###` headings
 *   - Unordered list items (`- `)
 *   - `**bold**` inline
 *   - `` `code` `` inline
 *   - Blank-line paragraph breaks
 *
 * External libraries are intentionally avoided to keep the extension
 * dependency-free.
 */
export function markdownToHtml(md: string): string {
    const lines = md.split('\n');
    const out: string[] = [];
    let inList = false;

    for (const rawLine of lines) {
        const line = rawLine.trimEnd();

        // Headings
        if (line.startsWith('### ')) {
            if (inList) { out.push('</ul>'); inList = false; }
            out.push(`<h3>${inlineMarkdown(line.slice(4))}</h3>`);
            continue;
        }
        if (line.startsWith('## ')) {
            if (inList) { out.push('</ul>'); inList = false; }
            out.push(`<h2>${inlineMarkdown(line.slice(3))}</h2>`);
            continue;
        }
        if (line.startsWith('# ')) {
            if (inList) { out.push('</ul>'); inList = false; }
            out.push(`<h1>${inlineMarkdown(line.slice(2))}</h1>`);
            continue;
        }

        // Unordered list items
        if (/^[-*] /.test(line)) {
            if (!inList) { out.push('<ul>'); inList = true; }
            out.push(`<li>${inlineMarkdown(line.slice(2))}</li>`);
            continue;
        }

        // Blank line — paragraph break
        if (line.trim() === '') {
            if (inList) { out.push('</ul>'); inList = false; }
            out.push('');
            continue;
        }

        // Plain paragraph text
        if (inList) { out.push('</ul>'); inList = false; }
        out.push(`<p>${inlineMarkdown(line)}</p>`);
    }

    if (inList) { out.push('</ul>'); }
    return out.join('\n');
}

/**
 * Process inline Markdown: bold, code, and HTML escaping.
 */
function inlineMarkdown(text: string): string {
    // Escape HTML first, then apply Markdown inline rules.
    let s = escapeHtml(text);
    // `code`
    s = s.replace(/`([^`]+)`/g, '<code>$1</code>');
    // **bold**
    s = s.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    return s;
}

function escapeHtml(text: string): string {
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}
