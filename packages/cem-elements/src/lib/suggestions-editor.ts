import { getCemEditorProvider, type CemEditorProvider } from './form-control-capability.js';
import { isCemDatalistInput } from './suggestions-placements.js';
import { interactionReference } from './interaction-reference.js';

export type CemSuggestionsEditor = { host: HTMLElement; provider: CemEditorProvider; editor: HTMLInputElement; code?: never }
    | { code: string; host?: never; provider?: never; editor?: never };

/** Endpoint identity only. The consuming lifecycle must separately admit foreign placements. */
export function resolveCemSuggestionsEditor(host: HTMLElement, profile: 'listbox' | 'native-datalist' = 'listbox'): CemSuggestionsEditor {
    const slots = [...host.children].filter(child => child.getAttribute('slot') === 'editor');
    const explicit = host.hasAttribute('editor-for'), typed = host.hasAttribute('data-cem-node-ref-editor-for');
    if (slots.length > 1 || explicit && slots.length > 0) return { code: 'suggestions-editor-conflict' };
    let target: Element | undefined;
    if (explicit) {
        const value = host.getAttribute('editor-for') ?? '';
        // This endpoint admits native projection or the scoped local-name convenience only.
        if (!typed && !/^@[\w.-]+$/.test(value)) return { code: 'suggestions-editor-reference-invalid' };
        const result = interactionReference(host, value, 'editor-for');
        if (!result.target) return { code: result.code ?? 'suggestions-editor-missing' };
        target = result.target;
    } else if (typed) return { code: 'suggestions-editor-reference-invalid' };
    else target = slots[0];
    if (!(target instanceof HTMLElement) || !host.isConnected || !target.isConnected || target.getRootNode() !== host.getRootNode()) {
        return { code: 'suggestions-editor-missing' };
    }
    const provider = getCemEditorProvider(target), editor = provider?.control;
    if (!provider || !(editor instanceof HTMLInputElement)) return { code: 'suggestions-editor-provider-invalid' };
    if (profile === 'native-datalist' ? !isCemDatalistInput(editor) : editor.type !== 'text' || editor.hasAttribute('list') || target.hasAttribute('list')) return { code: 'suggestions-editor-profile-conflict' };
    return { host: target, provider, editor };
}
