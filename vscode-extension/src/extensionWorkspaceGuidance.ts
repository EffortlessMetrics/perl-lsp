import * as crypto from 'crypto';
import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

const COEXISTENCE_GUIDE_URL =
  'https://github.com/EffortlessMetrics/perl-lsp/blob/master/vscode-extension/README.md#extension-coexistence';

export async function validateIncludePaths(context: vscode.ExtensionContext): Promise<void> {
  const workspaceFolders = vscode.workspace.workspaceFolders;
  if (!workspaceFolders || workspaceFolders.length === 0) {
    return;
  }

  const isWithinBasePath = (basePath: string, targetPath: string): boolean => {
    const relative = path.relative(basePath, targetPath);
    return relative === '' || (!relative.startsWith('..') && !path.isAbsolute(relative));
  };

  const hasSafeExistingAncestor = (workspaceRealPath: string, candidatePath: string): boolean => {
    let current = candidatePath;
    while (!fs.existsSync(current)) {
      const parent = path.dirname(current);
      if (parent === current) {
        return false;
      }
      current = parent;
    }

    try {
      const ancestorRealPath = fs.realpathSync(current);
      return isWithinBasePath(workspaceRealPath, ancestorRealPath);
    } catch {
      return false;
    }
  };

  for (const folder of workspaceFolders) {
    const cacheKey = `perl-lsp.includePathsWarning.${encodeURIComponent(folder.uri.toString())}`;
    const config = vscode.workspace.getConfiguration('perl-lsp', folder.uri);
    const includePaths: string[] = config.get('includePaths', ['lib', 'local/lib/perl5']);

    // Built-in include paths are optional hints. Only explicitly configured
    // paths are expectations worth reporting to the user.
    const inspected =
      typeof config.inspect === 'function' ? config.inspect<string[]>('includePaths') : undefined;
    const defaultPaths = new Set<string>([
      '.',
      ...(inspected?.defaultValue ?? ['lib', 'local/lib/perl5']),
    ]);
    const isDefaultPath = (includePath: string): boolean => defaultPaths.has(includePath);

    let workspaceRealPath: string;
    try {
      workspaceRealPath = fs.realpathSync(folder.uri.fsPath);
    } catch {
      continue;
    }
    const missingPaths = includePaths.filter((includePath) => {
      if (isDefaultPath(includePath)) {
        return false;
      }
      return !fs.existsSync(path.resolve(folder.uri.fsPath, includePath));
    });

    if (missingPaths.length === 0) {
      await context.globalState.update(cacheKey, undefined);
      continue;
    }

    const missingSignature = missingPaths.join('\n');
    if (context.globalState.get<string | undefined>(cacheKey) === missingSignature) {
      continue;
    }

    const firstMissing = missingPaths[0];
    const relativeNote = path.isAbsolute(firstMissing)
      ? 'absolute path'
      : 'relative to the workspace';
    const suffix =
      missingPaths.length > 1 ? ` ${missingPaths.length} include paths are missing.` : '';

    const creatablePaths = missingPaths.filter((includePath) => {
      if (path.isAbsolute(includePath)) {
        return false;
      }
      const resolved = path.resolve(folder.uri.fsPath, includePath);
      const relative = path.relative(folder.uri.fsPath, resolved);
      if (relative === '' || relative.startsWith('..') || path.isAbsolute(relative)) {
        return false;
      }
      return hasSafeExistingAncestor(workspaceRealPath, resolved);
    });
    const actions = ['Open Settings'];
    if (creatablePaths.length > 0) {
      actions.push('Create Missing Directories');
    }

    const choice = await vscode.window.showWarningMessage(
      `Perl LSP: configured include path "${firstMissing}" (${relativeNote}) does not exist.${suffix}`,
      ...actions,
    );

    if (choice === 'Open Settings') {
      void vscode.commands.executeCommand(
        'workbench.action.openSettings',
        '@ext:EffortlessMetrics.perl-lsp-rs perl-lsp.includePaths',
      );
    } else if (choice === 'Create Missing Directories') {
      const createdPaths: string[] = [];
      let creationFailed = false;
      for (const includePath of creatablePaths) {
        const resolved = path.resolve(folder.uri.fsPath, includePath);
        if (!fs.existsSync(resolved) && hasSafeExistingAncestor(workspaceRealPath, resolved)) {
          try {
            fs.mkdirSync(resolved, { recursive: true });
            createdPaths.push(includePath);
          } catch (err: unknown) {
            creationFailed = true;
            const msg = err instanceof Error ? err.message : String(err);
            void vscode.window.showWarningMessage(
              `Perl LSP: failed to create directory "${includePath}": ${msg}`,
            );
          }
        }
      }

      if (createdPaths.length > 0) {
        vscode.window.showInformationMessage(
          `Created ${createdPaths.length} include director${createdPaths.length === 1 ? 'y' : 'ies'}: ${createdPaths.join(', ')}.`,
        );
        await context.globalState.update(cacheKey, undefined);
        continue;
      }
      if (creationFailed) {
        continue;
      }
    }

    await context.globalState.update(cacheKey, missingSignature);
  }
}

