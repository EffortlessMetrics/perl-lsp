/**
 * Special Variables Reference — show a WebView panel listing all known
 * Perl special variables with descriptions and usage examples.
 *
 * Responsibilities:
 * - Export SPECIAL_VARS array for testability.
 * - Export buildSpecialVarsHtml() for testability.
 * - Export showSpecialVarsReference() to be wired up to the command.
 * - Never duplicate definitions from hover.rs — this is a self-contained
 *   quick-reference aid, not a copy of the LSP hover text.
 */

import * as vscode from 'vscode';

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

export interface SpecialVar {
    name: string;
    description: string;
    example: string;
}

export const SPECIAL_VARS: SpecialVar[] = [
    { name: '$_',   description: 'Default variable — used implicitly by foreach, map, grep, print, chomp', example: 'for (@items) { print }' },
    { name: '@_',   description: 'Subroutine arguments', example: 'my ($x, $y) = @_' },
    { name: '$!',   description: 'OS error (errno) — numeric or string context', example: 'open $fh, "<", $f or die $!' },
    { name: '$@',   description: 'Eval error', example: 'eval { die "oops" }; warn $@ if $@' },
    { name: '$/',   description: 'Input record separator (default: newline)', example: 'local $/; my $all = <$fh>' },
    { name: '$\\',  description: 'Output record separator appended after print', example: 'local $\\ = "\\n"' },
    { name: '$$',   description: 'Current process ID', example: 'print "PID: $$"' },
    { name: '$0',   description: 'Script name', example: 'print $0' },
    { name: '$;',   description: 'Subscript separator for multidimensional hash emulation', example: '$h{$a,$b}' },
    { name: '$,',   description: 'Output field separator for print', example: 'local $, = ", "' },
    { name: '$.',   description: 'Current line number of last read filehandle', example: 'print "$."' },
    { name: '$&',   description: 'Last successful regex match string', example: '"Hello" =~ /He/; print $&' },
    { name: "$'",   description: 'String following last regex match', example: '"Hello" =~ /He/; print $\'' },
    { name: '$`',   description: 'String preceding last regex match', example: '"Hello" =~ /el/; print $`' },
    { name: '$+',   description: 'Last bracket (capture group) matched', example: '"x" =~ /(x)/; print $+' },
    { name: '$?',   description: 'Child process status after system/backtick — exit code is $? >> 8', example: 'system("ls"); print $? >> 8' },
    { name: '$^W',  description: 'Warning flag — prefer use warnings for lexical scope', example: 'local $^W = 1' },
    { name: '$^O',  description: 'Operating system name (linux, darwin, MSWin32)', example: 'if ($^O eq "MSWin32") { }' },
    { name: '$^V',  description: 'Perl version as v-string', example: 'print $^V' },
    { name: '$^A',  description: 'Accumulator for format()/write()', example: 'formline("@<<<", "hi"); print $^A' },
    { name: '$^T',  description: 'Script start time (seconds since epoch)', example: 'print time() - $^T' },
    { name: '@ISA', description: 'Parent class list for method resolution', example: 'our @ISA = ("Animal")' },
    { name: '%ENV', description: 'Environment variables hash', example: 'my $home = $ENV{HOME}' },
    { name: '@INC', description: 'Module search paths', example: 'use lib "/my/modules"' },
    { name: '%INC', description: 'Map of loaded module filenames to full paths', example: 'print $INC{"Foo/Bar.pm"}' },
    { name: '@ARGV', description: 'Command-line arguments (not including script name)', example: 'my $file = shift @ARGV' },
    { name: '%SIG', description: 'Signal handlers hash', example: '$SIG{INT} = sub { exit 1 }' },
];

// ---------------------------------------------------------------------------
// HTML builder (exported for testing)
// ---------------------------------------------------------------------------

export function buildSpecialVarsHtml(): string {
    const rows = SPECIAL_VARS.map(v => {
        const name = escapeHtml(v.name);
        const desc = escapeHtml(v.description);
        const ex   = escapeHtml(v.example);
        return `  <tr>
    <td><code>${name}</code></td>
    <td>${desc}<br><code class="example">${ex}</code></td>
  </tr>`;
    }).join('\n');

    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
<title>Perl Special Variables Reference</title>
<style>
  body {
    font-family: var(--vscode-font-family, sans-serif);
    font-size: var(--vscode-font-size, 13px);
    color: var(--vscode-foreground);
    background: var(--vscode-editor-background);
    padding: 24px 32px;
    max-width: 860px;
    margin: 0 auto;
    line-height: 1.6;
  }
  h1 { font-size: 1.6em; margin-bottom: 0.25em; }
  p  { margin-top: 0; color: var(--vscode-descriptionForeground, #aaa); }
  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 1em;
  }
  th {
    text-align: left;
    border-bottom: 2px solid var(--vscode-panel-border, #444);
    padding: 6px 8px;
    font-weight: 600;
  }
  td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--vscode-panel-border, #333);
    vertical-align: top;
  }
  tr:hover td { background: var(--vscode-list-hoverBackground, rgba(255,255,255,0.04)); }
  code {
    font-family: var(--vscode-editor-font-family, monospace);
    background: var(--vscode-textCodeBlock-background, #1e1e1e);
    padding: 1px 4px;
    border-radius: 3px;
  }
  code.example {
    font-size: 0.9em;
    color: var(--vscode-descriptionForeground, #aaa);
    background: transparent;
    padding: 0;
  }
  a { color: var(--vscode-textLink-foreground, #4daafc); }
  .footer { margin-top: 1.5em; font-size: 0.9em; color: var(--vscode-descriptionForeground, #aaa); }
</style>
</head>
<body>
<h1>Perl Special Variables Reference</h1>
<p>Hover over any of these variables in the editor for inline documentation.</p>
<table>
  <thead>
    <tr><th>Variable</th><th>Description &amp; Example</th></tr>
  </thead>
  <tbody>
${rows}
  </tbody>
</table>
<p class="footer">
  <a href="https://perldoc.perl.org/perlvar">Full perlvar documentation</a>
</p>
</body>
</html>`;
}

// ---------------------------------------------------------------------------
// Public command entry point
// ---------------------------------------------------------------------------

export async function showSpecialVarsReference(
    _context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel,
): Promise<void> {
    const panel = vscode.window.createWebviewPanel(
        'perlLspSpecialVars',
        'Perl Special Variables Reference',
        vscode.ViewColumn.One,
        {
            enableScripts: false,
            localResourceRoots: [],
        },
    );

    panel.webview.html = buildSpecialVarsHtml();
    outputChannel.appendLine('[special-vars] Opened Special Variables Reference panel');
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function escapeHtml(text: string): string {
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}
