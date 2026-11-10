import { CEM_NAVIGATION_LANDMARK_CAPABILITY, cemNavigationLandmarkCheckpoint } from '../../navigation-landmark-capability.js';
import type { QuerySourceResource } from './query-source-resource.js';

const pending = new WeakMap<HTMLElement, object>();
const installed = new WeakMap<HTMLElement, () => void>();
const behaviorIdentity = 'cem-elements-navigation-landmark-v1';
interface BrowserFrame {
    tag: string;
    instance: string;
    behaviorIdentity: string;
    declarationIdentity: string;
    html: string;
}

/** The renderer supplies the connected host generation's lifetime and commits
 * the native HTML before calling commit. The session owns the behavior grant. */
export async function prepareApplicationBrowserFrame(
    resource: QuerySourceResource, element: HTMLElement, island: Uint8Array, instance: string, signal: AbortSignal,
) {
    const tag = element.localName;
    const document = element.ownerDocument;
    const candidate = {};
    if (signal.aborted || !element.isConnected) throw new Error('cem.application.browser-frame.stale');
    pending.set(element, candidate);
    const result = await resource.renderBrowserIsland({ tag, bytes: island, instance });
    const frame: Partial<BrowserFrame> | null = JSON.parse(result.receipt);
    if (frame?.tag !== tag || frame?.instance !== instance || frame?.behaviorIdentity !== behaviorIdentity
        || typeof frame.html !== 'string' || typeof frame.declarationIdentity !== 'string'
        || !/^cem-bin\/1\+blake3:[0-9a-f]{64}$/.test(frame.declarationIdentity)) {
        throw new Error('cem.application.browser-frame.receipt');
    }
    const current = () => !signal.aborted && result.isCurrent() && pending.get(element) === candidate
        && element.isConnected && element.ownerDocument === document;
    if (!current()) throw new Error('cem.application.browser-frame.stale');
    let released = false, committed = false;
    const release = () => {
        if (released) return;
        released = true;
        signal.removeEventListener('abort', release);
        result.signal.removeEventListener('abort', release);
        if (pending.get(element) === candidate) pending.delete(element);
        if (installed.get(element) === release) {
            installed.delete(element);
            CEM_NAVIGATION_LANDMARK_CAPABILITY.disconnected(element);
        }
    };
    signal.addEventListener('abort', release, { once: true });
    result.signal.addEventListener('abort', release, { once: true });
    return Object.freeze({
        html: frame.html,
        declarationIdentity: frame.declarationIdentity,
        commit(): boolean {
            if (released || committed) return false;
            const previous = installed.get(element);
            if (!current() || (!previous && cemNavigationLandmarkCheckpoint(element))) { release(); return false; }
            committed = true;
            pending.delete(element);
            installed.set(element, release);
            previous?.();
            CEM_NAVIGATION_LANDMARK_CAPABILITY.connected(element);
            return true;
        },
        release,
    });
}
