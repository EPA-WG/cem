import type { CemProcessingStylesheetResult } from './internal/runtime-support/processing-host.js';

export interface DeclarationStylesheetMarkup {
    index: number;
    scope: { kind: 'private' } | { kind: 'shared'; name: string };
    output: Pick<Extract<CemProcessingStylesheetResult, { status: 'ready' }>, 'css' | 'identity'>;
}

/** Shared SSR/browser metadata. Native output remains the authority on identity and CSS. */
export function declarationStylesheetAttributes(style: DeclarationStylesheetMarkup): Record<string, string> {
    return {
        'data-cem-declaration-style': style.scope.kind,
        'data-cem-style-key': style.output.identity.cacheKey,
        'data-cem-style-index': String(style.index),
        ...(style.scope.kind === 'shared' ? { 'data-cem-style-scope': style.scope.name } : {}),
    };
}

/** Place the returned HTML directly under the owning declaration, outside its template. */
export function serializeDeclarationStylesheets(styles: readonly DeclarationStylesheetMarkup[]): {
    html: string; contextMarker: string | null;
} {
    const indices = new Set(styles.map(style => style.index));
    const contexts = new Set(styles.map(style => style.output.identity.contextMarker).filter(value => value !== null));
    if (indices.size !== styles.length || contexts.size > 1
        || styles.some(style => !Number.isSafeInteger(style.index) || style.index < 0)) {
        throw new TypeError('invalid declaration stylesheet batch');
    }
    const html = styles.map(style => {
        if (/\0|<\/style/i.test(style.output.css)) throw new TypeError('unsafe stylesheet HTML');
        const attributes = Object.entries(declarationStylesheetAttributes(style)).map(([name, value]) => {
            if (value.includes('\0')) throw new TypeError('unsafe stylesheet attribute');
            const escaped = value.replaceAll('&', '&amp;').replaceAll('"', '&quot;')
                .replaceAll('<', '&lt;').replaceAll('>', '&gt;');
            return ` ${name}="${escaped}"`;
        }).join('');
        return `<style${attributes}>${style.output.css}</style>`;
    }).join('');
    return { html, contextMarker: contexts.values().next().value ?? null };
}
