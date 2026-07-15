/**
 * Packaged semantic-token smoke test (issue #3388, PR #3406).
 *
 * WHAT LAYER THIS COVERS
 * -----------------------
 * PR #3406 retired the legacy AST-only semantic-token renderer in favor of a
 * single renderer, and proved correctness at the *source-crate* level
 * (crates/perl-lsp-rs-core unit/integration tests exercising the Rust
 * provider function directly). That proves the logic is right when called
 * in-process from Rust. It does NOT prove that the *shipped* `perllsp`
 * binary — the one `scripts/bundle-lsp.js` copies into
 * `vscode-extension/bin/<platform>-<arch>/` for packaging into the VSIX, and
 * the one the managed-download path installs for end users — decodes
 * correctly through the actual LSP wire protocol using the legend the
 * server itself advertises.
 *
 * This test closes that gap at the lightest weight that still exercises the
 * real packaged artifact:
 *   - It spawns the actual compiled `perllsp` binary as a child process
 *     (not perl-lsp-rs-core's Rust test harness, not a mock).
 *   - It speaks raw LSP JSON-RPC over stdio (Content-Length framed), the
 *     same wire protocol `vscode-languageclient` uses when VS Code drives
 *     the server.
 *   - It decodes `textDocument/semanticTokens/full` results using the
 *     `semanticTokensProvider.legend` returned in the server's own
 *     `initialize` response — never a hardcoded legend — so the test fails
 *     if the shipped legend and the shipped token classification ever
 *     drift apart from each other.
 *
 * WHAT THIS TEST DELIBERATELY DOES NOT DO
 * ----------------------------------------
 * It does not launch a full VS Code Extension Host via
 * `@vscode/test-electron` (see `src/test/integration/managedBinarySmoke.test.ts`
 * for that heavier pattern). That harness downloads a VS Code/Electron test
 * binary over the network, which this test avoids entirely — it requires no
 * network access. This is intentionally the "lightest thing that still
 * exercises the packaged/bundled server rather than the source crate
 * directly" per the #3388 remainder's acceptance criteria: it flexes the
 * exact binary that ships, through the exact protocol VS Code uses, using
 * the exact legend the server advertises, without paying for a full
 * Extension Host boot.
 *
 * DOCUMENTED GAP
 * --------------
 * This test requires a built `perllsp`/`perl-lsp` binary to be present
 * (either the bundled path `vscode-extension/bin/<platform>-<arch>/` that
 * `npm run bundle-lsp` produces, or an explicit
 * `PERL_LSP_SMOKE_SERVER_PATH` override). If neither is available — e.g. a
 * fresh checkout that has not run `npm run bundle-lsp` or set the release
 * binary path — the test SKIPS with a clear console message rather than
 * faking a pass. CI legs that build/bundle the server before running
 * `npm test` will exercise it for real; legs that don't will see the skip
 * reason in the test output.
 */

import * as fs from 'fs';
import * as path from 'path';
import { spawn, type ChildProcessWithoutNullStreams } from 'child_process';

// ---------------------------------------------------------------------------
// Locate the packaged/bundled server binary
// ---------------------------------------------------------------------------

