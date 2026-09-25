import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/module-url-referrer.html', import.meta.url)),
    'utf8'
);

describe('module-url referrer demo source contract', () => {
    it('explains the URL-resolution boundary of referrer', () => {
        expect(DEMO_SOURCE).toContain('Purpose of <code>referrer</code>');
        expect(DEMO_SOURCE).toContain('select the matching import-map scope');
        expect(DEMO_SOURCE).toContain('Use <code>referrer-selector</code>');
    });

    it('declares the executable referrer import map', () => {
        const source = DEMO_SOURCE.match(/<script type="importmap">([\s\S]*?)<\/script>/u)?.[1];
        expect(source, 'page import map').toBeDefined();
        expect(() => JSON.parse(source ?? '')).not.toThrow();
        expect(JSON.parse(source ?? '{}')).toEqual({
            imports: {
                '@epa-wg/cem-ml/wasm': '../../cem-ml-npm/dist/wasm/browser/cem_ml.js',
                'demo-referrer-image': './lib-dir/Smiley.svg?referrer=default',
                'demo-module-referrer': './module-referrer/component.js',
            },
            scopes: {
                './relative-referrer/': {
                    'demo-referrer-image': './lib-dir/Smiley.svg?referrer=relative',
                },
                './module-referrer/': {
                    'demo-referrer-image': './confused.svg?referrer=module',
                },
                'https://referrer.example.test/absolute/': {
                    'demo-referrer-image': './wc-square.svg?referrer=absolute',
                },
            },
        });
    });

    it('owns the scalar src-by-referrer matrix moved from module-url.html', () => {
        const normalizedSource = DEMO_SOURCE.replace(/\s+/gu, ' ');
        expect(normalizedSource).toContain('legend="src by scalar referrer matrix"');
        const controls = [...normalizedSource.matchAll(/\{cem-module-url ([^}]+)\}/gu)].map(match => match[1]);
        const expected = [
            ['Relative', './relative-referrer/component.js'],
            ['Module', 'demo-module-referrer'],
            ['Absolute', 'https://referrer.example.test/absolute/component.js'],
        ].flatMap(([suffix, referrer]) => [
            ['relative', `../lib-dir/Smiley.svg?case=relative-${suffix.toLowerCase()}`],
            ['module', 'demo-referrer-image'],
            ['absolute', 'https://assets.example.test/logo.svg'],
        ].map(([prefix, src]) => {
            expect(normalizedSource).toContain(`{$datadom.slices.${prefix}By${suffix}}`);
            return `@slice=${prefix}By${suffix} @src="${src}" @referrer="${referrer}"`;
        }));
        expect(controls).toEqual(expected);
        expect(DEMO_SOURCE).toContain('<template>\n<cem-element>');
        expect(normalizedSource.match(/\{th @scope=col /gu)).toHaveLength(4);
        expect(normalizedSource.match(/\{th @scope=row /gu)).toHaveLength(3);
        expect(DEMO_SOURCE).toContain('href="./module-url.html"');
        expect(normalizedSource).not.toContain('{module-url');
    });
});
