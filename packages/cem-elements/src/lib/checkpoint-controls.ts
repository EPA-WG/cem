import { cemPlaintextEditorEventValue } from './plaintext-editor-capability.js';

/** Explicit browser control-state DTO; not a document or template AST. Native
 * validation is mandatory before storing this in canonical checkpoint form state. */
export interface CemCheckpointControlState {
    version: 'cem-checkpoint-controls/1';
    controls: {
        id: string; kind: 'text' | 'check' | 'select' | 'button'; inputType: string; value: string;
        checked: boolean; indeterminate: boolean; multiple: boolean;
        options: { value: string; selected: boolean; disabled: boolean }[];
        selection: { start: number; end: number; direction: string } | null;
        focused: boolean; disabled: boolean; readOnly: boolean; customValidity: string;
    }[];
}
export function captureCheckpointControls(elements: readonly Element[]): CemCheckpointControlState | undefined {
    if (elements.length > 256) return undefined;
    const controls: CemCheckpointControlState['controls'] = [];
    let size = 0;
    for (const element of elements) {
        const id = element.getAttribute('data-cem-render-node-id');
        if (!id || element.matches('input[type=password],input[type=file],[autocomplete~="current-password" i],[autocomplete~="new-password" i],[autocomplete~="one-time-code" i]')) return undefined;
        const plaintext = cemPlaintextEditorEventValue(element);
        const input = element as HTMLInputElement;
        const inputType = plaintext !== undefined ? 'plaintext' : element.localName === 'textarea' ? 'textarea' : input.type;
        if (['password', 'file'].includes(inputType)) return undefined;
        const kind = element.localName === 'select' ? 'select' : ['checkbox', 'radio'].includes(inputType) ? 'check'
            : element.localName === 'button' || ['button', 'submit', 'reset'].includes(inputType) ? 'button' : 'text';
        if (element.hasAttribute('contenteditable') && plaintext === undefined) return undefined;
        const value = plaintext ?? input.value;
        if (typeof value !== 'string') return undefined;
        size += value.length; if (size > 256 * 1024) return undefined;
        const select = element as HTMLSelectElement;
        if (kind === 'select' && select.options.length > 1024) return undefined;
        const selection = ['text', 'search', 'tel', 'url', 'textarea', 'plaintext'].includes(inputType)
            && typeof input.selectionStart === 'number' && typeof input.selectionEnd === 'number'
            ? { start: input.selectionStart, end: input.selectionEnd, direction: input.selectionDirection ?? 'none' } : null;
        const customValidity = input.validity?.customError ? input.validationMessage || undefined : '';
        if (customValidity === undefined) return undefined;
        const options = kind === 'select' ? Array.from(select.options, option => ({ value: option.value, selected: option.selected, disabled: option.disabled })) : [];
        for (const option of options) size += option.value.length;
        if (size > 256 * 1024) return undefined;
        controls.push({ id, kind, inputType, value, checked: kind === 'check' && input.checked,
            indeterminate: kind === 'check' && input.indeterminate, multiple: kind === 'select' && select.multiple,
            options, selection, focused: element.ownerDocument.activeElement === element,
            disabled: plaintext !== undefined ? element.getAttribute('aria-disabled') === 'true' : !!input.disabled,
            readOnly: plaintext !== undefined ? element.getAttribute('aria-readonly') === 'true' : !!input.readOnly,
            customValidity });
    }
    return { version: 'cem-checkpoint-controls/1', controls };
}
