import { afterEach, describe, expect, it, vi } from 'vitest';

import {
    CEM_STUDIO_THEME_MODES,
    createCemStudioInstallController,
    createCemStudioUpdateCoordinator,
    mountCemStudioApplicationShell,
} from './shell.js';
import { CEM_STUDIO_REPOSITORY_ID, createCemStudioProjectRepository } from './repository.js';

const repositories = [];
const mountedShells = [];

afterEach(async () => {
    for (const shell of mountedShells.splice(0)) shell.dispose();
    const names = [...new Set(repositories.map((repository) => repository.databaseName))];
    for (const repository of repositories.splice(0)) repository.close();
    for (const name of names) await deleteDatabase(name);
    document.body.replaceChildren();
});

describe('CEM Studio installable application shell', () => {
    it('renders production CEM controls and persists every repository-defined theme mode', async () => {
        const storage = memoryStorage({ 'cem-studio-theme': 'cem-theme-dark' });
        const root = document.createElement('main');
        document.body.append(root);

        const shell = await mountCemStudioApplicationShell({ root, storage });
        mountedShells.push(shell);

        expect(CEM_STUDIO_THEME_MODES).toEqual([
            'cem-theme-light',
            'cem-theme-dark',
            'cem-theme-contrast-light',
            'cem-theme-contrast-dark',
            'cem-theme-native',
        ]);
        expect(root.querySelector('cem-app-bar header')).not.toBeNull();
        expect(root.querySelector('cem-select .cem-select__control')).not.toBeNull();
        expect(root.querySelector('cem-action button')).not.toBeNull();
        expect(root.querySelectorAll('button:not(cem-action button):not(cem-select button)')).toHaveLength(0);

        for (const mode of CEM_STUDIO_THEME_MODES) {
            shell.theme.setMode(mode);
            expect(root.dataset.theme).toBe(mode);
            expect(root.classList.contains(mode)).toBe(true);
            expect(CEM_STUDIO_THEME_MODES.filter((candidate) => root.classList.contains(candidate))).toEqual([mode]);
            expect(storage.getItem('cem-studio-theme')).toBe(mode);
        }
    });

    it('surfaces browser install readiness without manufacturing an install prompt', async () => {
        const host = new EventTarget();
        const prompt = vi.fn(async () => undefined);
        const installEvent = new Event('beforeinstallprompt', { cancelable: true });
        Object.defineProperties(installEvent, {
            prompt: { value: prompt },
            userChoice: { value: Promise.resolve({ outcome: 'accepted', platform: 'web' }) },
        });
        const controller = createCemStudioInstallController({ eventTarget: host });

        host.dispatchEvent(installEvent);
        expect(installEvent.defaultPrevented).toBe(true);
        expect(controller.status()).toMatchObject({ state: 'ready' });
        await expect(controller.prompt()).resolves.toMatchObject({ outcome: 'accepted' });
        expect(prompt).toHaveBeenCalledOnce();
        controller.dispose();
    });

    it('blocks unsafe activation, persists a dirty project, and only then releases the waiting worker', async () => {
        const waiting = { postMessage: vi.fn() };
        const persistState = vi.fn(async () => undefined);
        const registration = eventTarget({ waiting });
        const coordinator = createCemStudioUpdateCoordinator({ registration, persistState });

        coordinator.setWaitingWorker(waiting);
        coordinator.setActiveRequestCount(1);
        await expect(coordinator.activateUpdate()).resolves.toMatchObject({ state: 'blocked', reason: 'active-work' });
        expect(waiting.postMessage).not.toHaveBeenCalled();

        coordinator.setActiveRequestCount(0);
        coordinator.setDirty(true);
        await expect(coordinator.activateUpdate()).resolves.toMatchObject({ state: 'activating' });
        expect(persistState).toHaveBeenCalledOnce();
        expect(waiting.postMessage).toHaveBeenCalledWith({ type: 'cem-studio-activate-update' });
        coordinator.dispose();

        const rejectedWorker = { postMessage: vi.fn() };
        const rejected = createCemStudioUpdateCoordinator({
            persistState: async () => {
                throw new Error('quota exceeded');
            },
        });
        rejected.setWaitingWorker(rejectedWorker);
        rejected.setDirty(true);
        await expect(rejected.activateUpdate()).resolves.toMatchObject({
            state: 'blocked',
            reason: 'persistence-failed',
            dirty: true,
        });
        expect(rejectedWorker.postMessage).not.toHaveBeenCalled();
        rejected.dispose();
    });

    it('reopens and exports identical project bytes from IndexedDB', async () => {
        const databaseName = `cem-studio-shell-${crypto.randomUUID()}`;
        const first = createRepository(databaseName);
        const bundle = await projectBundle('Offline project bytes');
        await first.execute(command('import-project', { bundle }));
        first.close();

        const reopened = createRepository(databaseName);
        const exported = await reopened.query(command('export-project', { projectId: 'offline-project' }));
        expect(exported.value.project).toEqual(bundle.project);
        expect(new TextDecoder().decode(exported.value.contents.source)).toBe('Offline project bytes');
    });
});

function createRepository(databaseName) {
    const repository = createCemStudioProjectRepository({
        databaseName,
        validateProject: async (bundle) => bundle,
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

async function projectBundle(content) {
    const bytes = new TextEncoder().encode(content);
    const digest = await crypto.subtle.digest('SHA-256', bytes);
    const sha256 = [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
    return {
        project: {
            $schema: 'https://cem.dev/ns/studio/project/1',
            schemaVersion: 1,
            id: 'offline-project',
            name: 'Offline project',
            rootUri: 'studio://offline-project/',
            revision: 1,
            createdAt: '2026-08-21T00:00:00Z',
            updatedAt: '2026-08-21T00:00:00Z',
            entries: [],
            resources: [{
                id: 'source',
                role: 'data',
                sourceKind: 'project-file',
                path: 'source.cem',
                contentType: 'application/cem',
                schema: 'https://cem.dev/ns/cem-ml/1',
                revision: 1,
                sha256,
            }],
        },
        contents: { source: bytes },
    };
}

function memoryStorage(initial = {}) {
    const values = new Map(Object.entries(initial));
    return {
        getItem: (key) => values.get(key) ?? null,
        setItem: (key, value) => values.set(key, String(value)),
        removeItem: (key) => values.delete(key),
    };
}

function eventTarget(properties) {
    return Object.assign(new EventTarget(), properties);
}

function deleteDatabase(name) {
    return new Promise((resolve, reject) => {
        const request = indexedDB.deleteDatabase(name);
        request.onsuccess = () => resolve(undefined);
        request.onerror = () => reject(request.error);
        request.onblocked = () => reject(new Error(`database ${name} remained open`));
    });
}
