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
        expect(normalizedSource).toContain(
            '@src="../lib-dir/Smiley.svg?case=relative-relative" @referrer="./relative-referrer/component.js"'
        );
        expect(normalizedSource).toContain(
            '@src="demo-referrer-image" @referrer="demo-module-referrer"'
        );
        expect(normalizedSource).toContain(
            '@src="https://assets.example.test/logo.svg" @referrer="https://referrer.example.test/absolute/component.js"'
        );
        expect(normalizedSource).toContain('{$datadom.slices.relativeByRelative}');
        expect(normalizedSource).toContain('$datadom.slices.moduleByModule');
        expect(normalizedSource).toContain('{$datadom.slices.absoluteByAbsolute}');
        expect(normalizedSource).not.toContain('{module-url');
    });
});
