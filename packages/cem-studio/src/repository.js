import { CEM_REPOSITORY_PROTOCOL_VERSION } from '@epa-wg/cem-elements';

export const CEM_STUDIO_REPOSITORY_ID = 'studio-projects';
export const CEM_STUDIO_DATABASE_VERSION = 1;
export const CEM_STUDIO_SEARCH_INDEX_VERSION = 1;

const DEFAULT_DATABASE_NAME = 'epa-cem-studio';
const MAX_SEARCH_TEXT_BYTES = 131_072;
const MAX_SEARCH_TERMS = 2_048;
/**
 * @typedef {object} CemStudioRepositoryOptions
 * @property {string} [databaseName]
 * @property {IDBFactory} [indexedDB]
 * @property {Crypto} [crypto]
 * @property {StorageManager} [storage]
 * @property {typeof BroadcastChannel} [BroadcastChannel]
 * @property {(bundle: unknown, options: {signal?: AbortSignal, operation: 'import' | 'export'}) => unknown | Promise<unknown>} validateProject
 * @property {() => string} [now]
 */

/**
 * @typedef {object} CemStudioRepositoryRequest
 * @property {1} protocolVersion
 * @property {string} repository
 * @property {string} operation
 * @property {number} requestRevision
 * @property {unknown} [parameters]
 */

/**
 * @typedef {object} RepositoryMutation
 * @property {unknown} value
 * @property {number} repositoryRevision
 * @property {number} changeCursor
 */

export class CemStudioRepositoryError extends Error {
    /**
     * @param {string} code
     * @param {string} message
     * @param {Record<string, unknown>} [details]
     * @param {unknown} [cause]
     */
    constructor(code, message, details = {}, cause) {
        super(message, cause === undefined ? undefined : { cause });
        this.name = 'CemStudioRepositoryError';
        this.code = code;
        this.details = Object.freeze(structuredClone(details));
    }
}

export class CemStudioIndexedDbRepository {
    /** @param {CemStudioRepositoryOptions} options */
    constructor(options) {
        if (!options || typeof options.validateProject !== 'function') {
            throw new CemStudioRepositoryError(
                'cem.studio.repository.project_validator_required',
                'a CEM-ML Studio project validator is required',
            );
        }
        this.databaseName = options.databaseName ?? DEFAULT_DATABASE_NAME;
        this.indexedDB = options.indexedDB ?? globalThis.indexedDB;
        this.crypto = options.crypto ?? globalThis.crypto;
        this.storage = options.storage ?? globalThis.navigator?.storage;
        this.BroadcastChannel = options.BroadcastChannel ?? globalThis.BroadcastChannel;
        this.validateProject = options.validateProject;
        this.now = options.now ?? (() => new Date().toISOString());
        this.lifecycleState = this.indexedDB ? 'idle' : 'unavailable';
        this.repositoryRevision = 0;
        this.openPromise = undefined;
        this.database = undefined;
        this.channel = undefined;
        this.subscribers = new Set();
        this.diagnostics = [];
    }

    async open() {
        if (!this.indexedDB) {
            throw new CemStudioRepositoryError(
                'cem.studio.repository.indexed_db_unavailable',
                'IndexedDB is unavailable in this browser context',
            );
        }
        if (this.database) return this.database;
        if (this.openPromise) return this.openPromise;
        this.lifecycleState = 'opening';
        this.openPromise = new Promise((resolve, reject) => {
            const request = this.indexedDB.open(this.databaseName, CEM_STUDIO_DATABASE_VERSION);
            request.onupgradeneeded = (event) => {
                this.lifecycleState = 'migrating';
                migrateDatabase(request.result, request.transaction, event.oldVersion);
            };
            request.onblocked = () => {
                this.lifecycleState = 'blocked';
                this.diagnostics = [
                    diagnostic(
                        'cem.studio.repository.upgrade_blocked',
                        'another Studio tab must close its older database connection before migration can continue',
                        'warning',
                    ),
                ];
            };
            request.onerror = () => {
                const error = normalizeRepositoryError(request.error, 'open');
                this.lifecycleState = error.code.endsWith('unavailable') ? 'unavailable' : 'failed';
                this.diagnostics = [diagnostic(error.code, error.message, 'error')];
                this.openPromise = undefined;
                reject(error);
            };
            request.onsuccess = async () => {
                const database = request.result;
                database.onversionchange = () => {
                    database.close();
                    if (this.database === database) this.database = undefined;
                    this.lifecycleState = 'blocked';
                    this.diagnostics = [
                        diagnostic(
                            'cem.studio.repository.version_changed',
                            'a newer Studio database version is ready; preserve the active draft and reload',
                            'warning',
                        ),
                    ];
                };
                database.onclose = () => {
                    if (this.database === database) this.database = undefined;
                };
                this.database = database;
                this.ensureBroadcastChannel();
                try {
                    this.repositoryRevision = await readRepositoryRevision(database);
                    this.lifecycleState = 'ready';
                    this.diagnostics = [];
                    resolve(database);
                } catch (error) {
                    database.close();
                    this.database = undefined;
                    this.lifecycleState = 'failed';
                    reject(normalizeRepositoryError(error, 'open'));
                }
            };
        });
        return this.openPromise;
    }

