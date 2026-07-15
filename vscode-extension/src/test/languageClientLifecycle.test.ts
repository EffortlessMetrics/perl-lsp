import { describe, expect, jest, test } from '@jest/globals';
import {
  LanguageClientLifecycle,
  type LifecycleClient,
  type LifecycleDisposable,
  type LifecycleHooks,
  type LifecycleState,
} from '../languageClientLifecycle';

interface TestClientEvent {
  readonly state: 'starting' | 'running' | 'stopped';
}

class Deferred<T> {
  readonly promise: Promise<T>;
  private resolvePromise!: (value: T) => void;
  private rejectPromise!: (error: unknown) => void;

  constructor() {
    this.promise = new Promise<T>((resolve, reject) => {
      this.resolvePromise = resolve;
      this.rejectPromise = reject;
    });
  }

  resolve(value: T): void {
    this.resolvePromise(value);
  }

  reject(error: unknown): void {
    this.rejectPromise(error);
  }
}

class FakeClient implements LifecycleClient<TestClientEvent> {
  private readonly listeners = new Set<(event: TestClientEvent) => void>();
  private disposed = false;
  startGate: Promise<void> = Promise.resolve();

  start(): Promise<void> {
    return this.startGate;
  }

  stop(): Promise<void> {
    return Promise.resolve();
  }

  dispose(): void {
    this.disposed = true;
  }

  onDidChangeState(listener: (event: TestClientEvent) => void): LifecycleDisposable {
    this.listeners.add(listener);
    return {
      dispose: () => {
        this.listeners.delete(listener);
      },
    };
  }

  emit(event: TestClientEvent): void {
    for (const listener of this.listeners) {
      listener(event);
    }
  }

  listenerCount(): number {
    return this.listeners.size;
  }

  isDisposed(): boolean {
    return this.disposed;
  }
}

interface Harness {
  controller: LanguageClientLifecycle<FakeClient, TestClientEvent>;
  hooks: LifecycleHooks<FakeClient, TestClientEvent>;
  clients: FakeClient[];
  states: LifecycleState[];
  callbackErrors: Array<{ error: unknown; phase: string }>;
}

function makeHarness(
  resolveServerPath: () => Promise<string | null> = async () => '/server/perllsp',
  options: { stopTimeoutMs?: number } = {},
): Harness {
  const clients: FakeClient[] = [];
  const states: LifecycleState[] = [];
  const callbackErrors: Array<{ error: unknown; phase: string }> = [];
  const hooks: LifecycleHooks<FakeClient, TestClientEvent> = {
    resolveServerPath,
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      return client;
    },
    onStateChange: (snapshot) => {
      states.push(snapshot.state);
    },
    onCallbackError: (error, phase) => {
      callbackErrors.push({ error, phase });
    },
  };
  return {
    controller: new LanguageClientLifecycle(hooks, options),
    hooks,
    clients,
    states,
    callbackErrors,
  };
}

