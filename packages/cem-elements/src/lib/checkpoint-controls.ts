import type { CheckpointSource, CheckpointSuggestions, CheckpointPlacement, CheckpointSurface, CheckpointPopup, CheckpointMenu, CheckpointNativePublication, CheckpointManualListbox } from './checkpoint-interactions.js';
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
/** Named capability/resource lifecycle DTO. Document bodies are original source
 * bytes for native import; they are never parsed or projected into a JS AST. */
export interface CemCheckpointRuntimeState {
    version: 'cem-checkpoint-runtime/1';
    references?: CheckpointSource;
    nativePublication?: CheckpointNativePublication;
    manualListbox?: CheckpointManualListbox;
    placements?: CheckpointPlacement[];
    focus?: { target: 'host' | 'node'; nodeId: string | null };
    capability: null | { kind: 'suggestions'; state: CheckpointSuggestions } | { kind: 'navigation-landmark' }
        | { kind: 'tree-view'; focusValue: string; typeahead: string; typeaheadAt: number }
        | { kind: 'tab-panels'; focusIndex: number; tabCount: number }
        | { kind: 'composite-menu'; activeNode: string | null; typeahead: string; typeaheadAt: number; open: boolean; state?: CheckpointMenu }
        | { kind: 'popup'; open: boolean; state?: CheckpointPopup }
        | { kind: 'native-surface'; phase: 'closed' | 'open'; defaultsInitialized: true; state?: CheckpointSurface }
        | { kind: 'form-control'; changeSuppression?: { revision: number; value: string; disposition: 'suppress-once' }; provider?: { revision: number; consumer: string; validity: string }; authoredValue: string | null; formDisabled: boolean; customValidity: string; selectionRequest: string | null }
        | { kind: 'plaintext-editor'; authored: string | null; selectionRequest: string | null }
        | { kind: 'checkable-control'; authoredIndeterminate: boolean }
        | { kind: 'choice-select'; selection: string[]; defaultValues: string[]; activeIndex: number; rangeAnchor: number;
            expanded: boolean; previewSelection: string[]; mode: string; authoredValue: string | null; valueRequest: string | null;
            formDisabled: boolean; typeahead: string; typeaheadAt: number; focusedPart: string | null }
        | { kind: 'editable-choice'; activeIndex: number; authoredValue: string | null; committedOptionValue: string | null;
            defaultCommittedOptionValue: string | null; defaultDisplayValue: string; defaultValue: string; displayValue: string;
            edited: boolean; expanded: boolean; formDisabled: boolean; value: string };
    resources: { name: string; kind: string; disposition: string; revision: number; cursor: number; live: boolean; state: string;
        url?: string; method?: string; document?: { base64: string; contentType: string; sourceUri: string; digest: string | null } }[];
    writers: { index: number; trigger: string | null; implicit: boolean }[];
    writeDisposition: 'suppress-until-new-input';
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

/** Existing scalar repository DTOs must survive DOM-native record encoding
 * exactly; structured-clone support alone also permits nonportable host values. */
export function checkpointScalarPortable(value: unknown): boolean {
    const seen = new Set<object>(); let count = 0, length = 0;
    const visit = (value: unknown, depth: number): boolean => {
        if (++count > 16384 || depth > 64) return false;
        if (typeof value === 'string') { length += value.length; return length <= 512 * 1024; }
        if (value === null || typeof value === 'boolean') return true;
        if (typeof value === 'number') return Number.isFinite(value);
        if (typeof value !== 'object' || !value || seen.has(value)) return false;
        if (!Array.isArray(value) && Object.getPrototypeOf(value) !== Object.prototype && Object.getPrototypeOf(value) !== null) return false;
        if (Array.isArray(value) && value.length > 16384) return false;
        const keys = Reflect.ownKeys(value);
        if (keys.length > 16385) return false;
        for (const key of keys) {
            if (typeof key !== 'string') return false;
            if (Array.isArray(value) && key === 'length') continue;
            const property = Object.getOwnPropertyDescriptor(value, key);
            if (!property?.enumerable || !Object.hasOwn(property, 'value')) return false;
        }
        if (Array.isArray(value) && keys.length !== value.length + 1) return false;
        seen.add(value);
        let result = true;
        if (Array.isArray(value)) {
            if (value.length > 16384) result = false;
            else for (const child of value) if (!visit(child, depth + 1)) { result = false; break; }
        } else {
            for (const key in value) {
                if (!Object.hasOwn(value, key)) continue;
                length += key.length;
                if (length > 512 * 1024 || !visit((value as Record<string, unknown>)[key], depth + 1)) { result = false; break; }
            }
        }
        seen.delete(value); return result;
    };
    return visit(value, 0);
}