    /** @param {CemStudioRepositoryRequest} request @param {AbortSignal} [signal] */
    async query(request, signal) {
        assertRequest(request);
        assertNotAborted(signal);
        const database = await this.open();
        assertNotAborted(signal);
        let value;
        switch (request.operation) {
            case 'get-project':
                value = await this.getProject(database, parameters(request).projectId, false);
                break;
            case 'export-project':
                value = await this.getProject(database, parameters(request).projectId, true);
                if (value) value = await this.validateExport(value, signal);
                break;
            case 'list-projects':
                value = await listProjects(database, parameters(request).includeTrash === true);
                break;
            case 'list-changes':
                value = await listChanges(database, numberParameter(request, 'after', 0));
                break;
            case 'search':
                value = await searchRepository(database, parameters(request));
                break;
            default:
                throw unsupportedOperation(request.operation, 'query');
        }
        this.repositoryRevision = await readRepositoryRevision(database);
        return response(request, this.repositoryRevision, value);
    }

    /** @param {CemStudioRepositoryRequest} request @param {AbortSignal} [signal] */
    async execute(request, signal) {
        assertRequest(request);
        assertNotAborted(signal);
        const database = await this.open();
        assertNotAborted(signal);
        /** @type {RepositoryMutation} */
        let mutation;
        switch (request.operation) {
            case 'import-project':
                mutation = await this.importProject(database, request, signal);
                break;
            case 'save-resource':
                mutation = await this.saveResource(database, request, signal);
                break;
            case 'trash-project':
                mutation = await this.setProjectTrash(database, request, true);
                break;
            case 'restore-project':
                mutation = await this.setProjectTrash(database, request, false);
                break;
            default:
                throw unsupportedOperation(request.operation, 'command');
        }
        this.repositoryRevision = mutation.repositoryRevision;
        this.publishChange(mutation.changeCursor, mutation.repositoryRevision);
        return {
            ...response(request, mutation.repositoryRevision, mutation.value),
            changeCursor: mutation.changeCursor,
        };
    }

    async status() {
        let usage;
        let quota;
        let persisted;
        if (this.storage) {
            try {
                const estimate = await this.storage.estimate();
                usage = estimate.usage;
                quota = estimate.quota;
                persisted = await this.storage.persisted();
            } catch (error) {
                this.diagnostics = [
                    diagnostic(
                        'cem.studio.repository.storage_status_failed',
                        error instanceof Error ? error.message : 'storage status is unavailable',
                        'warning',
                    ),
                ];
            }
        }
        return {
            protocolVersion: CEM_REPOSITORY_PROTOCOL_VERSION,
            repository: CEM_STUDIO_REPOSITORY_ID,
            state: this.lifecycleState,
            repositoryRevision: this.repositoryRevision,
            schemaVersion: this.database?.version,
            ...(usage === undefined ? {} : { usage }),
            ...(quota === undefined ? {} : { quota }),
            ...(persisted === undefined ? {} : { persisted }),
            diagnostics: structuredClone(this.diagnostics),
        };
    }

    /** @param {number} cursor @param {(change: {protocolVersion: 1, repository: string, cursor: number, repositoryRevision: number}) => void} notify */
    subscribe(cursor, notify) {
        if (!Number.isSafeInteger(cursor) || cursor < 0) {
            throw new CemStudioRepositoryError(
                'cem.studio.repository.cursor_invalid',
                'change cursor must be a non-negative safe integer',
            );
        }
        const subscriber = { cursor, notify };
        this.subscribers.add(subscriber);
        return () => this.subscribers.delete(subscriber);
    }

    close() {
        this.channel?.close();
        this.channel = undefined;
        this.database?.close();
        this.database = undefined;
        this.openPromise = undefined;
        this.lifecycleState = 'closed';
        this.subscribers.clear();
    }

    async deleteDatabase() {
        this.close();
        if (!this.indexedDB) return;
        await new Promise((resolve, reject) => {
            const request = this.indexedDB.deleteDatabase(this.databaseName);
            request.onsuccess = () => resolve(undefined);
            request.onerror = () => reject(normalizeRepositoryError(request.error, 'delete'));
            request.onblocked = () =>
                reject(
                    new CemStudioRepositoryError(
                        'cem.studio.repository.delete_blocked',
                        `database \`${this.databaseName}\` is still open in another Studio context`,
                    ),
                );
        });
    }

    /** @param {IDBDatabase} database @param {string} projectId @param {boolean} includeContents */
    async getProject(database, projectId, includeContents) {
        assertIdentity(projectId, 'project id');
        const storeNames = includeContents
            ? ['projects', 'entries', 'resources', 'blobs']
            : ['projects', 'entries', 'resources'];
        return withTransaction(database, storeNames, 'readonly', async (transaction) => {
            const projectRecord = await requestValue(transaction.objectStore('projects').get(projectId));
            if (!projectRecord) return null;
            const [entries, resources] = await Promise.all([
                requestValue(transaction.objectStore('entries').index('byProject').getAll(projectId)),
                requestValue(transaction.objectStore('resources').index('byProject').getAll(projectId)),
            ]);
            const project = portableProject(projectRecord, entries, resources);
            if (!includeContents) return project;
            const contents = {};
            for (const resource of resources) {
                const blob = await requestValue(transaction.objectStore('blobs').get(resource.blobHash));
                if (!blob) {
                    throw new CemStudioRepositoryError(
                        'cem.studio.repository.blob_missing',
                        `resource \`${resource.id}\` references missing blob \`${resource.blobHash}\``,
                    );
                }
                contents[resource.id] = blob.bytes;
            }
            return { project, contents };
        });
    }

    /** @param {unknown} bundle @param {AbortSignal} [signal] */
    async validateExport(bundle, signal) {
        let validated;
        try {
            validated = await this.validateProject(structuredClone(bundle), { signal, operation: 'export' });
        } catch (error) {
            throw new CemStudioRepositoryError(
                'cem.studio.repository.export_validation_failed',
                `CEM-ML rejected the project export: ${error instanceof Error ? error.message : String(error)}`,
                {},
                error,
            );
        }
        assertNotAborted(signal);
        const normalized = normalizeValidatedBundle(validated);
        await prepareImport(normalized, this.crypto);
        return normalized;
    }

