import { installCemComponentPrimitives } from '@epa-wg/cem-components';
import { CemElementRuntime } from '@epa-wg/cem-elements';

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
 * @param {{root: Element, storage?: Storage, storageKey?: string, defaultMode?: string}} options
 */
export function createCemStudioThemeController(options) {
    const { root } = options;
    const storage = options.storage ?? browserStorage();
    const storageKey = options.storageKey ?? THEME_STORAGE_KEY;
    const storedMode = readStoredMode(storage, storageKey);
    let mode = storedMode ?? validThemeMode(options.defaultMode) ?? 'cem-theme-native';

    function setMode(nextMode) {
        const accepted = validThemeMode(nextMode);
        if (!accepted) throw new TypeError(`unsupported CEM Studio theme mode: ${nextMode}`);
        root.classList.remove(...CEM_STUDIO_THEME_MODES);
        root.classList.add(accepted);
        root.setAttribute('data-theme', accepted);
        try {
            storage?.setItem(storageKey, accepted);
        } catch {
            // Theme application must remain available when browser storage is denied.
        }
        mode = accepted;
        return mode;
    }

    setMode(mode);
    return Object.freeze({
        get mode() {
            return mode;
        },
        setMode,
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
        if (!deferredPrompt || typeof deferredPrompt.prompt !== 'function') return status();
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
        publish();
    }

    async function activateUpdate() {
        if (!waitingWorker) return status();
        if (activeRequestCount > 0) {
            state = 'blocked';
            reason = 'active-work';
            publish();
            return status();
        }
        if (dirty) {
            state = 'persisting';
            reason = undefined;
            publish();
            try {
                await persistState();
            } catch {
                state = 'blocked';
                reason = 'persistence-failed';
                publish();
                return status();
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
        operation = nextOperation;
        state = 'working';
        message = fileSystemWorkingLabel(nextOperation);
        lastResult = undefined;
        lastError = undefined;
        publish();
        try {
            const result = await action();
            lastResult = result;
            if (result?.projectId) projectId = result.projectId;
            if (nextOperation === 'reconnect' && typeof result?.state === 'string') {
                state = result.state;
                message = fileSystemProviderLabel(result);
            } else {
                state = 'ready';
                message = fileSystemSuccessLabel(nextOperation, result);
            }
        } catch (error) {
            lastError = Object.freeze({
                code: error?.code ?? 'cem.studio.file_system.failed',
                message: error instanceof Error ? error.message : String(error),
                details: error?.details,
            });
            state = fileSystemErrorState(lastError.code);
            message = `${lastError.message} IndexedDB import/export remains available.`;
        } finally {
            operation = undefined;
        }
        return publish();
    }

    async function refresh() {
        if (!provider) return publish();
        if (!projectId) {
            state = provider.capabilities().available ? 'unbound' : 'unsupported';
            return publish();
        }
        try {
            const result = await provider.status({ projectId });
            lastResult = result;
            state = result.state;
            message = fileSystemProviderLabel(result);
        } catch (error) {
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
            projectId = nextProjectId;
            resourceId = nextResourceId;
            return refresh();
        },
        openProjectDirectory: () => perform('open-project', () => provider.openProjectDirectory()),
        bindProjectDirectory: () => perform('bind-project', () => provider.bindProjectDirectory({ projectId })),
        openResource: () => perform('open-resource', () => provider.openResource({ projectId, resourceId })),
        reconnect: () => perform('reconnect', () => provider.reconnect({
            projectId,
            requestPermission: true,
        })),
        writeBack: () => perform('write-project', () => provider.writeProjectDirectory({
            projectId,
            requestPermission: true,
        })),
        importFallback: () => perform('import-fallback', async () => {
            if (typeof options.selectImport !== 'function') throw new Error('Project archive upload is unavailable.');
            const archive = await options.selectImport();
            return provider.importFallback({ archive });
        }),
        exportFallback: () => perform('export-fallback', async () => {
            if (typeof options.downloadExport !== 'function') throw new Error('Project archive download is unavailable.');
            const result = await provider.exportFallback({ projectId });
            await options.downloadExport(result.archive);
            return result;
        }),
        subscribe(notify) {
            subscribers.add(notify);
            notify(status());
            return () => subscribers.delete(notify);
        },
        dispose() {
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
    const theme = createCemStudioThemeController({ root, storage: options.storage });
    const install = createCemStudioInstallController();
    const update = createCemStudioUpdateCoordinator({
        registration: options.registration,
        persistState: options.persistState,
    });
    const fileSystem = createCemStudioFileSystemController(options.fileSystem);

    root.setAttribute('data-cem-studio-state', 'ready');
    root.innerHTML = shellMarkup(theme.mode);
    await settleShellComponents(components.runtime, root);
    const themeSelect = root.querySelector('cem-select[data-cem-studio-theme]');
    const installAction = root.querySelector('cem-action[data-cem-studio-install]');
    const updateAction = root.querySelector('cem-action[data-cem-studio-update]');
    const installBadge = root.querySelector('cem-badge[data-cem-studio-install-state]');
    const updateAlert = root.querySelector('cem-alert[data-cem-studio-update-state]');
    const storageBadge = root.querySelector('cem-badge[data-cem-studio-storage-state]');
    const providerBadge = root.querySelector('cem-badge[data-cem-studio-provider-state]');
    const providerAlert = root.querySelector('cem-alert[data-cem-studio-provider-alert]');
    const openProjectAction = root.querySelector('cem-action[data-cem-studio-open-project]');
    const bindProjectAction = root.querySelector('cem-action[data-cem-studio-bind-project]');
    const reconnectAction = root.querySelector('cem-action[data-cem-studio-provider-reconnect]');
    const writeBackAction = root.querySelector('cem-action[data-cem-studio-provider-write]');
    const openResourceAction = root.querySelector('cem-action[data-cem-studio-open-resource]');
    const importFallbackAction = root.querySelector('cem-action[data-cem-studio-import-fallback]');
    const exportFallbackAction = root.querySelector('cem-action[data-cem-studio-export-fallback]');
    themeSelect.value = theme.mode;

    const themeChanged = () => theme.setMode(themeSelect.value);
    const installRequested = () => install.prompt();
    const updateRequested = () => update.activateUpdate();
    const openProjectRequested = () => fileSystem.openProjectDirectory();
    const bindProjectRequested = () => fileSystem.bindProjectDirectory();
    const reconnectRequested = () => fileSystem.reconnect();
    const writeBackRequested = () => fileSystem.writeBack();
    const openResourceRequested = () => fileSystem.openResource();
    const importFallbackRequested = () => fileSystem.importFallback();
    const exportFallbackRequested = () => fileSystem.exportFallback();
    themeSelect.addEventListener('change', themeChanged);
    installAction.addEventListener('click', installRequested);
    updateAction.addEventListener('click', updateRequested);
    openProjectAction.addEventListener('click', openProjectRequested);
    bindProjectAction.addEventListener('click', bindProjectRequested);
    reconnectAction.addEventListener('click', reconnectRequested);
    writeBackAction.addEventListener('click', writeBackRequested);
    openResourceAction.addEventListener('click', openResourceRequested);
    importFallbackAction.addEventListener('click', importFallbackRequested);
    exportFallbackAction.addEventListener('click', exportFallbackRequested);

    const unsubscribeInstall = install.subscribe(({ state: installState }) => {
        installBadge.setAttribute('label', installLabel(installState));
        installAction.toggleAttribute('disabled', installState !== 'ready');
    });
    const unsubscribeUpdate = update.subscribe((snapshot) => {
        updateAlert.setAttribute('label', updateLabel(snapshot));
        updateAlert.setAttribute('tone', snapshot.state === 'blocked' ? 'warning' : 'info');
        updateAction.toggleAttribute('disabled', !snapshot.updateReady || snapshot.state === 'activating');
    });
    const unsubscribeFileSystem = fileSystem.subscribe((snapshot) => {
        const busy = snapshot.state === 'working';
        const hasProject = Boolean(snapshot.projectId);
        const hasResource = hasProject && Boolean(snapshot.resourceId);
        providerBadge.setAttribute('label', fileSystemStateLabel(snapshot.state));
        providerBadge.setAttribute('tone', fileSystemTone(snapshot.state));
        providerAlert.setAttribute('label', snapshot.message);
        providerAlert.setAttribute('tone', fileSystemTone(snapshot.state));
        openProjectAction.toggleAttribute('disabled', busy || !snapshot.capabilities.directory);
        bindProjectAction.toggleAttribute('disabled', busy || !hasProject || !snapshot.capabilities.directory);
        reconnectAction.toggleAttribute('disabled', busy || !hasProject || snapshot.state === 'unbound');
        writeBackAction.toggleAttribute('disabled', busy || !hasProject || snapshot.state !== 'ready');
        openResourceAction.toggleAttribute('disabled', busy || !hasResource || !snapshot.capabilities.openFile);
        importFallbackAction.toggleAttribute('disabled', busy || typeof options.fileSystem?.selectImport !== 'function');
        exportFallbackAction.toggleAttribute('disabled', busy || !hasProject || typeof options.fileSystem?.downloadExport !== 'function');
    });
    await fileSystem.refresh();
    await settleShellComponents(components.runtime, root);

    if (options.repository) {
        options.repository.status().then(
            (status) => {
                storageBadge.setAttribute('label', status.persisted ? 'Local storage persisted' : `Local storage ${status.state}`);
                storageBadge.setAttribute('tone', status.state === 'ready' ? 'success' : 'warning');
            },
            () => {
                storageBadge.setAttribute('label', 'Local storage unavailable');
                storageBadge.setAttribute('tone', 'danger');
            },
        );
    } else if ('indexedDB' in globalThis) {
        storageBadge.setAttribute('label', 'Local storage available');
        storageBadge.setAttribute('tone', 'info');
    } else {
        storageBadge.setAttribute('label', 'Local storage unavailable');
        storageBadge.setAttribute('tone', 'danger');
    }

    return Object.freeze({
        root,
        theme,
        install,
        update,
        fileSystem,
        dispose() {
            themeSelect.removeEventListener('change', themeChanged);
            installAction.removeEventListener('click', installRequested);
            updateAction.removeEventListener('click', updateRequested);
            openProjectAction.removeEventListener('click', openProjectRequested);
            bindProjectAction.removeEventListener('click', bindProjectRequested);
            reconnectAction.removeEventListener('click', reconnectRequested);
            writeBackAction.removeEventListener('click', writeBackRequested);
            openResourceAction.removeEventListener('click', openResourceRequested);
            importFallbackAction.removeEventListener('click', importFallbackRequested);
            exportFallbackAction.removeEventListener('click', exportFallbackRequested);
            unsubscribeInstall();
            unsubscribeUpdate();
            unsubscribeFileSystem();
            install.dispose();
            update.dispose();
            fileSystem.dispose();
        },
    });
}

async function settleShellComponents(runtime, root) {
    await Promise.resolve();
    const instances = [...root.querySelectorAll('cem-app-bar, cem-badge, cem-select, cem-action, cem-card, cem-alert')];
    await Promise.all(instances.map((instance) => runtime.whenRenderSettled(instance)));
    await Promise.resolve();
    await Promise.all(instances.map((instance) => runtime.whenRenderSettled(instance)));
}

async function installComponents() {
    const runtime = new CemElementRuntime({ declarationTag: 'cem-studio-component-declaration' });
    const result = await installCemComponentPrimitives(runtime);
    const hardDiagnostics = result.diagnostics.filter(({ severity }) => severity === 'error' || severity === 'fatal');
    if (hardDiagnostics.length > 0) {
        throw new Error(`CEM Studio component installation failed: ${hardDiagnostics[0].message}`);
    }
    return Object.freeze({ runtime, result });
}

function shellMarkup(mode) {
    return `
        <div data-cem-studio-shell>
            <cem-app-bar label="CEM Studio"><span slot="title">CEM Studio</span></cem-app-bar>
            <section aria-label="Application controls">
                <cem-badge data-cem-studio-storage-state label="Local storage checking" tone="info"></cem-badge>
                <cem-badge data-cem-studio-install-state label="Install availability waiting" tone="info"></cem-badge>
                <cem-select data-cem-studio-theme name="theme" value="${mode}">
                    <span slot="label">Theme</span>
                    <option value="cem-theme-light">Light</option>
                    <option value="cem-theme-dark">Dark</option>
                    <option value="cem-theme-contrast-light">Contrast light</option>
                    <option value="cem-theme-contrast-dark">Contrast dark</option>
                    <option value="cem-theme-native">System</option>
                </cem-select>
                <cem-action data-cem-studio-install variant="quiet" disabled>Install</cem-action>
            </section>
            <cem-card label="Local-first workbench">
                <span slot="title">Local-first CEM-ML workbench</span>
                <p>Projects remain in this browser while CEM-ML commands run in the dedicated local worker.</p>
                <cem-badge label="Offline ready" tone="success"></cem-badge>
            </cem-card>
            <cem-card label="Local file provider">
                <span slot="title">Opt-in local files</span>
                <p>Local handles remain outside portable projects and require explicit permission before write-back.</p>
            </cem-card>
            <section aria-label="Local file provider controls">
                <cem-badge data-cem-studio-provider-state label="Local files checking" tone="info"></cem-badge>
                <cem-action data-cem-studio-open-project variant="secondary">Open project folder</cem-action>
                <cem-action data-cem-studio-bind-project variant="quiet" disabled>Bind current project folder</cem-action>
                <cem-action data-cem-studio-open-resource variant="quiet" disabled>Open resource file</cem-action>
                <cem-action data-cem-studio-provider-reconnect variant="quiet" disabled>Reconnect permission</cem-action>
                <cem-action data-cem-studio-provider-write variant="secondary" disabled>Save back to folder</cem-action>
                <cem-action data-cem-studio-import-fallback variant="quiet" disabled>Import project backup</cem-action>
                <cem-action data-cem-studio-export-fallback variant="quiet" disabled>Export project backup</cem-action>
                <cem-alert data-cem-studio-provider-alert label="Local file provider checking" tone="info"></cem-alert>
            </section>
            <cem-alert data-cem-studio-update-state label="Application is current" tone="info"></cem-alert>
            <cem-action data-cem-studio-update variant="secondary" disabled>Apply ready update</cem-action>
        </div>`;
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

function installLabel(state) {
    return {
        waiting: 'Install availability waiting',
        ready: 'Install ready',
        prompting: 'Install prompt open',
        installed: 'Installed',
        dismissed: 'Install dismissed',
    }[state] ?? 'Install unavailable';
}

function updateLabel(status) {
    if (status.state === 'ready') return 'An update is ready; save work before applying it';
    if (status.state === 'persisting') return 'Persisting project before update';
    if (status.state === 'blocked' && status.reason === 'persistence-failed') {
        return 'Update blocked because project persistence failed';
    }
    if (status.state === 'blocked') return 'Update waiting for active work to finish';
    if (status.state === 'activating') return 'Applying update';
    return 'Application is current';
}

function fileSystemErrorState(code) {
    if (code?.includes('conflict')) return 'conflict';
    if (code?.includes('permission')) return 'permission-required';
    if (code?.includes('unsupported')) return 'unsupported';
    if (code?.includes('cancelled')) return 'unbound';
    return 'fallback';
}

function fileSystemStateLabel(state) {
    return {
        unsupported: 'Local files unsupported',
        unbound: 'Local files not connected',
        'prompt-permission': 'Local file permission required',
        'denied-permission': 'Local file permission denied',
        'permission-required': 'Local file permission required',
        conflict: 'External file conflict',
        fallback: 'Using IndexedDB fallback',
        working: 'Local file operation running',
        ready: 'Local files connected',
    }[state] ?? 'Local file provider ready';
}

function fileSystemTone(state) {
    if (state === 'ready') return 'success';
    if (state === 'conflict' || state?.includes('permission')) return 'warning';
    if (state === 'fallback') return 'danger';
    return 'info';
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
