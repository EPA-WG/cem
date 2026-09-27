import {
    assertCemDeclarationScopeActive, onCemDeclarationScopeDispose, type CemDeclarationScope,
} from '../../declaration-scope.js';
import type { DeclarationStyleOwnership } from '../../declaration-style-ownership.js';
import { cemProcessingFailureDiagnostics } from './processing-host.js';
import {
    installRetainedStylesheets, type CemStylesheetInstallation, type CemStylesheetInstallationOptions,
} from './stylesheet-installation.js';

type Result = Awaited<CemStylesheetInstallation['ready']>;
type SourceOptions = Pick<CemStylesheetInstallationOptions, 'host' | 'artifact' | 'baseUrl' | 'occurrences' | 'read'>;

export interface CemStylesheetSource extends SourceOptions {
    /** Effective declaration identity; tag names alone cannot distinguish logical scopes. */
    declaration: object;
    scope: CemDeclarationScope;
    ownership: DeclarationStyleOwnership;
}

export interface CemStylesheetConsumer {
    element: HTMLElement;
    declaration: object;
    sharedScope: string | null;
    scope: CemDeclarationScope;
    context: CemStylesheetInstallationOptions['context'];
}

export interface CemStylesheetConnection {
    /** Includes sources added or removed while the current load is pending. */
    whenReady(): Promise<Result>;
    release(): void;
}

interface Source {
    options: CemStylesheetSource;
    unobserve: () => void;
}
interface Connection {
    options: CemStylesheetConsumer;
    loads: Map<Source, { installation?: CemStylesheetInstallation; ready: Promise<Result> }>;
    revision: number;
    released: boolean;
    unobserve: () => void;
}

/** Routes native source handles to live private/shared consumers; retains no CSS trees. */
export class CemStylesheetRegistry {
    private readonly sources = new Map<object, Source>();
    private readonly connections = new Set<Connection>();
    private readonly current = new WeakMap<HTMLElement, Connection>();
    private readonly cleanup = new Set<Promise<void>>();
    private readonly cleanupErrors: unknown[] = [];
    private readonly observer: MutationObserver | undefined;
    private disposed = false;

    constructor(private readonly document: Document) {
        const Observer = document.defaultView?.MutationObserver;
        if (Observer) {
            this.observer = new Observer(records => this.reconcile(records));
            this.observer.observe(document, { childList: true, subtree: true });
        }
    }

    register(options: CemStylesheetSource): { release(): void } {
        this.assertActive(options.scope);
        this.assertActive(options.host.ownerScope);
        if (this.sources.has(options.declaration)) throw new Error('stylesheet declaration already registered');
        const source: Source = { options: { ...options,
            ...structuredClone({ artifact: options.artifact, occurrences: options.occurrences }) }, unobserve: () => undefined };
        this.sources.set(options.declaration, source);
        source.unobserve = observeScopes([options.scope, options.host.ownerScope], () => this.removeSource(source));
        this.reconcile(this.observer?.takeRecords() ?? []);
        for (const connection of this.connections) this.attach(source, connection);
        return { release: () => this.removeSource(source) };
    }

    connect(options: CemStylesheetConsumer): CemStylesheetConnection {
        this.assertActive(options.scope);
        if (options.element.ownerDocument !== this.document || !options.element.isConnected) {
            throw new Error('stylesheet consumer must be connected to the registry document');
        }
        this.reconcile(this.observer?.takeRecords() ?? []);
        const connection: Connection = { options: { ...options, context: structuredClone(options.context) },
            loads: new Map(), revision: 0, released: false, unobserve: () => undefined };
        const previous = this.current.get(options.element);
        this.current.set(options.element, connection);
        this.connections.add(connection);
        // Publish first: an abort handler may synchronously connect another generation.
        if (previous) this.removeConnection(previous);
        if (!connection.released) {
            connection.unobserve = observeScopes([options.scope], () => this.removeConnection(connection));
            for (const source of this.sources.values()) this.attach(source, connection);
        }
        return { whenReady: () => this.whenReady(connection), release: () => this.removeConnection(connection) };
    }