    /** @param {IDBDatabase} database @param {CemStudioRepositoryRequest} request @param {AbortSignal} [signal] */
    async importProject(database, request, signal) {
        const input = parameters(request).bundle;
        if (!input || typeof input !== 'object') {
            throw new CemStudioRepositoryError(
                'cem.studio.repository.import_invalid',
                'import-project requires a bundle object',
            );
        }
        let validated;
        try {
            validated = await this.validateProject(structuredClone(input), { signal, operation: 'import' });
        } catch (error) {
            throw new CemStudioRepositoryError(
                'cem.studio.repository.import_validation_failed',
                `CEM-ML rejected the project import: ${error instanceof Error ? error.message : String(error)}`,
                {},
                error,
            );
        }
        assertNotAborted(signal);
        const bundle = normalizeValidatedBundle(validated);
        const prepared = await prepareImport(bundle, this.crypto);
        assertNotAborted(signal);
        const mode = parameters(request).mode ?? 'create';
        if (mode !== 'create' && mode !== 'replace') {
            throw new CemStudioRepositoryError(
                'cem.studio.repository.import_mode_invalid',
                'import mode must be create or replace',
            );
        }
        const stores = ['meta', 'projects', 'entries', 'resources', 'blobs', 'trash', 'changes', 'searchDocuments'];
        return withTransaction(
            database,
            stores,
            'readwrite',
            async (transaction) => {
                const projects = transaction.objectStore('projects');
                const existing = await requestValue(projects.get(bundle.project.id));
                if (existing && mode === 'create') {
                    throw conflict('project', bundle.project.id, undefined, existing.revision, undefined, undefined);
                }
                if (existing && parameters(request).expectedRevision !== existing.revision) {
                    throw conflict(
                        'project',
                        bundle.project.id,
                        parameters(request).expectedRevision,
                        existing.revision,
                        undefined,
                        undefined,
                    );
                }
                await Promise.all([
                    deleteByIndex(transaction.objectStore('entries').index('byProject'), bundle.project.id),
                    deleteByIndex(transaction.objectStore('resources').index('byProject'), bundle.project.id),
                    deleteByIndex(transaction.objectStore('searchDocuments').index('byProject'), bundle.project.id),
                ]);
                const projectRecord = projectStorageRecord(bundle.project);
                projects.put(projectRecord);
                transaction.objectStore('trash').delete(['project', bundle.project.id, bundle.project.id]);
                for (const entry of bundle.project.entries) {
                    transaction.objectStore('entries').put(entryStorageRecord(bundle.project.id, entry));
                }
                for (const item of prepared.resources) {
                    transaction.objectStore('blobs').put(item.blob);
                    transaction.objectStore('resources').put(resourceStorageRecord(bundle.project.id, item.resource));
                }
                for (const document of searchDocumentsForBundle(bundle.project, prepared.resources)) {
                    transaction.objectStore('searchDocuments').put(document);
                }
                const repositoryRevision = await advanceRepositoryRevision(transaction);
                const changeCursor = await appendChange(transaction, {
                    repositoryRevision,
                    projectId: bundle.project.id,
                    operation: 'import-project',
                    projectRevision: bundle.project.revision,
                    committedAt: this.now(),
                });
                return {
                    value: portableProject(
                        projectRecord,
                        bundle.project.entries.map((entry) => entryStorageRecord(bundle.project.id, entry)),
                        prepared.resources.map((item) => resourceStorageRecord(bundle.project.id, item.resource)),
                    ),
                    repositoryRevision,
                    changeCursor,
                };
            },
            'strict',
        );
    }

    /** @param {IDBDatabase} database @param {CemStudioRepositoryRequest} request @param {AbortSignal} [signal] */
    async saveResource(database, request, signal) {
        const input = parameters(request);
        const projectId = stringParameter(request, 'projectId');
        const resourceId = stringParameter(request, 'resourceId');
        const bytes = toBytes(input.content);
        const sha256 = await sha256Hex(this.crypto, bytes);
        assertNotAborted(signal);
        return withTransaction(
            database,
            ['meta', 'projects', 'resources', 'blobs', 'changes', 'searchDocuments'],
            'readwrite',
            async (transaction) => {
                const projects = transaction.objectStore('projects');
                const resources = transaction.objectStore('resources');
                const project = await requestValue(projects.get(projectId));
                const resource = await requestValue(resources.get([projectId, resourceId]));
                if (!project || !resource) {
                    throw new CemStudioRepositoryError(
                        'cem.studio.repository.resource_not_found',
                        `resource \`${resourceId}\` was not found in project \`${projectId}\``,
                    );
                }
                assertExpectedRevision('project', projectId, input.expectedProjectRevision, project.revision);
                assertExpectedRevision(
                    'resource',
                    resourceId,
                    input.expectedResourceRevision,
                    resource.revision,
                    resource.sha256,
                );
                const updatedAt = typeof input.updatedAt === 'string' ? input.updatedAt : this.now();
                const nextResource = {
                    ...resource,
                    revision: resource.revision + 1,
                    sha256,
                    blobHash: sha256,
                };
                const nextProject = {
                    ...project,
                    revision: project.revision + 1,
                    commitRevision: project.revision + 1,
                    updatedAt,
                };
                transaction
                    .objectStore('blobs')
                    .put({ sha256, bytes: exactArrayBuffer(bytes), byteLength: bytes.byteLength });
                resources.put(nextResource);
                projects.put(nextProject);
                transaction.objectStore('searchDocuments').put(searchDocumentForResource(nextResource, bytes, false));
                const repositoryRevision = await advanceRepositoryRevision(transaction);
                const changeCursor = await appendChange(transaction, {
                    repositoryRevision,
                    projectId,
                    resourceId,
                    operation: 'save-resource',
                    projectRevision: nextProject.revision,
                    resourceRevision: nextResource.revision,
                    sha256,
                    committedAt: updatedAt,
                });
                return {
                    value: {
                        projectRevision: nextProject.revision,
                        resourceRevision: nextResource.revision,
                        sha256,
                    },
                    repositoryRevision,
                    changeCursor,
                };
            },
            'strict',
        );
    }

