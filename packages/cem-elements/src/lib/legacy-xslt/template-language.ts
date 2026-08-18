export type CemDeclarationTemplateLanguage = 'dom' | 'cem-ml' | 'legacy-xslt';

export interface CemDeclarationTemplateLanguageInput {
    type: string | null;
    lang: string | null;
    source: string;
}

/**
 * Select the browser declaration path without sniffing legacy markup. The
 * `custom-element-xslt` identity belongs to the engine/CLI content-type boundary;
 * browser compatibility is the exact, explicit migration annotation below.
 */
export function decideCemDeclarationTemplateLanguage(
    input: CemDeclarationTemplateLanguageInput
): CemDeclarationTemplateLanguage {
    if (input.type === 'text/cem-ml' || input.type === 'application/cem-ml') {
        return 'cem-ml';
    }
    if (input.lang === 'custom-element-v0') {
        return 'legacy-xslt';
    }
    const source = input.source.trim();
    if (source.startsWith('@doc') || source.startsWith('{')) {
        return 'cem-ml';
    }
    return 'dom';
}
