/**
 * Unit tests for Perl debug adapter configuration and descriptor factory.
 */

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
  PerlDebugAdapterDescriptorFactory,
  PerlDebugConfigurationProvider,
  buildDapExecutableArgs,
  buildLaunchJsonContent,
  hasLaunchJson,
  offerDebugConfigOnFirstPerlOpen,
  parseDebugTestLaunchTarget,
  resetDebugConfigPromptFlag,
  rewriteTestLensCommand,
  VSCODE_DEBUG_TEST_COMMAND,
  VSCODE_RUN_TEST_COMMAND,
} from '../debugAdapter';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function makeContext(storagePath?: string): any {
  const dir = storagePath ?? fs.mkdtempSync(path.join(os.tmpdir(), 'dap-test-'));
  return {
    globalStorageUri: { fsPath: dir },
    extensionPath: dir,
    subscriptions: [],
  };
}

// ---------------------------------------------------------------------------
// PerlDebugConfigurationProvider
// ---------------------------------------------------------------------------
describe('PerlDebugConfigurationProvider', () => {
  let provider: PerlDebugConfigurationProvider;

  beforeEach(() => {
    provider = new PerlDebugConfigurationProvider();
  });

  describe('resolveDebugConfiguration', () => {
    test('fills in defaults for empty config when active editor is Perl', () => {
      const vscode = require('vscode');
      vscode.window.activeTextEditor = {
        document: { languageId: 'perl', uri: { fsPath: '/test.pl' } },
      };

      const config: any = {};
      provider.resolveDebugConfiguration(undefined, config);

      expect(config.type).toBe('perl');
      expect(config.name).toBe('Launch Perl');
      expect(config.request).toBe('launch');
      expect(config.program).toBe('${file}');

      vscode.window.activeTextEditor = undefined;
    });

    test('does not modify config with existing type/request/name', () => {
      const config: any = {
        type: 'perl',
        request: 'launch',
        name: 'Custom Debug',
        program: '/my/script.pl',
      };
      const result = provider.resolveDebugConfiguration(undefined, config);
      expect(result).toBeDefined();
      expect((result as any).program).toBe('/my/script.pl');
    });

    test('sets attach defaults for TCP mode (no processId)', () => {
      const config: any = {
        type: 'perl',
        request: 'attach',
        name: 'Attach',
      };
      const result = provider.resolveDebugConfiguration(undefined, config);

      expect((result as any).host).toBe('localhost');
      expect((result as any).port).toBe(13603);
    });

    test('preserves user-supplied attach host and port', () => {
      const config: any = {
        type: 'perl',
        request: 'attach',
        name: 'Attach Custom',
        host: '10.0.0.1',
        port: 5000,
      };
      const result = provider.resolveDebugConfiguration(undefined, config);

      expect((result as any).host).toBe('10.0.0.1');
      expect((result as any).port).toBe(5000);
    });

    test('skips TCP defaults when processId is provided', () => {
      const config: any = {
        type: 'perl',
        request: 'attach',
        name: 'Attach PID',
        processId: 42,
      };
      const result = provider.resolveDebugConfiguration(undefined, config);

      expect((result as any).host).toBeUndefined();
      expect((result as any).port).toBeUndefined();
    });

    test('returns undefined when launch has no program', async () => {
      const config: any = {
        type: 'perl',
        request: 'launch',
        name: 'No Program',
      };
      const result = provider.resolveDebugConfiguration(undefined, config);

      if (result && typeof (result as any).then === 'function') {
        const resolved = await result;
        expect(resolved).toBeUndefined();
      }
    });
  });

  describe('provideDebugConfigurations', () => {
    test('provides at least 3 default configurations', () => {
      const configs = provider.provideDebugConfigurations(undefined);
      expect(Array.isArray(configs)).toBe(true);
      expect((configs as any[]).length).toBeGreaterThanOrEqual(3);
    });

    test('includes launch, attach by TCP, and attach by PID templates', () => {
      const configs = provider.provideDebugConfigurations(undefined) as any[];

      const hasLaunch = configs.some((c) => c.request === 'launch');
      const hasTCPAttach = configs.some((c) => c.request === 'attach' && c.port);
      const hasPIDAttach = configs.some((c) => c.request === 'attach' && c.processId);

      expect(hasLaunch).toBe(true);
      expect(hasTCPAttach).toBe(true);
      expect(hasPIDAttach).toBe(true);
    });

    test('all configurations have type "perl"', () => {
      const configs = provider.provideDebugConfigurations(undefined) as any[];
      for (const config of configs) {
        expect(config.type).toBe('perl');
      }
    });
  });
});

