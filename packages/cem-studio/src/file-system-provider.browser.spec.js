import { afterEach, describe, expect, it, vi } from 'vitest';

import {
    CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID,
    createCemStudioFileSystemProvider,
    loadCemStudioExampleProjects,
    serializeCemStudioProjectBundle,
} from './file-system-provider.js';
import { CEM_STUDIO_REPOSITORY_ID, createCemStudioProjectRepository } from './repository.js';

const repositories = [];
const opfsEntries = [];

afterEach(async () => {
    const names = [...new Set(repositories.map((repository) => repository.databaseName))];
    for (const repository of repositories.splice(0)) repository.close();
    for (const name of names) await deleteDatabase(name);
    const root = await navigator.storage.getDirectory();
    for (const name of opfsEntries.splice(0)) await root.removeEntry(name, { recursive: true }).catch(() => undefined);
});

describe('CEM Studio File System Access provider', () => {
    it('loads arbitrary example projects lazily with catalog compatibility and archive integrity', async () => {
        const repository = await repositoryWithProject('Owned example');
        const bundle = await exportProject(repository);
        const archive = serializeCemStudioProjectBundle(bundle);
        const sha256 = [...new Uint8Array(await crypto.subtle.digest('SHA-256', archive))].map(value => value.toString(16).padStart(2, '0')).join('');
        const legacyCollections = ['conference', 'conference/2026', 'My examples', '日本語', ' padded identity '];
        const record = { id: 'custom-example', name: 'Custom example', version: '1.0.0', collections: legacyCollections, archive: './custom.cem-studio.json', sha256 };
        const catalog = { schemaVersion: 1, commonVersion: 'test', projects: [record], starter: record };
        const fetch = vi.fn(async url => new Response(String(url).endsWith('/index.json') ? JSON.stringify(catalog) : archive));
        const options = { baseUrl: 'https://studio.test/examples/', commonVersion: 'test', fetch };
        const loaded = await loadCemStudioExampleProjects(options);
        expect(fetch).toHaveBeenCalledTimes(1);
        expect(loaded.examples[0]).toMatchObject({ id: 'custom-example', name: 'Custom example', collections: legacyCollections });
        const decoded = { project: bundle.project, contents: Object.fromEntries(Object.entries(bundle.contents).map(([id, bytes]) => [id, new Uint8Array(bytes)])) };
        expect(await loaded.examples[0].load()).toEqual(decoded);
        expect(await loaded.starter.load()).toEqual(decoded);
        await expect(loadCemStudioExampleProjects({ ...options, commonVersion: 'other' })).rejects.toThrow('incompatible');
        catalog.projects.push(record);
        await expect(loadCemStudioExampleProjects(options)).rejects.toThrow('duplicate');
        catalog.projects.pop();
        const corrupt = await loadCemStudioExampleProjects({ ...options, fetch: async url => new Response(String(url).endsWith('/index.json') ? JSON.stringify(catalog) : 'corrupt') });
        await expect(corrupt.examples[0].load()).rejects.toThrow('integrity');
        catalog.projects[0] = { ...record, archive: '../outside.json' };
        await expect(loadCemStudioExampleProjects(options)).rejects.toThrow('metadata');
    });

    it('accepts immutable nested discovery metadata without loading or duplicating ordinary projects', async () => {
        const discoveryCollections = ['presentation/2026', 'My examples', '日本語', ' padded identity '];
        const record = { id: 'shared-project', name: 'Shared project', description: 'An ordinary project in multiple discovery folders.',
            version: '1.0.0', collections: discoveryCollections, archive: './shared.cem-studio.json', sha256: '0'.repeat(64) };
        const catalog = { schemaVersion: 1, commonVersion: 'test', collections: [
            { id: 'presentation/2026', name: 'Presentation' },
            { id: 'content-types', name: 'Content types and schemas', description: 'Discover supported content.' },
            { id: 'My examples', name: 'My examples', parentId: 'presentation/2026' },
            { id: '日本語', name: '日本語', parentId: 'My examples' },
            { id: ' padded identity ', name: 'Padded identity', parentId: '日本語' },
            { id: 'schema-authoring', name: 'Schema authoring', parentId: 'content-types' },
        ], projects: [record], starter: { ...record, id: 'starter', collections: [] } };
        const fetch = vi.fn(async () => new Response(JSON.stringify(catalog)));
        const loaded = await loadCemStudioExampleProjects({ baseUrl: 'https://studio.test/examples/', commonVersion: 'test', fetch });
        expect(fetch).toHaveBeenCalledTimes(1);
        expect(loaded.examples).toHaveLength(1);
        expect(loaded.examples[0]).toMatchObject({ id: 'shared-project', description: record.description, collections: discoveryCollections });
        expect(loaded.catalog.collections).toEqual(catalog.collections);
        expect(loaded.catalog.collections.at(-1)).toEqual({ id: 'schema-authoring', name: 'Schema authoring', parentId: 'content-types' });
        expect(Object.isFrozen(loaded.catalog)).toBe(true);
        expect(Object.isFrozen(loaded.catalog.collections)).toBe(true);
        expect(Object.isFrozen(loaded.catalog.collections[2])).toBe(true);
        expect(Object.isFrozen(loaded.catalog.projects[0])).toBe(true);
        expect(Object.isFrozen(loaded.catalog.projects[0].collections)).toBe(true);
        expect(Reflect.set(loaded.catalog.collections[2], 'parentId', 'presentation')).toBe(false);
        expect(Reflect.set(loaded.catalog.projects[0], 'archive', './changed.cem-studio.json')).toBe(false);
        expect(() => loaded.examples[0].collections.push('schema-authoring')).toThrow(TypeError);
        catalog.collections[2].parentId = 'content-types';
        record.collections.push('schema-authoring');
        expect(loaded.catalog.collections[2].parentId).toBe('presentation/2026');
        expect(loaded.examples[0].collections).toEqual(['presentation/2026', 'My examples', '日本語', ' padded identity ']);
    });

    it('rejects malformed collection hierarchies and memberships before fetching an archive', async () => {
        const record = { id: 'example', name: 'Example', version: '1.0.0', collections: ['data'],
            archive: './example.cem-studio.json', sha256: '0'.repeat(64) };
        const catalog = { schemaVersion: 1, commonVersion: 'test', collections: [
            { id: 'content-types', name: 'Content types and schemas' }, { id: 'data', name: 'Data', parentId: 'content-types' },
        ], projects: [record], starter: { ...record, id: 'starter', collections: [] } };
        const invalid = [
            { ...catalog, collections: null },
            { ...catalog, collections: [null] },
            { ...catalog, collections: [{ id: '', name: 'Invalid' }] },
            { ...catalog, collections: [{ id: '   ', name: 'Invalid' }] },
            { ...catalog, collections: [{ id: 'bad\u0000id', name: 'Invalid' }] },
            { ...catalog, collections: [{ id: 'bad\u007fid', name: 'Invalid' }] },
            { ...catalog, collections: [{ id: 'data', name: '   ' }] },
            { ...catalog, collections: [{ id: 'data', name: 'Data', description: 1 }] },
            { ...catalog, collections: [{ id: 'data', name: 'Data', parentId: '' }] },
            { ...catalog, collections: [catalog.collections[0], catalog.collections[0]] },
            { ...catalog, collections: [{ id: 'data', name: 'Data', parentId: 'missing' }] },
            { ...catalog, collections: [{ id: 'data', name: 'Data', parentId: 'data' }] },
            { ...catalog, collections: [{ id: 'data', name: 'Data', parentId: 'other' }, { id: 'other', name: 'Other', parentId: 'data' }] },
            { ...catalog, collections: [] },
            { ...catalog, projects: [{ ...record, collections: ['missing'] }] },
            { ...catalog, projects: [{ ...record, collections: ['data', 'data'] }] },
            { ...catalog, projects: [{ ...record, collections: [''] }] },
            { ...catalog, projects: [{ ...record, collections: [' \t '] }] },
            { ...catalog, projects: [{ ...record, collections: ['bad\u0085id'] }] },
            { ...catalog, projects: [{ ...record, collections: 'data' }] },
            { ...catalog, starter: { ...record, collections: ['missing'] } },
        ];
        for (const metadata of invalid) {
            const fetch = vi.fn(async () => new Response(JSON.stringify(metadata)));
            await expect(loadCemStudioExampleProjects({ baseUrl: 'https://studio.test/examples/', commonVersion: 'test', fetch })).rejects.toThrow(/collection|metadata/);
            expect(fetch).toHaveBeenCalledTimes(1);
        }
    });

    it('retains an exact resource handle, pulls external bytes, and rejects stale write-back before truncation', async () => {
        const repository = await repositoryWithProject('IndexedDB source');
        const directory = await fixtureDirectory();
        const handle = await directory.getFileHandle('source.cem', { create: true });
        await writeHandle(handle, 'External source');
        const provider = createProvider(repository);

        await expect(provider.openResource({
            projectId: 'file-project',
            resourceId: 'source',
            handle,
        })).resolves.toMatchObject({ status: 'opened', projectRevision: 2, resourceRevision: 2 });
        const imported = await exportProject(repository);
        expect(text(imported.contents.source)).toBe('External source');
        await expect(repository.query(command('get-provider-binding', {
            projectId: 'file-project',
            scope: 'resource',
            resourceId: 'source',
        }))).resolves.toMatchObject({
            value: {
                provider: CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID,
                handle: { kind: 'file', name: 'source.cem' },
                base: { resourceRevision: 2 },
            },
        });

        await repository.execute(command('save-resource', {
            projectId: 'file-project',
            resourceId: 'source',
            expectedProjectRevision: 2,
            expectedResourceRevision: 2,
            content: 'Local edit',
        }));
        await expect(provider.writeResource({
            projectId: 'file-project',
            resourceId: 'source',
        })).resolves.toMatchObject({ status: 'written', projectRevision: 3, resourceRevision: 3 });
        expect(await readHandle(handle)).toBe('Local edit');

        await writeHandle(handle, 'External concurrent edit');
        await repository.execute(command('save-resource', {
            projectId: 'file-project',
            resourceId: 'source',
            expectedProjectRevision: 3,
            expectedResourceRevision: 3,
            content: 'Second local edit',
        }));
        await expect(provider.writeResource({
            projectId: 'file-project',
            resourceId: 'source',
        })).rejects.toMatchObject({
            code: 'cem.studio.file_system.external_conflict',
            details: { recommendedAction: 'review-and-rebind' },
        });
        expect(await readHandle(handle)).toBe('External concurrent edit');
    });

    it('creates a portable directory, retains it across repository reopen, and imports exact directory bytes', async () => {
        const repository = await repositoryWithProject('Directory source');
        const directory = await fixtureDirectory();
        const provider = createProvider(repository);

        await expect(provider.bindProjectDirectory({
            projectId: 'file-project',
            handle: directory,
        })).resolves.toMatchObject({ status: 'bound', missingFiles: 2 });
        await expect(provider.writeProjectDirectory({ projectId: 'file-project' })).resolves.toMatchObject({
            status: 'written',
            fileCount: 3,
        });
        expect(await readPath(directory, 'data/source.cem')).toBe('Directory source');
        expect(await readPath(directory, 'snapshots/remote.cem')).toBe('Retained remote snapshot');
        expect(JSON.parse(await readPath(directory, 'project.cem'))).toMatchObject({
            id: 'file-project',
            rootUri: 'studio://file-project/',
        });

        const databaseName = repository.databaseName;
        repository.close();
        const reopened = createRepository(databaseName);
        await expect(createProvider(reopened).reconnect({ projectId: 'file-project' })).resolves.toMatchObject({
            state: 'ready',
            name: directory.name,
        });

        const importedRepository = createRepository();
        await expect(createProvider(importedRepository).openProjectDirectory({ handle: directory })).resolves.toMatchObject({
            status: 'imported',
            projectId: 'file-project',
        });
        const imported = await exportProject(importedRepository);
        expect(imported.project.rootUri).toBe('studio://file-project/');
        expect(text(imported.contents.source)).toBe('Directory source');
        expect(text(imported.contents.remote)).toBe('Retained remote snapshot');

        const externalHandle = await pathHandle(directory, 'data/source.cem');
        await writeHandle(externalHandle, 'External directory edit');
        await reopened.execute(command('save-resource', {
            projectId: 'file-project',
            resourceId: 'source',
            expectedProjectRevision: 1,
            expectedResourceRevision: 1,
            content: 'Local directory edit',
        }));
        await expect(createProvider(reopened).writeProjectDirectory({ projectId: 'file-project' })).rejects.toMatchObject({
            code: 'cem.studio.file_system.external_conflict',
        });
        expect(await readHandle(externalHandle)).toBe('External directory edit');
    });

    it('never requests retained permission implicitly and keeps fallback import/export available when unsupported', async () => {
        const handle = {
            kind: 'directory',
            name: 'permission-fixture',
            queryPermission: vi.fn(async () => 'prompt'),
            requestPermission: vi.fn(async () => 'denied'),
        };
        const execute = vi.fn(async (request) => ({
            value: request.operation === 'put-provider-binding'
                ? { ...request.parameters.binding, revision: 2 }
                : undefined,
        }));
        const repository = {
            query: vi.fn(async (request) => ({
                value: request.operation === 'get-provider-binding'
                    ? {
                        provider: CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID,
                        scope: 'project',
                        projectId: 'file-project',
                        handle,
                        name: handle.name,
                        permission: 'prompt',
                        base: {},
                        revision: 1,
                    }
                    : { id: 'file-project', revision: 1 },
            })),
            execute,
        };
        const retained = createCemStudioFileSystemProvider({
            repository,
            showOpenFilePicker: null,
            showDirectoryPicker: null,
        });
        await expect(retained.reconnect({ projectId: 'file-project' })).resolves.toMatchObject({
            state: 'prompt-permission',
        });
        expect(handle.requestPermission).not.toHaveBeenCalled();
        await expect(retained.reconnect({
            projectId: 'file-project',
            requestPermission: true,
        })).resolves.toMatchObject({ state: 'denied-permission' });
        expect(handle.requestPermission).toHaveBeenCalledOnce();
        expect(execute).toHaveBeenCalledWith(expect.objectContaining({ operation: 'put-provider-binding' }), undefined);

        const localRepository = await repositoryWithProject('Fallback source');
        const unsupported = createCemStudioFileSystemProvider({
            repository: localRepository,
            showOpenFilePicker: null,
            showDirectoryPicker: null,
        });
        expect(unsupported.capabilities()).toMatchObject({ available: false, indexedDbFallback: true });
        await expect(unsupported.status({ projectId: 'file-project' })).resolves.toMatchObject({ state: 'unsupported' });
        const fallback = await unsupported.exportFallback({ projectId: 'file-project' });
        expect(text(fallback.bundle.contents.source)).toBe('Fallback source');
        const repeated = await unsupported.exportFallback({ projectId: 'file-project' });
        expect([...repeated.archive.bytes]).toEqual([...fallback.archive.bytes]);
        const fallbackImportRepository = createRepository();
        const fallbackImport = createCemStudioFileSystemProvider({
            repository: fallbackImportRepository,
            showOpenFilePicker: null,
            showDirectoryPicker: null,
        });
        await expect(fallbackImport.importFallback({ archive: fallback.archive.bytes })).resolves.toMatchObject({
            status: 'imported',
        });
        expect(text((await exportProject(fallbackImportRepository)).contents.source)).toBe('Fallback source');
        await expect(unsupported.openProjectDirectory()).rejects.toMatchObject({
            code: 'cem.studio.file_system.codec_unavailable',
            details: { fallback: 'indexeddb-import-export' },
        });
    });
});

