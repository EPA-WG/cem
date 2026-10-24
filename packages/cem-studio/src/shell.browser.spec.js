import { afterEach, beforeAll, describe, expect, it, onTestFinished, vi } from 'vitest';

import {
    CEM_STUDIO_THEME_MODES,
    createCemStudioThemeController,
    createCemStudioFileSystemController,
    createCemStudioInstallController,
    createCemStudioUpdateCoordinator,
    mountCemStudioApplicationShell,
} from './shell.js';
import { CEM_STUDIO_REPOSITORY_ID, createCemStudioProjectRepository } from './repository.js';

// Supply the same resource identity as the production page's import map.
beforeAll(() => {
    const map = document.createElement('script'); map.type = 'importmap';
    map.textContent = JSON.stringify({ imports: {
        '@epa-wg/cem-components/components.xhtml': new URL('../../cem-components/dist/components.xhtml', import.meta.url).href,
    } });
    document.head.append(map);
});

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
    it('loads the saved theme before mount and defaults missing or invalid storage to light', () => {
        for (const saved of [null, 'invalid', 'cem-theme-dark']) {
            const storage = memoryStorage(saved === null ? {} : { 'cem-studio-theme': saved });
            const theme = createCemStudioThemeController({ storage });
            expect(theme.mode).toBe(saved === 'cem-theme-dark' ? saved : 'cem-theme-light');
        }
    });
    it('serializes host operations during the asynchronous declarative disabled-state update', async () => {
        let release;
        const pending = new Promise(resolve => { release = resolve; });
        const provider = { capabilities: () => ({ available: true }), openProjectDirectory: vi.fn(() => pending) };
        const files = createCemStudioFileSystemController({ provider });
        const first = files.openProjectDirectory();
        const duplicateFile = files.openProjectDirectory();
        expect(provider.openProjectDirectory).toHaveBeenCalledOnce();
        release({ projectId: 'opened' }); await Promise.all([first, duplicateFile]); files.dispose();

        const target = new EventTarget();
        const install = createCemStudioInstallController({ eventTarget: target });
        let choose;
        const choice = new Promise(resolve => { choose = resolve; });
        const prompt = vi.fn(async () => undefined);
        const offer = new Event('beforeinstallprompt', { cancelable: true });
        Object.defineProperties(offer, { prompt: { value: prompt }, userChoice: { value: choice } });
        target.dispatchEvent(offer);
        const installing = install.prompt();
        const duplicateInstall = install.prompt();
        expect(prompt).toHaveBeenCalledOnce();
        choose({ outcome: 'accepted' }); await Promise.all([installing, duplicateInstall]); install.dispose();

        let persist;
        const persistence = new Promise(resolve => { persist = resolve; });
        const waiting = { postMessage: vi.fn() };
        const save = vi.fn(() => persistence);
        const update = createCemStudioUpdateCoordinator({ registration: eventTarget({ waiting }), persistState: save });
        update.setDirty(true);
        const activating = update.activateUpdate();
        const duplicateUpdate = update.activateUpdate();
        expect(save).toHaveBeenCalledOnce();
        persist(); await Promise.all([activating, duplicateUpdate]); await update.activateUpdate();
        expect(waiting.postMessage).toHaveBeenCalledOnce();
        update.dispose();
    });

    it('binds the theme declaratively and omits storage/install/update controls while retaining workbench state', async () => {
        const { userEvent } = await import('vitest/browser');
        const root = document.createElement('main');
        document.body.append(root);
        const storage = memoryStorage({ 'cem-studio-theme': 'cem-theme-native' });
        const shell = await mountCemStudioApplicationShell({ root, storage });
        mountedShells.push(shell);
        expect(customElements.get('cem-icon-button')).toBeDefined();
        expect(customElements.get('cem-icon')).toBeDefined();
        const theme = root.querySelector('cem-theme-switch[data-cem-studio-theme]');
        const trigger = theme.querySelector('[part=trigger] button');
        await userEvent.click(trigger);
        await shell.whenSettled();
        const option = theme.querySelector('[role="menuitemradio"][value="cem-theme-dark"]');
        expect(option).toBeTruthy();
        await userEvent.click(option);
        await shell.whenSettled();
        expect(shell.theme.mode).toBe('cem-theme-dark');
        expect(shell.view.element.dataset.theme).toBe('cem-theme-dark');
        expect(storage.getItem('cem-studio-theme')).toBe('cem-theme-dark');
        expect(theme.querySelector('[part=trigger] button')).toBe(trigger);
        expect(document.activeElement).toBe(trigger);

        // Real theme items contain nested icon/label nodes. Every part of a
        // menu item must deliver its enclosing native button's value.
        for (const mode of ['cem-theme-light', 'cem-theme-contrast-dark', 'cem-theme-contrast-light', 'cem-theme-native', 'cem-theme-dark']) {
            await userEvent.click(trigger); await shell.whenSettled();
            const item = theme.querySelector(`cem-menu cem-icon-button[value="${mode}"]`);
            await userEvent.click(item.querySelector('[part=glyph]'));
            await shell.whenSettled();
            expect(shell.theme.mode).toBe(mode);
            expect(shell.view.element.dataset.theme).toBe(mode);
            expect(storage.getItem('cem-studio-theme')).toBe(mode);
        }

        const workbench = document.createElement('section');
        const editor = document.createElement('textarea');
        editor.setAttribute('aria-label', 'Retained workbench editor');
        workbench.append(editor);
        shell.view.element.append(workbench);
        await shell.whenSettled();
        editor.value = 'unsaved source'; editor.focus(); editor.setSelectionRange(2, 8);
        shell.theme.setMode('cem-theme-contrast-dark');
        await shell.whenSettled();
        expect(shell.view.element.contains(workbench)).toBe(true);
        expect(workbench.querySelector('textarea')).toBe(editor);
        expect(editor.value).toBe('unsaved source');
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 8]);
        expect(document.activeElement).toBe(editor);

        expect(root.querySelector('[data-cem-studio-install]')).toBeNull();
        const prompt = vi.fn(async () => undefined);
        const offer = new Event('beforeinstallprompt', { cancelable: true });
        Object.defineProperties(offer, { prompt: { value: prompt }, userChoice: { value: Promise.resolve({ outcome: 'accepted' }) } });
        window.dispatchEvent(offer);
        shell.update.setWaitingWorker({ postMessage: vi.fn() });
        await shell.whenSettled();
        expect(root.querySelector('[data-cem-studio-install], [host-command="install"], [host-command="update"]')).toBeNull();
        expect(root.textContent).not.toContain('Local storage');
        expect(root.textContent).not.toContain('Install');
        expect(prompt).not.toHaveBeenCalled();
        expect(shell.view.errors).toEqual([]);
        shell.dispose();
        expect(root.children).toHaveLength(0);
    });

    it('uses shared advisory recovery status without enabling capture, and retains disclosure through shell updates', async () => {
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root, storage: memoryStorage({}) }); mountedShells.push(shell);
        const host = root.querySelector('footer cem-recovery-status'), details = host.querySelector('details'), summary = host.querySelector('summary');
        expect(shell.recovery.snapshot()).toMatchObject({ state: 'unconfigured', automaticRecovery: false, credentialPolicy: 'block-entered' });
        expect(summary.textContent).toContain('Recovery · Not configured');
        expect(details.textContent).toContain('Entered passwords or one-time codes block recovery');
        expect(details.textContent).toContain('Automatic recovery is off');
        summary.click(); await new Promise(resolve => setTimeout(resolve, 0)); await shell.whenSettled(); summary.focus();
        shell.setWorkspaceState('Login · login.html - CEM Studio', 'dirty'); shell.recovery.refresh(); await shell.whenSettled();
        expect(root.querySelector('footer cem-recovery-status')).toBe(host); expect(host.querySelector('details')).toBe(details);
        expect(details.open).toBe(true); expect(document.activeElement).toBe(summary);
        expect(host.querySelector('button, a, input')).toBeNull(); expect(shell.view.errors).toEqual([]);
        shell.dispose(); expect(shell.recovery.snapshot().state).toBe('unavailable');
    });

    it('uses compact flush chrome while retaining inherited shared layout rhythm', async () => {
        const root = document.createElement('main');
        root.style.setProperty('--cem-gap-related', '31px');
        root.style.setProperty('--cem-layout-stack-gap', '19px');
        root.style.setProperty('--cem-coupling-guard-min', '8px');
        root.style.setProperty('--cem-resource-project-color', 'rgb(101, 20, 30)');
        root.style.setProperty('--cem-resource-file-color', 'rgb(30, 80, 120)');
        document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root, storage: memoryStorage({}) });
        mountedShells.push(shell);
        expect(customElements.get('cem-stack')).toBeDefined();
        expect(customElements.get('cem-grid')).toBeDefined();
        const section = root.querySelector('cem-studio-shell > [part=root]');
        const status = root.querySelector('footer[aria-label="Studio status"]');
        expect(getComputedStyle(section).display).toBe('grid');
        expect(root.querySelector('cem-studio-shell').getAttribute('data-cem-density')).toBe('super-compact');
        expect(root.querySelector('cem-studio-shell').getAttribute('data-cem-spacing')).toBe('normal');
        expect(getComputedStyle(section).rowGap).toBe('0px');
        expect(getComputedStyle(status).columnGap).toBe('8px');
        shell.setWorkspaceState('Login · source.cem - CEM Studio', 'Unsaved changes', { projectName: 'Login', path: 'source.cem', pathKind: 'file' });
        await shell.whenSettled();
        expect(root.querySelector('h1').innerText.trim().replace(/\s+/gu, ' ')).toBe('Login · source.cem - CEM Studio');
        expect(root.querySelector('h1').getAttribute('aria-label')).toBe('Login · source.cem - CEM Studio');
        expect(getComputedStyle(root.querySelector('h1 cem-text[kind=project] [part~=root]')).color).toBe('rgb(101, 20, 30)');
        expect(getComputedStyle(root.querySelector('h1 cem-text[kind=resource] [part~=root]')).color).toBe('rgb(30, 80, 120)');
        expect(status.textContent).toContain('Unsaved changes');
        root.querySelector('cem-studio-shell').style.setProperty('--cem-coupling-guard-min', '37px');
        expect(getComputedStyle(section).rowGap).toBe('0px');
        expect(getComputedStyle(status).columnGap).toBe('37px');
    });

    it('keeps real header theme controls at content height in the page super-compact profile', async () => {
        const css = document.createElement('link'); css.rel = 'stylesheet';
        css.href = new URL('../../cem-theme/dist/lib/css/cem-combined.css', import.meta.url).href;
        onTestFinished(() => css.remove());
        const ready = new Promise((resolve, reject) => { css.onload = resolve; css.onerror = reject; });
        document.head.append(css); await ready;
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root, storage: memoryStorage({}) }); mountedShells.push(shell);
        const host = root.querySelector('cem-studio-shell');
        const header = root.querySelector('header.cem-app-bar');
        const button = header.querySelector('cem-theme-switch [part=trigger] button');
        expect(host.getAttribute('data-cem-density')).toBe('super-compact');
        expect(host.getAttribute('data-cem-spacing')).toBe('normal');
        for (const mode of CEM_STUDIO_THEME_MODES) {
            shell.theme.setMode(mode); await shell.whenSettled();
            expect(header.querySelector('cem-theme-switch [part=trigger] button')).toBe(button);
            const controlStyle = getComputedStyle(button), headerStyle = getComputedStyle(header);
            expect(controlStyle.getPropertyValue('--cem-control-height').trim()).toBe('0px');
            expect(controlStyle.getPropertyValue('--cem-coupling-zone-min').trim()).toBe('0px');
            expect(controlStyle.minBlockSize).toBe('0px');
            expect(controlStyle.paddingBlockStart).toBe('0px');
            expect(controlStyle.paddingBlockEnd).toBe('0px');
            expect(headerStyle.paddingBlockStart).toBe('0px');
            expect(headerStyle.paddingBlockEnd).toBe('0px');
            expect(button.getBoundingClientRect().height).toBeLessThan(48);
            const childrenHeight = Math.max(...[...header.children].map(node => node.getBoundingClientRect().height));
            const borders = parseFloat(headerStyle.borderTopWidth) + parseFloat(headerStyle.borderBottomWidth);
            expect(Math.abs(header.getBoundingClientRect().height - childrenHeight - borders)).toBeLessThan(0.1);
            expect(header.getBoundingClientRect().height).toBeLessThan(48);
        }
        host.style.setProperty('--cem-control-height', '44px');
        host.style.setProperty('--cem-control-padding-y', '3px');
        expect(button.getBoundingClientRect().height).toBeGreaterThanOrEqual(44);
        expect(getComputedStyle(button).paddingBlockStart).toBe('3px');
        expect(shell.view.errors).toEqual([]);
    });

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
        expect(root.querySelector('cem-theme-switch [part=trigger] button')).not.toBeNull();
        expect(root.querySelector('h1').textContent).toBe('Examples - CEM Studio');
        expect(root.querySelectorAll('button:not(cem-action button):not(cem-icon-button button):not(cem-select button):not(cem-dropdown button):not(cem-expansion button):not(cem-tabs button):not(cem-tree button)')).toHaveLength(0);
        for (const action of root.querySelectorAll('cem-action')) {
            expect(['primary', 'explicit', 'contextual', 'alternate', 'destructive']).toContain(action.getAttribute('variant'));
        }
        expect(root.querySelector('[data-cem-studio-open-project]')).toBeNull();

        for (const mode of CEM_STUDIO_THEME_MODES) {
            shell.theme.setMode(mode);
            await shell.whenSettled();
            expect(shell.view.element.dataset.theme).toBe(mode);
            expect(shell.view.element.classList.contains(mode)).toBe(true);
            expect(CEM_STUDIO_THEME_MODES.filter((candidate) => shell.view.element.classList.contains(candidate))).toEqual([mode]);
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

    it('ignores stale provider completions and preserves explicit permission and archive targets', async () => {
        let finishStatus, finishPicker;
        const provider = {
            capabilities: () => ({ available: true }),
            status: vi.fn(({ projectId }) => projectId === 'alpha'
                ? new Promise(resolve => { finishStatus = resolve; }) : Promise.resolve({ state: 'ready', name: projectId })),
            openResource: vi.fn(() => new Promise(resolve => { finishPicker = resolve; })),
            exportFallback: vi.fn(async ({ projectId }) => ({ archive: { filename: projectId, bytes: new Uint8Array([1]) } })),
            reconnect: vi.fn(async () => { throw Object.assign(new Error('denied'), { code: 'cem.studio.file_system.permission_denied' }); }),
        };
        const controller = createCemStudioFileSystemController({ provider, downloadExport: vi.fn(async () => undefined) });
        const first = controller.setProject('alpha', 'a');
        await controller.setProject('beta', 'b');
        finishStatus({ state: 'denied-permission', name: 'alpha' }); await first;
        expect(controller.status()).toMatchObject({ projectId: 'beta', resourceId: 'b', state: 'ready' });
        const picker = controller.openResource();
        expect(provider.openResource).toHaveBeenCalledWith({ projectId: 'beta', resourceId: 'b' });
        await controller.setProject('gamma', 'c');
        finishPicker({ projectId: 'beta', state: 'ready' });
        await expect(picker).resolves.toMatchObject({ projectId: 'gamma', state: 'ready', operation: undefined });
        expect(controller.status()).toMatchObject({ projectId: 'gamma', resourceId: 'c', state: 'ready' });
        await controller.reconnect();
        expect(provider.reconnect).toHaveBeenCalledWith({ projectId: 'gamma', requestPermission: true });
        await controller.exportFallback();
        expect(provider.exportFallback).toHaveBeenCalledWith({ projectId: 'gamma' });
        const cancelled = controller.openResource(); finishPicker(Promise.reject(new DOMException('cancelled', 'AbortError'))); await cancelled;
        expect(controller.status().projectId).toBe('gamma');
        controller.dispose();
    });

    it('does not activate an update over edits made while persistence is pending', async () => {
        let finish;
        const waiting = { postMessage: vi.fn() };
        const coordinator = createCemStudioUpdateCoordinator({ persistState: () => new Promise(resolve => { finish = resolve; }) });
        coordinator.setWaitingWorker(waiting); coordinator.setDirty(true);
        const activation = coordinator.activateUpdate();
        coordinator.setDirty(true); finish();
        await expect(activation).resolves.toMatchObject({ state: 'blocked', dirty: true });
        expect(waiting.postMessage).not.toHaveBeenCalled();
        coordinator.setDirty(false);
        await expect(coordinator.activateUpdate()).resolves.toMatchObject({ state: 'activating' });
        coordinator.dispose();
    });

    it('reopens and exports identical project bytes from IndexedDB', async () => {
        const databaseName = `cem-studio-shell-${crypto.randomUUID()}`;
        const first = createRepository(databaseName);
        const bundle = await projectBundle('Offline project bytes');
        await first.execute(command('import-project', { bundle }));
        first.close();

        const reopened = createRepository(databaseName);
        const exported = await reopened.query(command('export-project', { projectId: 'offline-project' }));
        expect(exported.value.project).toEqual({ ...bundle.project, resources: bundle.project.resources.map(resource => ({ ...resource, referencePath: resource.path })) });
        expect(new TextDecoder().decode(exported.value.contents.source)).toBe('Offline project bytes');
    });

    it('retains local-file permission and backup services without obsolete shell controls', async () => {
        const provider = {
            capabilities: () => ({
                available: true,
                openFile: true,
                directory: true,
                indexedDbFallback: true,
                importExportFallback: true,
            }),
            status: vi.fn(async () => ({
                provider: 'file-system-access',
                state: 'prompt-permission',
                name: 'retained-project',
            })),
            reconnect: vi.fn(async () => ({
                provider: 'file-system-access',
                state: 'denied-permission',
                permission: 'denied',
                name: 'retained-project',
            })),
            openProjectDirectory: vi.fn(async () => ({ status: 'imported', projectId: 'opened-project' })),
            bindProjectDirectory: vi.fn(async () => ({ status: 'bound', projectId: 'offline-project' })),
            openResource: vi.fn(async () => ({ status: 'opened', projectId: 'offline-project' })),
            writeProjectDirectory: vi.fn(async () => ({ status: 'written', fileCount: 2 })),
            importFallback: vi.fn(async () => ({ status: 'imported', project: { id: 'backup-project' } })),
            exportFallback: vi.fn(async () => ({
                status: 'exported',
                archive: {
                    filename: 'offline-project.cem-studio.json',
                    contentType: 'application/vnd.cem.studio-project-bundle+json',
                    bytes: new Uint8Array([1, 2, 3]),
                },
            })),
        };
        const selectImport = vi.fn(async () => new Uint8Array([4, 5, 6]));
        const downloadExport = vi.fn(async () => undefined);
        const root = document.createElement('main');
        document.body.append(root);
        const shell = await mountCemStudioApplicationShell({
            root,
            fileSystem: {
                provider,
                projectId: 'offline-project',
                resourceId: 'source',
                selectImport,
                downloadExport,
            },
        });
        mountedShells.push(shell);

        expect(root.querySelectorAll('button:not(cem-action button):not(cem-icon-button button):not(cem-select button):not(cem-dropdown button):not(cem-expansion button):not(cem-tabs button):not(cem-tree button)')).toHaveLength(0);
        expect(provider.reconnect).not.toHaveBeenCalled();
        expect(shell.fileSystem.status().state).toBe('prompt-permission');
        await shell.fileSystem.reconnect();
        await vi.waitFor(() => expect(provider.reconnect).toHaveBeenCalledWith({
            projectId: 'offline-project',
            requestPermission: true,
        }));
        await vi.waitFor(() => expect(shell.fileSystem.status()).toMatchObject({ state: 'denied-permission' }));
        await shell.fileSystem.exportFallback();
        await vi.waitFor(() => expect(downloadExport).toHaveBeenCalledWith(expect.objectContaining({
            filename: 'offline-project.cem-studio.json',
        })));
        await shell.fileSystem.importFallback();
        await vi.waitFor(() => expect(provider.importFallback).toHaveBeenCalledWith({
            archive: expect.any(Uint8Array),
        }));
        expect(selectImport).toHaveBeenCalledOnce();
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
