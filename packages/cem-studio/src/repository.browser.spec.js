import { afterEach, describe, expect, it, vi } from 'vitest';
import { commandSchema, parseCemMlCommandText, parseCemMlCommandResource, serializeCemMlCommandResource, CEM_ML_CLI_COMMAND_CONTENT_TYPE, CEM_ML_CLI_COMMAND_SCHEMA } from '@epa-wg/cem-ml-cli/browser-commands';

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
    it('persists exact pinned builds across reopen without changing sources, and scopes reads and deletion', async () => {
        const repository = createRepository();
        const bundle = await projectBundle('saved draft-independent source');
        await repository.execute(command('import-project', { bundle }));
        const before = (await repository.query(query('export-project', { projectId: 'feature-tour' }))).value;
        const snapshot = await pinnedBuild();
        const { value: pin } = await repository.execute(command('pin-build-snapshot', { snapshot }));
        snapshot.outputs[0].bytes.fill(0); // caller does not retain ownership of stored bytes
        repository.close();
        const reopened = createRepository({ databaseName: repository.databaseName });
        const { value } = await reopened.query(query('get-build-snapshot', { projectId: 'feature-tour', key: pin.key }));
        expect(new TextDecoder().decode(value.outputs[0].bytes)).toBe('Pinned café 🙂\r\n');
        expect(new Uint8Array(value.outputs[1].bytes)).toEqual(new Uint8Array([0, 255, 128, 1]));
        expect(value.run.artifacts[0].provenance).toEqual([{ uri: 'studio://feature-tour/data/tour.cem', range: { start: 0, end: 4 } }]);
        expect(value.run.executionIdentity).toEqual({ engineVersion: 'test-native-1' });
        expect(value.run.configuration).toEqual((await pinnedBuild()).run.configuration);
        expect(value.run.artifacts[0]).not.toHaveProperty('handleId');
        expect((await reopened.query(query('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
        await expect(reopened.query(query('get-build-snapshot', { projectId: 'another', key: pin.key }))).rejects.toMatchObject({ code: 'cem.studio.snapshot.not_found' });
        await expect(reopened.execute(command('delete-build-snapshot', { projectId: 'another', key: pin.key }))).rejects.toMatchObject({ code: 'cem.studio.snapshot.not_found' });
        const aborted = new AbortController(); aborted.abort();
        await expect(reopened.execute(command('delete-build-snapshot', { projectId: 'feature-tour', key: pin.key }), aborted.signal)).rejects.toMatchObject({ name: 'AbortError' });
        await reopened.execute(command('delete-build-snapshot', { projectId: 'feature-tour', key: pin.key }));
        expect((await reopened.query(query('list-build-snapshots', { projectId: 'feature-tour' }))).value).toEqual([]);
        expect((await reopened.query(query('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
    });

    it('rejects corrupt, incomplete, oversized and excess pins atomically without eviction', async () => {
        const repository = createRepository();
        const source = await pinnedBuild();
        const invalid = [
            value => { value.run.status = 'failed'; },
            value => { value.outputs.pop(); },
            value => { value.outputs[0].bytes[0] ^= 1; },
            value => { value.run.artifacts[1].path = value.run.artifacts[0].path; },
            value => { value.run.byteLength++; },
            value => { value.run.artifacts[0].provenance = [{ uri: 'x'.repeat(1024 * 1024) }]; },
            value => { value.outputs[0].bytes = new Uint8Array(16 * 1024 * 1024 + 1); value.run.artifacts[0].byteLength = value.outputs[0].bytes.byteLength; },
        ];
        for (const corrupt of invalid) {
            const value = structuredClone(source); corrupt(value);
            await expect(repository.execute(command('pin-build-snapshot', { snapshot: value }))).rejects.toMatchObject({ code: 'cem.studio.snapshot.invalid' });
        }
        expect((await repository.query(query('list-build-snapshots', { projectId: 'feature-tour' }))).value).toEqual([]);
        const results = await Promise.allSettled(Array.from({ length: 9 }, () => repository.execute(command('pin-build-snapshot', { snapshot: source }))));
        expect(results.filter(item => item.status === 'fulfilled')).toHaveLength(8);
        expect(results.find(item => item.status === 'rejected').reason.code).toBe('cem.studio.snapshot.capacity');
        const pins = (await repository.query(query('list-build-snapshots', { projectId: 'feature-tour' }))).value;
        expect(pins).toHaveLength(8);
        const database = await repository.open();
        await new Promise((resolve, reject) => {
            const transaction = database.transaction('resultSnapshots', 'readwrite'), store = transaction.objectStore('resultSnapshots');
            const read = store.get(pins[0].key);
            read.onsuccess = () => { const record = read.result; new Uint8Array(record.outputs[1].bytes)[0] = 42; store.put(record); };
            transaction.oncomplete = resolve; transaction.onabort = () => reject(transaction.error);
        });
        await expect(repository.query(query('get-build-snapshot', { projectId: 'feature-tour', key: pins[0].key }))).rejects.toMatchObject({ code: 'cem.studio.snapshot.invalid' });
        await repository.execute(command('delete-build-snapshot', { projectId: 'feature-tour', key: pins[0].key }));
        await repository.execute(command('pin-build-snapshot', { snapshot: source }));
        expect((await repository.query(query('list-build-snapshots', { projectId: 'feature-tour' }))).value).toHaveLength(8);
    });

    it('enforces aggregate pin bytes across complete builds and releases space on explicit deletion', async () => {
        const repository = createRepository();
        const snapshot = await pinnedBuild(), bytes = new Uint8Array(16 * 1024 * 1024);
        snapshot.run.artifacts = [{ ...snapshot.run.artifacts[0], byteLength: bytes.byteLength, sha256: await digest(bytes) }];
        snapshot.run.byteLength = bytes.byteLength;
        snapshot.outputs = [{ uri: snapshot.run.artifacts[0].uri, bytes }];
        const pins = [];
        for (let i = 0; i < 3; i++) pins.push((await repository.execute(command('pin-build-snapshot', { snapshot }))).value);
        await expect(repository.execute(command('pin-build-snapshot', { snapshot }))).rejects.toMatchObject({ code: 'cem.studio.snapshot.capacity' });
        expect((await repository.query(query('list-build-snapshots', { projectId: 'feature-tour' }))).value).toHaveLength(3);
        await repository.execute(command('delete-build-snapshot', { projectId: 'feature-tour', key: pins[0].key }));
        await repository.execute(command('pin-build-snapshot', { snapshot }));
        expect((await repository.query(query('list-build-snapshots', { projectId: 'feature-tour' }))).value).toHaveLength(3);
    }, 30_000);

    it('closes an in-flight open and cancels pin reads without reopening a disposed repository', async () => {
        const repository = createRepository();
        const opening = repository.open(); repository.close();
        await expect(opening).rejects.toMatchObject({ name: 'AbortError' });
        expect(repository.database).toBeUndefined();
        await repository.open();
        const controller = new AbortController();
        const reading = repository.query(query('list-build-snapshots', { projectId: 'feature-tour' }), controller.signal);
        controller.abort(); repository.close();
        await expect(reading).rejects.toMatchObject({ name: 'AbortError' });
        expect(repository.database).toBeUndefined();
        await repository.deleteDatabase();
    });

    it('commits structure, preserves portable order, copies exact bytes and restores with increasing revisions', async () => {
        const repository = createRepository();
        const bundle = await projectBundle('exact café 🙂\r\n');
        bundle.project.entries = [
            { id: 'z-parent', kind: 'subproject', name: 'Parent' },
            { id: 'a-child', kind: 'data-set', name: 'Child', parentId: 'z-parent', resourceIds: ['tour-source'] },
            { id: 'b-root', kind: 'subproject', name: 'Second' },
        ];
        await repository.execute(command('import-project', { bundle }));
        let revision = 1;
        const edit = async value => {
            const result = await repository.execute(command('edit-project', { projectId: 'feature-tour', expectedRevision: revision, edit: value }));
            revision = result.value.project.revision;
            return result.value;
        };
        const renamed = await edit({ kind: 'rename-project', name: 'Renamed project' });
        expect(renamed.project.entries.map(entry => entry.id)).toEqual(['z-parent', 'a-child', 'b-root']);
        expect(renamed.project).not.toHaveProperty('entryOrder');
        await edit({ kind: 'reorder-entry', id: 'b-root', direction: 'up' });
        const copied = await edit({ kind: 'copy-entry', id: 'z-parent', name: 'Copied parent' });
        const parent = copied.project.entries.find(entry => entry.name === 'Copied parent');
        expect(copied.project.entries.find(entry => entry.parentId === parent.id).resourceIds).toEqual(['tour-source']);
        const resourceCopy = await edit({ kind: 'copy-resource', id: 'tour-source', path: 'data/copied.cem' });
        const copy = resourceCopy.project.resources.find(resource => resource.path === 'data/copied.cem');
        const exported = (await repository.query(query('export-project', { projectId: 'feature-tour' }))).value;
        expect(new Uint8Array(exported.contents[copy.id])).toEqual(new Uint8Array(exported.contents['tour-source']));
        expect(exported.project.entries[0].id).toBe('b-root');
        const undone = await edit({ kind: 'restore', bundle: resourceCopy.before });
        expect(undone.project.resources).toHaveLength(1);
        expect(undone.project.resources[0].revision).toBeGreaterThan(1);
        expect(undone.project.revision).toBe(6);
        const redone = await edit({ kind: 'restore', bundle: exported });
        expect(redone.project.resources).toHaveLength(2);
        expect(redone.project.revision).toBe(7);
        await expect(repository.execute(command('edit-project', { projectId: 'feature-tour', expectedRevision: 1, edit: { kind: 'rename-project', name: 'Stale' } }))).rejects.toMatchObject({ code: 'cem.studio.repository.revision_conflict' });
        await expect(edit({ kind: 'move-entry', id: 'z-parent', parentId: 'a-child' })).rejects.toThrow();
        await expect(edit({ kind: 'move-entry', id: 'z-parent', parentId: 'z-parent' })).rejects.toThrow('own subtree');
        await expect(edit({ kind: 'copy-resource', id: 'tour-source', path: 'data/copied.cem' })).rejects.toThrow('new project-relative');
        await repository.execute(command('trash-project', { projectId: 'feature-tour', expectedRevision: revision }));
        const trashed = (await repository.query(query('get-project', { projectId: 'feature-tour' }))).value;
        await expect(repository.execute(command('edit-project', { projectId: 'feature-tour', expectedRevision: trashed.revision, edit: { kind: 'rename-project', name: 'Hidden' } }))).rejects.toMatchObject({ code: 'cem.studio.repository.project_unavailable' });
    });

    it('rechecks structural revision inside the commit after native validation', async () => {
        const repository = createRepository();
        await repository.execute(command('import-project', { bundle: await projectBundle('unchanged') }));
        const results = await Promise.allSettled(['First', 'Second'].map(name => repository.execute(command('edit-project', {
            projectId: 'feature-tour', expectedRevision: 1, edit: { kind: 'rename-project', name },
        }))));
        expect(results.filter(result => result.status === 'fulfilled')).toHaveLength(1);
        expect(results.filter(result => result.status === 'rejected')).toHaveLength(1);
        expect((await repository.query(query('get-project', { projectId: 'feature-tour' }))).value.revision).toBe(2);
    });

    it('renames dataset metadata without rewriting consumers and preserves references through undo', async () => {
        const repository = createRepository();
        const bundle = await projectBundle('exact café 🙂\r\n');
        const commandText = serializeCemMlCommandResource(parseCemMlCommandText('cem-ml inspect studio://feature-tour/data/tour.cem --show tree --namespace a=urn:a --namespace b=urn:b'));
        const bytes = new TextEncoder().encode(commandText);
        bundle.project.resources.push({ id: 'command', role: 'run-config', sourceKind: 'project-file', path: 'commands/inspect.json', contentType: CEM_ML_CLI_COMMAND_CONTENT_TYPE, schema: CEM_ML_CLI_COMMAND_SCHEMA, revision: 1, sha256: await digest(bytes) });
        bundle.contents.command = bytes;
        bundle.project.entries = [{ id: 'inspection', kind: 'inspection', name: 'Inspect source', runConfigResourceId: 'command', resourceIds: ['tour-source', 'command'] }];
        await repository.execute(command('import-project', { bundle }));
        const edit = (revision, value) => repository.execute(command('edit-project', { projectId: 'feature-tour', expectedRevision: revision, edit: value }));
        const moved = await edit(1, { kind: 'rename-resource', id: 'tour-source', path: 'data/renamed.cem' });
        const exported = (await repository.query(query('export-project', { projectId: 'feature-tour' }))).value;
        expect(exported.project.entries).toEqual(bundle.project.entries);
        expect(new Uint8Array(exported.contents['tour-source'])).toEqual(bundle.contents['tour-source']);
        expect(exported.project.resources.find(item => item.id === 'tour-source')).toMatchObject({ path: 'data/renamed.cem', revision: 2, sha256: bundle.project.resources[0].sha256 });
        const parsed = parseCemMlCommandResource(new Uint8Array(exported.contents.command)).command;
        expect(parsed.positionals.input).toBe('studio://feature-tour/data/tour.cem');
        expect(new Uint8Array(exported.contents.command)).toEqual(bytes);
        expect(exported.project.resources.find(item => item.id === 'tour-source').referencePath).toBe('data/tour.cem');
        expect(parsed.options).toMatchObject({ show: 'tree', namespaces: ['a=urn:a', 'b=urn:b'] });
        expect(exported.project.resources.find(item => item.id === 'command').revision).toBe(1);
        await expect(edit(1, { kind: 'rename-resource', id: 'tour-source', path: 'stale.cem' })).rejects.toMatchObject({ code: 'cem.studio.repository.revision_conflict' });
        await expect(edit(2, { kind: 'delete-resource', id: 'tour-source' })).rejects.toThrow('referenced');
        await expect(edit(2, { kind: 'delete-resource', id: 'command' })).rejects.toThrow('referenced');
        await expect(edit(2, { kind: 'rename-resource', id: 'tour-source', path: 'commands/inspect.json' })).rejects.toThrow('new project-relative');
        expect((await repository.query(query('get-project', { projectId: 'feature-tour' }))).value.revision).toBe(2);
        await edit(2, { kind: 'restore', bundle: moved.value.before });
        const restored = (await repository.query(query('export-project', { projectId: 'feature-tour' }))).value;
        expect(new Uint8Array(restored.contents.command)).toEqual(bytes);
        expect(restored.project.resources.find(item => item.id === 'tour-source').path).toBe('data/tour.cem');
        await edit(3, { kind: 'copy-resource', id: 'tour-source', path: 'unused.cem' });
        const copy = (await repository.query(query('get-project', { projectId: 'feature-tour' }))).value.resources.find(item => item.path === 'unused.cem');
        const removed = await edit(4, { kind: 'delete-resource', id: copy.id });
        expect(removed.value.project.resources.some(item => item.id === copy.id)).toBe(false);
        await edit(5, { kind: 'restore', bundle: removed.value.before });
        expect((await repository.query(query('get-project', { projectId: 'feature-tour' }))).value.resources.some(item => item.id === copy.id)).toBe(true);
    });

    it('renames arbitrary dataset resources without inspecting their payload or page kind', async () => {
        const repository = createRepository();
        const bundle = await projectBundle('uninterpreted template bytes');
        bundle.project.resources[0].role = 'template';
        await repository.execute(command('import-project', { bundle }));
        await repository.execute(command('edit-project', { projectId: 'feature-tour', expectedRevision: 1,
            edit: { kind: 'rename-resource', id: 'tour-source', path: 'moved/template.cem' } }));
        repository.close();
        const after = (await repository.query(query('export-project', { projectId: 'feature-tour' }))).value;
        expect(after.project.resources.find(item => item.id === 'tour-source')).toMatchObject({
            id: 'tour-source', path: 'moved/template.cem', referencePath: 'data/tour.cem', revision: 2 });
        expect(new Uint8Array(after.contents['tour-source'])).toEqual(bundle.contents['tour-source']);
        expect(after.project.entries).toEqual(bundle.project.entries);
        const abort = new AbortController(); abort.abort();
        await expect(repository.execute(command('edit-project', { projectId: 'feature-tour', expectedRevision: 2,
            edit: { kind: 'rename-resource', id: 'tour-source', path: 'cancelled.cem' } }), abort.signal)).rejects.toThrow();
        expect((await repository.query(query('get-project', { projectId: 'feature-tour' }))).value.revision).toBe(2);
    });

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
        expect(exported.value.project).toEqual({ ...bundle.project, resources: bundle.project.resources.map(resource => ({ ...resource, referencePath: resource.path })) });
        expect(new TextDecoder().decode(exported.value.contents['tour-source'])).toBe('Tour source');
        await expect(repository.query(query('list-changes', { after: 0 }))).resolves.toMatchObject({
            value: [{ operation: 'import-project', projectId: 'feature-tour', repositoryRevision: 1, sequence: 1 }],
        });
    });

    it('retains host-only provider bindings without leaking them into portable exports', async () => {
        const repository = createRepository();
        const bundle = await projectBundle('Provider source');
        await repository.execute(command('import-project', { bundle }));
        const stored = await repository.execute(command('put-provider-binding', {
            expectedProjectRevision: 1,
            binding: {
                provider: 'file-system-access',
                scope: 'project',
                projectId: 'feature-tour',
                handle: { kind: 'directory', name: 'fixture-project' },
                name: 'fixture-project',
                permission: 'granted',
                base: {
                    projectRevision: 1,
                    manifest: { state: 'missing' },
                    resources: [{ resourceId: 'tour-source', path: 'data/tour.cem', sha256: bundle.project.resources[0].sha256 }],
                },
            },
        }));
        expect(stored.value).toMatchObject({
            provider: 'file-system-access',
            scope: 'project',
            projectId: 'feature-tour',
            revision: 1,
        });
        await expect(repository.query(query('get-provider-binding', {
            projectId: 'feature-tour',
            scope: 'project',
        }))).resolves.toMatchObject({ value: { handle: { kind: 'directory', name: 'fixture-project' } } });
        await expect(repository.query(query('list-provider-bindings', {
            projectId: 'feature-tour',
        }))).resolves.toMatchObject({ value: [{ revision: 1 }] });

        const exported = await repository.query(query('export-project', { projectId: 'feature-tour' }));
        expect(exported.value.project).toEqual({ ...bundle.project, resources: bundle.project.resources.map(resource => ({ ...resource, referencePath: resource.path })) });
        expect([...new Uint8Array(exported.value.contents['tour-source'])]).toEqual([
            ...new Uint8Array(bundle.contents['tour-source']),
        ]);
        expect(JSON.stringify(exported.value)).not.toContain('provider');

        const updated = await repository.execute(command('put-provider-binding', {
            expectedProjectRevision: 1,
            expectedBindingRevision: 1,
            binding: {
                ...stored.value,
                permission: 'prompt',
                base: { ...stored.value.base, projectRevision: 1 },
            },
        }));
        expect(updated.value).toMatchObject({ revision: 2, permission: 'prompt' });
        await expect(repository.execute(command('put-provider-binding', {
            expectedProjectRevision: 1,
            expectedBindingRevision: 1,
            binding: updated.value,
        }))).rejects.toMatchObject({ code: 'cem.studio.repository.revision_conflict' });

        await repository.execute(command('delete-provider-binding', {
            projectId: 'feature-tour',
            scope: 'project',
            expectedBindingRevision: 2,
        }));
        await expect(repository.query(query('get-provider-binding', {
            projectId: 'feature-tour',
            scope: 'project',
        }))).resolves.toMatchObject({ value: null });
        await expect(repository.query(query('list-changes', { after: 1 }))).resolves.toMatchObject({
            value: [
                expect.objectContaining({ operation: 'put-provider-binding', bindingRevision: 1 }),
                expect.objectContaining({ operation: 'put-provider-binding', bindingRevision: 2 }),
                expect.objectContaining({ operation: 'delete-provider-binding', bindingRevision: 2 }),
            ],
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

    it('rejects a stale command apply from another repository instance', async () => {
        const databaseName = uniqueDatabaseName();
        const first = createRepository({ databaseName });
        const second = createRepository({ databaseName });
        await Promise.all([first.open(), second.open()]);
        await first.execute(command('import-project', { bundle: await commandProjectBundle() }));
        await second.query(query('get-project', { projectId: 'feature-tour' }));

        const changeReceived = new Promise((resolve, reject) => {
            const timeout = setTimeout(() => reject(new Error('command apply invalidation was not delivered')), 2_000);
            second.subscribe(1, (change) => {
                clearTimeout(timeout);
                resolve(change);
            });
        });
        await first.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'current', entryId: 'inspect-source' },
            commandResource: commandResource('inspect', 'summary'),
            referencedResourceIds: ['tour-source'],
        }));
        await expect(changeReceived).resolves.toMatchObject({ cursor: 2, repositoryRevision: 2 });
        await expect(second.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'current', entryId: 'inspect-source' },
            commandResource: commandResource('parse', 'events'),
            referencedResourceIds: ['tour-source'],
        }))).rejects.toMatchObject({
            code: 'cem.studio.repository.revision_conflict',
            details: { expectedRevision: 1, currentRevision: 2 },
        });
    });

    it('atomically applies exact command bytes to current and name-resolved compatible pages', async () => {
        const repository = createRepository();
        await repository.execute(command('import-project', {
            bundle: await commandProjectBundle(),
        }));
        const inspectCommand = commandResource('inspect', 'source-offsets');

        const applied = await repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'current', entryId: 'inspect-source' },
            commandResource: inspectCommand,
            referencedResourceIds: ['tour-source'],
        }));
        expect(applied.value).toMatchObject({
            disposition: 'updated',
            operation: 'inspect',
            pageKind: 'inspection',
            projectRevision: 2,
            entryRevision: 2,
            resourceRevision: 2,
            entry: {
                id: 'inspect-source',
                kind: 'inspection',
                runConfigResourceId: 'inspect-command',
                resourceIds: ['tour-source', 'inspect-command'],
            },
            commandResource: {
                id: 'inspect-command',
                contentType: 'application/vnd.cem.cli-command+json',
                schema: 'https://cem.dev/ns/cli/command/1',
                revision: 2,
            },
        });
        expect(new TextDecoder().decode(applied.value.commandBytes)).toBe(inspectCommand);

        const parseCommand = commandResource('parse', 'ast');
        const resolved = await repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 2,
            target: { mode: 'existing', name: 'inspect source', parentId: 'workbenches' },
            commandResource: parseCommand,
            referencedResourceIds: ['tour-source'],
        }));
        expect(resolved.value).toMatchObject({
            disposition: 'updated',
            operation: 'parse',
            projectRevision: 3,
            entryRevision: 3,
            resourceRevision: 3,
            entry: { id: 'inspect-source' },
        });

        const exported = await repository.query(query('export-project', { projectId: 'feature-tour' }));
        expect(new TextDecoder().decode(exported.value.contents['inspect-command'])).toBe(parseCommand);
        expect(exported.value.project.revision).toBe(3);
        const search = await repository.query(query('search', {
            query: 'inspect source',
            projectId: 'feature-tour',
            kinds: ['entry'],
        }));
        expect(search.value).toContainEqual(
            expect.objectContaining({ entityId: 'inspect-source', sourceRevision: 3 }),
        );
        await expect(repository.query(query('list-changes', { after: 1 }))).resolves.toMatchObject({
            value: [
                expect.objectContaining({ operation: 'apply-command-page', entryId: 'inspect-source' }),
                expect.objectContaining({ operation: 'apply-command-page', entryId: 'inspect-source' }),
            ],
        });
    });

    it('creates stable new pages and isolates a command resource shared by another page', async () => {
        const repository = createRepository();
        await repository.execute(command('import-project', {
            bundle: await commandProjectBundle({ sharedCommand: true }),
        }));
        const inspectCommand = commandResource('inspect', 'summary');

        const isolated = await repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'current', entryId: 'inspect-source' },
            commandResource: inspectCommand,
            referencedResourceIds: ['tour-source'],
        }));
        expect(isolated.value).toMatchObject({
            disposition: 'updated',
            projectRevision: 2,
            resourceRevision: 1,
            entry: { id: 'inspect-source', runConfigResourceId: 'inspect-source-command' },
            commandResource: { id: 'inspect-source-command', path: 'config/inspect-source-command.command.json' },
        });

        const created = await repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 2,
            target: { mode: 'new', name: 'Parse Results', parentId: 'workbenches' },
            commandResource: commandResource('parse', 'events'),
            referencedResourceIds: ['tour-source'],
        }));
        expect(created.value).toMatchObject({
            disposition: 'created',
            projectRevision: 3,
            entryRevision: 3,
            resourceRevision: 1,
            entry: {
                id: 'parse-results',
                parentId: 'workbenches',
                kind: 'inspection',
                name: 'Parse Results',
                runConfigResourceId: 'parse-results-command',
            },
            commandResource: {
                id: 'parse-results-command',
                path: 'config/parse-results-command.command.json',
            },
        });

        const exported = await repository.query(query('export-project', { projectId: 'feature-tour' }));
        const shared = exported.value.project.entries.find(({ id }) => id === 'inspect-source-copy');
        expect(shared.runConfigResourceId).toBe('inspect-command');
        expect(new TextDecoder().decode(exported.value.contents['inspect-command'])).toBe(
            commandResource('parse', 'ast'),
        );
        expect(exported.value.project.entries).toContainEqual(expect.objectContaining({ id: 'parse-results' }));
    });

    it('maps every portable authored operation to its exact Studio page kind', async () => {
        const repository = createRepository();
        await repository.execute(command('import-project', {
            bundle: await commandProjectBundle(),
        }));
        const input = 'studio://feature-tour/data/tour.cem';
        const cases = [
            ['convert', 'conversion', ['convert', input, '--to-format', 'dom-json']],
            ['query', 'query', [
                'query', input, '--query', '//item', '--query-content-type',
                'application/vnd.cem.xpath', '--output', 'json',
            ]],
            ['transform', 'transformation', [
                'transform', input, '--data-content-type', 'application/xml', '--template', input,
                '--template-content-type', 'application/vnd.cem.xpath', '--to-content-type',
                'application/vnd.cem.xpath-result+json',
            ]],
            ['trace', 'trace', ['trace', input, '--format', 'json']],
            ['transform-graph', 'transformation-graph', ['transform', '--config', input]],
        ];
        let projectRevision = 1;
        for (const [name, pageKind, argv] of cases) {
            const applied = await repository.execute(command('apply-command-page', {
                projectId: 'feature-tour',
                expectedProjectRevision: projectRevision,
                target: { mode: 'new', name: `Portable ${name}`, parentId: 'workbenches' },
                commandResource: portableCommandResource(argv),
                referencedResourceIds: ['tour-source'],
            }));
            projectRevision += 1;
            expect(applied.value).toMatchObject({
                operation: argv[0],
                pageKind,
                projectRevision,
                entry: { kind: pageKind },
            });
        }
    });

    it('fails closed for invalid, unresolved, duplicate, incompatible, rejected, and stale applies', async () => {
        const repository = createRepository();
        await repository.execute(command('import-project', {
            bundle: await commandProjectBundle(),
        }));
        const validCommand = commandResource('inspect', 'summary');

        await expect(repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'current', entryId: 'inspect-source' },
            commandResource: validCommand.replace('"cem-ml"', '"other-cli"'),
            referencedResourceIds: ['tour-source'],
        }))).rejects.toMatchObject({ code: 'cem.studio.repository.command_invalid' });
        await expect(repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'current', entryId: 'inspect-source' },
            commandResource: validCommand,
            referencedResourceIds: ['missing-source'],
        }))).rejects.toMatchObject({ code: 'cem.studio.repository.command_resource_unresolved' });
        await expect(repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'current', entryId: 'inspect-source' },
            commandResource: commandResource(
                'inspect',
                'summary',
                'studio://feature-tour/data/missing.cem',
            ),
            referencedResourceIds: ['tour-source'],
        }))).rejects.toMatchObject({ code: 'cem.studio.repository.command_resource_unresolved' });
        await expect(repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'new', name: 'Inspect source', parentId: 'workbenches' },
            commandResource: validCommand,
            referencedResourceIds: ['tour-source'],
        }))).rejects.toMatchObject({
            code: 'cem.studio.repository.command_target_name_conflict',
            details: { existingEntryId: 'inspect-source' },
        });
        await expect(repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'existing', entryId: 'convert-source' },
            commandResource: validCommand,
            referencedResourceIds: ['tour-source'],
        }))).rejects.toMatchObject({
            code: 'cem.studio.repository.command_target_incompatible',
            details: {
                existingKind: 'conversion',
                requiredKind: 'inspection',
                recommendedDisposition: 'new',
                requiresConfirmation: true,
            },
        });
        const unchanged = await repository.query(query('export-project', { projectId: 'feature-tour' }));
        expect(unchanged.value.project.revision).toBe(1);

        const replaced = await repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: {
                mode: 'existing',
                entryId: 'convert-source',
                confirmIncompatibleReplacement: true,
            },
            commandResource: validCommand,
            referencedResourceIds: ['tour-source'],
        }));
        expect(replaced.value).toMatchObject({
            disposition: 'updated',
            projectRevision: 2,
            entry: { id: 'convert-source', kind: 'inspection' },
        });
        await expect(repository.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'current', entryId: 'inspect-source' },
            commandResource: validCommand,
            referencedResourceIds: ['tour-source'],
        }))).rejects.toMatchObject({ code: 'cem.studio.repository.revision_conflict' });

        const validationRejected = createRepository({
            validateProject: async (bundle) => {
                if (bundle.project.revision > 1) throw new Error('proposed project rejected');
                return bundle;
            },
        });
        await validationRejected.execute(command('import-project', {
            bundle: await commandProjectBundle(),
        }));
        await expect(validationRejected.execute(command('apply-command-page', {
            projectId: 'feature-tour',
            expectedProjectRevision: 1,
            target: { mode: 'current', entryId: 'inspect-source' },
            commandResource: validCommand,
            referencedResourceIds: ['tour-source'],
        }))).rejects.toMatchObject({ code: 'cem.studio.repository.apply_validation_failed' });
        const rejectedExport = await validationRejected.query(query('export-project', { projectId: 'feature-tour' }));
        expect(rejectedExport.value.project.revision).toBe(1);
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

