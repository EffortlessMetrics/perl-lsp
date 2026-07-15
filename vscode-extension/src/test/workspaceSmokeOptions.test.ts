import { workspaceSmokeLaunchArgs } from './workspaceSmokeOptions';

describe('workspace smoke launch options', () => {
  const originalTrustMode = process.env.PERL_LSP_SMOKE_WORKSPACE_TRUST;

  afterEach(() => {
    if (originalTrustMode === undefined) {
      delete process.env.PERL_LSP_SMOKE_WORKSPACE_TRUST;
    } else {
      process.env.PERL_LSP_SMOKE_WORKSPACE_TRUST = originalTrustMode;
    }
  });

  test('keeps the existing trust-disabled default explicit', () => {
    delete process.env.PERL_LSP_SMOKE_WORKSPACE_TRUST;

    expect(workspaceSmokeLaunchArgs('workspace.code-workspace')).toEqual([
      'workspace.code-workspace',
      '--disable-workspace-trust',
    ]);
  });

  test('can launch a genuinely untrusted workspace', () => {
    process.env.PERL_LSP_SMOKE_WORKSPACE_TRUST = '  untrusted  ';

    expect(workspaceSmokeLaunchArgs('workspace')).toEqual(['workspace']);
  });

  test('rejects an unknown trust mode instead of silently changing the claim', () => {
    process.env.PERL_LSP_SMOKE_WORKSPACE_TRUST = 'maybe';

    expect(() => workspaceSmokeLaunchArgs('workspace')).toThrow(/disabled or untrusted/);
  });
});