    /** Await all submitted native cleanup; readers that ignore abort cannot delay it. */
    async flush(): Promise<void> {
        while (this.cleanup.size) await Promise.all(this.cleanup);
        if (this.cleanupErrors.length) throw new AggregateError(this.cleanupErrors.splice(0), 'stylesheet registry cleanup failed');
    }

    async dispose(): Promise<void> {
        this.disposed = true;
        this.observer?.disconnect();
        for (const connection of this.connections) this.removeConnection(connection);
        for (const source of this.sources.values()) this.removeSource(source);
        await this.flush();
    }

    private attach(source: Source, connection: Connection): void {
        if (connection.released || connection.loads.has(source)) return;
        const { options } = connection;
        const occurrences = source.options.occurrences.filter(occurrence => occurrence.scope.kind === 'private'
            ? source.options.declaration === options.declaration : occurrence.scope.name === options.sharedScope);
        if (!occurrences.length) return;
        connection.revision++;
        try {
            const installation = installRetainedStylesheets({ ...source.options, occurrences,
                consumer: `registry:${crypto.randomUUID()}`, context: options.context,
                lease: source.options.ownership.beginConsumer(options.element, options.scope) });
            if (connection.released || this.sources.get(source.options.declaration) !== source) {
                this.clean(installation);
                return;
            }
            connection.loads.set(source, { installation, ready: installation.ready });
        } catch (error) {
            connection.loads.set(source, { ready: Promise.resolve({ status: 'ready', installed: 0,
                diagnostics: cemProcessingFailureDiagnostics(error) }) });
        }
    }

    private async whenReady(connection: Connection): Promise<Result> {
        for (;;) {
            this.reconcile(this.observer?.takeRecords() ?? []);
            if (connection.released) return { status: 'cancelled', installed: 0, diagnostics: [] };
            const revision = connection.revision;
            const results = await Promise.all(Array.from(connection.loads.values(), load => load.ready));
            this.reconcile(this.observer?.takeRecords() ?? []);
            if (revision !== connection.revision) continue;
            return { status: 'ready', installed: results.reduce((count, result) => count + result.installed, 0),
                diagnostics: results.flatMap(result => result.diagnostics) };
        }
    }

    private removeSource(source: Source): void {
        if (this.sources.get(source.options.declaration) !== source) return;
        this.sources.delete(source.options.declaration);
        source.unobserve();
        for (const connection of this.connections) {
            const load = connection.loads.get(source);
            if (!load) continue;
            connection.loads.delete(source);
            connection.revision++;
            if (load.installation) this.clean(load.installation);
        }
    }

    private removeConnection(connection: Connection): void {
        if (!this.connections.delete(connection)) return;
        connection.released = true;
        connection.revision++;
        connection.unobserve();
        if (this.current.get(connection.options.element) === connection) this.current.delete(connection.options.element);
        for (const load of connection.loads.values()) if (load.installation) this.clean(load.installation);
        connection.loads.clear();
    }

    private clean(installation: CemStylesheetInstallation): void {
        const pending = installation.dispose().catch(error => { this.cleanupErrors.push(error); })
            .finally(() => this.cleanup.delete(pending));
        this.cleanup.add(pending);
    }

    private reconcile(records: readonly MutationRecord[]): void {
        for (const connection of this.connections) {
            const element = connection.options.element;
            if (!element.isConnected || element.ownerDocument !== this.document || records.some(record =>
                Array.from(record.removedNodes).some(node => node === element || node.contains(element)))) {
                this.removeConnection(connection);
            }
        }
    }

    private assertActive(scope: CemDeclarationScope): void {
        if (this.disposed) throw new Error('stylesheet registry is disposed');
        assertCemDeclarationScopeActive(scope);
        if (scope.document !== this.document) throw new Error('stylesheet scope belongs to another document');
    }
}

function observeScopes(scopes: CemDeclarationScope[], release: () => void): () => void {
    const ancestors = new Set<CemDeclarationScope>();
    for (const scope of scopes) for (let current: CemDeclarationScope | null = scope; current; current = current.parent) ancestors.add(current);
    const removers = Array.from(ancestors, scope => onCemDeclarationScopeDispose(scope, release));
    return () => { for (const remove of removers) remove(); };
}