const DISCOVERY_CANDIDATE_DIRS = ['src', 'local', 'vendor', 'lib', 't/lib', 'blib/lib', 'modules'];

function directoryContainsPerlModule(dir: string, maxDepth = 2): boolean {
  let budget = 200;
  const walk = (current: string, depth: number): boolean => {
    if (depth > maxDepth || budget <= 0) {
      return false;
    }
    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(current, { withFileTypes: true });
    } catch {
      return false;
    }
    for (const entry of entries) {
      if (budget-- <= 0) {
        return false;
      }
      if (entry.isFile() && entry.name.endsWith('.pm')) {
        return true;
      }
    }
    for (const entry of entries) {
      if (entry.isDirectory() && !entry.name.startsWith('.')) {
        if (walk(path.join(current, entry.name), depth + 1)) {
          return true;
        }
      }
    }
    return false;
  };
  return walk(dir, 0);
}

export async function suggestDiscoveredIncludePaths(
  context: vscode.ExtensionContext,
): Promise<void> {
  const workspaceFolders = vscode.workspace.workspaceFolders;
  if (!workspaceFolders || workspaceFolders.length === 0) {
    return;
  }

  for (const folder of workspaceFolders) {
    const config = vscode.workspace.getConfiguration('perl-lsp', folder.uri);
    const includePaths: string[] = config.get('includePaths', ['lib', 'local/lib/perl5']);
    const covered = new Set(includePaths.map((includePath) => path.normalize(includePath)));
    const isCandidateCovered = (candidate: string): boolean => {
      const candidateNorm = path.normalize(candidate);
      return (
        covered.has(candidateNorm) ||
        [...covered].some(
          (configured) =>
            configured === candidateNorm + path.sep ||
            configured.startsWith(candidateNorm + path.sep) ||
            configured.startsWith(candidateNorm + '/'),
        )
      );
    };

    const discovered: string[] = [];
    for (const candidate of DISCOVERY_CANDIDATE_DIRS) {
      if (isCandidateCovered(candidate)) {
        continue;
      }
      const resolved = path.resolve(folder.uri.fsPath, candidate);
      try {
        if (!fs.statSync(resolved).isDirectory()) {
          continue;
        }
      } catch {
        continue;
      }
      if (directoryContainsPerlModule(resolved)) {
        discovered.push(candidate);
      }
    }

    if (discovered.length === 0) {
      continue;
    }

    const signature = crypto
      .createHash('sha256')
      .update(discovered.slice().sort().join('\n'))
      .digest('hex');
    const cacheKey = `perl-lsp.includePathsSuggestion.${encodeURIComponent(folder.uri.toString())}`;
    if (context.globalState.get<string | undefined>(cacheKey) === signature) {
      continue;
    }

    const primary = discovered[0];
    const extra = discovered.length > 1 ? ` (and ${discovered.length - 1} more)` : '';
    const choice = await vscode.window.showInformationMessage(
      `Perl LSP: found Perl modules in "${primary}"${extra}, but it is not in your include paths. Add it so hover, go-to-definition, and completion work?`,
      'Add to Include Paths',
      'Open Settings',
      'Dismiss',
    );

    if (choice === 'Add to Include Paths') {
      const next = Array.from(new Set([...includePaths, ...discovered]));
      try {
        await config.update('includePaths', next, vscode.ConfigurationTarget.Workspace);
        void vscode.window.showInformationMessage(
          `Added ${discovered.join(', ')} to perl-lsp.includePaths.`,
        );
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        void vscode.window.showWarningMessage(`Perl LSP: could not update include paths: ${msg}`);
        continue;
      }
    } else if (choice === 'Open Settings') {
      void vscode.commands.executeCommand(
        'workbench.action.openSettings',
        '@ext:EffortlessMetrics.perl-lsp-rs perl-lsp.includePaths',
      );
    }

    await context.globalState.update(cacheKey, signature);
  }
}