// ---------------------------------------------------------------------------
// PerlDebugAdapterDescriptorFactory
// ---------------------------------------------------------------------------
describe('PerlDebugAdapterDescriptorFactory', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dap-factory-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test('returns undefined and shows an actionable warning when perl-dap is not found anywhere', () => {
    const ctx = makeContext(tmpDir);
    const factory = new PerlDebugAdapterDescriptorFactory(ctx);
    const vscode = require('vscode');

    const origPath = process.env.PATH;
    const origHome = process.env.HOME;
    const origCargo = process.env.CARGO_HOME;
    process.env.PATH = tmpDir;
    process.env.HOME = tmpDir;
    process.env.CARGO_HOME = tmpDir;

    try {
      const result = factory.createDebugAdapterDescriptor({} as any, undefined);
      expect(result).toBeUndefined();
      expect(vscode.window.showErrorMessage).toHaveBeenCalledWith(
        expect.stringContaining('perl-dap'),
        'Reinstall',
        'Open Debugging Guide',
      );
    } finally {
      process.env.PATH = origPath;
      process.env.HOME = origHome;
      process.env.CARGO_HOME = origCargo;
    }
  });

  test('finds perl-dap in the auto-download directory', () => {
    const binDir = path.join(tmpDir, 'bin', `${process.platform}-${process.arch}`);
    fs.mkdirSync(binDir, { recursive: true });
    const dapName = process.platform === 'win32' ? 'perl-dap.exe' : 'perl-dap';
    const dapPath = path.join(binDir, dapName);
    fs.writeFileSync(dapPath, '#!/bin/sh\necho ok');
    if (process.platform !== 'win32') {
      fs.chmodSync(dapPath, 0o755);
    }

    const ctx = makeContext(tmpDir);
    const factory = new PerlDebugAdapterDescriptorFactory(ctx);
    const result = factory.createDebugAdapterDescriptor({} as any, undefined) as any;

    expect(result).toBeDefined();
    expect(result.command).toBe(dapPath);
  });

  test('descriptor includes RUST_LOG=debug environment variable', () => {
    const binDir = path.join(tmpDir, 'bin', `${process.platform}-${process.arch}`);
    fs.mkdirSync(binDir, { recursive: true });
    const dapName = process.platform === 'win32' ? 'perl-dap.exe' : 'perl-dap';
    const dapPath = path.join(binDir, dapName);
    fs.writeFileSync(dapPath, '#!/bin/sh\necho ok');
    if (process.platform !== 'win32') {
      fs.chmodSync(dapPath, 0o755);
    }

    const ctx = makeContext(tmpDir);
    const factory = new PerlDebugAdapterDescriptorFactory(ctx);
    const result = factory.createDebugAdapterDescriptor({} as any, undefined) as any;

    expect(result.options.env.RUST_LOG).toBe('debug');
  });

  test('passes --external-peer through to the descriptor when the session sets externalPeer', () => {
    const binDir = path.join(tmpDir, 'bin', `${process.platform}-${process.arch}`);
    fs.mkdirSync(binDir, { recursive: true });
    const dapName = process.platform === 'win32' ? 'perl-dap.exe' : 'perl-dap';
    const dapPath = path.join(binDir, dapName);
    fs.writeFileSync(dapPath, '#!/bin/sh\necho ok');
    if (process.platform !== 'win32') {
      fs.chmodSync(dapPath, 0o755);
    }

    const ctx = makeContext(tmpDir);
    const factory = new PerlDebugAdapterDescriptorFactory(ctx);
    const session = { configuration: { externalPeer: 'localhost:9000' } };
    const result = factory.createDebugAdapterDescriptor(session as any, undefined) as any;

    expect(result.args).toEqual(['--external-peer', 'localhost:9000']);
  });

  test('uses empty args for a plain launch session', () => {
    const binDir = path.join(tmpDir, 'bin', `${process.platform}-${process.arch}`);
    fs.mkdirSync(binDir, { recursive: true });
    const dapName = process.platform === 'win32' ? 'perl-dap.exe' : 'perl-dap';
    const dapPath = path.join(binDir, dapName);
    fs.writeFileSync(dapPath, '#!/bin/sh\necho ok');
    if (process.platform !== 'win32') {
      fs.chmodSync(dapPath, 0o755);
    }

    const ctx = makeContext(tmpDir);
    const factory = new PerlDebugAdapterDescriptorFactory(ctx);
    const session = { configuration: { request: 'launch', program: '/tmp/x.pl' } };
    const result = factory.createDebugAdapterDescriptor(session as any, undefined) as any;

    expect(result.args).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// debug test command wiring
// ---------------------------------------------------------------------------
describe('debug test command helpers', () => {
  test('rewrites server debug-test code lenses to the VS Code command', () => {
    const lens = {
      command: {
        title: 'Debug Test',
        command: 'perl.debugTest',
        arguments: ['file:///tmp/basic.t::test_basic'],
      },
    };

    expect(rewriteTestLensCommand(lens).command.command).toBe(VSCODE_DEBUG_TEST_COMMAND);
  });

  test('rewrites server run-test code lenses to the VS Code command', () => {
    const lens = {
      command: {
        title: 'Run Test',
        command: 'perl.runTest',
        arguments: ['file:///tmp/basic.t::test_basic'],
      },
    };

    expect(rewriteTestLensCommand(lens).command.command).toBe(VSCODE_RUN_TEST_COMMAND);
  });

  test('leaves unrelated code lenses unchanged', () => {
    const lens = {
      command: {
        title: 'Go to definition',
        command: 'perl.goToDefinition',
      },
    };

    expect(rewriteTestLensCommand(lens)).toEqual(lens);
  });

  test('parses a code-lens test id into a launch target', () => {
    const fileUri = process.platform === 'win32' ? 'file:///C:/tmp/basic.t' : 'file:///tmp/basic.t';
    const expectedProgram =
      process.platform === 'win32' ? path.normalize('C:/tmp/basic.t') : '/tmp/basic.t';

    expect(parseDebugTestLaunchTarget(`${fileUri}::test_basic`)).toEqual({
      label: 'test_basic',
      program: expectedProgram,
      args: [],
    });
  });

  test('parses a TestItem-like object into a launch target', () => {
    expect(
      parseDebugTestLaunchTarget({
        label: 'constructor',
        uri: { fsPath: path.normalize('/workspace/t/basic.t') },
        args: ['--verbose'],
      }),
    ).toEqual({
      label: 'constructor',
      program: path.normalize('/workspace/t/basic.t'),
      args: ['--verbose'],
    });
  });

  test('returns undefined for an invalid debug target payload', () => {
    expect(parseDebugTestLaunchTarget(null)).toBeUndefined();
    expect(parseDebugTestLaunchTarget({ label: 'missing-uri' })).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// buildLaunchJsonContent
// ---------------------------------------------------------------------------
describe('buildLaunchJsonContent', () => {
  test('launch-script template produces valid JSON with perl type', () => {
    const content = buildLaunchJsonContent('launch-script');
    const parsed = JSON.parse(content);
    expect(parsed.version).toBe('0.2.0');
    expect(Array.isArray(parsed.configurations)).toBe(true);
    const cfg = parsed.configurations[0];
    expect(cfg.type).toBe('perl');
    expect(cfg.request).toBe('launch');
  });

  test('attach-process template produces attach config with host and port', () => {
    const content = buildLaunchJsonContent('attach-process');
    const parsed = JSON.parse(content);
    const cfg = parsed.configurations[0];
    expect(cfg.type).toBe('perl');
    expect(cfg.request).toBe('attach');
    expect(cfg.host).toBe('localhost');
    expect(cfg.port).toBe(13603);
  });

  test('remote-ssh template produces attach config with configurable host', () => {
    const content = buildLaunchJsonContent('remote-ssh');
    const parsed = JSON.parse(content);
    const cfg = parsed.configurations[0];
    expect(cfg.type).toBe('perl');
    expect(cfg.request).toBe('attach');
    expect(typeof cfg.host).toBe('string');
  });

  test('all template produces multiple configurations', () => {
    const content = buildLaunchJsonContent('all');
    const parsed = JSON.parse(content);
    expect(parsed.configurations.length).toBeGreaterThanOrEqual(3);
    const types = parsed.configurations.map((c: any) => c.type);
    expect(types.every((t: string) => t === 'perl')).toBe(true);
  });

  test('unknown template falls back to launch-script', () => {
    const content = buildLaunchJsonContent('unknown-template');
    const parsed = JSON.parse(content);
    const cfg = parsed.configurations[0];
    expect(cfg.type).toBe('perl');
    expect(cfg.request).toBe('launch');
  });

  test('external-peer template carries the externalPeer field', () => {
    const content = buildLaunchJsonContent('external-peer');
    const parsed = JSON.parse(content);
    const cfg = parsed.configurations[0];
    expect(cfg.type).toBe('perl');
    expect(cfg.request).toBe('attach');
    expect(typeof cfg.externalPeer).toBe('string');
    expect(cfg.externalPeer).toMatch(/^[^\s:]+:\d+$/);
  });

  test('all template includes the external-peer configuration', () => {
    const content = buildLaunchJsonContent('all');
    const parsed = JSON.parse(content);
    const hasPeer = parsed.configurations.some((c: any) => typeof c.externalPeer === 'string');
    expect(hasPeer).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// buildDapExecutableArgs
// ---------------------------------------------------------------------------
describe('buildDapExecutableArgs', () => {
  test('passes --external-peer through for a valid host:port', () => {
    expect(buildDapExecutableArgs({ externalPeer: 'localhost:9000' } as any)).toEqual([
      '--external-peer',
      'localhost:9000',
    ]);
  });

  test('trims surrounding whitespace on the peer address', () => {
    expect(buildDapExecutableArgs({ externalPeer: '  127.0.0.1:13700  ' } as any)).toEqual([
      '--external-peer',
      '127.0.0.1:13700',
    ]);
  });

  test('returns no args when externalPeer is absent', () => {
    expect(buildDapExecutableArgs({ request: 'launch' } as any)).toEqual([]);
    expect(buildDapExecutableArgs(undefined)).toEqual([]);
  });

  test('ignores a malformed peer address rather than passing it through', () => {
    expect(buildDapExecutableArgs({ externalPeer: 'not-a-peer' } as any)).toEqual([]);
    expect(buildDapExecutableArgs({ externalPeer: 'host:' } as any)).toEqual([]);
    expect(buildDapExecutableArgs({ externalPeer: 42 } as any)).toEqual([]);
  });

  test('falls back to native for a non-connectable port in the flat shape', () => {
    // `host:0` (0 = "allocate", not connectable) and out-of-range ports must
    // fall back to the native adapter, consistent with the structured shape —
    // not spawn an unconnectable `--external-peer host:0`.
    expect(buildDapExecutableArgs({ externalPeer: 'localhost:0' } as any)).toEqual([]);
    expect(buildDapExecutableArgs({ externalPeer: 'localhost:70000' } as any)).toEqual([]);
  });

  test('falls back to the native adapter for a bracketed IPv6 peer address', () => {
    // The validator requires the host segment to contain no ':' (so a plain
    // "host:port" split is unambiguous), so a bracketed IPv6 literal like
    // "[::1]:9000" does not match and the adapter falls back to native mode
    // rather than passing an unvalidated value through. This documents a
    // known scope limit, not a crash or injection risk.
    expect(buildDapExecutableArgs({ externalPeer: '[::1]:9000' } as any)).toEqual([]);
    expect(buildDapExecutableArgs({ externalPeer: '::1:9000' } as any)).toEqual([]);
  });

  test('rejects a peer address with embedded whitespace instead of splitting on it', () => {
    // Guards against argv smuggling: a value like "host --some-flag:9000"
    // must not turn into a second, attacker-controlled CLI argument for the
    // spawned perl-dap process.
    expect(buildDapExecutableArgs({ externalPeer: 'host --flag:9000' } as any)).toEqual([]);
    expect(buildDapExecutableArgs({ externalPeer: 'host:9000 --flag' } as any)).toEqual([]);
  });

  test('translates the shipped structured externalDebugger (connect) shape', () => {
    const config = {
      debuggerBackend: 'external',
      externalDebugger: {
        kind: 'ptkdb',
        mode: 'connect',
        control: 'mirror',
        host: '127.0.0.1',
        port: 13604,
      },
    };
    expect(buildDapExecutableArgs(config as any)).toEqual(['--external-peer', '127.0.0.1:13604']);
  });

  test('defaults host to 127.0.0.1 and mode to connect for the structured shape', () => {
    expect(
      buildDapExecutableArgs({
        debuggerBackend: 'external',
        externalDebugger: { port: 9001 },
      } as any),
    ).toEqual(['--external-peer', '127.0.0.1:9001']);
  });

  test('wires listen mode to --external-peer-listen (port 0 = allocate ephemeral)', () => {
    // A concrete port binds it; port 0 / absent asks perl-dap to allocate one,
    // so only the host is passed as the bind spec.
    expect(
      buildDapExecutableArgs({
        debuggerBackend: 'external',
        externalDebugger: { mode: 'listen', control: 'mirror', host: '127.0.0.1', port: 13604 },
      } as any),
    ).toEqual(['--external-peer-listen', '127.0.0.1:13604']);
    expect(
      buildDapExecutableArgs({
        debuggerBackend: 'external',
        externalDebugger: { mode: 'listen', control: 'mirror', host: '127.0.0.1', port: 0 },
      } as any),
    ).toEqual(['--external-peer-listen', '127.0.0.1']);
    // Defaults host to 127.0.0.1 when omitted.
    expect(
      buildDapExecutableArgs({
        debuggerBackend: 'external',
        externalDebugger: { mode: 'listen' },
      } as any),
    ).toEqual(['--external-peer-listen', '127.0.0.1']);
  });

  test('does not fabricate an address for unimplemented modes or connect port 0', () => {
    // launchPeer is not wired; connect requires a concrete port — fall back to native.
    expect(
      buildDapExecutableArgs({
        debuggerBackend: 'external',
        externalDebugger: { mode: 'launchPeer', port: 13604 },
      } as any),
    ).toEqual([]);
    expect(
      buildDapExecutableArgs({
        debuggerBackend: 'external',
        externalDebugger: { mode: 'connect', port: 0 },
      } as any),
    ).toEqual([]);
  });

  test('rejects a listen host that could smuggle extra argv tokens', () => {
    expect(
      buildDapExecutableArgs({
        debuggerBackend: 'external',
        externalDebugger: { mode: 'listen', host: 'host --flag' },
      } as any),
    ).toEqual([]);
    expect(
      buildDapExecutableArgs({
        debuggerBackend: 'external',
        externalDebugger: { mode: 'listen', host: 'a:b' },
      } as any),
    ).toEqual([]);
  });

  test('the native backend (or absent debuggerBackend) yields no bridge args', () => {
    expect(buildDapExecutableArgs({ debuggerBackend: 'native', program: '/x.pl' } as any)).toEqual(
      [],
    );
    expect(buildDapExecutableArgs({ request: 'launch', program: '/x.pl' } as any)).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// hasLaunchJson
// ---------------------------------------------------------------------------
describe('hasLaunchJson', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'launch-json-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test('returns false when .vscode/launch.json does not exist', () => {
    expect(hasLaunchJson(tmpDir)).toBe(false);
  });

  test('returns false when .vscode directory is missing', () => {
    expect(hasLaunchJson(path.join(tmpDir, 'nonexistent'))).toBe(false);
  });

  test('returns true when .vscode/launch.json exists', () => {
    const vscodDir = path.join(tmpDir, '.vscode');
    fs.mkdirSync(vscodDir);
    fs.writeFileSync(path.join(vscodDir, 'launch.json'), '{}');
    expect(hasLaunchJson(tmpDir)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// offerDebugConfigOnFirstPerlOpen
// ---------------------------------------------------------------------------
describe('offerDebugConfigOnFirstPerlOpen', () => {
  const vscode = require('vscode');

  beforeEach(() => {
    resetDebugConfigPromptFlag();
    jest.clearAllMocks();
    vscode.workspace.workspaceFolders = undefined;
  });

  afterEach(() => {
    vscode.workspace.workspaceFolders = undefined;
  });

  test('does nothing for non-perl documents', async () => {
    const doc = { languageId: 'javascript' };
    await offerDebugConfigOnFirstPerlOpen(doc as any);
    expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
  });

  test('does nothing when no workspace folders are open', async () => {
    vscode.workspace.workspaceFolders = [];
    const doc = { languageId: 'perl' };
    await offerDebugConfigOnFirstPerlOpen(doc as any);
    expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
  });

  test('shows onboarding prompt for perl document in workspace without launch.json', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'onboard-test-'));
    try {
      vscode.workspace.workspaceFolders = [{ uri: { fsPath: tmpDir }, name: 'test' }];
      const doc = { languageId: 'perl' };
      await offerDebugConfigOnFirstPerlOpen(doc as any);
      expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
        expect.stringContaining('debug configuration'),
        expect.any(String),
        expect.any(String),
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test('does not show prompt when launch.json already exists', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'onboard-exists-'));
    try {
      const vscodDir = path.join(tmpDir, '.vscode');
      fs.mkdirSync(vscodDir);
      fs.writeFileSync(path.join(vscodDir, 'launch.json'), '{}');
      vscode.workspace.workspaceFolders = [{ uri: { fsPath: tmpDir }, name: 'test' }];
      const doc = { languageId: 'perl' };
      await offerDebugConfigOnFirstPerlOpen(doc as any);
      expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test('shows prompt only once per session even with multiple perl opens', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'onboard-once-'));
    try {
      vscode.workspace.workspaceFolders = [{ uri: { fsPath: tmpDir }, name: 'test' }];
      const doc = { languageId: 'perl' };
      await offerDebugConfigOnFirstPerlOpen(doc as any);
      await offerDebugConfigOnFirstPerlOpen(doc as any);
      expect(vscode.window.showInformationMessage).toHaveBeenCalledTimes(1);
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });
});
