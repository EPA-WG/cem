/**
 * @typedef {object} CemStudioMountOptions
 * @property {Element | string} [root]
 * @property {string | URL} [baseUrl]
 */

/**
 * @typedef {object} CemStudioMount
 * @property {Element} root
 * @property {URL} baseUrl
 * @property {() => void} dispose
 */

/**
 * Mount the application bootstrap without registering a service worker or
 * starting a CEM-ML command worker.
 *
 * @param {CemStudioMountOptions} [options]
 * @returns {CemStudioMount}
 */
export function mountCemStudio(options = {}) {
    const root = resolveRoot(options.root);
    const baseUrl = new URL(options.baseUrl ?? document.baseURI, document.baseURI);
    root.setAttribute('data-cem-studio-mounted', 'true');
    root.setAttribute('data-cem-studio-base-url', baseUrl.href);

    return Object.freeze({
        root,
        baseUrl,
        dispose() {
            root.removeAttribute('data-cem-studio-mounted');
            root.removeAttribute('data-cem-studio-base-url');
        },
    });
}

/** Load the public browser command surface on demand. */
export async function loadCemMlBrowser() {
    return import('@epa-wg/cem-ml-cli/browser');
}

/**
 * Register the graph-emitted service worker only after an embedding host opts
 * in. Importing this module has no service-worker side effect.
 *
 * @param {RegistrationOptions} [options]
 */
export async function registerCemStudioServiceWorker(options = {}) {
    if (!('serviceWorker' in navigator)) {
        throw new Error('CEM Studio service workers are unavailable in this browser');
    }
    const serviceWorkerUrl = new URL('../../service-worker.js', import.meta.url);
    return navigator.serviceWorker.register(serviceWorkerUrl, {
        scope: options.scope,
        type: 'module',
        updateViaCache: 'none',
    });
}

/** @param {Element | string | undefined} candidate */
function resolveRoot(candidate) {
    if (candidate instanceof Element) return candidate;
    const selector = candidate ?? '[data-cem-studio-root]';
    const root = document.querySelector(selector);
    if (!root) throw new Error(`CEM Studio mount root not found: ${selector}`);
    return root;
}
