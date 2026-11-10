import { bindCemRetainedTextProvider, isCemCredentialControl } from '../../form-control-capability.js';
import { isDomInstance } from '../../dom-realm.js';
import type { QuerySourceResource } from './query-source-resource.js';

interface TextProviderFrame {
    tag: string;
    instance: string;
    behaviorIdentity: string;
    html: string;
    value: string;
    declarationIdentity: string;
}

/** Prepare the native-owned receiver before publishing a fresh shared provider.
 * The renderer keeps the admitted input node and ends its lifetime when it
 * replaces that receiver or control. */
export async function prepareApplicationTextProvider(
    resource: QuerySourceResource, host: HTMLElement, instance: string, signal: AbortSignal,
) {
    const document = host.ownerDocument;
    const tag = host.localName;
    const result = await resource.renderBrowserReceiver({ tag, instance });
    const frame: Partial<TextProviderFrame> | null = JSON.parse(result.receipt);
    if (frame?.tag !== tag || frame?.instance !== instance || frame?.behaviorIdentity !== 'cem-elements-form-control-v4'
        || typeof frame.html !== 'string' || typeof frame.value !== 'string' || typeof frame.declarationIdentity !== 'string'
        || !/^cem-bin\/1\+blake3:[0-9a-f]{64}$/.test(frame.declarationIdentity)) {
        throw Error('cem.application.text-provider.receipt');
    }
    let binding: ReturnType<typeof bindCemRetainedTextProvider>;
    let released = false;
    const live = () => !released && !signal.aborted && result.isCurrent() && host.isConnected && host.ownerDocument === document;
    if (!live()) throw Error('cem.application.text-provider.stale');
    const release = () => {
        if (released) return;
        released = true;
        signal.removeEventListener('abort', release);
        result.signal.removeEventListener('abort', release);
        binding?.release();
    };
    signal.addEventListener('abort', release, { once: true });
    result.signal.addEventListener('abort', release, { once: true });
    return Object.freeze({
        html: frame.html,
        declarationIdentity: frame.declarationIdentity,
        commit() {
            if (binding || released) return undefined;
            const input = host.firstElementChild;
            const current = () => live() && isDomInstance(input, 'HTMLInputElement') && host.children.length === 1
                && input.parentElement === host && input.id === instance && input.getAttribute('type') === 'text'
                && input.form === null && !isCemCredentialControl(input) && input.getAttribute('part') === 'control'
                && !!input.getAttribute('aria-label')?.trim();
            if (isDomInstance(input, 'HTMLInputElement') && current() && input.value === frame.value) {
                binding = bindCemRetainedTextProvider(host, input, current);
            }
            if (!binding) release();
            return binding?.provider;
        },
        release,
    });
}
