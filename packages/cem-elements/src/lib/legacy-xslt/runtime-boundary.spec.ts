import { describe, expect, it } from 'vitest';

import { decideCemDeclarationTemplateLanguage } from './template-language.js';

describe('legacy bridge runtime boundary', () => {
    it.each([
        [{ type: null, lang: 'custom-element-v0', source: '<if test="$label">yes</if>' }, 'legacy-xslt'],
        [{ type: null, lang: 'custom-element-xslt', source: '<if test="$label">yes</if>' }, 'dom'],
        [{ type: null, lang: null, source: '<if test="$label">yes</if>' }, 'dom'],
        [{ type: null, lang: null, source: '<xsl:variable name="value">yes</xsl:variable>' }, 'dom'],
        [{ type: 'text/cem-ml', lang: 'custom-element-v0', source: '{span | canonical}' }, 'cem-ml'],
        [{ type: 'application/cem-ml', lang: null, source: '{span | canonical}' }, 'cem-ml'],
        [{ type: null, lang: null, source: '  @doc {span | canonical}' }, 'cem-ml'],
        [{ type: null, lang: null, source: '  {span | canonical}' }, 'cem-ml'],
        [{ type: null, lang: 'CUSTOM-ELEMENT-V0', source: '<if test="$label">yes</if>' }, 'dom'],
    ] as const)(
        'selects $expected only for its explicit boundary: $input',
        (input, expected) => {
            expect(decideCemDeclarationTemplateLanguage(input)).toBe(expected);
        }
    );

    it('does not recognize legacy markup by element names', () => {
        expect(decideCemDeclarationTemplateLanguage({
            type: null,
            lang: null,
            source: '<choose><when test="$kind">kind</when><otherwise>none</otherwise></choose>',
        })).toBe('dom');
    });
});