    /** @param {IDBDatabase} database @param {CemStudioRepositoryRequest} request @param {boolean} trashed */
    async setProjectTrash(database, request, trashed) {
        const input = parameters(request);
        const projectId = stringParameter(request, 'projectId');
        return withTransaction(
            database,
            ['meta', 'projects', 'trash', 'changes', 'searchDocuments'],
            'readwrite',
            async (transaction) => {
                const projects = transaction.objectStore('projects');
                const project = await requestValue(projects.get(projectId));
                if (!project) {
                    throw new CemStudioRepositoryError(
                        'cem.studio.repository.project_not_found',
                        `project \`${projectId}\` was not found`,
                    );
                }
                assertExpectedRevision('project', projectId, input.expectedRevision, project.revision);
                if (Boolean(project.trashedAt) === trashed) {
                    throw new CemStudioRepositoryError(
                        trashed ? 'cem.studio.repository.already_trashed' : 'cem.studio.repository.not_trashed',
                        `project \`${projectId}\` is ${trashed ? 'already' : 'not'} in trash`,
                    );
                }
                const committedAt = this.now();
                const nextProject = {
                    ...project,
                    revision: project.revision + 1,
                    commitRevision: project.revision + 1,
                    updatedAt: committedAt,
                    trashedAt: trashed ? committedAt : null,
                };
                projects.put(nextProject);
                const trashKey = ['project', projectId, projectId];
                if (trashed) {
                    transaction.objectStore('trash').put({
                        key: trashKey,
                        kind: 'project',
                        projectId,
                        entityId: projectId,
                        deletedAt: committedAt,
                        projectRevision: nextProject.revision,
                    });
                } else {
                    transaction.objectStore('trash').delete(trashKey);
                }
                await updateSearchTrash(
                    transaction.objectStore('searchDocuments').index('byProject'),
                    projectId,
                    trashed,
                    nextProject.revision,
                );
                const repositoryRevision = await advanceRepositoryRevision(transaction);
                const operation = trashed ? 'trash-project' : 'restore-project';
                const changeCursor = await appendChange(transaction, {
                    repositoryRevision,
                    projectId,
                    operation,
                    projectRevision: nextProject.revision,
                    committedAt,
                });
                return {
                    value: { projectId, projectRevision: nextProject.revision, trashedAt: nextProject.trashedAt },
                    repositoryRevision,
                    changeCursor,
                };
            },
            'strict',
        );
    }

    ensureBroadcastChannel() {
        if (this.channel || !this.BroadcastChannel) return;
        this.channel = new this.BroadcastChannel(`${this.databaseName}:changes`);
        this.channel.onmessage = (event) => this.receiveChange(event.data);
    }

    /** @param {number} cursor @param {number} repositoryRevision */
    publishChange(cursor, repositoryRevision) {
        const change = {
            protocolVersion: CEM_REPOSITORY_PROTOCOL_VERSION,
            repository: CEM_STUDIO_REPOSITORY_ID,
            cursor,
            repositoryRevision,
        };
        this.receiveChange(change);
        this.channel?.postMessage(change);
    }

    /** @param {unknown} value */
    receiveChange(value) {
        if (!value || typeof value !== 'object') return;
        const change = value;
        if (
            change.protocolVersion !== CEM_REPOSITORY_PROTOCOL_VERSION ||
            change.repository !== CEM_STUDIO_REPOSITORY_ID ||
            !Number.isSafeInteger(change.cursor) ||
            !Number.isSafeInteger(change.repositoryRevision)
        )
            return;
        this.repositoryRevision = Math.max(this.repositoryRevision, change.repositoryRevision);
        for (const subscriber of this.subscribers) {
            if (change.cursor <= subscriber.cursor) continue;
            subscriber.cursor = change.cursor;
            subscriber.notify(structuredClone(change));
        }
    }
}

/** @param {CemStudioRepositoryOptions} options */
export function createCemStudioProjectRepository(options) {
    return new CemStudioIndexedDbRepository(options);
}

