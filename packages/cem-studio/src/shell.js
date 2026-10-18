import { CemElementRuntime, mountCemView } from '@epa-wg/cem-elements';

const THEME_STORAGE_KEY = 'cem-studio-theme';
const ACTIVATE_UPDATE_MESSAGE = 'cem-studio-activate-update';

export const CEM_STUDIO_THEME_MODES = Object.freeze([
    'cem-theme-light',
    'cem-theme-dark',
    'cem-theme-contrast-light',
    'cem-theme-contrast-dark',
    'cem-theme-native',
]);

let componentInstallPromise;

/** Install the production CEM component declarations through cem-elements. */
export function installCemStudioShellComponents() {
    componentInstallPromise ??= installComponents();
    return componentInstallPromise;
}

/**
 * @param {{storage?: Storage, storageKey?: string, defaultMode?: string}} [options]
 */
export function createCemStudioThemeController(options = {}) {
    const subscribers = new Set();
    const storage = options.storage ?? browserStorage();
    const storageKey = options.storageKey ?? THEME_STORAGE_KEY;
    const storedMode = readStoredMode(storage, storageKey);
    let mode = storedMode ?? validThemeMode(options.defaultMode) ?? 'cem-theme-light';

    function setMode(nextMode) {
        const accepted = validThemeMode(nextMode);
        if (!accepted) throw new TypeError(`unsupported CEM Studio theme mode: ${nextMode}`);
        try {
            storage?.setItem(storageKey, accepted);
        } catch {
            // Theme application must remain available when browser storage is denied.
        }
        mode = accepted;
        for (const notify of subscribers) notify(mode);
        return mode;
    }

    setMode(mode);
    return Object.freeze({
        get mode() {
            return mode;
        },
        setMode,
        subscribe(notify) {
            subscribers.add(notify);
            notify(mode);
            return () => subscribers.delete(notify);
        },
    });
}

/**
 * Keep the browser-owned install prompt behind an explicit CEM action.
 * @param {{eventTarget?: EventTarget}} [options]
 */
export function createCemStudioInstallController(options = {}) {
    const eventTarget = options.eventTarget ?? globalThis;
    const subscribers = new Set();
    let deferredPrompt;
    let state = 'waiting';
    let outcome;

    const beforeInstallPrompt = (event) => {
        event.preventDefault();
        deferredPrompt = event;
        state = 'ready';
        publish();
    };
    const installed = () => {
        deferredPrompt = undefined;
        state = 'installed';
        outcome = 'accepted';
        publish();
    };
    eventTarget.addEventListener('beforeinstallprompt', beforeInstallPrompt);
    eventTarget.addEventListener('appinstalled', installed);

    function status() {
        return Object.freeze({ state, outcome });
    }

    async function prompt() {
        if (state === 'prompting' || !deferredPrompt || typeof deferredPrompt.prompt !== 'function') return status();
        state = 'prompting';
        publish();
        await deferredPrompt.prompt();
        const choice = await deferredPrompt.userChoice;
        outcome = choice?.outcome ?? 'dismissed';
        state = outcome === 'accepted' ? 'installed' : 'dismissed';
        deferredPrompt = undefined;
        publish();
        return Object.freeze({ ...status(), platform: choice?.platform });
    }

    function subscribe(notify) {
        subscribers.add(notify);
        notify(status());
        return () => subscribers.delete(notify);
    }

    function publish() {
        const snapshot = status();
        for (const notify of subscribers) notify(snapshot);
    }

    return Object.freeze({
        status,
        prompt,
        subscribe,
        dispose() {
            eventTarget.removeEventListener('beforeinstallprompt', beforeInstallPrompt);
            eventTarget.removeEventListener('appinstalled', installed);
            subscribers.clear();
        },
    });
}

/**
 * Coordinate explicit service-worker activation with active work and durable
 * project state. The worker is never released merely because it finished
 * installing.
 * @param {{
 *   registration?: ServiceWorkerRegistration,
 *   persistState?: () => Promise<void>,
 *   serviceWorkerContainer?: ServiceWorkerContainer,
 *   reload?: () => void,
 * }} [options]
 */
