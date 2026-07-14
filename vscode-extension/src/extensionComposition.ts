import {
  LanguageClientLifecycle,
  type LanguageClientLifecycleOptions,
  type LifecycleClient,
  type LifecycleHooks,
  type LifecycleSnapshot,
  type LifecycleState,
} from './languageClientLifecycle';

export type ExtensionClientAvailability<TClient> =
  | {
      readonly kind: 'unavailable';
      readonly state: LifecycleState;
      readonly snapshot: LifecycleSnapshot;
    }
  | {
      readonly kind: 'starting';
      readonly state: LifecycleState;
      readonly snapshot: LifecycleSnapshot;
    }
  | {
      readonly kind: 'failed';
      readonly state: 'failed';
      readonly error: unknown;
      readonly snapshot: LifecycleSnapshot;
    }
  | {
      readonly kind: 'ready';
      readonly state: 'running';
      readonly client: TClient;
      readonly serverPath: string;
      readonly snapshot: LifecycleSnapshot;
    };

/**
 * Extension-facing composition around the UI-free lifecycle controller.
 *
 * The controller owns the active generation and authoritative snapshot. The
 * client and path accessors here are deliberately projections for legacy
 * command code; they are never used to perform lifecycle transitions.
 */
export class ExtensionLanguageClientLifecycle<
  TClient extends LifecycleClient<TEvent>,
  TEvent = unknown,
> {
  readonly controller: LanguageClientLifecycle<TClient, TEvent>;

  private projectedClient: TClient | undefined;
  private nextServerPath: string | undefined;

  constructor(
    hooks: LifecycleHooks<TClient, TEvent>,
    options: LanguageClientLifecycleOptions = {},
  ) {
    this.controller = new LanguageClientLifecycle(
      {
        ...hooks,
        resolveServerPath: async () => {
          if (this.nextServerPath !== undefined) {
            const serverPath = this.nextServerPath;
            this.nextServerPath = undefined;
            return serverPath;
          }
          return hooks.resolveServerPath();
        },
        onStarted: async (client, serverPath) => {
          this.projectedClient = client;
          await hooks.onStarted?.(client, serverPath);
        },
        onStopped: async (snapshot) => {
          this.projectedClient = undefined;
          await hooks.onStopped?.(snapshot);
        },
        onFailed: async (snapshot) => {
          this.projectedClient = undefined;
          await hooks.onFailed?.(snapshot);
        },
      },
      options,
    );
  }

  get snapshot(): LifecycleSnapshot {
    return this.controller.snapshot;
  }

  /** Compatibility projection for command/provider code. */
  get client(): TClient | undefined {
    if (this.snapshot.state !== 'running') {
      return undefined;
    }
    return this.projectedClient;
  }

  /** Authoritative path projection for the current controller generation. */
  get serverPath(): string | null {
    return this.snapshot.serverPath;
  }

  get hasPendingServerPathOverride(): boolean {
    return this.nextServerPath !== undefined;
  }

  get availability(): ExtensionClientAvailability<TClient> {
    const snapshot = this.snapshot;
    if (snapshot.state === 'failed') {
      return {
        kind: 'failed',
        state: snapshot.state,
        error: snapshot.error,
        snapshot,
      };
    }

    if (snapshot.state === 'resolving' || snapshot.state === 'starting') {
      return { kind: 'starting', state: snapshot.state, snapshot };
    }

    if (snapshot.state === 'running' && this.projectedClient && snapshot.serverPath) {
      return {
        kind: 'ready',
        state: snapshot.state,
        client: this.projectedClient,
        serverPath: snapshot.serverPath,
        snapshot,
      };
    }

    return { kind: 'unavailable', state: snapshot.state, snapshot };
  }

  setServerPathOverride(serverPath: string): void {
    this.nextServerPath = serverPath;
  }

  start(): Promise<TClient | undefined> {
    return this.controller.start();
  }

  restart(): Promise<TClient | undefined> {
    return this.controller.restart();
  }

  stop(): Promise<void> {
    return this.controller.stop();
  }
}