/** @param {IDBDatabase} database @param {IDBTransaction | null} transaction @param {number} oldVersion */
function migrateDatabase(database, transaction, oldVersion) {
    if (!transaction) {
        throw new CemStudioRepositoryError(
            'cem.studio.repository.migration_transaction_missing',
            'IndexedDB did not provide the required versionchange transaction',
        );
    }
    if (oldVersion < 1) {
        const meta = database.createObjectStore('meta', { keyPath: 'key' });
        const projects = database.createObjectStore('projects', { keyPath: 'id' });
        projects.createIndex('byUpdatedAt', 'updatedAt');
        projects.createIndex('byTrashedAt', 'trashedAt');
        const entries = database.createObjectStore('entries', { keyPath: 'key' });
        entries.createIndex('byProject', 'projectId');
        entries.createIndex('byParent', ['projectId', 'parentId']);
        entries.createIndex('byKind', 'kind');
        const resources = database.createObjectStore('resources', { keyPath: 'key' });
        resources.createIndex('byProject', 'projectId');
        resources.createIndex('byPath', ['projectId', 'path'], { unique: true });
        resources.createIndex('byHash', 'sha256');
        database.createObjectStore('blobs', { keyPath: 'sha256' });
        const runs = database.createObjectStore('runs', { keyPath: 'key' });
        runs.createIndex('byProject', 'projectId');
        const results = database.createObjectStore('resultSnapshots', { keyPath: 'key' });
        results.createIndex('byProject', 'projectId');
        const providers = database.createObjectStore('providerBindings', { keyPath: 'key' });
        providers.createIndex('byProject', 'projectId');
        const syncQueue = database.createObjectStore('syncQueue', { keyPath: 'id', autoIncrement: true });
        syncQueue.createIndex('byProject', 'projectId');
        const trash = database.createObjectStore('trash', { keyPath: 'key' });
        trash.createIndex('byProject', 'projectId');
        const changes = database.createObjectStore('changes', { keyPath: 'sequence', autoIncrement: true });
        changes.createIndex('byProject', 'projectId');
        changes.createIndex('byRevision', 'repositoryRevision', { unique: true });
        const search = database.createObjectStore('searchDocuments', { keyPath: 'key' });
        search.createIndex('byProject', 'projectId');
        search.createIndex('byKind', 'kind');
        search.createIndex('byToken', 'tokens', { multiEntry: true });
        search.createIndex('byTrigram', 'trigrams', { multiEntry: true });
        search.createIndex('byTrashed', 'trashed');
        meta.put({ key: 'repositoryRevision', value: 0 });
        meta.put({ key: 'logicalSchemaVersion', value: 1 });
        meta.put({ key: 'searchIndexVersion', value: CEM_STUDIO_SEARCH_INDEX_VERSION });
    }
}

/** @param {IDBDatabase} database */
async function readRepositoryRevision(database) {
    return withTransaction(database, ['meta'], 'readonly', async (transaction) => {
        const record = await requestValue(transaction.objectStore('meta').get('repositoryRevision'));
        return Number.isSafeInteger(record?.value) ? record.value : 0;
    });
}

/** @param {IDBTransaction} transaction */
async function advanceRepositoryRevision(transaction) {
    const store = transaction.objectStore('meta');
    const current = await requestValue(store.get('repositoryRevision'));
    const next = (Number.isSafeInteger(current?.value) ? current.value : 0) + 1;
    store.put({ key: 'repositoryRevision', value: next });
    return next;
}

/** @param {IDBTransaction} transaction @param {Record<string, unknown>} change */
async function appendChange(transaction, change) {
    const cursor = await requestValue(transaction.objectStore('changes').add(change));
    if (!Number.isSafeInteger(cursor)) {
        throw new CemStudioRepositoryError(
            'cem.studio.repository.change_cursor_invalid',
            'IndexedDB returned a non-numeric change cursor',
        );
    }
    return cursor;
}

/** @param {IDBDatabase} database @param {boolean} includeTrash */
async function listProjects(database, includeTrash) {
    return withTransaction(database, ['projects'], 'readonly', async (transaction) => {
        const projects = await requestValue(transaction.objectStore('projects').getAll());
        return projects
            .filter((project) => includeTrash || !project.trashedAt)
            .sort((left, right) => left.id.localeCompare(right.id))
            .map(({ commitRevision: _commitRevision, ...project }) => structuredClone(project));
    });
}

/** @param {IDBDatabase} database @param {number} after */
async function listChanges(database, after) {
    return withTransaction(database, ['changes'], 'readonly', async (transaction) => {
        const range = IDBKeyRange.lowerBound(after, true);
        return requestValue(transaction.objectStore('changes').getAll(range));
    });
}

/** @param {IDBDatabase} database @param {Record<string, unknown>} input */
async function searchRepository(database, input) {
    const query = typeof input.query === 'string' ? input.query : '';
    const projectId = typeof input.projectId === 'string' ? input.projectId : undefined;
    const kinds = Array.isArray(input.kinds)
        ? new Set(input.kinds.filter((value) => typeof value === 'string'))
        : undefined;
    const limit = Number.isSafeInteger(input.limit) ? Math.max(1, Math.min(100, input.limit)) : 20;
    const terms = tokenize(query);
    if (terms.length === 0) return [];
    return withTransaction(database, ['searchDocuments'], 'readonly', async (transaction) => {
        const store = transaction.objectStore('searchDocuments');
        const candidateMap = new Map();
        for (const term of terms) {
            for (const document of await requestValue(store.index('byToken').getAll(term))) {
                candidateMap.set(document.key, document);
            }
        }
        if (candidateMap.size === 0) {
            const trigrams = unique(terms.flatMap((term) => termTrigrams(term))).slice(0, 12);
            for (const trigram of trigrams) {
                for (const document of await requestValue(store.index('byTrigram').getAll(trigram))) {
                    candidateMap.set(document.key, document);
                }
            }
        }
        return [...candidateMap.values()]
            .filter((document) => !document.trashed)
            .filter((document) => !projectId || document.projectId === projectId)
            .filter((document) => !kinds || kinds.has(document.kind))
            .map((document) => ({ document, score: searchScore(document, terms) }))
            .filter(({ score }) => score > 0)
            .sort((left, right) => right.score - left.score || left.document.key.localeCompare(right.document.key))
            .slice(0, limit)
            .map(({ document, score }) => ({
                id: document.key,
                projectId: document.projectId,
                kind: document.kind,
                entityId: document.entityId,
                title: document.title,
                path: document.path,
                score,
                snippet: searchSnippet(document.text, terms),
                sourceRevision: document.sourceRevision,
            }));
    });
}