export function createCemStudioUpdateCoordinator(options = {}) {
    const registration = options.registration;
    const persistState = options.persistState ?? (async () => undefined);
    const serviceWorkerContainer = options.serviceWorkerContainer ?? globalThis.navigator?.serviceWorker;
    const reload = options.reload ?? (() => globalThis.location?.reload());
    const subscribers = new Set();
    let waitingWorker = registration?.waiting;
    let activeRequestCount = 0;
    let dirty = false;
    let dirtyVersion = 0;
    let state = waitingWorker ? 'ready' : 'current';
    let reason;
    let activationRequested = false;

    const controllerChanged = () => {
        if (activationRequested) reload();
    };
    const updateFound = () => watchInstallingWorker(registration?.installing);
    registration?.addEventListener?.('updatefound', updateFound);
    serviceWorkerContainer?.addEventListener?.('controllerchange', controllerChanged);
    watchInstallingWorker(registration?.installing);

    function watchInstallingWorker(worker) {
        if (!worker) return;
        const stateChanged = () => {
            if (worker.state === 'installed' && serviceWorkerContainer?.controller) {
                setWaitingWorker(registration?.waiting ?? worker);
            }
        };
        worker.addEventListener?.('statechange', stateChanged);
    }

    function status() {
        return Object.freeze({ state, reason, activeRequestCount, dirty, updateReady: Boolean(waitingWorker) });
    }

    function setWaitingWorker(worker) {
        waitingWorker = worker;
        state = worker ? 'ready' : 'current';
        reason = undefined;
        publish();
    }

    function setActiveRequestCount(count) {
        if (!Number.isSafeInteger(count) || count < 0) throw new TypeError('active request count must be a safe integer');
        activeRequestCount = count;
        if (state === 'blocked' && reason === 'active-work' && count === 0) {
            state = waitingWorker ? 'ready' : 'current';
            reason = undefined;
        }
        publish();
    }

    function setDirty(nextDirty) {
        dirty = Boolean(nextDirty);
        dirtyVersion += 1;
        publish();
    }

    async function activateUpdate() {
        if (activationRequested || state === 'persisting' || !waitingWorker) return status();
        if (activeRequestCount > 0) {
            state = 'blocked';
            reason = 'active-work';
            publish();
            return status();
        }
        if (dirty) {
            const version = dirtyVersion;
            state = 'persisting';
            reason = undefined;
            publish();
            try {
                await persistState();
            } catch (error) {
                state = 'blocked';
                reason = error?.code === 'unsaved-work' ? 'unsaved-work' : 'persistence-failed';
                publish();
                return status();
            }
            if (dirty && version !== dirtyVersion) {
                state = 'blocked'; reason = 'persistence-failed'; publish(); return status();
            }
            dirty = false;
        }
        if (activeRequestCount > 0) {
            state = 'blocked';
            reason = 'active-work';
            publish();
            return status();
        }
        state = 'activating';
        activationRequested = true;
        publish();
        waitingWorker.postMessage({ type: ACTIVATE_UPDATE_MESSAGE });
        return status();
    }

    function subscribe(notify) {
        subscribers.add(notify);
        notify(status());
        return () => subscribers.delete(notify);
    }

    function publish() {
        const snapshot = status();
        for (const notify of subscribers) notify(snapshot);
    }

    return Object.freeze({
        status,
        setWaitingWorker,
        setActiveRequestCount,
        setDirty,
        activateUpdate,
        subscribe,
        dispose() {
            registration?.removeEventListener?.('updatefound', updateFound);
            serviceWorkerContainer?.removeEventListener?.('controllerchange', controllerChanged);
            subscribers.clear();
        },
    });
}

/**
 * Keep picker, permission, conflict, and fallback operations behind explicit
 * application actions while exposing one accessible status stream to the shell.
 * @param {{
 *   provider?: Record<string, Function>,
 *   projectId?: string,
 *   resourceId?: string,
 *   selectImport?: () => Promise<Uint8Array | ArrayBuffer | ArrayBufferView>,
 *   downloadExport?: (archive: {filename: string, contentType: string, bytes: Uint8Array}) => Promise<void>,
 * }} [options]
 */
