export type WorkspaceSmokeTrustMode = 'disabled' | 'untrusted';

export function workspaceSmokeTrustMode(): WorkspaceSmokeTrustMode {
  const trustMode = process.env.PERL_LSP_SMOKE_WORKSPACE_TRUST?.trim() || 'disabled';
  if (trustMode === 'disabled' || trustMode === 'untrusted') {
    return trustMode;
  }
  throw new Error(`PERL_LSP_SMOKE_WORKSPACE_TRUST must be disabled or untrusted, got ${trustMode}`);
}

export function workspaceSmokeLaunchArgs(workspacePath: string): string[] {
  const trustMode = workspaceSmokeTrustMode();
  if (trustMode === 'disabled') {
    return [workspacePath, '--disable-workspace-trust'];
  }
  return [workspacePath];
}
