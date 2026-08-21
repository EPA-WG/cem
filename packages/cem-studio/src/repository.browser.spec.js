import { afterEach, describe, expect, it, vi } from 'vitest';

import {
    CEM_STUDIO_DATABASE_VERSION,
    CEM_STUDIO_REPOSITORY_ID,
    createCemStudioProjectRepository,
    normalizeRepositoryError,
} from './repository.js';

const repositories = [];

afterEach(async () => {
    const names = [...new Set(repositories.map((repository) => repository.databaseName))];
    for (const repository of repositories.splice(0)) repository.close();
    for (const name of names) await deleteDatabase(name);
});

describe('CemStudioIndexedDbRepository', () => {
    it('creates the complete v1 schema and reports browser storage health', async () => {
        const storage = {
            estimate: vi.fn(async () => ({ usage: 1024, quota: 4096 })),
            persisted: vi.fn(async () => true),
        };
        const repository = createRepository({ storage });
        const database = await repository.open();

        expect(database.version).toBe(CEM_STUDIO_DATABASE_VERSION);
        expect([...database.objectStoreNames]).toEqual([
            'blobs',
            'changes',
            'entries',
            'meta',
            'projects',
            'providerBindings',
            'resources',
            'resultSnapshots',
            'runs',
            'searchDocuments',
            'syncQueue',
            'trash',
        ]);
        await expect(repository.status()).resolves.toMatchObject({
            repository: CEM_STUDIO_REPOSITORY_ID,
            state: 'ready',
            schemaVersion: 1,
            repositoryRevision: 0,
            usage: 1024,
            quota: 4096,
            persisted: true,
        });
    });

    it('validates before an atomic import and exports the same project bytes', async () => {
        const validateProject = vi.fn(async (bundle) => bundle);
        const repository = createRepository({ validateProject });
        const bundle = await projectBundle('Tour source');

        const imported = await repository.execute(command('import-project', { bundle }));
        expect(validateProject).toHaveBeenCalledTimes(1);
        expect(imported.repositoryRevision).toBe(1);
        expect(imported.changeCursor).toBe(1);

        const exported = await repository.query(query('export-project', { projectId: 'feature-tour' }));
        expect(validateProject).toHaveBeenCalledTimes(2);
        expect(validateProject.mock.calls.map(([, options]) => options.operation)).toEqual(['import', 'export']);
        expect(exported.value.project).toEqual(bundle.project);
        expect(new TextDecoder().decode(exported.value.contents['tour-source'])).toBe('Tour source');
        await expect(repository.query(query('list-changes', { after: 0 }))).resolves.toMatchObject({
            value: [{ operation: 'import-project', projectId: 'feature-tour', repositoryRevision: 1, sequence: 1 }],
        });
    });

    it('rejects failed validation and hash mismatches before any project write', async () => {
        const rejected = createRepository({
            validateProject: async () => {
                throw new Error('schema rejected');
            },
        });
        await expect(
            rejected.execute(command('import-project', { bundle: await projectBundle('invalid') })),
        ).rejects.toMatchObject({ code: 'cem.studio.repository.import_validation_failed' });
        await expect(rejected.query(query('list-projects'))).resolves.toMatchObject({ value: [] });

        const mismatch = createRepository();
        const bundle = await projectBundle('actual bytes');
        bundle.project.resources[0].sha256 = '0'.repeat(64);
        await expect(mismatch.execute(command('import-project', { bundle }))).rejects.toMatchObject({
            code: 'cem.studio.repository.import_hash_mismatch',
        });
        await expect(mismatch.query(query('list-projects'))).resolves.toMatchObject({ value: [] });
    });

    it('atomically autosaves revisions, content hashes, search records, and conflict checks', async () => {
        const repository = createRepository();
        await repository.execute(command('import-project', { bundle: await projectBundle('Tour source') }));

        const saved = await repository.execute(
            command('save-resource', {
                projectId: 'feature-tour',
                resourceId: 'tour-source',
                expectedProjectRevision: 1,
                expectedResourceRevision: 1,
                content: 'Updated searchable transformation source',
                updatedAt: '2026-08-21T01:00:00Z',
            }),
        );
        expect(saved.value).toMatchObject({ projectRevision: 2, resourceRevision: 2 });
        expect(saved.value.sha256).toMatch(/^[a-f0-9]{64}$/);

        const search = await repository.query(query('search', { query: 'transformation', projectId: 'feature-tour' }));
        expect(search.value).toEqual([
            expect.objectContaining({
                id: 'feature-tour:resource:tour-source',
                entityId: 'tour-source',
                sourceRevision: 2,
            }),
        ]);

        await expect(
            repository.execute(
                command('save-resource', {
                    projectId: 'feature-tour',
                    resourceId: 'tour-source',
                    expectedProjectRevision: 1,
                    expectedResourceRevision: 1,
                    content: 'stale overwrite',
                }),
            ),
        ).rejects.toMatchObject({
            code: 'cem.studio.repository.revision_conflict',
            details: { expectedRevision: 1, currentRevision: 2 },
        });
        const exported = await repository.query(query('export-project', { projectId: 'feature-tour' }));
        expect(new TextDecoder().decode(exported.value.contents['tour-source'])).toBe(
            'Updated searchable transformation source',
        );
    });

    it('trashes and restores without losing identity or returning deleted search results', async () => {
        const repository = createRepository();
        await repository.execute(command('import-project', { bundle: await projectBundle('Tour source') }));
        await repository.execute(command('trash-project', { projectId: 'feature-tour', expectedRevision: 1 }));

        await expect(repository.query(query('list-projects'))).resolves.toMatchObject({ value: [] });
        await expect(repository.query(query('search', { query: 'tour' }))).resolves.toMatchObject({ value: [] });
        const trashed = await repository.query(query('list-projects', { includeTrash: true }));
        expect(trashed.value[0]).toMatchObject({ id: 'feature-tour', revision: 2 });
        expect(trashed.value[0].trashedAt).toBeTruthy();

        await repository.execute(command('restore-project', { projectId: 'feature-tour', expectedRevision: 2 }));
        const restored = await repository.query(query('get-project', { projectId: 'feature-tour' }));
        expect(restored.value).toMatchObject({ id: 'feature-tour', revision: 3 });
        const search = await repository.query(query('search', { query: 'tour' }));
        expect(search.value).toContainEqual(expect.objectContaining({ projectId: 'feature-tour' }));
        expect(search.value.find((result) => result.kind === 'project')).toMatchObject({ sourceRevision: 3 });
    });

    it('uses durable revisions for multi-instance conflicts and BroadcastChannel invalidation', async () => {
        const databaseName = uniqueDatabaseName();
        const first = createRepository({ databaseName });
        const second = createRepository({ databaseName });
        await Promise.all([first.open(), second.open()]);
        await first.execute(command('import-project', { bundle: await projectBundle('Tour source') }));

        const changeReceived = new Promise((resolve, reject) => {
            const timeout = setTimeout(() => reject(new Error('cross-tab change was not delivered')), 2_000);
            second.subscribe(1, (change) => {
                clearTimeout(timeout);
                resolve(change);
            });
        });
        await first.execute(
            command('save-resource', {
                projectId: 'feature-tour',
                resourceId: 'tour-source',
                expectedProjectRevision: 1,
                expectedResourceRevision: 1,
                content: 'second revision',
            }),
        );
        await expect(changeReceived).resolves.toMatchObject({ cursor: 2, repositoryRevision: 2 });

        await expect(
            second.execute(
                command('save-resource', {
                    projectId: 'feature-tour',
                    resourceId: 'tour-source',
                    expectedProjectRevision: 1,
                    expectedResourceRevision: 1,
                    content: 'stale second tab',
                }),
            ),
        ).rejects.toMatchObject({ code: 'cem.studio.repository.revision_conflict' });
        await expect(second.query(query('list-changes', { after: 1 }))).resolves.toMatchObject({
            value: [expect.objectContaining({ sequence: 2, operation: 'save-resource' })],
        });
    });

    it('normalizes quota and version failures into stable diagnostics', () => {
        expect(normalizeRepositoryError(new DOMException('full', 'QuotaExceededError'), 'save')).toMatchObject({
            code: 'cem.studio.repository.quota_exceeded',
        });
        expect(normalizeRepositoryError(new DOMException('future', 'VersionError'), 'open')).toMatchObject({
            code: 'cem.studio.repository.version_unsupported',
        });
    });
});