async function commandProjectBundle(options = {}) {
    const initialCommand = commandResource('parse', 'ast');
    const definitions = [
        {
            resource: {
                id: 'tour-source',
                role: 'data',
                sourceKind: 'project-file',
                path: 'data/tour.cem',
                contentType: 'application/cem',
                schema: 'https://cem.dev/ns/cem-ml/1',
            },
            content: 'Tour source',
        },
        {
            resource: {
                id: 'inspect-command',
                role: 'run-config',
                sourceKind: 'project-file',
                path: 'config/inspect.command.json',
                contentType: 'application/vnd.cem.cli-command+json',
                schema: 'https://cem.dev/ns/cli/command/1',
            },
            content: initialCommand,
        },
        {
            resource: {
                id: 'convert-config',
                role: 'run-config',
                sourceKind: 'project-file',
                path: 'config/convert.json',
                contentType: 'application/json',
                schema: 'https://cem.dev/ns/cli/run-config/1',
            },
            content: '{}\n',
        },
    ];
    const contents = {};
    const resources = [];
    for (const definition of definitions) {
        const bytes = new TextEncoder().encode(definition.content);
        contents[definition.resource.id] = bytes;
        resources.push({
            ...definition.resource,
            revision: 1,
            sha256: await digest(bytes),
        });
    }
    const entries = [
        { id: 'workbenches', kind: 'subproject', name: 'Workbenches' },
        {
            id: 'inspect-source',
            parentId: 'workbenches',
            kind: 'inspection',
            name: 'Inspect source',
            runConfigResourceId: 'inspect-command',
            resourceIds: ['tour-source', 'inspect-command'],
            tags: ['inspection'],
        },
        {
            id: 'convert-source',
            parentId: 'workbenches',
            kind: 'conversion',
            name: 'Convert source',
            runConfigResourceId: 'convert-config',
            resourceIds: ['tour-source', 'convert-config'],
            tags: ['conversion'],
        },
    ];
    if (options.sharedCommand) {
        entries.push({
            id: 'inspect-source-copy',
            parentId: 'workbenches',
            kind: 'inspection',
            name: 'Inspect source copy',
            runConfigResourceId: 'inspect-command',
            resourceIds: ['tour-source', 'inspect-command'],
            tags: ['inspection'],
        });
    }
    return {
        project: {
            $schema: 'https://cem.dev/ns/studio/project/1',
            schemaVersion: 1,
            id: 'feature-tour',
            name: 'CEM-ML Feature Tour',
            description: 'Command apply repository fixture',
            rootUri: 'studio://feature-tour/',
            revision: 1,
            createdAt: '2026-08-20T00:00:00Z',
            updatedAt: '2026-08-20T00:00:00Z',
            entries,
            resources,
        },
        contents,
    };
}

