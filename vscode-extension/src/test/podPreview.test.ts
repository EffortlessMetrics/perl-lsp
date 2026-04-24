/**
 * Contract tests for the POD preview panel feature (issue #2062).
 *
 * These are static contract tests — they verify that package.json declares
 * the command with correct metadata and that the POD-to-HTML conversion
 * logic handles common POD constructs correctly.
 * No live VSCode extension host is needed.
 */

import * as fs from 'fs';
import * as path from 'path';
import { podToHtml } from '../podPreview';

const EXT_ROOT = path.resolve(__dirname, '..', '..');

function readPackageJson(): any {
  return JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
}

// ---------------------------------------------------------------------------
// package.json contract: command registration
// ---------------------------------------------------------------------------
describe('perl-lsp.previewPod command (issue #2062)', () => {
  let pkg: any;
  let commandIds: string[];
  let paletteEntries: any[];

  beforeAll(() => {
    pkg = readPackageJson();
    commandIds = pkg.contributes.commands.map((c: any) => c.command);
    paletteEntries = pkg.contributes.menus?.commandPalette ?? [];
  });

  test('perl-lsp.previewPod is registered in contributes.commands', () => {
    expect(commandIds).toContain('perl-lsp.previewPod');
  });

  test('perl-lsp.previewPod has category Perl', () => {
    const cmd = pkg.contributes.commands.find((c: any) => c.command === 'perl-lsp.previewPod');
    expect(cmd?.category).toBe('Perl');
  });

  test('perl-lsp.previewPod has a non-empty title', () => {
    const cmd = pkg.contributes.commands.find((c: any) => c.command === 'perl-lsp.previewPod');
    expect(cmd?.title).toBeTruthy();
  });

  test('perl-lsp.previewPod appears in command palette', () => {
    const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.previewPod');
    expect(entry).toBeDefined();
  });

  test('perl-lsp.previewPod is guarded by editorLangId == perl', () => {
    const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.previewPod');
    expect(entry?.when).toContain('editorLangId == perl');
  });

  test('perl-lsp.previewPod has an activation event', () => {
    expect(pkg.activationEvents).toContain('onCommand:perl-lsp.previewPod');
  });
});

// ---------------------------------------------------------------------------
// POD-to-HTML conversion logic
// ---------------------------------------------------------------------------
describe('podToHtml', () => {
  test('renders =head1 as <h1>', () => {
    const html = podToHtml('=head1 NAME\n');
    expect(html).toContain('<h1>NAME</h1>');
  });

  test('renders =head2 as <h2>', () => {
    const html = podToHtml('=head2 SYNOPSIS\n');
    expect(html).toContain('<h2>SYNOPSIS</h2>');
  });

  test('renders =head3 as <h3>', () => {
    const html = podToHtml('=head3 Details\n');
    expect(html).toContain('<h3>Details</h3>');
  });

  test('renders =head4 as <h4>', () => {
    const html = podToHtml('=head4 Notes\n');
    expect(html).toContain('<h4>Notes</h4>');
  });

  test('renders =over/=item/=back as a list', () => {
    const pod = '=over 4\n\n=item * First\n\n=item * Second\n\n=back\n';
    const html = podToHtml(pod);
    expect(html).toContain('<ul>');
    expect(html).toContain('<li>First</li>');
    expect(html).toContain('<li>Second</li>');
    expect(html).toContain('</ul>');
  });

  test('renders =over/=item with numbers as <ol>', () => {
    const pod = '=over 4\n\n=item 1. First\n\n=item 2. Second\n\n=back\n';
    const html = podToHtml(pod);
    expect(html).toContain('<ol>');
    expect(html).toContain('</ol>');
  });

  test('renders verbatim block (indented) as <pre><code>', () => {
    const pod = '=pod\n\n    my $x = 1;\n    print $x;\n\n=cut\n';
    const html = podToHtml(pod);
    expect(html).toContain('<pre><code>');
    expect(html).toContain('my $x = 1;');
  });

  test('renders plain paragraph as <p>', () => {
    const pod = '=pod\n\nThis is a plain paragraph.\n\n=cut\n';
    const html = podToHtml(pod);
    expect(html).toContain('<p>This is a plain paragraph.</p>');
  });

  test('renders B<bold> as <strong>', () => {
    const pod = '=pod\n\nThis is B<bold> text.\n\n=cut\n';
    const html = podToHtml(pod);
    expect(html).toContain('<strong>bold</strong>');
  });

  test('renders I<italic> as <em>', () => {
    const pod = '=pod\n\nThis is I<italic> text.\n\n=cut\n';
    const html = podToHtml(pod);
    expect(html).toContain('<em>italic</em>');
  });

  test('renders C<code> as <code>', () => {
    const pod = '=pod\n\nUse C<my $x> for a scalar.\n\n=cut\n';
    const html = podToHtml(pod);
    expect(html).toContain('<code>my $x</code>');
  });

  test('renders L<link> as anchor', () => {
    const pod = '=pod\n\nSee L<perldoc>.\n\n=cut\n';
    const html = podToHtml(pod);
    expect(html).toContain('<a');
    expect(html).toContain('perldoc');
  });

  test('renders E<lt> as &lt;', () => {
    const pod = '=pod\n\nUse E<lt>tag E<gt> syntax.\n\n=cut\n';
    const html = podToHtml(pod);
    expect(html).toContain('&lt;');
    expect(html).toContain('&gt;');
  });

  test('skips non-POD code before =pod marker', () => {
    const source = 'sub foo { return 1; }\n\n=head1 NAME\n\nMyModule\n\n=cut\n\nsub bar { }\n';
    const html = podToHtml(source);
    expect(html).toContain('<h1>NAME</h1>');
    expect(html).not.toContain('sub foo');
  });

  test('returns empty content message when no POD found', () => {
    const html = podToHtml('# just a comment\nmy $x = 1;\n');
    expect(html).toContain('No POD documentation found');
  });

  test('handles =pod/=cut markers', () => {
    const pod = '=pod\n\nThis is documentation.\n\n=cut\n';
    const html = podToHtml(pod);
    expect(html).toContain('<p>This is documentation.</p>');
  });

  test('escapes HTML special characters in plain text', () => {
    const pod = '=pod\n\nUse 3 < 5 & check > 0.\n\n=cut\n';
    const html = podToHtml(pod);
    expect(html).toContain('&lt;');
    expect(html).toContain('&amp;');
    expect(html).toContain('&gt;');
  });
});
