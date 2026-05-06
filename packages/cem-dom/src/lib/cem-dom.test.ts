import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { formatDiagnostics, parseCemDom, validateCemDom } from './cem-dom.ts';

describe('parseCemDom', () => {
    it('parses semantic elements, attributes, and text', () => {
        const document = parseCemDom(`
            <main data-cem-screen="login" aria-labelledby="title">
                <h1 id="title">Sign in</h1>
                <button data-cem-action="submit">Continue</button>
            </main>
        `);

        assert.equal(document.diagnostics.length, 0);
        assert.equal(document.elements[0]?.tagName, 'main');
        assert.equal(document.elements.length, 3);
        assert.equal(document.elements[1]?.children[0]?.type, 'text');
        assert.equal(document.elements[1]?.children[0]?.type === 'text' && document.elements[1].children[0].value, 'Sign in');
    });

    it('records parse diagnostics for misnested markup', () => {
        const document = parseCemDom('<section><p>Broken</section>');

        assert.equal(document.diagnostics.some((diagnostic) => diagnostic.code === 'parse.misnested-element'), true);
    });
});

describe('validateCemDom', () => {
    it('accepts labelled semantic CEM documents', () => {
        const diagnostics = validateCemDom(`
            <main data-cem-screen="login" aria-labelledby="title">
                <h1 id="title">Sign in</h1>
                <form data-cem-form="credentials" aria-label="Credentials">
                    <label for="email">Email</label>
                    <input id="email" name="email" />
                    <button data-cem-action="submit" aria-label="Continue"></button>
                </form>
            </main>
        `);

        assert.deepEqual(diagnostics, []);
    });

    it('flags broken references and unsafe content', () => {
        const diagnostics = validateCemDom(`
            <main data-cem-screen="login" aria-labelledby="missing">
                <a href="javascript:alert(1)">Bad link</a>
                <script>alert(1)</script>
            </main>
        `);

        assert.equal(diagnostics.some((diagnostic) => diagnostic.code === 'validate.broken-reference'), true);
        assert.equal(diagnostics.some((diagnostic) => diagnostic.code === 'validate.unsafe-url'), true);
        assert.equal(diagnostics.some((diagnostic) => diagnostic.code === 'validate.unsafe-script'), true);
    });
});

describe('formatDiagnostics', () => {
    it('formats empty and non-empty diagnostic sets', () => {
        assert.equal(formatDiagnostics([]), 'No CEM DOM diagnostics.');

        assert.match(
            formatDiagnostics([
                {
                    code: 'example',
                    severity: 'warning',
                    message: 'Example warning.',
                    location: { offset: 0, line: 1, column: 1 },
                },
            ]),
            /WARNING example 1:1 Example warning\./,
        );
    });
});
