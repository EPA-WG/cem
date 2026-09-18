import { readFileSync } from 'node:fs';
import { beforeAll, expect, it } from 'vitest';
// eslint-disable-next-line @nx/enforce-module-boundaries -- exercise the production native document capability.
import { initSync, compileTemplate, disposeTemplate, retainCemDocument, disposeCemDocument,
    retainCemtXPathFunctions, disposeCemtXPathFunctions, renderTemplateWithCemDocuments,
} from '../../../cem_ql/dist/wasm/cem_ql.js';

beforeAll(() => {
    initSync({ module: readFileSync(new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
});

it('renders equivalent imported documents and rejects stale or ambiguous bindings through WASM', () => {
    const library = readFileSync(new URL('../../demo/http-data.cemt', import.meta.url), 'utf8');
    const { companionId } = JSON.parse(retainCemtXPathFunctions(library, 'memory:http-data.cemt'));
    const template = JSON.parse(compileTemplate(`{output | {$native:call("http.field",
        native:call("http.rows", datadom.slices.response.data), "name")}}`, '["datadom"]'));
    expect(template.diagnostics).toEqual([]);
    // This is control metadata only; the external document is always raw bytes.
    const metadata = '{"datadom":{"slices":{"response":{"state":"loaded","data":null}}}}';
    try {
        for (const [source, mime] of [
            ['{"results":[{"name":"alpha"}]}', 'application/json'],
            ['<catalog><item name="alpha"/></catalog>', 'application/xml'],
        ]) {
            const documentId = retainCemDocument(new TextEncoder().encode(source), mime, 'memory:response');
            const bindings = JSON.stringify([{ slice: 'response', documentId }]);
            const render = (control = metadata, binding = bindings) => JSON.parse(
                renderTemplateWithCemDocuments(template.artifactId, companionId, control, binding));
            try {
                const result = render();
                expect(result.diagnostics).toEqual([]);
                expect(result.nodes[0].children[0].text).toBe('alpha');
                expect(render(metadata, JSON.stringify([{ slice: 'response', documentId }, { slice: 'response', documentId }]))
                    .diagnostics[0].code).toBe('cem.ql.wasm.invalid_document_binding');
                expect(render('{}').diagnostics[0].code).toBe('cem.ql.wasm.invalid_document_binding');
            } finally {
                expect(disposeCemDocument(documentId)).toBe(true);
            }
            expect(render().diagnostics[0].code).toBe('cem.ql.wasm.unknown_document');
            expect(disposeCemDocument(documentId)).toBe(false);
        }
        expect(() => retainCemDocument(new TextEncoder().encode('{'), 'application/json', 'memory:invalid')).toThrow();
    } finally {
        disposeTemplate(template.artifactId);
        disposeCemtXPathFunctions(companionId);
    }
});