function commandResource(operation, mode, inputUri = 'studio://feature-tour/data/tour.cem') {
    const argv = operation === 'parse'
        ? [
            'parse',
            inputUri,
            '--content-type',
            'application/cem',
            '--schema',
            'https://cem.dev/ns/cem-ml/1',
            '--format',
            mode,
            '--no-color',
        ]
        : [
            'inspect',
            inputUri,
            '--show',
            mode,
            '--format',
            'cem',
        ];
    return `${JSON.stringify({
        $schema: 'https://cem.dev/ns/cli/command/1',
        schemaVersion: 1,
        commandSchemaVersion: commandSchema.schemaVersion,
        commonVersion: commandSchema.commonVersion,
        binaryName: commandSchema.binaryName,
        argv,
    }, null, 2)}\n`;
}

function portableCommandResource(argv) {
    return `${JSON.stringify({
        $schema: 'https://cem.dev/ns/cli/command/1',
        schemaVersion: 1,
        commandSchemaVersion: commandSchema.schemaVersion,
        commonVersion: commandSchema.commonVersion,
        binaryName: commandSchema.binaryName,
        argv,
    }, null, 2)}\n`;
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

async function pinnedBuild() {
    const contents = [new TextEncoder().encode('Pinned café 🙂\r\n'), new Uint8Array([0, 255, 128, 1])];
    const artifacts = await Promise.all(contents.map(async (bytes, i) => ({ id: `native-result#${i}`, uri: `studio://feature-tour/generated/${i}.txt`,
        path: `generated/${i}.txt`, contentType: i ? 'application/octet-stream' : 'text/plain', byteLength: bytes.byteLength, sha256: await digest(bytes), available: true,
        provenance: [{ uri: 'studio://feature-tour/data/tour.cem', range: { start: 0, end: 4 } }], sourceMaps: [] })));
    return { version: 1, run: { id: 'native-result', projectId: 'feature-tour', projectRootUri: 'studio://feature-tour/', projectRevision: 1,
        configuration: { id: 'build', name: 'Build', kind: 'transformation', revision: 1, sha256: 'a'.repeat(64), entryArtifact: 'generated/0.txt' },
        inputs: [{ resourceId: 'tour-source', uri: 'studio://feature-tour/data/tour.cem', path: 'data/tour.cem', revision: 1, sha256: 'b'.repeat(64) }],
        status: 'succeeded', executionIdentity: { engineVersion: 'test-native-1' }, artifacts, byteLength: contents.reduce((sum, bytes) => sum + bytes.byteLength, 0) },
        outputs: artifacts.map((artifact, i) => ({ uri: artifact.uri, bytes: contents[i] })) };
}
