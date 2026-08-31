export type CemExternalDeclarationSourceKind = 'html' | 'xslt' | 'invalid';

export interface CemExternalDeclarationSourceFormatInput {
    specifier: string;
    contentType?: string;
}

export interface CemExternalDeclarationSourceFormat {
    kind: CemExternalDeclarationSourceKind;
    contentType: string | null;
    diagnosticCode?: 'cem-element.src_content_type_mismatch';
    message?: string;
}

const XSLT_CONTENT_TYPES = new Set([
    'application/xslt+xml',
    'text/xsl',
    'custom-element-xslt',
    'text/custom-element-xslt',
    'application/custom-element-xslt',
    'text/x-custom-element-xslt',
]);

const GENERIC_XML_CONTENT_TYPES = new Set([
    'application/xml',
    'text/xml',
    'application/octet-stream',
]);

/**
 * Select the parser/compiler boundary for an external declaration resource.
 * Explicit XSLT media types win. A `.xsl`/`.xslt` extension is only a fallback
 * for absent or generic XML metadata; a contradictory concrete media type fails
 * closed instead of silently parsing stylesheet bytes as HTML.
 */
export function analyzeExternalDeclarationSourceFormat(
    input: CemExternalDeclarationSourceFormatInput,
): CemExternalDeclarationSourceFormat {
    const contentType = mediaType(input.contentType);
    const xsltExtension = hasXsltExtension(input.specifier);

    if (contentType && XSLT_CONTENT_TYPES.has(contentType)) {
        return { kind: 'xslt', contentType };
    }
    if (xsltExtension && (!contentType || GENERIC_XML_CONTENT_TYPES.has(contentType))) {
        return { kind: 'xslt', contentType };
    }
    if (xsltExtension) {
        return {
            kind: 'invalid',
            contentType,
            diagnosticCode: 'cem-element.src_content_type_mismatch',
            message: `XSLT declaration source \`${input.specifier}\` was served as \`${contentType}\``,
        };
    }
    return { kind: 'html', contentType };
}

function mediaType(value: string | null | undefined): string | null {
    const normalized = value?.split(';', 1)[0]?.trim().toLowerCase() ?? '';
    return normalized.length > 0 ? normalized : null;
}

function hasXsltExtension(specifier: string): boolean {
    const path = specifier.split(/[?#]/u, 1)[0]?.toLowerCase() ?? '';
    return path.endsWith('.xsl') || path.endsWith('.xslt');
}
