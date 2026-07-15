import packageJson from '../../package.json';
import { describeWorkspaceTopology, WORKSPACE_CAPABILITY_MATRIX } from '../workspaceTopology';

function folder(scheme: string, fsPath?: string) {
  return { uri: { scheme, fsPath } };
}

function document(scheme: string, fsPath?: string) {
  return { uri: { scheme, fsPath } };
}

describe('workspace topology capability contract', () => {
  test('keeps manifest capability claims tied to the tested contract', () => {
    expect(packageJson.extensionKind).toEqual(['workspace']);
    expect(packageJson.capabilities?.virtualWorkspaces).toBe(true);
    expect(packageJson.capabilities?.untrustedWorkspaces).toEqual({ supported: true });
  });

  test('classifies trusted single-root and multi-root file workspaces', () => {
    const singleRoot = describeWorkspaceTopology({
      folders: [folder('file', '/workspace')],
      documents: [document('file', '/workspace/main.pl')],
      isTrusted: true,
    });
    const multiRoot = describeWorkspaceTopology({
      folders: [folder('file', '/workspace/a'), folder('file', '/workspace/b')],
      documents: [],
      isTrusted: true,
    });

    expect(singleRoot.mode).toBe('single-root');
    expect(singleRoot.capabilities).toEqual(WORKSPACE_CAPABILITY_MATRIX['single-root']);
    expect(multiRoot.mode).toBe('multi-root');
    expect(multiRoot.folder_count).toBe(2);
    expect(multiRoot.capabilities.workspace_root).toBe('supported');
  });

  test('marks untrusted workspaces without hiding their topology', () => {
    const topology = describeWorkspaceTopology({
      folders: [folder('file', '/workspace')],
      documents: [],
      isTrusted: false,
    });

    expect(topology.mode).toBe('single-root');
    expect(topology.trust).toBe('untrusted');
    expect(topology.folder_uri_schemes).toEqual(['file']);
  });

  test('classifies virtual and untitled documents with explicit limitations', () => {
    const virtual = describeWorkspaceTopology({
      folders: [folder('git', '')],
      documents: [document('git', '')],
      isTrusted: true,
    });
    const untitled = describeWorkspaceTopology({
      folders: [],
      documents: [document('untitled')],
      isTrusted: undefined,
    });

    expect(virtual.mode).toBe('virtual');
    expect(virtual.virtual_document_count).toBe(1);
    expect(virtual.capabilities.file_backed_operations).toBe('unsupported');
    expect(virtual.limitations.join(' ')).toMatch(/file-backed operations/);
    expect(untitled.mode).toBe('empty');
    expect(untitled.untitled_document_count).toBe(1);
    expect(untitled.trust).toBe('unknown');
    expect(untitled.limitations.join(' ')).toMatch(/untitled documents/);
  });

  test('identifies remote hosts without claiming local-host proof', () => {
    const topology = describeWorkspaceTopology({
      folders: [folder('file', '/remote/workspace')],
      documents: [],
      isTrusted: true,
      remoteName: 'ssh-remote',
    });

    expect(topology.host_kind).toBe('remote');
    expect(topology.remote_name).toBe('ssh-remote');
    expect(topology.limitations.join(' ')).toMatch(/remote-host receipt/);
  });
});