function findBundledServerPath(): string | undefined {
  const override = process.env.PERL_LSP_SMOKE_SERVER_PATH;
  if (override && fs.existsSync(override)) {
    return override;
  }

  const extRoot = path.resolve(__dirname, '..', '..');
  const platformDir = path.join(extRoot, 'bin', `${process.platform}-${process.arch}`);
  const candidateNames =
    process.platform === 'win32' ? ['perllsp.exe', 'perl-lsp.exe'] : ['perllsp', 'perl-lsp'];

  for (const name of candidateNames) {
    const candidate = path.join(platformDir, name);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  return undefined;
}

const serverPath = findBundledServerPath();

// ---------------------------------------------------------------------------
// Minimal LSP JSON-RPC client over stdio (Content-Length framed)
// ---------------------------------------------------------------------------

interface JsonRpcMessage {
  jsonrpc: '2.0';
  id?: number;
  method?: string;
  params?: unknown;
  result?: unknown;
  error?: { code: number; message: string };
}

class MinimalLspClient {
  private child: ChildProcessWithoutNullStreams;
  private buffer = Buffer.alloc(0);
  private nextId = 1;
  private pending = new Map<
    number,
    { resolve: (v: unknown) => void; reject: (e: Error) => void }
  >();

  constructor(serverBinary: string) {
    this.child = spawn(serverBinary, [], {
      stdio: ['pipe', 'pipe', 'pipe'],
      windowsHide: true,
    });
    this.child.stdout.on('data', (chunk: Buffer) => this.onData(chunk));
    this.child.stderr.on('data', () => {
      /* drain — diagnostics not needed for this smoke */
    });
  }

  private onData(chunk: Buffer): void {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    for (;;) {
      const headerEnd = this.buffer.indexOf('\r\n\r\n');
      if (headerEnd === -1) {
        return;
      }
      const header = this.buffer.subarray(0, headerEnd).toString('utf8');
      const match = /Content-Length:\s*(\d+)/i.exec(header);
      if (!match) {
        // Malformed frame — drop the header and keep scanning.
        this.buffer = this.buffer.subarray(headerEnd + 4);
        continue;
      }
      const contentLengthText = match[1];
      if (contentLengthText === undefined) {
        this.buffer = this.buffer.subarray(headerEnd + 4);
        continue;
      }
      const contentLength = parseInt(contentLengthText, 10);
      const bodyStart = headerEnd + 4;
      const bodyEnd = bodyStart + contentLength;
      if (this.buffer.length < bodyEnd) {
        return; // wait for more data
      }
      const body = this.buffer.subarray(bodyStart, bodyEnd).toString('utf8');
      this.buffer = this.buffer.subarray(bodyEnd);
      this.dispatch(JSON.parse(body) as JsonRpcMessage);
    }
  }

  private dispatch(message: JsonRpcMessage): void {
    if (
      typeof message.id === 'number' &&
      (message.result !== undefined || message.error !== undefined)
    ) {
      const waiter = this.pending.get(message.id);
      if (waiter) {
        this.pending.delete(message.id);
        if (message.error) {
          waiter.reject(new Error(`LSP error ${message.error.code}: ${message.error.message}`));
        } else {
          waiter.resolve(message.result);
        }
      }
    }
    // Notifications from the server (e.g. window/logMessage) are ignored —
    // this smoke only cares about the semantic-tokens request/response pair.
  }

  private send(message: Record<string, unknown>): void {
    const body = JSON.stringify(message);
    const header = `Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n`;
    this.child.stdin.write(header + body);
  }

  request(method: string, params: unknown, timeoutMs: number): Promise<unknown> {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`${method} timed out after ${timeoutMs}ms`));
      }, timeoutMs);
      this.pending.set(id, {
        resolve: (v) => {
          clearTimeout(timeout);
          resolve(v);
        },
        reject: (e) => {
          clearTimeout(timeout);
          reject(e);
        },
      });
      this.send({ jsonrpc: '2.0', id, method, params });
    });
  }

  notify(method: string, params: unknown): void {
    this.send({ jsonrpc: '2.0', method, params });
  }

  dispose(): void {
    try {
      if (!this.child.killed && this.child.exitCode === null) {
        this.child.kill();
      }
    } catch {
      // best-effort
    }
  }
}

// ---------------------------------------------------------------------------
// Semantic token decoding (LSP spec: groups of 5 deltas per token)
// ---------------------------------------------------------------------------

interface DecodedToken {
  line: number;
  startChar: number;
  length: number;
  type: string;
  modifiers: number;
}

function decodeSemanticTokens(data: number[], legend: { tokenTypes: string[] }): DecodedToken[] {
  const tokens: DecodedToken[] = [];
  let line = 0;
  let char = 0;
  for (let i = 0; i + 4 < data.length; i += 5) {
    const deltaLine = data[i];
    const deltaStart = data[i + 1];
    const length = data[i + 2];
    const typeIdx = data[i + 3];
    const modifiers = data[i + 4];
    if (
      deltaLine === undefined ||
      deltaStart === undefined ||
      length === undefined ||
      typeIdx === undefined ||
      modifiers === undefined
    ) {
      continue;
    }

    line += deltaLine;
    char = deltaLine === 0 ? char + deltaStart : deltaStart;

    tokens.push({
      line,
      startChar: char,
      length,
      type: legend.tokenTypes[typeIdx] ?? `<unknown:${typeIdx}>`,
      modifiers,
    });
  }
  return tokens;
}