function createProvider(repository) {
    return createCemStudioFileSystemProvider({
        repository,
        encodeProjectManifest: async (project) => `${JSON.stringify(project)}\n`,
        decodeProjectManifest: async (bytes) => JSON.parse(text(bytes)),
    });
}

async function repositoryWithProject(content) {
    const repository = createRepository();
    await repository.execute(command('import-project', { bundle: await projectBundle(content) }));
    return repository;
}

function createRepository(databaseName = `cem-studio-provider-${crypto.randomUUID()}`) {
    const repository = createCemStudioProjectRepository({
        databaseName,
        validateProject: async (bundle) => bundle,
        now: () => '2026-08-22T00:00:00Z',
    });
    repositories.push(repository);
    return repository;
}

async function projectBundle(content) {
    const bytes = new TextEncoder().encode(content);
    const remoteBytes = new TextEncoder().encode('Retained remote snapshot');
    return {
        project: {
            $schema: 'https://cem.dev/ns/studio/project/1',
            schemaVersion: 1,
            id: 'file-project',
            name: 'File project',
            rootUri: 'studio://file-project/',
            revision: 1,
            createdAt: '2026-08-22T00:00:00Z',
            updatedAt: '2026-08-22T00:00:00Z',
            entries: [],
            resources: [
                {
                    id: 'source',
                    role: 'data',
                    sourceKind: 'project-file',
                    path: 'data/source.cem',
                    contentType: 'application/cem',
                    schema: 'https://cem.dev/ns/cem-ml/1',
                    revision: 1,
                    sha256: await digest(bytes),
                },
                {
                    id: 'remote',
                    role: 'data',
                    sourceKind: 'url',
                    path: 'snapshots/remote.cem',
                    url: 'https://example.test/remote.cem',
                    contentType: 'application/cem',
                    schema: 'https://cem.dev/ns/cem-ml/1',
                    revision: 1,
                    sha256: await digest(remoteBytes),
                },
            ],
        },
        contents: { source: bytes, remote: remoteBytes },
    };
}