export function createCemStudioFileSystemController(options = {}) {
    const provider = options.provider;
    const subscribers = new Set();
    let projectId = options.projectId;
    let resourceId = options.resourceId;
    let state = provider?.capabilities?.().available ? 'unbound' : 'unsupported';
    let operation;
    let message = state === 'unsupported'
        ? 'File System Access is unavailable; IndexedDB import/export remains available.'
        : 'No local file or project directory is connected.';
    let lastResult;
    let lastError;
    let targetVersion = 0;
    let refreshVersion = 0;
    let disposed = false;

    function status() {
        return Object.freeze({
            state,
            operation,
            message,
            projectId,
            resourceId,
            capabilities: provider?.capabilities?.() ?? Object.freeze({
                available: false,
                openFile: false,
                directory: false,
                indexedDbFallback: true,
                importExportFallback: true,
            }),
            lastResult,
            lastError,
        });
    }

    function publish() {
        const snapshot = status();
        for (const notify of subscribers) notify(snapshot);
        return snapshot;
    }

    async function perform(nextOperation, action) {
        if (disposed || operation) return status();
        const version = targetVersion;
        const target = Object.freeze({ projectId, resourceId });
        ++refreshVersion;
        operation = nextOperation;
        state = 'working';
        message = fileSystemWorkingLabel(nextOperation);
        lastResult = undefined;
        lastError = undefined;
        publish();
        try {
            const result = await action(target);
            if (!disposed && version === targetVersion) {
                lastResult = result;
                // Imported projects enter through guarded Studio navigation.
                // Provider results never retarget the active session.
                if (nextOperation === 'reconnect' && typeof result?.state === 'string') {
                    state = result.state;
                    message = fileSystemProviderLabel(result);
                } else {
                    state = 'ready';
                    message = fileSystemSuccessLabel(nextOperation, result);
                }
            }
        } catch (error) {
            if (!disposed && version === targetVersion) {
                lastError = Object.freeze({
                    code: typeof error?.code === 'string' ? error.code : error?.name === 'AbortError' ? 'cem.studio.file_system.cancelled' : 'cem.studio.file_system.failed',
                    message: error instanceof Error ? error.message : String(error),
                    details: error?.details,
                });
                state = fileSystemErrorState(lastError.code);
                message = `${lastError.message} IndexedDB import/export remains available.`;
            }
        } finally {
            operation = undefined;
            if (!disposed && version !== targetVersion) await refresh();
        }
        return disposed ? status() : publish();
    }

    async function refresh() {
        const version = ++refreshVersion;
        const target = targetVersion;
        if (disposed || !provider) return status();
        if (!projectId) {
            state = provider.capabilities().available ? 'unbound' : 'unsupported';
            message = 'Choose a project to connect local files; archive import remains available.';
            return publish();
        }
        try {
            const result = await provider.status({ projectId });
            if (disposed || version !== refreshVersion || target !== targetVersion || operation) return status();
            lastResult = result;
            state = result.state;
            message = fileSystemProviderLabel(result);
        } catch (error) {
            if (disposed || version !== refreshVersion || target !== targetVersion || operation) return status();
            if (error?.code !== 'cem.studio.file_system.binding_not_found') throw error;
            state = provider.capabilities().available ? 'unbound' : 'unsupported';
            message = 'No local project directory is connected; IndexedDB remains authoritative.';
        }
        return publish();
    }

    return Object.freeze({
        status,
        refresh,
        setProject(nextProjectId, nextResourceId) {
            ++targetVersion;
            lastResult = undefined; lastError = undefined;
            projectId = nextProjectId;
            resourceId = nextResourceId;
            if (!operation) { state = 'unbound'; message = 'Checking the active project connection.'; }
            publish();
            return refresh();
        },
        openProjectDirectory: () => perform('open-project', () => provider.openProjectDirectory()),
        bindProjectDirectory: () => perform('bind-project', target => provider.bindProjectDirectory({ projectId: target.projectId })),
        openResource: () => perform('open-resource', target => provider.openResource(target)),
        reconnect: () => perform('reconnect', target => provider.reconnect({
            projectId: target.projectId,
            requestPermission: true,
        })),
        writeBack: () => perform('write-project', target => provider.writeProjectDirectory({
            projectId: target.projectId,
            requestPermission: true,
        })),
        importFallback: () => perform('import-fallback', async () => {
            if (typeof options.selectImport !== 'function') throw new Error('Project archive upload is unavailable.');
            const archive = await options.selectImport();
            return provider.importFallback({ archive });
        }),
        exportFallback: () => perform('export-fallback', async target => {
            if (typeof options.downloadExport !== 'function') throw new Error('Project archive download is unavailable.');
            const result = await provider.exportFallback({ projectId: target.projectId });
            await options.downloadExport(result.archive);
            return result;
        }),
        subscribe(notify) {
            subscribers.add(notify);
            notify(status());
            return () => subscribers.delete(notify);
        },
        dispose() {
            disposed = true; ++targetVersion; ++refreshVersion;
            subscribers.clear();
        },
    });
}