// ---------------------------------------------------------------------------
// The test
// ---------------------------------------------------------------------------

const describeOrSkip = serverPath ? describe : describe.skip;

if (!serverPath) {
  // eslint-disable-next-line no-console
  console.warn(
    '[packagedSemanticTokensSmoke] SKIPPED: no packaged perllsp binary found at ' +
      `vscode-extension/bin/${process.platform}-${process.arch}/ and PERL_LSP_SMOKE_SERVER_PATH is unset. ` +
      'Run `npm run bundle-lsp` (or set PERL_LSP_SMOKE_SERVER_PATH to a built perllsp/perl-lsp binary) ' +
      'to exercise this smoke against the real packaged server. This is a documented gap, not a passing test.',
  );
}

describeOrSkip(
  'packaged perllsp binary: semantic tokens through the advertised legend (#3388)',
  () => {
    let client: MinimalLspClient;

    afterEach(() => {
      client?.dispose();
    });

    test('sub -> keyword, foo -> function, no function token at column 0', async () => {
      // Guaranteed defined inside this block: describeOrSkip only registers
      // this suite when serverPath is defined.
      const binary = serverPath as string;
      client = new MinimalLspClient(binary);

      const initializeResult = (await client.request(
        'initialize',
        {
          processId: process.pid,
          rootUri: null,
          capabilities: {
            textDocument: {
              synchronization: { dynamicRegistration: false },
              semanticTokens: { dynamicRegistration: false },
            },
          },
        },
        30_000,
      )) as {
        capabilities?: {
          semanticTokensProvider?: {
            legend?: { tokenTypes: string[]; tokenModifiers: string[] };
          };
        };
      };

      const legend = initializeResult.capabilities?.semanticTokensProvider?.legend;
      expect(legend).toBeDefined();
      expect(Array.isArray(legend?.tokenTypes)).toBe(true);
      expect(legend?.tokenTypes.length ?? 0).toBeGreaterThan(0);

      client.notify('initialized', {});

      const uri = 'file:///smoke.pl';
      const text = 'sub foo { return 1; }';
      client.notify('textDocument/didOpen', {
        textDocument: { uri, languageId: 'perl', version: 1, text },
      });

      const tokensResult = (await client.request(
        'textDocument/semanticTokens/full',
        {
          textDocument: { uri },
        },
        30_000,
      )) as { data?: number[] } | null;

      expect(tokensResult).toBeTruthy();
      const data = tokensResult?.data ?? [];
      expect(data.length).toBeGreaterThan(0);

      const tokens = decodeSemanticTokens(data, legend as { tokenTypes: string[] });
      expect(tokens.length).toBeGreaterThan(0);

      // "sub" spans columns 0-3 on line 0 and must be classified as a keyword.
      const subToken = tokens.find((t) => t.line === 0 && t.startChar === 0);
      expect(subToken).toBeDefined();
      expect(subToken?.type).toBe('keyword');
      expect(subToken?.length).toBe(3);

      // "foo" spans columns 4-7 on line 0 and must be classified as a function
      // declaration — this is the precise NodeKind::Subroutine name_span the
      // single renderer (#3406) is responsible for emitting.
      const fooToken = tokens.find((t) => t.line === 0 && t.startChar === 4);
      expect(fooToken).toBeDefined();
      expect(fooToken?.type).toBe('function');
      expect(fooToken?.length).toBe(3);

      // Regression guard for the #3388 defect class: no token classified as
      // "function" may sit at column 0 (that column belongs to the "sub"
      // keyword). The legacy AST-only renderer emitted a function token at
      // the subroutine node's own start span when `name_span` was absent,
      // which the single renderer replaced with the precise name span.
      const functionTokenAtColumnZero = tokens.find(
        (t) => t.startChar === 0 && t.type === 'function',
      );
      expect(functionTokenAtColumnZero).toBeUndefined();

      await client.request('shutdown', null, 10_000).catch(() => {
        /* best-effort */
      });
      client.notify('exit', null);
    }, 45_000);
  },
);
