import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import { hasLaunchJson } from './debugAdapter';
import { BinaryDownloader } from './downloader';
import { describeWorkspaceTopology } from './workspaceTopology';

function incrementCount(target: Record<string, number>, key: string): void {
  target[key] = (target[key] ?? 0) + 1;
}

function classifyLaunchPath(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    return 'empty';
  }
  if (trimmed.includes('${workspaceFolder')) {
    return 'workspace_variable';
  }
  if (trimmed.includes('${file')) {
    return 'file_variable';
  }
  if (/\$\{[^}]+}/.test(trimmed)) {
    return 'other_variable';
  }
  if (trimmed.startsWith('~')) {
    return 'home_relative';
  }
  if (path.isAbsolute(trimmed)) {
    return 'absolute';
  }
  if (!trimmed.includes('/') && !trimmed.includes('\\')) {
    return 'command';
  }
  return 'relative';
}

function collectLaunchConfigurationState(
  workspaceFolders: readonly vscode.WorkspaceFolder[],
): Record<string, unknown> {
  const includePathKindCounts: Record<string, number> = {};
  const perlPathKindCounts: Record<string, number> = {};
  const programPathKindCounts: Record<string, number> = {};
  const cwdPathKindCounts: Record<string, number> = {};
  let configurationCount = 0;
  let perlConfigurationCount = 0;
  let launchRequestCount = 0;
  let attachRequestCount = 0;
  let perlPathConfiguredCount = 0;
  let includePathsConfiguredCount = 0;
  let includePathEntryCount = 0;
  let nonStringIncludePathCount = 0;
  let programConfiguredCount = 0;
  let cwdConfiguredCount = 0;

  for (const folder of workspaceFolders) {
    const launchConfigurations = vscode.workspace
      .getConfiguration('launch', folder.uri)
      .get<unknown[]>('configurations', []);
    if (!Array.isArray(launchConfigurations)) {
      continue;
    }

    for (const entry of launchConfigurations) {
      if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
        continue;
      }

      configurationCount += 1;
      const config = entry as Record<string, unknown>;
      if (config.type !== 'perl') {
        continue;
      }

      perlConfigurationCount += 1;
      if (config.request === 'launch') {
        launchRequestCount += 1;
      } else if (config.request === 'attach') {
        attachRequestCount += 1;
      }

      if (typeof config.perlPath === 'string') {
        perlPathConfiguredCount += 1;
        incrementCount(perlPathKindCounts, classifyLaunchPath(config.perlPath));
      }

      if (typeof config.program === 'string') {
        programConfiguredCount += 1;
        incrementCount(programPathKindCounts, classifyLaunchPath(config.program));
      }

      if (typeof config.cwd === 'string') {
        cwdConfiguredCount += 1;
        incrementCount(cwdPathKindCounts, classifyLaunchPath(config.cwd));
      }

      if (Array.isArray(config.includePaths)) {
        includePathsConfiguredCount += 1;
        for (const includePath of config.includePaths) {
          if (typeof includePath !== 'string') {
            nonStringIncludePathCount += 1;
            continue;
          }
          includePathEntryCount += 1;
          incrementCount(includePathKindCounts, classifyLaunchPath(includePath));
        }
      }
    }
  }

  return {
    status: 'client_launch_config_reported',
    configuration_count: configurationCount,
    perl_configuration_count: perlConfigurationCount,
    launch_request_count: launchRequestCount,
    attach_request_count: attachRequestCount,
    perl_path_configured_count: perlPathConfiguredCount,
    include_paths_configured_count: includePathsConfiguredCount,
    include_path_entry_count: includePathEntryCount,
    non_string_include_path_count: nonStringIncludePathCount,
    program_configured_count: programConfiguredCount,
    cwd_configured_count: cwdConfiguredCount,
    include_path_kind_counts: includePathKindCounts,
    perl_path_kind_counts: perlPathKindCounts,
    program_path_kind_counts: programPathKindCounts,
    cwd_path_kind_counts: cwdPathKindCounts,
    claim_boundary:
      'Launch configuration state is summarized from VS Code configuration only. It reports counts and path classes, not raw paths, and does not start DAP, resolve Perl, probe modules, or change debug behavior.',
  };
}

export function workspaceTrustClientRuntimeState(
  context?: vscode.ExtensionContext,
): Record<string, unknown> {
  const workspaceFolders = vscode.workspace.workspaceFolders ?? [];
  const managedDapPath = context ? BinaryDownloader.getLocalDapPath(context) : undefined;
  const managedAdapterExists = managedDapPath ? fs.existsSync(managedDapPath) : false;
  const launchJsonWorkspaceCount = workspaceFolders.filter((folder) =>
    hasLaunchJson(folder.uri.fsPath),
  ).length;
  const activeDebugSession = vscode.debug.activeDebugSession;
  const topology = describeWorkspaceTopology({
    folders: workspaceFolders,
    documents: vscode.workspace.textDocuments,
    isTrusted: vscode.workspace.isTrusted,
    remoteName: vscode.env.remoteName,
  });

  return {
    schema_version: 'workspace_trust_client_runtime.v1',
    source: 'vscode-extension',
    perldoc: {
      status: 'client_surface_registered',
      uri_scheme: 'perldoc',
      client_surface: 'perldoc virtual documents are served by the LSP textDocumentContent path',
    },
    dap: {
      status: 'client_state_reported',
      adapter_registered: true,
      active_perl_debug_session: activeDebugSession?.type === 'perl',
      managed_adapter_exists: managedAdapterExists,
      launch_json_workspace_count: launchJsonWorkspaceCount,
      workspace_folder_count: workspaceFolders.length,
      launch_configuration: collectLaunchConfigurationState(workspaceFolders),
    },
    topology,
    claim_boundary:
      'VS Code client runtime state reads extension/debugger state only. It does not start DAP, run perldoc, probe Perl, or change provider behavior.',
  };
}
