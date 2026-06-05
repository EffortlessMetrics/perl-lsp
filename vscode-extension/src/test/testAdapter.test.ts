import { EventEmitter } from 'events';
import * as vscode from 'vscode';

const mockSpawn = jest.fn();

jest.mock('child_process', () => ({
  spawn: mockSpawn,
}));

import { PerlTestAdapter, PROVE_UNAVAILABLE_GUIDANCE } from '../testAdapter';

describe('Perl Test Explorer guidance', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    (vscode.workspace.findFiles as jest.Mock).mockResolvedValue([]);
    (vscode.workspace as any).getWorkspaceFolder = jest.fn(() => undefined);
  });

  test('missing prove errors explain install and PATH recovery steps', async () => {
    const proc = new EventEmitter() as EventEmitter & {
      stdout: EventEmitter;
      stderr: EventEmitter;
      kill: jest.Mock;
    };
    proc.stdout = new EventEmitter();
    proc.stderr = new EventEmitter();
    proc.kill = jest.fn();
    mockSpawn.mockReturnValue(proc);

    const adapter = Object.create(PerlTestAdapter.prototype);
    const run = {
      errored: jest.fn(),
    };
    const fileItem = {
      uri: vscode.Uri.file('/tmp/example.t'),
      label: 'example.t',
    };
    const subtest = {
      uri: vscode.Uri.file('/tmp/example.t'),
      label: 'loads',
    };
    const token = {
      isCancellationRequested: false,
      onCancellationRequested: jest.fn(() => ({ dispose: jest.fn() })),
    };

    const runPromise = (adapter as any).runProve('/tmp/example.t', fileItem, [subtest], run, token);
    proc.emit('error', new Error('spawn prove ENOENT'));
    await runPromise;

    expect(PROVE_UNAVAILABLE_GUIDANCE).toContain('cpanm Test::Harness');
    expect(run.errored).toHaveBeenCalledWith(
      fileItem,
      expect.objectContaining({
        message: expect.stringContaining(PROVE_UNAVAILABLE_GUIDANCE),
      })
    );
    expect(run.errored).toHaveBeenCalledWith(
      subtest,
      expect.objectContaining({
        message: PROVE_UNAVAILABLE_GUIDANCE,
      })
    );
  });
});
