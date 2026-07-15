import type { ReinstallCommandResult } from '../commandResults';

function makeResult(versionProperty: 'omitted' | 'undefined'): ReinstallCommandResult {
  const result: ReinstallCommandResult = {
    ok: true,
    serverPath: '/server/perllsp',
    target: 'x86_64-unknown-linux-gnu',
    source: 'existing',
    checksumVerified: true,
  };
  if (versionProperty === 'undefined') {
    result.version = undefined;
  }
  return result;
}

test('reinstall result distinguishes omitted and explicitly unavailable versions', () => {
  const omitted = makeResult('omitted');
  const explicitUndefined = makeResult('undefined');

  expect('version' in omitted).toBe(false);
  expect('version' in explicitUndefined).toBe(true);
  expect(explicitUndefined.version).toBeUndefined();
});
