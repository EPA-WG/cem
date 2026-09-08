import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

interface SampleContract {
    legend: string;
    declarationCount: number;
    includes: readonly string[];
}

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/form.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS: readonly SampleContract[] = [
    {
        legend: '1. Simple validation',
        declarationCount: 1,
        includes: ['@slice=signin', '{cem:choose', 'Sign in', 'Form valid:'],
    },
    {
        legend: '2. Form lifecycle',
        declarationCount: 1,
        includes: ['@slice=lifecycle', '@name=confirmBy', 'Message and data rates may apply.', 'Select a confirmation method.'],
    },
    {
        legend: '3. Native control validity message',
        declarationCount: 1,
        includes: ['@slice=nativeMessage', '@required=true', 'controls.email.validationMessage'],
    },
    {
        legend: '4. Form custom validity message',
        declarationCount: 1,
        includes: ['@slice=customMessage', 'Current length:', 'customMessage.validationMessage'],
    },
    {
        legend: '5. DCE as a form input',
        declarationCount: 2,
        includes: [
            '<cem-element tag="cem-form-fruit-choice" capability="choice-select">',
            '@data-option-index="{$option.index}"',
            '@slice=fruitForm',
            '{cem-form-fruit-choice @name=firstFruit',
            '{cem-form-fruit-choice @name=secondFruit',
        ],
    },
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<cem-demo-element[\s\S]*?legend="([^"]+)"[\s\S]*?<\/cem-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('form demo source contracts', () => {
    it('adapts the legacy page description to current form and validation state', () => {
        expect(DEMO_SOURCE).toContain('datadom.formData.&lt;slice&gt;');
        expect(DEMO_SOURCE).toContain('boolean or a');
        expect(DEMO_SOURCE).toContain('datadom.validationState');
        expect(DEMO_SOURCE).toContain('href="https://developer.mozilla.org/en-US/docs/Web/API/FormData"');
    });

    it('ports each legacy use case into its own cem-demo-element', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(({ legend }) => legend));
        expect(samples).toHaveLength(5);
    });

    it.each(SAMPLE_CONTRACTS)('$legend owns a flush-left, minimal declaration set', ({ legend, declarationCount, includes }) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();

        const source = sample?.source ?? '';
        const normalizedSource = source.replace(/\s+/gu, ' ');
        expect(source).toMatch(/<template>\n<cem-element/u);
        expect(source.match(/<cem-element(?:\s|>)/gu)).toHaveLength(declarationCount);
        expect(source).not.toContain('data-role=');
        expect(source).not.toContain('data-role="');
        for (const required of includes) {
            expect(normalizedSource, `${legend} must include ${required}`).toContain(
                required.replace(/\s+/gu, ' ')
            );
        }
    });

    it('names only the reusable form-associated DCE', () => {
        const namedDeclarations = Array.from(DEMO_SOURCE.matchAll(/<cem-element\s+[^>]*\btag="([^"]+)"/gu));
        expect(namedDeclarations.map((match) => match[1])).toEqual(['cem-form-fruit-choice']);
    });
});
