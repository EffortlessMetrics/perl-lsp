import {
  ExtensionLanguageClientLifecycle,
  type ExtensionClientAvailability,
} from '../extensionComposition';
import type { LifecycleClient, LifecycleDisposable } from '../languageClientLifecycle';

type TestEvent = { oldState: string; newState: string };

class FakeClient implements LifecycleClient<TestEvent> {
  readonly listeners: Array<(event: TestEvent) => void> = [];
  start = jest.fn(async () => undefined);
  stop = jest.fn(async () => undefined);
  dispose = jest.fn(async () => undefined);

  onDidChangeState(listener: (event: TestEvent) => void): LifecycleDisposable {
    this.listeners.push(listener);
    return {
      dispose: () => {
        const index = this.listeners.indexOf(listener);
        if (index >= 0) {
          this.listeners.splice(index, 1);
        }
      },
    };
  }
}

function availabilityKind(
  availability: ExtensionClientAvailability<FakeClient>,
): ExtensionClientAvailability<FakeClient>['kind'] {
  return availability.kind;
}

describe('ExtensionLanguageClientLifecycle', () => {
  test('projects the controller state without taking ownership away from it', async () => {
    const clients: FakeClient[] = [];
    const lifecycle = new ExtensionLanguageClientLifecycle<FakeClient, TestEvent>({
      resolveServerPath: async () => 'perllsp',
      createClient: () => {
        const client = new FakeClient();
        clients.push(client);
        return client;
      },
    });

    expect(availabilityKind(lifecycle.availability)).toBe('unavailable');
    const started = await lifecycle.start();

    expect(started).toBe(clients[0]);
    expect(lifecycle.client).toBe(clients[0]);
    expect(lifecycle.serverPath).toBe('perllsp');
    expect(availabilityKind(lifecycle.availability)).toBe('ready');

    const stopping = lifecycle.stop();
    expect(lifecycle.snapshot.state).toBe('stopping');
    expect(lifecycle.client).toBeUndefined();
    await stopping;

    expect(lifecycle.client).toBeUndefined();
    expect(lifecycle.serverPath).toBeNull();
    expect(availabilityKind(lifecycle.availability)).toBe('unavailable');
    expect(clients[0]?.stop).toHaveBeenCalledTimes(1);
    expect(clients[0]?.dispose).toHaveBeenCalledTimes(1);
  });

  test('runs the started hook for initial start and restart generations', async () => {
    const onStarted = jest.fn();
    const lifecycle = new ExtensionLanguageClientLifecycle<FakeClient, TestEvent>({
      resolveServerPath: async () => 'perllsp',
      createClient: () => new FakeClient(),
      onStarted,
    });

    await lifecycle.start();
    await lifecycle.stop();
    await lifecycle.restart();

    expect(onStarted).toHaveBeenCalledTimes(2);
    expect(onStarted).toHaveBeenNthCalledWith(1, expect.any(FakeClient), 'perllsp');
    expect(onStarted).toHaveBeenNthCalledWith(2, expect.any(FakeClient), 'perllsp');
  });

  test('reports starting while resolving and consumes a reinstall path override', async () => {
    let resolvePath: (() => void) | undefined;
    const resolving = new Promise<void>((resolve) => {
      resolvePath = resolve;
    });
    const createdPaths: string[] = [];
    const lifecycle = new ExtensionLanguageClientLifecycle<FakeClient, TestEvent>({
      resolveServerPath: async () => {
        await resolving;
        return 'configured-perllsp';
      },
      createClient: (serverPath) => {
        createdPaths.push(serverPath);
        return new FakeClient();
      },
    });

    const pendingStart = lifecycle.start();
    expect(availabilityKind(lifecycle.availability)).toBe('starting');
    resolvePath?.();
    await pendingStart;
    await lifecycle.stop();

    lifecycle.setServerPathOverride('reinstalled-perllsp');
    expect(lifecycle.hasPendingServerPathOverride).toBe(true);
    await lifecycle.start();

    expect(createdPaths).toEqual(['configured-perllsp', 'reinstalled-perllsp']);
    expect(lifecycle.hasPendingServerPathOverride).toBe(false);
  });

  test('reports a failed generation separately from an unavailable client', async () => {
    const failure = new Error('start failed');
    const lifecycle = new ExtensionLanguageClientLifecycle<FakeClient, TestEvent>({
      resolveServerPath: async () => 'perllsp',
      createClient: () => {
        const client = new FakeClient();
        client.start.mockRejectedValueOnce(failure);
        return client;
      },
    });

    await expect(lifecycle.start()).rejects.toBe(failure);

    expect(availabilityKind(lifecycle.availability)).toBe('failed');
    expect(lifecycle.availability.kind === 'failed' && lifecycle.availability.error).toBe(failure);
    expect(lifecycle.client).toBeUndefined();
  });
});