/** @param {unknown} validated */
function normalizeValidatedBundle(validated) {
    if (!validated || typeof validated !== 'object') {
        throw new CemStudioRepositoryError(
            'cem.studio.repository.import_validation_invalid',
            'the CEM-ML validator did not return a normalized bundle',
        );
    }
    const bundle = validated.bundle && typeof validated.bundle === 'object' ? validated.bundle : validated;
    if (
        !bundle.project ||
        typeof bundle.project !== 'object' ||
        !bundle.contents ||
        typeof bundle.contents !== 'object'
    ) {
        throw new CemStudioRepositoryError(
            'cem.studio.repository.import_validation_invalid',
            'the normalized import must contain project and contents objects',
        );
    }
    const project = structuredClone(bundle.project);
    if (
        project.schemaVersion !== 1 ||
        typeof project.id !== 'string' ||
        !Array.isArray(project.entries) ||
        !Array.isArray(project.resources)
    ) {
        throw new CemStudioRepositoryError(
            'cem.studio.repository.import_validation_invalid',
            'the normalized import is not a Studio project v1 projection',
        );
    }
    return { project, contents: bundle.contents };
}

/** @param {{project: Record<string, any>, contents: Record<string, unknown>}} bundle @param {Crypto} crypto */
async function prepareImport(bundle, crypto) {
    const resources = [];
    for (const resource of bundle.project.resources) {
        const content = bundle.contents[resource.id];
        if (content === undefined) {
            throw new CemStudioRepositoryError(
                'cem.studio.repository.import_resource_missing',
                `import bundle does not contain resource \`${resource.id}\``,
            );
        }
        const bytes = toBytes(content);
        const sha256 = await sha256Hex(crypto, bytes);
        if (resource.sha256 !== sha256) {
            throw new CemStudioRepositoryError(
                'cem.studio.repository.import_hash_mismatch',
                `resource \`${resource.id}\` hash does not match its validated manifest`,
                { resourceId: resource.id, expectedHash: resource.sha256, actualHash: sha256 },
            );
        }
        resources.push({
            resource: { ...resource, sha256, blobHash: sha256 },
            bytes,
            blob: { sha256, bytes: exactArrayBuffer(bytes), byteLength: bytes.byteLength },
        });
    }
    return { resources };
}

/** @param {Record<string, any>} project */
function projectStorageRecord(project) {
    const { entries: _entries, resources: _resources, ...metadata } = project;
    return { ...structuredClone(metadata), commitRevision: project.revision, trashedAt: null };
}

/** @param {string} projectId @param {Record<string, any>} entry */
function entryStorageRecord(projectId, entry) {
    return { ...structuredClone(entry), key: [projectId, entry.id], projectId };
}

/** @param {string} projectId @param {Record<string, any>} resource */
function resourceStorageRecord(projectId, resource) {
    return { ...structuredClone(resource), key: [projectId, resource.id], projectId };
}

/** @param {Record<string, any>} project @param {Record<string, any>[]} entries @param {Record<string, any>[]} resources */
function portableProject(project, entries, resources) {
    const { commitRevision: _commitRevision, trashedAt: _trashedAt, ...metadata } = project;
    return {
        ...structuredClone(metadata),
        entries: entries
            .map(({ key: _key, projectId: _projectId, ...entry }) => structuredClone(entry))
            .sort((left, right) => left.id.localeCompare(right.id)),
        resources: resources
            .map(({ key: _key, projectId: _projectId, blobHash: _blobHash, ...resource }) => structuredClone(resource))
            .sort((left, right) => left.id.localeCompare(right.id)),
    };
}

/** @param {Record<string, any>} project @param {Array<{resource: Record<string, any>, bytes: Uint8Array}>} resources */
function searchDocumentsForBundle(project, resources) {
    return [
        searchDocument(
            'project',
            project.id,
            project.id,
            project.name,
            '',
            project.description ?? '',
            [],
            project.revision,
            false,
        ),
        ...project.entries.map((entry) =>
            searchDocument(
                'entry',
                project.id,
                entry.id,
                entry.name,
                '',
                entry.description ?? '',
                entry.tags ?? [],
                project.revision,
                false,
            ),
        ),
        ...resources.map(({ resource, bytes }) =>
            searchDocumentForResource(resourceStorageRecord(project.id, resource), bytes, false),
        ),
    ];
}

/** @param {Record<string, any>} resource @param {Uint8Array} bytes @param {boolean} trashed */
function searchDocumentForResource(resource, bytes, trashed) {
    const text = searchableText(resource.contentType, bytes);
    return searchDocument(
        'resource',
        resource.projectId,
        resource.id,
        resource.path ?? resource.id,
        resource.path ?? '',
        text,
        [resource.role, resource.contentType, resource.schema].filter((value) => typeof value === 'string'),
        resource.revision,
        trashed,
    );
}

/** @param {string} kind @param {string} projectId @param {string} entityId @param {string} title @param {string} path @param {string} text @param {string[]} tags @param {number} sourceRevision @param {boolean} trashed */
function searchDocument(kind, projectId, entityId, title, path, text, tags, sourceRevision, trashed) {
    const tokens = unique(tokenize(`${title} ${path} ${tags.join(' ')} ${text}`)).slice(0, MAX_SEARCH_TERMS);
    return {
        key: `${projectId}:${kind}:${entityId}`,
        projectId,
        kind,
        entityId,
        title,
        path,
        text,
        tags: unique(tags.map(normalizeSearchText).filter(Boolean)),
        tokens,
        trigrams: unique(tokens.flatMap((token) => termTrigrams(token))).slice(0, MAX_SEARCH_TERMS),
        sourceRevision,
        trashed,
    };
}

