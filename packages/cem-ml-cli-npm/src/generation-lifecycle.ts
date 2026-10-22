// eslint-disable-next-line @nx/enforce-module-boundaries -- shared lifecycle controls are vendored into this deployment.
export * from '../../cem_ql/runtime/native-generation.mjs';

/** Internal startup cancellation: terminate promptly while the ordinary startup
 * verifier settles its own bounded listener/timer cleanup. */
export async function initializeFreshGeneration<T>(signal: AbortSignal | undefined, initialize: () => Promise<T>, terminate: () => void): Promise<T> {
    if (!signal) return initialize();
    if (signal.aborted) { terminate(); throw new Error('cem.runtime.allocation.cancelled'); }
    let cancel: () => void = () => undefined;
    try {
        return await Promise.race([initialize(), new Promise<never>((_resolve, reject) => {
            cancel = () => { terminate(); reject(new Error('cem.runtime.allocation.cancelled')); };
            if (signal.aborted) cancel(); else signal.addEventListener('abort', cancel, { once: true });
        })]);
    } finally { signal.removeEventListener('abort', cancel); }
}