async function flush(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

describe('LanguageClientLifecycle', () => {
  test('starts through resolving, starting, and running', async () => {
    const harness = makeHarness();

    const client = await harness.controller.start();

    expect(client).toBe(harness.clients[0]);
    expect(harness.controller.snapshot.state).toBe('running');
    expect(harness.states).toEqual(['resolving', 'starting', 'running']);
  });

  test('enters failed state when the server path cannot be resolved', async () => {
    const harness = makeHarness(async () => null);

    await expect(harness.controller.start()).rejects.toThrow(
      'Language server path could not be resolved.',
    );

    expect(harness.clients).toHaveLength(0);
    expect(harness.controller.snapshot.state).toBe('failed');
    expect(harness.states).toEqual(['resolving', 'failed']);
  });

  test('coalesces concurrent starts into one resolution and client start', async () => {
    const startDeferred = new Deferred<void>();
    const harness = makeHarness();
    harness.hooks.createClient = () => {
      const client = new FakeClient();
      client.startGate = startDeferred.promise;
      harness.clients.push(client);
      return client;
    };
    const first = harness.controller.start();
    await flush();
    const created = harness.clients[0];
    expect(created).toBeDefined();

    const second = harness.controller.start();
    expect(second).toBe(first);
    startDeferred.resolve();

    await expect(first).resolves.toBe(created);
    expect(harness.clients).toHaveLength(1);
  });

  test('returns the running client without re-resolving', async () => {
    const resolveServerPath = jest.fn(async () => '/server/perllsp');
    const harness = makeHarness(resolveServerPath);

    const client = await harness.controller.start();
    const again = await harness.controller.start();

    expect(again).toBe(client);
    expect(resolveServerPath).toHaveBeenCalledTimes(1);
    expect(harness.clients).toHaveLength(1);
  });

  test('coalesces concurrent restarts and disposes the old client once', async () => {
    const harness = makeHarness();
    const oldClient = await harness.controller.start();
    expect(oldClient).toBeDefined();
    const stop = jest.spyOn(oldClient!, 'stop');
    const dispose = jest.spyOn(oldClient!, 'dispose');

    const first = harness.controller.restart();
    const second = harness.controller.restart();

    expect(second).toBe(first);
    const restarted = await first;
    expect(harness.clients).toHaveLength(2);
    expect(restarted).toBe(harness.clients[1]);
    expect(stop).toHaveBeenCalledTimes(1);
    expect(dispose).toHaveBeenCalledTimes(1);
    expect(harness.states).toEqual([
      'resolving',
      'starting',
      'running',
      'stopping',
      'stopped',
      'resolving',
      'starting',
      'running',
    ]);
  });

  test('coalesces concurrent stops while the client is shutting down', async () => {
    const harness = makeHarness();
    const client = await harness.controller.start();
    const stopGate = new Deferred<void>();
    jest.spyOn(client!, 'stop').mockReturnValue(stopGate.promise);

    const first = harness.controller.stop();
    const second = harness.controller.stop();

    expect(second).toBe(first);
    stopGate.resolve();
    await expect(first).resolves.toBeUndefined();
    expect(harness.controller.snapshot.state).toBe('stopped');
  });

  test('queues a new start behind an in-progress stop', async () => {
    const harness = makeHarness();
    const firstClient = await harness.controller.start();
    const stopGate = new Deferred<void>();
    jest.spyOn(firstClient!, 'stop').mockReturnValue(stopGate.promise);

    const stop = harness.controller.stop();
    const start = harness.controller.start();
    expect(harness.clients).toHaveLength(1);

    stopGate.resolve();
    await expect(stop).resolves.toBeUndefined();
    const secondClient = await start;

    expect(secondClient).toBe(harness.clients[1]);
    expect(harness.controller.snapshot.state).toBe('running');
  });

  test('invalidates a pending resolution without creating a stale client', async () => {
    const resolution = new Deferred<string | null>();
    const harness = makeHarness(() => resolution.promise);
    const start = harness.controller.start();
    await flush();

    const stop = harness.controller.stop();
    resolution.resolve('/stale/server');

    await expect(stop).resolves.toBeUndefined();
    await expect(start).resolves.toBeUndefined();
    expect(harness.clients).toHaveLength(0);
    expect(harness.controller.snapshot.state).toBe('stopped');
  });

  test('starts a fresh generation after stopping pending startup', async () => {
    const firstResolution = new Deferred<string | null>();
    let resolutionCalls = 0;
    const harness = makeHarness(async () => {
      resolutionCalls += 1;
      if (resolutionCalls === 1) {
        return firstResolution.promise;
      }
      return '/fresh/server';
    });

    const firstStart = harness.controller.start();
    await flush();
    await expect(harness.controller.stop()).resolves.toBeUndefined();

    const secondStart = harness.controller.start();
    expect(secondStart).not.toBe(firstStart);
    firstResolution.resolve('/stale/server');

    const freshClient = await secondStart;
    expect(freshClient).toBe(harness.clients[0]);
    await expect(firstStart).resolves.toBeUndefined();
    expect(harness.clients).toHaveLength(1);
    expect(harness.controller.snapshot.serverPath).toBe('/fresh/server');
  });

  test('restarts cleanly when restart is requested during startup', async () => {
    const oldStart = new Deferred<void>();
    const harness = makeHarness();
    harness.hooks.createClient = () => {
      const client = new FakeClient();
      client.startGate = oldStart.promise;
      harness.clients.push(client);
      return client;
    };
    const start = harness.controller.start();
    await flush();
    const oldClient = harness.clients[0];

    const restart = harness.controller.restart();
    await flush();
    expect(['stopping', 'stopped']).toContain(harness.controller.snapshot.state);
    expect(oldClient).toBeDefined();

    oldStart.resolve();
    await expect(start).resolves.toBeUndefined();
    await expect(restart).resolves.toBe(harness.clients[1]);
    expect(oldClient.isDisposed()).toBe(true);
  });

  test('recovers after a failed start', async () => {
    const harness = makeHarness();
    const firstStartError = new Error('first start failed');
    let createCount = 0;
    harness.hooks.createClient = () => {
      const client = new FakeClient();
      createCount += 1;
      if (createCount === 1) {
        client.startGate = Promise.reject(firstStartError);
      }
      harness.clients.push(client);
      return client;
    };
    const first = harness.controller.start();
    await flush();

    await expect(first).rejects.toBe(firstStartError);
    expect(harness.controller.snapshot.state).toBe('failed');
    expect(harness.clients[0].isDisposed()).toBe(true);

    const recovered = await harness.controller.start();
    expect(harness.clients).toHaveLength(2);
    expect(recovered).toBe(harness.clients[1]);
    expect(harness.controller.snapshot.state).toBe('running');
  });

  test('bounds a hung stop and still disposes the client', async () => {
    jest.useFakeTimers();
    try {
      const harness = makeHarness(undefined, { stopTimeoutMs: 10 });
      const client = await harness.controller.start();
      expect(client).toBeDefined();
      const hungStop = new Deferred<void>();
      jest.spyOn(client!, 'stop').mockReturnValue(hungStop.promise);
      const dispose = jest.spyOn(client!, 'dispose');

      const stopPromise = harness.controller.stop();
      await jest.advanceTimersByTimeAsync(10);
      await expect(stopPromise).resolves.toBeUndefined();

      expect(dispose).toHaveBeenCalledTimes(1);
      expect(harness.controller.snapshot.state).toBe('failed');
    } finally {
      jest.useRealTimers();
    }
  });

  test('stop does not wait for onStarted finalization', async () => {
    const started = new Deferred<void>();
    const harness = makeHarness();
    harness.hooks.onStarted = jest.fn(() => started.promise);
    const start = harness.controller.start();
    await flush();
    const client = harness.clients[0];
    expect(client).toBeDefined();

    const stop = harness.controller.stop();
    await expect(stop).resolves.toBeUndefined();
    expect(client.isDisposed()).toBe(true);
    expect(harness.controller.snapshot.state).toBe('stopped');

    started.resolve();
    await expect(start).resolves.toBeUndefined();
    expect(harness.controller.snapshot.state).toBe('stopped');
  });

  test('callback failures do not prevent cleanup or become unhandled rejections', async () => {
    const startedError = new Error('presentation failed');
    const harness = makeHarness();
    harness.hooks.onStarted = () => {
      throw startedError;
    };
    harness.hooks.onStopped = () => {
      throw new Error('stop presentation failed');
    };

    await expect(harness.controller.start()).rejects.toBe(startedError);
    expect(harness.clients[0].isDisposed()).toBe(true);
    expect(harness.controller.snapshot.state).toBe('failed');

    await expect(harness.controller.stop()).resolves.toBeUndefined();
    await flush();
    expect(harness.controller.snapshot.state).toBe('stopped');
    expect(harness.callbackErrors).toHaveLength(1);
    expect(harness.callbackErrors[0]?.phase).toBe('stopped');
  });

  test('state and client listeners are disposed exactly once', async () => {
    const harness = makeHarness();
    const client = await harness.controller.start();
    expect(client).toBeDefined();
    const clientState = jest.fn((_: FakeClient, __: TestClientEvent): void => undefined);
    harness.hooks.onClientStateChange = clientState;

    client!.emit({ state: 'running' });
    expect(clientState).toHaveBeenCalledTimes(1);
    expect(client!.listenerCount()).toBe(1);

    await harness.controller.stop();
    expect(client!.listenerCount()).toBe(0);
    client!.emit({ state: 'stopped' });
    expect(clientState).toHaveBeenCalledTimes(1);
    expect(client!.isDisposed()).toBe(true);
  });

  test('client-state callback failures are isolated from lifecycle cleanup', async () => {
    const harness = makeHarness();
    const client = await harness.controller.start();
    expect(client).toBeDefined();
    harness.hooks.onClientStateChange = () => {
      throw new Error('client state presentation failed');
    };

    client!.emit({ state: 'running' });
    await harness.controller.stop();
    await flush();

    expect(client!.isDisposed()).toBe(true);
    expect(harness.controller.snapshot.state).toBe('stopped');
    expect(harness.callbackErrors).toHaveLength(1);
    expect(harness.callbackErrors[0]?.phase).toBe('client-state');
  });

  test('ignores client state events when no presentation callback is installed', async () => {
    const harness = makeHarness();
    const client = await harness.controller.start();

    client!.emit({ state: 'running' });

    await expect(harness.controller.stop()).resolves.toBeUndefined();
  });

  test('does not require a callback-error reporter for presentation failures', async () => {
    const harness = makeHarness();
    harness.hooks.onStateChange = () => {
      throw new Error('presentation failed');
    };

    const client = await harness.controller.start();
    expect(client).toBe(harness.clients[0]);
    await flush();
    expect(harness.controller.snapshot.state).toBe('running');
  });

  test('contains callback-error reporter failures', async () => {
    const harness = makeHarness();
    harness.hooks.onStateChange = () => {
      throw new Error('presentation failed');
    };
    harness.hooks.onCallbackError = () => {
      throw new Error('logging failed');
    };

    const client = await harness.controller.start();
    expect(client).toBe(harness.clients[0]);
    await flush();
    expect(harness.controller.snapshot.state).toBe('running');
  });

  test('continues cleanup when listener disposal and client disposal fail', async () => {
    const listenerError = new Error('listener disposal failed');
    const disposeError = new Error('client disposal failed');
    const harness = makeHarness();
    harness.hooks.createClient = () => {
      const client = new FakeClient();
      jest.spyOn(client, 'onDidChangeState').mockReturnValue({
        dispose: () => {
          throw listenerError;
        },
      });
      jest.spyOn(client, 'dispose').mockImplementation(() => {
        throw disposeError;
      });
      harness.clients.push(client);
      return client;
    };

    const client = await harness.controller.start();
    await expect(harness.controller.stop()).resolves.toBeUndefined();

    expect(client).toBe(harness.clients[0]);
    expect(harness.controller.snapshot.state).toBe('failed');
    expect(harness.controller.snapshot.error).toBe(listenerError);
  });
});
