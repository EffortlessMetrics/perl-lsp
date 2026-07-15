export type WorkspaceMode = 'empty' | 'single-root' | 'multi-root' | 'virtual';
export type WorkspaceTrust = 'trusted' | 'untrusted' | 'unknown';
export type CapabilityStatus = 'supported' | 'degraded' | 'unsupported' | 'not-observed';

export interface WorkspaceTopologyInput {
  folders: readonly {
    uri: {
      scheme?: string | undefined;
      fsPath?: string | undefined;
    };
  }[];
  documents: readonly {
    uri: {
      scheme?: string | undefined;
      fsPath?: string | undefined;
    };
  }[];
  isTrusted: boolean | undefined;
  remoteName?: string | undefined;
}

export interface WorkspaceTopology {
  schema_version: 'workspace_topology.v1';
  mode: WorkspaceMode;
  trust: WorkspaceTrust;
  host_kind: 'local' | 'remote';
  remote_name: string | null;
  folder_count: number;
  folder_uri_schemes: string[];
  untitled_document_count: number;
  virtual_document_count: number;
  capabilities: {
    workspace_root: CapabilityStatus;
    file_backed_operations: CapabilityStatus;
    language_server_documents: CapabilityStatus;
  };
  limitations: string[];
  claim_boundary: string;
}

export const WORKSPACE_CAPABILITY_MATRIX: Readonly<
  Record<WorkspaceMode, WorkspaceTopology['capabilities']>
> = {
  empty: {
    workspace_root: 'degraded',
    file_backed_operations: 'degraded',
    language_server_documents: 'supported',
  },
  'single-root': {
    workspace_root: 'supported',
    file_backed_operations: 'supported',
    language_server_documents: 'supported',
  },
  'multi-root': {
    workspace_root: 'supported',
    file_backed_operations: 'supported',
    language_server_documents: 'supported',
  },
  virtual: {
    workspace_root: 'degraded',
    file_backed_operations: 'unsupported',
    language_server_documents: 'supported',
  },
};

function uriScheme(uri: { scheme?: string | undefined; fsPath?: string | undefined }): string {
  const scheme = uri.scheme?.trim();
  if (scheme) {
    return scheme;
  }
  return uri.fsPath ? 'file' : 'unknown';
}

function workspaceMode(folderSchemes: readonly string[]): WorkspaceMode {
  if (folderSchemes.some((scheme) => scheme !== 'file')) {
    return 'virtual';
  }
  if (folderSchemes.length > 1) {
    return 'multi-root';
  }
  if (folderSchemes.length === 1) {
    return 'single-root';
  }
  return 'empty';
}

function trustState(isTrusted: boolean | undefined): WorkspaceTrust {
  if (isTrusted === undefined) {
    return 'unknown';
  }
  return isTrusted ? 'trusted' : 'untrusted';
}

export function describeWorkspaceTopology(input: WorkspaceTopologyInput): WorkspaceTopology {
  const folderUriSchemes = input.folders.map((folder) => uriScheme(folder.uri));
  const documentSchemes = input.documents.map((document) => uriScheme(document.uri));
  const untitledDocumentCount = documentSchemes.filter((scheme) => scheme === 'untitled').length;
  const virtualDocumentCount = documentSchemes.filter(
    (scheme) => scheme !== 'file' && scheme !== 'untitled',
  ).length;
  const mode = workspaceMode(folderUriSchemes);
  const remoteName = input.remoteName?.trim() || null;
  const limitations: string[] = [];

  if (mode === 'empty') {
    limitations.push('workspace-root-dependent operations require a file-backed folder');
  }
  if (mode === 'virtual') {
    limitations.push('file-backed operations require a file URI and may be unavailable');
  }
  if (untitledDocumentCount > 0) {
    limitations.push('untitled documents do not provide a file-backed path');
  }
  if (remoteName) {
    limitations.push('remote host behavior is identified but requires a remote-host receipt');
  }

  return {
    schema_version: 'workspace_topology.v1',
    mode,
    trust: trustState(input.isTrusted),
    host_kind: remoteName ? 'remote' : 'local',
    remote_name: remoteName,
    folder_count: input.folders.length,
    folder_uri_schemes: folderUriSchemes,
    untitled_document_count: untitledDocumentCount,
    virtual_document_count: virtualDocumentCount,
    capabilities: { ...WORKSPACE_CAPABILITY_MATRIX[mode] },
    limitations,
    claim_boundary:
      'Topology describes VS Code host state and capability classification. It does not prove server initialization, provider behavior, remote execution, or file-system access for unsupported URI schemes.',
  };
}