/** @param {string} contentType @param {Uint8Array} bytes */
function searchableText(contentType, bytes) {
    if (bytes.byteLength > MAX_SEARCH_TEXT_BYTES) return '';
    if (!/^(?:text\/|application\/(?:json|cem|xml|yaml|javascript)|[^;]+\+(?:json|xml))/.test(contentType)) return '';
    try {
        return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    } catch {
        return '';
    }
}

/** @param {IDBIndex} index @param {IDBValidKey} key */
function deleteByIndex(index, key) {
    return cursorMutation(index.openKeyCursor(IDBKeyRange.only(key)), (cursor) => cursor.delete());
}

/** @param {IDBIndex} index @param {string} projectId @param {boolean} trashed @param {number} projectRevision */
function updateSearchTrash(index, projectId, trashed, projectRevision) {
    return cursorMutation(index.openCursor(IDBKeyRange.only(projectId)), (cursor) => {
        cursor.update({
            ...cursor.value,
            trashed,
            ...(cursor.value.kind === 'project' ? { sourceRevision: projectRevision } : {}),
        });
    });
}

/** @param {IDBRequest<IDBCursor | IDBCursorWithValue | null>} request @param {(cursor: IDBCursor | IDBCursorWithValue) => void} mutate */
function cursorMutation(request, mutate) {
    return new Promise((resolve, reject) => {
        request.onerror = () => reject(request.error);
        request.onsuccess = () => {
            const cursor = request.result;
            if (!cursor) {
                resolve(undefined);
                return;
            }
            mutate(cursor);
            cursor.continue();
        };
    });
}

/** @param {IDBDatabase} database @param {string[]} storeNames @param {IDBTransactionMode} mode @param {(transaction: IDBTransaction) => Promise<any>} callback @param {IDBTransactionDurability} [durability] */
async function withTransaction(database, storeNames, mode, callback, durability) {
    const transaction = database.transaction(storeNames, mode, durability ? { durability } : undefined);
    const completion = transactionCompletion(transaction);
    try {
        const value = await callback(transaction);
        await completion;
        return value;
    } catch (error) {
        try {
            transaction.abort();
        } catch {
            // The transaction already committed or aborted; preserve the original error.
        }
        await completion.catch(() => undefined);
        throw normalizeRepositoryError(error, 'transaction');
    }
}

/** @param {IDBTransaction} transaction */
function transactionCompletion(transaction) {
    return new Promise((resolve, reject) => {
        transaction.oncomplete = () => resolve(undefined);
        transaction.onabort = () => reject(transaction.error ?? new DOMException('transaction aborted', 'AbortError'));
        transaction.onerror = () => reject(transaction.error ?? new DOMException('transaction failed', 'UnknownError'));
    });
}

/** @template T @param {IDBRequest<T>} request @returns {Promise<T>} */
function requestValue(request) {
    return new Promise((resolve, reject) => {
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });
}

/** @param {CemStudioRepositoryRequest} request */
function assertRequest(request) {
    if (
        !request ||
        request.protocolVersion !== CEM_REPOSITORY_PROTOCOL_VERSION ||
        request.repository !== CEM_STUDIO_REPOSITORY_ID ||
        !/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/.test(request.operation) ||
        !Number.isSafeInteger(request.requestRevision) ||
        request.requestRevision < 0
    ) {
        throw new CemStudioRepositoryError(
            'cem.studio.repository.request_invalid',
            'repository request does not satisfy protocol v1',
        );
    }
}

/** @param {CemStudioRepositoryRequest} request */
function parameters(request) {
    return request.parameters && typeof request.parameters === 'object' ? request.parameters : {};
}

/** @param {CemStudioRepositoryRequest} request @param {string} name */
function stringParameter(request, name) {
    const value = parameters(request)[name];
    if (typeof value !== 'string' || value.length === 0) {
        throw new CemStudioRepositoryError(
            'cem.studio.repository.parameter_invalid',
            `repository parameter \`${name}\` must be a non-empty string`,
        );
    }
    return value;
}

/** @param {CemStudioRepositoryRequest} request @param {string} name @param {number} fallback */
function numberParameter(request, name, fallback) {
    const value = parameters(request)[name];
    if (value === undefined) return fallback;
    if (!Number.isSafeInteger(value) || value < 0) {
        throw new CemStudioRepositoryError(
            'cem.studio.repository.parameter_invalid',
            `repository parameter \`${name}\` must be a non-negative safe integer`,
        );
    }
    return value;
}

/** @param {string} value @param {string} label */
function assertIdentity(value, label) {
    if (typeof value !== 'string' || value.length === 0) {
        throw new CemStudioRepositoryError(
            'cem.studio.repository.identity_invalid',
            `${label} must be a non-empty string`,
        );
    }
}

/** @param {AbortSignal} [signal] */
function assertNotAborted(signal) {
    if (signal?.aborted) throw signal.reason ?? new DOMException('operation aborted', 'AbortError');
}

/** @param {string} kind @param {string} id @param {unknown} expected @param {number} current @param {string} [currentHash] */
function assertExpectedRevision(kind, id, expected, current, currentHash) {
    if (!Number.isSafeInteger(expected) || expected !== current) {
        throw conflict(kind, id, expected, current, undefined, currentHash);
    }
}

