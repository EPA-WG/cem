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
 * Compose visible application chrome exclusively from production CEM controls.
 * @param {{
 *   root: Element,
 *   storage?: Storage,
 *   registration?: ServiceWorkerRegistration,
 *   repository?: {status: () => Promise<{state: string, persisted?: boolean}>},
 *   persistState?: () => Promise<void>,
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

    root.setAttribute('data-cem-studio-state', 'ready');
    root.innerHTML = shellMarkup(theme.mode);
    await settleShellComponents(components.runtime, root);
    const themeSelect = root.querySelector('cem-select[data-cem-studio-theme]');
    const installAction = root.querySelector('cem-action[data-cem-studio-install]');
    const updateAction = root.querySelector('cem-action[data-cem-studio-update]');
    const installBadge = root.querySelector('cem-badge[data-cem-studio-install-state]');
    const updateAlert = root.querySelector('cem-alert[data-cem-studio-update-state]');
    const storageBadge = root.querySelector('cem-badge[data-cem-studio-storage-state]');
    themeSelect.value = theme.mode;

    const themeChanged = () => theme.setMode(themeSelect.value);
    const installRequested = () => install.prompt();
    const updateRequested = () => update.activateUpdate();
    themeSelect.addEventListener('change', themeChanged);
    installAction.addEventListener('click', installRequested);
    updateAction.addEventListener('click', updateRequested);

    const unsubscribeInstall = install.subscribe(({ state: installState }) => {
        installBadge.setAttribute('label', installLabel(installState));
        installAction.toggleAttribute('disabled', installState !== 'ready');
    });
    const unsubscribeUpdate = update.subscribe((snapshot) => {
        updateAlert.setAttribute('label', updateLabel(snapshot));
        updateAlert.setAttribute('tone', snapshot.state === 'blocked' ? 'warning' : 'info');
        updateAction.toggleAttribute('disabled', !snapshot.updateReady || snapshot.state === 'activating');
    });

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
        dispose() {
            themeSelect.removeEventListener('change', themeChanged);
            installAction.removeEventListener('click', installRequested);
            updateAction.removeEventListener('click', updateRequested);
            unsubscribeInstall();
            unsubscribeUpdate();
            install.dispose();
            update.dispose();
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
