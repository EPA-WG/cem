import { describe, expect, it } from 'vitest';

import { analyzeExternalDeclarationSourceFormat } from './external-declaration-source.js';

describe('external declaration source format', () => {
    it.each([
        ['application/xslt+xml', '/templates/tree.xsl'],
        ['application/xslt+xml; charset=utf-8', '/templates/tree'],
        ['text/xsl', '/templates/tree'],
        ['custom-element-xslt', '/templates/tree'],
    ])('selects standalone XSLT from %s', (contentType, specifier) => {
        expect(analyzeExternalDeclarationSourceFormat({ specifier, contentType })).toEqual({
            kind: 'xslt',
            contentType: contentType.split(';', 1)[0],
        });
    });

    it.each([undefined, 'application/xml', 'text/xml', 'application/octet-stream'])(
        'permits .xsl extension fallback for %s metadata',
        (contentType) => {
            expect(
                analyzeExternalDeclarationSourceFormat({
                    specifier: './tree.xsl?revision=1#main',
                    contentType,
                }).kind,
            ).toBe('xslt');
        },
    );

    it('fails closed when an XSLT path has a contradictory concrete media type', () => {
        expect(
            analyzeExternalDeclarationSourceFormat({
                specifier: './tree.xsl',
                contentType: 'text/html; charset=utf-8',
            }),
        ).toEqual({
            kind: 'invalid',
            contentType: 'text/html',
            diagnosticCode: 'cem-element.src_content_type_mismatch',
            message: 'XSLT declaration source `./tree.xsl` was served as `text/html`',
        });
    });

    it('keeps ordinary HTML and CEM-ML declaration resources on the document path', () => {
        expect(
            analyzeExternalDeclarationSourceFormat({
                specifier: './template.html',
                contentType: 'text/html',
            }).kind,
        ).toBe('html');
        expect(
            analyzeExternalDeclarationSourceFormat({
                specifier: './template.cemt',
                contentType: 'text/cem-ml',
            }).kind,
        ).toBe('html');
    });
});