/**
 * Compose visible application chrome exclusively from production CEM controls.
 * @param {{
 *   root: Element,
 *   storage?: Storage,
 *   registration?: ServiceWorkerRegistration,
 *   repository?: {status: () => Promise<{state: string, persisted?: boolean}>},
 *   persistState?: () => Promise<void>,
 *   fileSystem?: Parameters<typeof createCemStudioFileSystemController>[0],
 * }} options
 */
export async function mountCemStudioApplicationShell(options) {
    const components = await installCemStudioShellComponents();
    const { root } = options;
    const theme = createCemStudioThemeController({ storage: options.storage });
    const install = createCemStudioInstallController();
    const update = createCemStudioUpdateCoordinator({
        registration: options.registration,
        persistState: options.persistState,
    });
    const fileSystem = createCemStudioFileSystemController(options.fileSystem);

    const view = mountCemView(components.runtime, {
        root,
        tag: 'cem-studio-shell',
        state: {
            themeMode: theme.mode,
        },
        commands: {
            theme: ({ value }) => theme.setMode(value),
        },
    });
    const unsubscribeTheme = theme.subscribe(themeMode => view.update({ themeMode }));
    await fileSystem.refresh();
    await view.whenSettled();
    return Object.freeze({
        root, theme, install, update, fileSystem, view,
        setWorkspaceState(pageTitle, workspaceStatus) { view.update({ pageTitle, workspaceStatus }); },
        whenSettled: () => view.whenSettled(),
        dispose() {
            unsubscribeTheme();
            install.dispose(); update.dispose(); fileSystem.dispose(); view.dispose();
        },
    });
}

