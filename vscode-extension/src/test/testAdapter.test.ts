import { parseSubtestResults, parseTapOutput } from '../testAdapter';

describe('test adapter TAP parsing', () => {
  test('summarizes top-level TAP without counting indented subtests', () => {
    const output = [
      '# Subtest: nested',
      '    ok 1 - inner',
      '    1..1',
      'ok 1 - nested',
      '1..1',
    ].join('\n');

    expect(parseTapOutput(output)).toEqual({ total: 1, passed: 1, failed: 0, bailOut: null });
  });

  test('preserves bailout text and plan count', () => {
    expect(parseTapOutput('Bail out! database unavailable\n1..3')).toEqual({
      total: 3,
      passed: 0,
      failed: 0,
      bailOut: 'database unavailable',
    });
  });

  test('maps successful and failed subtests with diagnostics', () => {
    const output = [
      '# Subtest: passes',
      '    ok 1 - assertion',
      '    1..1',
      'ok 1 - passes',
      '# Subtest: fails',
      '    not ok 1 - assertion',
      "    # Failed test 'assertion'",
      '    1..1',
      'not ok 2 - fails',
    ].join('\n');

    expect([...parseSubtestResults(output).entries()]).toEqual([
      ['passes', { ok: true, diagnostic: '', duration: 0 }],
      ['fails', { ok: false, diagnostic: "# Failed test 'assertion'", duration: 0 }],
    ]);
  });
});