/** @param {string} kind @param {string} id @param {unknown} expectedRevision @param {number} currentRevision @param {string} [expectedHash] @param {string} [currentHash] */
function conflict(kind, id, expectedRevision, currentRevision, expectedHash, currentHash) {
    return new CemStudioRepositoryError(
        'cem.studio.repository.revision_conflict',
        `${kind} \`${id}\` changed after the caller's base revision`,
        { kind, id, expectedRevision, currentRevision, expectedHash, currentHash },
    );
}

/** @param {string} operation @param {string} disposition */
function unsupportedOperation(operation, disposition) {
    return new CemStudioRepositoryError(
        'cem.studio.repository.operation_unsupported',
        `${disposition} operation \`${operation}\` is unsupported`,
    );
}

/** @param {CemStudioRepositoryRequest} request @param {number} repositoryRevision @param {unknown} value */
function response(request, repositoryRevision, value) {
    return {
        protocolVersion: CEM_REPOSITORY_PROTOCOL_VERSION,
        repository: CEM_STUDIO_REPOSITORY_ID,
        operation: request.operation,
        requestRevision: request.requestRevision,
        repositoryRevision,
        value: structuredClone(value),
        diagnostics: [],
    };
}

/** @param {string} code @param {string} message @param {'info' | 'warning' | 'error' | 'fatal'} severity */
function diagnostic(code, message, severity) {
    return { code, message, severity };
}

/** @param {unknown} error @param {string} operation */
export function normalizeRepositoryError(error, operation) {
    if (error instanceof CemStudioRepositoryError) return error;
    if (error instanceof DOMException) {
        const codes = {
            AbortError: 'cem.studio.repository.aborted',
            QuotaExceededError: 'cem.studio.repository.quota_exceeded',
            VersionError: 'cem.studio.repository.version_unsupported',
            InvalidStateError: 'cem.studio.repository.indexed_db_unavailable',
            NotReadableError: 'cem.studio.repository.not_readable',
        };
        const code = codes[error.name] ?? 'cem.studio.repository.indexed_db_failed';
        return new CemStudioRepositoryError(code, `${operation} failed: ${error.message || error.name}`, {}, error);
    }
    return new CemStudioRepositoryError(
        'cem.studio.repository.failed',
        `${operation} failed: ${error instanceof Error ? error.message : String(error)}`,
        {},
        error,
    );
}

/** @param {Crypto} crypto @param {Uint8Array} bytes */
async function sha256Hex(crypto, bytes) {
    if (!crypto?.subtle) {
        throw new CemStudioRepositoryError(
            'cem.studio.repository.crypto_unavailable',
            'Web Crypto SHA-256 is required for project persistence',
        );
    }
    const digest = await crypto.subtle.digest('SHA-256', exactArrayBuffer(bytes));
    return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

/** @param {unknown} content */
function toBytes(content) {
    if (typeof content === 'string') return new TextEncoder().encode(content);
    if (content instanceof ArrayBuffer) return new Uint8Array(content.slice(0));
    if (ArrayBuffer.isView(content)) {
        return new Uint8Array(content.buffer.slice(content.byteOffset, content.byteOffset + content.byteLength));
    }
    throw new CemStudioRepositoryError(
        'cem.studio.repository.content_invalid',
        'resource content must be a string, ArrayBuffer, or typed-array view',
    );
}

/** @param {Uint8Array} bytes */
function exactArrayBuffer(bytes) {
    return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
}

/** @param {string} value */
function normalizeSearchText(value) {
    return value
        .normalize('NFKD')
        .toLocaleLowerCase('und')
        .replace(/\p{M}+/gu, '');
}

/** @param {string} value */
function tokenize(value) {
    return normalizeSearchText(value)
        .split(/[^\p{L}\p{N}_-]+/u)
        .filter(Boolean);
}

/** @param {string} term */
function termTrigrams(term) {
    if (term.length < 3) return [term];
    const padded = `  ${term} `;
    return Array.from({ length: padded.length - 2 }, (_, index) => padded.slice(index, index + 3));
}

/** @template T @param {T[]} values */
function unique(values) {
    return [...new Set(values)];
}

/** @param {Record<string, any>} document @param {string[]} terms */
function searchScore(document, terms) {
    const title = normalizeSearchText(document.title);
    const path = normalizeSearchText(document.path ?? '');
    const text = normalizeSearchText(document.text);
    const tokens = new Set(document.tokens);
    const tags = new Set(document.tags);
    let score = 0;
    for (const term of terms) {
        if (title === term) score += 40;
        else if (title.includes(term)) score += 20;
        if (tokens.has(term)) score += 12;
        if (tags.has(term)) score += 10;
        if (path.includes(term)) score += 8;
        if (text.includes(term)) score += 4;
        const requested = new Set(termTrigrams(term));
        const overlap = document.trigrams.filter((trigram) => requested.has(trigram)).length;
        score += requested.size === 0 ? 0 : overlap / requested.size;
    }
    return Number(score.toFixed(4));
}

/** @param {string} text @param {string[]} terms */
function searchSnippet(text, terms) {
    if (!text) return '';
    const normalized = normalizeSearchText(text);
    const offsets = terms.map((term) => normalized.indexOf(term)).filter((offset) => offset >= 0);
    const offset = offsets.length > 0 ? Math.min(...offsets) : 0;
    const start = Math.max(0, offset - 60);
    const end = Math.min(text.length, start + 180);
    return `${start > 0 ? '…' : ''}${text.slice(start, end)}${end < text.length ? '…' : ''}`;
}