async function installComponents() {
    const runtime = new CemElementRuntime({ declarationTag: 'cem-studio-component-declaration' });
    const source = import.meta.resolve('@epa-wg/cem-components/components.xhtml');
    const registered = [], diagnostics = [];
    for (const tag of ['cem-icon', 'cem-icon-button', 'cem-theme-switch', 'cem-dropdown', 'cem-menu', 'cem-menu-item', 'cem-workspace', 'cem-expansion', 'cem-stack', 'cem-grid', 'cem-text', 'cem-fieldset', 'cem-action', 'cem-alert', 'cem-app-bar', 'cem-badge', 'cem-card', 'cem-media-preview', 'cem-select', 'cem-dialog', 'cem-list', 'cem-tab', 'cem-table', 'cem-tree', 'cem-tabs', 'cem-text-field', 'cem-textarea']) {
        if (customElements.get(tag)) continue;
        const declaration = document.createElement(runtime.declarationTag);
        declaration.setAttribute('tag', tag);
        declaration.setAttribute('src', `${source}#${tag}`);
        const capability = { 'cem-dropdown': 'popup', 'cem-menu': 'composite-menu', 'cem-menu-item': 'action-control', 'cem-expansion': 'disclosure-panel', 'cem-action': 'action-command', 'cem-select': 'choice-select', 'cem-dialog': 'native-surface', 'cem-list': 'native-list', 'cem-tree': 'tree-view', 'cem-tabs': 'tab-panels', 'cem-text-field': 'form-control', 'cem-textarea': 'form-control' }[tag];
        if (capability) declaration.setAttribute('capability', capability);
        document.head.append(declaration);
        runtime.registerDeclaration(declaration);
        await runtime.whenDeclarationSettled(declaration);
        const current = runtime.diagnosticsFor(declaration);
        diagnostics.push(...current);
        const error = current.find(({ severity }) => severity === 'error' || severity === 'fatal');
        if (error) throw new Error(`CEM Studio component installation failed: ${error.message}`);
        registered.push(tag);
    }
    for (const [tag, source] of [
        ['cem-studio-shell', './shell.xhtml#cem-studio-shell'],
        ['cem-studio-project-picker', './project-picker.xhtml#cem-studio-project-picker'],
        ['cem-studio-workbench-navigation', './workbench-navigation.xhtml#cem-studio-workbench-navigation'],
        ['cem-studio-workbench-controls', './workbench-controls.xhtml#cem-studio-workbench-controls'],
        ['cem-studio-workbench-command', './workbench-command.xhtml#cem-studio-workbench-command'],
        ['cem-studio-workbench-layout', './workbench-layout.xhtml#cem-studio-workbench-layout'],
        ['cem-studio-workbench-results', './workbench-results.xhtml#cem-studio-workbench-results'],
        ['cem-studio-workbench-preview', './workbench-preview.xhtml#cem-studio-workbench-preview'],
        ['cem-studio-workbench-tools', './workbench-tools.xhtml#cem-studio-workbench-tools'],
    ]) {
        const declaration = document.createElement(runtime.declarationTag);
        declaration.setAttribute('tag', tag);
        declaration.setAttribute('src', new URL(source, import.meta.url).href);
        document.head.append(declaration);
        runtime.registerDeclaration(declaration);
        await runtime.whenDeclarationSettled(declaration);
        const current = runtime.diagnosticsFor(declaration);
        diagnostics.push(...current);
        const error = current.find(({ severity }) => severity === 'error' || severity === 'fatal');
        if (error) throw new Error(`CEM Studio view declaration failed: ${error.message}`);
    }
    const result = Object.freeze({ registered, diagnostics });
    return Object.freeze({ runtime, result });
}

function validThemeMode(value) {
    return CEM_STUDIO_THEME_MODES.includes(value) ? value : undefined;
}

function readStoredMode(storage, key) {
    try {
        return validThemeMode(storage?.getItem(key));
    } catch {
        return undefined;
    }
}

function browserStorage() {
    try {
        return globalThis.localStorage;
    } catch {
        return undefined;
    }
}

function fileSystemErrorState(code) {
    if (code?.includes('conflict')) return 'conflict';
    if (code?.includes('permission')) return 'permission-required';
    if (code?.includes('unsupported')) return 'unsupported';
    if (code?.includes('cancelled')) return 'unbound';
    return 'fallback';
}

function fileSystemProviderLabel(result) {
    if (result.state === 'ready') return `Connected to ${result.name ?? 'the retained project directory'}.`;
    if (result.state?.includes('permission')) return 'Reconnect permission from an explicit action or continue in IndexedDB.';
    if (result.state === 'unsupported') return 'File System Access is unavailable; IndexedDB import/export remains available.';
    return 'No local project directory is connected; IndexedDB remains authoritative.';
}

function fileSystemWorkingLabel(operation) {
    return {
        'open-project': 'Opening and validating a project directory.',
        'bind-project': 'Binding the current IndexedDB project to a directory.',
        'open-resource': 'Opening a local resource file.',
        reconnect: 'Requesting local file permission.',
        'write-project': 'Checking external revisions before write-back.',
        'import-fallback': 'Validating the selected project backup.',
        'export-fallback': 'Preparing a validated project backup.',
    }[operation] ?? 'Running a local file operation.';
}

function fileSystemSuccessLabel(operation, result) {
    return {
        'open-project': `Imported project ${result?.projectId ?? ''} into IndexedDB.`,
        'bind-project': 'Connected the current IndexedDB project to the selected directory.',
        'open-resource': 'Imported exact local file bytes into IndexedDB.',
        reconnect: 'Local file permission is ready.',
        'write-project': `Saved ${result?.fileCount ?? 0} checked files back to the project directory.`,
        'import-fallback': 'Imported and validated the project backup into IndexedDB.',
        'export-fallback': 'Exported a validated project backup.',
    }[operation] ?? 'Local file operation completed.';
}