function createRepository(overrides = {}) {
    const repository = createCemStudioProjectRepository({
        databaseName: overrides.databaseName ?? uniqueDatabaseName(),
        validateProject: overrides.validateProject ?? (async (bundle) => bundle),
        storage: overrides.storage,
        now: () => '2026-08-21T00:00:00Z',
    });
    repositories.push(repository);
    return repository;
}

function command(operation, parameters = {}) {
    return {
        protocolVersion: 1,
        repository: CEM_STUDIO_REPOSITORY_ID,
        operation,
        requestRevision: 1,
        parameters,
    };
}

function query(operation, parameters = {}) {
    return command(operation, parameters);
}

async function projectBundle(content) {
    const bytes = new TextEncoder().encode(content);
    const sha256 = await digest(bytes);
    return {
        project: {
            $schema: 'https://cem.dev/ns/studio/project/1',
            schemaVersion: 1,
            id: 'feature-tour',
            name: 'CEM-ML Feature Tour',
            description: 'Editable local-first Studio seed',
            rootUri: 'studio://feature-tour/',
            revision: 1,
            createdAt: '2026-08-20T00:00:00Z',
            updatedAt: '2026-08-20T00:00:00Z',
            entries: [
                {
                    id: 'validate-source',
                    kind: 'validation',
                    name: 'Validate source',
                    description: 'Run local validation',
                    resourceIds: ['tour-source'],
                    tags: ['tour', 'validation'],
                },
            ],
            resources: [
                {
                    id: 'tour-source',
                    role: 'data',
                    sourceKind: 'project-file',
                    path: 'data/tour.cem',
                    contentType: 'application/cem',
                    schema: 'https://cem.dev/ns/cem-ml/1',
                    revision: 1,
                    sha256,
                },
            ],
        },
        contents: { 'tour-source': bytes },
    };
}

async function digest(bytes) {
    const value = await crypto.subtle.digest('SHA-256', bytes);
    return [...new Uint8Array(value)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function uniqueDatabaseName() {
    return `cem-studio-test-${crypto.randomUUID()}`;
}

function deleteDatabase(name) {
    return new Promise((resolve, reject) => {
        const request = indexedDB.deleteDatabase(name);
        request.onsuccess = () => resolve(undefined);
        request.onerror = () => reject(request.error);
        request.onblocked = () => reject(new Error(`database ${name} remained open`));
    });
}