export async function suggestAiCompletionIfSupported(
  context: vscode.ExtensionContext,
  client: { initializeResult?: { capabilities?: unknown } } | undefined,
): Promise<void> {
  if (!client) {
    return;
  }

  const config = vscode.workspace.getConfiguration('perl-lsp');
  if (config.get<boolean>('aiCompletion.enabled', false)) {
    return;
  }

  const capabilities = client.initializeResult?.capabilities;
  const inlineProvider =
    !!capabilities && typeof capabilities === 'object'
      ? (capabilities as Record<string, unknown>).inlineCompletionProvider
      : undefined;
  if (inlineProvider === undefined || inlineProvider === false || inlineProvider === null) {
    return;
  }

  const stateKey = 'perl-lsp.aiCompletion.firstRunNotificationShown';
  if (context.workspaceState.get<boolean>(stateKey, false)) {
    return;
  }
  await context.workspaceState.update(stateKey, true);

  const choice = await vscode.window.showInformationMessage(
    'Perl LSP: your language server supports AI-powered inline completions. They are off by default — enable them now?',
    'Enable',
    'Learn More',
    'Dismiss',
  );

  if (choice === 'Enable') {
    try {
      await config.update('aiCompletion.enabled', true, vscode.ConfigurationTarget.Workspace);
      void vscode.window.showInformationMessage('AI-powered inline completions enabled.');
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      void vscode.window.showWarningMessage(`Perl LSP: could not enable AI completions: ${msg}`);
    }
  } else if (choice === 'Learn More') {
    void vscode.commands.executeCommand(
      'workbench.action.openSettings',
      '@ext:EffortlessMetrics.perl-lsp-rs perl-lsp.aiCompletion',
    );
  }
}

export async function openDemoProjectCommand(context: vscode.ExtensionContext): Promise<void> {
  const demoPath = path.join(context.extensionPath, 'assets', 'demo-project');
  if (!fs.existsSync(path.join(demoPath, 'main.pl'))) {
    void vscode.window.showErrorMessage(
      'Perl LSP: demo project is not available in this installation.',
    );
    return;
  }

  await context.globalState.update('perl-lsp.demoProjectOpened', true);
  void vscode.window.showInformationMessage(
    'Opening the Perl demo project. Try code completion (Ctrl+Space) in main.pl, or hover over Utils / Database for go-to-definition.',
  );
  await vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(demoPath), {
    forceNewWindow: true,
  });
}

type ExtensionPackage = {
  publisher?: string;
  name?: string;
  version?: string;
  displayName?: string;
  description?: string;
  keywords?: string[];
  contributes?: { languages?: Array<{ id?: string }> };
};

type InstalledExtension = {
  id?: string;
  packageJSON?: ExtensionPackage;
};

function isPerlLanguageExtension(extension: InstalledExtension): boolean {
  const packageJSON = extension.packageJSON;
  if (!packageJSON) {
    return false;
  }

  if ((packageJSON.contributes?.languages ?? []).some((language) => language.id === 'perl')) {
    return true;
  }

  const haystack = [
    extension.id,
    packageJSON.publisher && packageJSON.name
      ? `${packageJSON.publisher}.${packageJSON.name}`
      : undefined,
    packageJSON.displayName,
    packageJSON.name,
    packageJSON.description,
    ...(packageJSON.keywords ?? []),
  ]
    .filter((value): value is string => typeof value === 'string' && value.length > 0)
    .join(' ')
    .toLowerCase();

  return /\bperl(?:\b|[-:]|navigator|critic|tidy|lsp)/i.test(haystack);
}

export async function warnAboutPerlExtensionConflicts(
  context: vscode.ExtensionContext,
): Promise<void> {
  const packageJSON = context.extension.packageJSON as ExtensionPackage;
  const currentMajor = String(packageJSON.version ?? '0').split('.')[0] ?? '0';
  const warnedMajor = context.globalState.get<string>('perl-lsp.conflictWarningMajorVersion');
  if (warnedMajor === currentMajor) {
    return;
  }

  const selfId =
    `${packageJSON.publisher ?? 'EffortlessMetrics'}.${packageJSON.name ?? 'perl-lsp-rs'}`.toLowerCase();
  const conflicts = (vscode.extensions.all as unknown as InstalledExtension[]).filter(
    (extension) =>
      extension && extension.id?.toLowerCase() !== selfId && isPerlLanguageExtension(extension),
  );

  if (conflicts.length === 0) {
    return;
  }

  const names = conflicts
    .map((extension) => extension.packageJSON?.displayName ?? extension.id ?? 'unknown extension')
    .slice(0, 3);
  const label =
    names.length === 1
      ? names[0]
      : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
  const extra =
    conflicts.length > names.length ? ` (+${conflicts.length - names.length} more)` : '';
  const choice = await vscode.window.showWarningMessage(
    `Perl LSP detected ${conflicts.length} other Perl extension${conflicts.length === 1 ? '' : 's'}: ${label}${extra}. These can conflict with completion, hover, diagnostics, or formatting. See the coexistence guide for details.`,
    'Open Coexistence Guide',
  );

  if (choice === 'Open Coexistence Guide') {
    await vscode.env.openExternal(vscode.Uri.parse(COEXISTENCE_GUIDE_URL));
  }

  await context.globalState.update('perl-lsp.conflictWarningMajorVersion', currentMajor);
}