async function exportProject(repository) {
    return (await repository.query(command('export-project', { projectId: 'file-project' }))).value;
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

async function fixtureDirectory() {
    const root = await navigator.storage.getDirectory();
    const name = `cem-studio-provider-${crypto.randomUUID()}`;
    opfsEntries.push(name);
    return root.getDirectoryHandle(name, { create: true });
}

async function pathHandle(directory, path) {
    const parts = path.split('/');
    let current = directory;
    for (const part of parts.slice(0, -1)) current = await current.getDirectoryHandle(part);
    return current.getFileHandle(parts.at(-1));
}

async function readPath(directory, path) {
    return readHandle(await pathHandle(directory, path));
}

async function writeHandle(handle, value) {
    const writable = await handle.createWritable();
    await writable.write(value);
    await writable.close();
}

async function readHandle(handle) {
    return (await handle.getFile()).text();
}

function text(value) {
    return new TextDecoder().decode(value instanceof Uint8Array ? value : new Uint8Array(value));
}

async function digest(bytes) {
    const value = await crypto.subtle.digest('SHA-256', bytes);
    return [...new Uint8Array(value)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function deleteDatabase(name) {
    return new Promise((resolve, reject) => {
        const request = indexedDB.deleteDatabase(name);
        request.onsuccess = () => resolve(undefined);
        request.onerror = () => reject(request.error);
        request.onblocked = () => reject(new Error(`database ${name} remained open`));
    });
}
