import { afterEach, beforeAll, describe, expect, it, onTestFinished, vi } from 'vitest';
import { createCemStudioNavigation, createCemStudioProjectPreview, createCemStudioSourcePreview } from './navigation.js';
import { mountCemStudioFeatureTourWorkbench } from './workbench.js';
import { mountCemStudioApplicationShell } from './shell.js';

beforeAll(() => {
    const map = document.createElement('script'); map.type = 'importmap';
    map.textContent = JSON.stringify({ imports: { '@epa-wg/cem-components/components.xhtml': new URL('../../cem-components/dist/components.xhtml', import.meta.url).href } });
    document.head.append(map);
});

const services = [];
afterEach(() => { for (const service of services.splice(0)) service.dispose(); });

async function fixture(url = 'https://studio.test/?project=demo&file=article', readOnly = false, { ambiguousSource = false, alternateBuild = false, plaintextSource = false, tabularSource = false, validator,
    sampleCollections = ['nested'], collections = [{ id: 'nested', name: 'Nested examples' }], binarySource = false, sourceBytes, missingSource = false, commandBytes, missingCommand = false, missingCommandResource = false, commandContentType = 'application/cem' } = {}) {
    const encoder = new TextEncoder();
    const sourceResource = binarySource
        ? { id: 'article', path: 'projection.cem-bin', contentType: 'application/vnd.cem.ast+cem-bin', schema: 'https://cem.dev/ns/projection/ast/1' }
        : plaintextSource
        ? { id: 'article', path: 'geometry.txt', contentType: 'text/plain' }
        : tabularSource ? { id: 'article', path: 'content/article.csv', contentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' }
            : { id: 'article', path: 'content/article.md', contentType: 'text/markdown', schema: 'https://cem.dev/ns/data/markdown/1' };
    const project = { id: 'demo', name: 'Demo', rootUri: 'studio://demo/', revision: 1, resources: [
        { ...sourceResource, revision: 1, sha256: 'article' },
        { id: 'other', path: 'content/other.md', contentType: 'text/markdown', revision: 1, sha256: 'other' },
        ...(!missingCommandResource ? [{ id: 'command', path: 'config/build.cem', contentType: commandContentType, schema: 'https://cem.dev/ns/cli/command/1', revision: 1, sha256: 'command' }] : []),
    ], entries: [{ id: 'build', name: 'Build site', kind: 'transformation-graph', runConfigResourceId: 'command', entryArtifact: 'generated/index.html' },
        ...(alternateBuild ? [{ id: 'alternate', name: 'Alternate build', kind: 'transformation-graph', runConfigResourceId: 'command', entryArtifact: 'generated/index.html' }] : [])] };
    const sourceText = tabularSource ? 'id,name\n1,Ada\n' : '# Article';
    const bundle = { project, contents: { ...(missingSource ? {} : { article: sourceBytes ?? (binarySource ? new Uint8Array([67, 69, 77, 0, 255]) : encoder.encode(sourceText)) }), other: encoder.encode('# Other'), ...(missingCommand ? {} : { command: commandBytes ?? encoder.encode('{}') }) } };
    const repository = { async query({ operation }) {
        if (operation === 'list-projects') return { value: [project] };
        if (operation === 'get-project') return { value: project };
        if (operation === 'export-project') return { value: bundle };
        if (operation === 'list-build-snapshots') return { value: [] };
        throw new Error(operation);
    } };
    const listeners = new Set();
    let state = { workbenchId: '', projectId: 'demo', resourceId: 'article', status: 'ready', contentType: sourceResource.contentType, projectRevision: 1, resourceRevision: 1, path: sourceResource.path, resourceAvailable: true, draft: '# Article', persistedText: '# Article', dirty: false, ...(readOnly ? { editable: false, status: 'read-only', draft: '', persistedText: '' } : {}) };
    const publish = () => { for (const listener of listeners) listener(state); };
    const workbench = { snapshot: () => state, subscribe(listener) { listeners.add(listener); listener(state); return () => listeners.delete(listener); },
        async selectWorkbench(id) { const target = JSON.parse(id); state = { ...state, workbenchId: id, entryId: target.entryId, resourceId: target.resourceId ?? (target.entryId ? 'article' : undefined) }; publish(); },
        async resolveNavigation() { return undefined; }, dispose() { return undefined; }, updateDraft(text) { state = { ...state, draft: text, dirty: true }; publish(); } };
    const browser = new EventTarget(); browser.location = new URL(url);
    browser.history = { pushState(_, __, next) { browser.location = new URL(next); }, replaceState(_, __, next) { browser.location = new URL(next); } };
    let fail = false, wait;
    const loadExample = vi.fn(async () => bundle);
    const service = await createCemStudioNavigation({ repository, workbench, browser, validator,
        examples: [{ id: 'demo', name: 'Demo sample', collections: sampleCollections, load: loadExample }], collections, openBuildWorkbench: async ({ configuration }) => {
        const artifacts = [ ['generated/index.html', ambiguousSource ? 'studio://demo/content/article.md' : undefined], ['generated/posts/article.html', 'studio://demo/content/article.md'], ['generated/invalid.json', undefined] ].map(([path, uri]) => ({
            id: path, path, uri: `studio://demo/${path}`, contentType: path.endsWith('.json') ? 'application/json' : 'text/html', provenance: uri ? [{ uri }] : [],
        }));
        const outputs = artifacts.map(item => ({ uri: item.uri, contentType: item.contentType, text: `<p>${item.path}</p>`, bytes: item.path.endsWith('.json') ? new Uint8Array([255, 0]) : encoder.encode(`<p>${item.path}</p>`) }));
        let projection = { exitCode: fail ? 1 : 0, buildRun: { id: 'run', projectId: project.id, configuration: { id: configuration.id, name: configuration.name }, inputs: project.resources.map(item => ({ resourceId: item.id, revision: item.revision, sha256: item.sha256 })), artifacts }, outputs,
            output: outputs[0], preview: { kind: 'sandboxed-html', label: 'Built output', source: outputs[0].text } };
        return { snapshot: () => ({ projection }), async runPersistedOperation() { if (wait) await wait; }, dispose() { return undefined; }, async copyProjection(write) { await write(projection.output.text); }, async downloadProjection(save) { await save({ filename: projection.output.uri.split('/').at(-1), contentType: projection.output.contentType, bytes: projection.output.bytes }); }, selectArtifact(id) {
            const index = artifacts.findIndex(item => item.id === id); projection = { ...projection, output: outputs[index], preview: { ...projection.preview, source: outputs[index].text } };
        } };
    } }); services.push(service);
    return { service, browser, workbench, loadExample, setState(next) { state = { ...state, ...next }; publish(); }, fail: () => { fail = true; }, delay: promise => { wait = promise; } };
}

function invocation({ projectId = 'demo', resourceId = 'article', configuration = resourceId, live = false } = {}) {
    const text = live ? '{"native":"parsed"}' : 'Native invocation result';
    const output = { uri: `studio://${projectId}/generated/invocation.txt`, contentType: live ? 'application/json' : 'text/plain', available: true, text, bytes: new TextEncoder().encode(text) };
    const preview = live ? { kind: 'live-cem', label: 'Retained live invocation', source: '{p | Retained live invocation}', sourceUri: 'cem-preview://fixture/view.cem', resourceSet: JSON.stringify({ resources: [], declarations: [], styles: [] }) }
        : { kind: 'text', label: 'Native invocation', text };
    return { kind: 'parse', preview, output, outputs: [output], stale: false, revision: { resourceRevision: 1 }, diagnostics: [], provenance: [], trace: [], graph: [], summary: {},
        buildRun: { id: 'invocation', projectId, configuration: { id: configuration, name: configuration }, inputs: [{ resourceId, revision: 1 }], artifacts: [{ id: 'invocation-output', path: 'generated/invocation.txt', uri: output.uri, contentType: output.contentType }] } };
}

describe('Studio compact navigation service', () => {
    const catalog = {
        collections: [
            { id: 'conference', name: 'Conference' },
            { id: 'content-types-and-schemas', name: 'Content types and schemas', description: 'Supported source workflows' },
            { id: 'content-types-data', name: 'Data', parentId: 'content-types-and-schemas' },
            { id: 'content-types-documents-ui', name: 'Documents and UI', parentId: 'content-types-and-schemas' },
            { id: 'content-types-queries-transformations', name: 'Queries and transformations', parentId: 'content-types-and-schemas' },
            { id: 'content-types-schema-authoring', name: 'Schema authoring', parentId: 'content-types-and-schemas' },
        ],
        sampleCollections: ['conference', 'content-types-data', 'content-types-documents-ui'],
    };

    it('models nested and empty catalog folders without loading projects and gives every occurrence a unique route', async () => {
        const { service, browser, loadExample } = await fixture('https://studio.test/', false, catalog);
        const samples = service.snapshot().navigation.samples;
        expect(samples.children.map(item => item.name)).toEqual(['Conference', 'Content types and schemas']);
        const categories = samples.children[1].children;
        expect(categories.map(item => item.name)).toEqual(['Data', 'Documents and UI', 'Queries and transformations', 'Schema authoring']);
        expect(categories[0].children[0]).toMatchObject({ key: 'collection=content-types-data&sample=demo', route: { sample: 'demo', collection: 'content-types-data' } });
        expect(categories[1].children[0]).toMatchObject({ key: 'collection=content-types-documents-ui&sample=demo', route: { sample: 'demo', collection: 'content-types-documents-ui' } });
        const keys = [];
        const collect = item => { keys.push(item.key); item.children.forEach(collect); };
        collect(samples);
        expect(new Set(keys).size).toBe(keys.length);
        await service.openEntity('collection=content-types-and-schemas');
        expect(service.snapshot().navigation).toMatchObject({ content: 'folder', count: 4, countKind: 'folder', selected: { description: 'Supported source workflows' } });
        await service.openEntity('collection=content-types-queries-transformations');
        expect(service.snapshot().navigation).toMatchObject({ content: 'folder', count: 0, countKind: 'project', children: [], canClone: false, canDelete: false });
        expect(browser.location.searchParams.get('collection')).toBe('content-types-queries-transformations');
        expect(service.snapshot().navigation.trees.projects.expanded).toContain('collection=content-types-and-schemas');
        expect(loadExample).not.toHaveBeenCalled();
    });

    it('retains explicit collection context through source, configuration and output routes and refuses false memberships', async () => {
        const { service, browser } = await fixture('https://studio.test/?collection=content-types-data&sample=demo&file=article', false, catalog);
        expect(service.snapshot().navigation).toMatchObject({ content: 'sample-file', selection: { collection: 'content-types-data', sample: 'demo', file: 'article' } });
        expect(service.snapshot().navigation.trees.projects.selected).toBe('collection=content-types-data&sample=demo');
        expect(service.snapshot().navigation.trees.projects.expanded).toContain('collection=content-types-and-schemas');
        expect(service.snapshot().navigation.trees.projects.expanded).toContain('collection=content-types-data');
        const sourceFolder = service.snapshot().navigation.files.children.find(item => item.name === 'content');
        expect(sourceFolder.children[0].route.collection).toBe('content-types-data');
        const configuration = service.snapshot().navigation.files.children.find(item => item.name === 'Configurations').children[0];
        await service.openEntity(configuration.key);
        expect(browser.location.searchParams.get('collection')).toBe('content-types-data');
        expect(browser.location.searchParams.get('configuration')).toBe('build');
        await service.buildProjectPreview();
        const generated = service.snapshot().navigation.output.children[0];
        await service.openEntity(generated.key);
        expect(service.snapshot().navigation.selection).toMatchObject({ collection: 'content-types-data', sample: 'demo', selection: 'output', 'output-folder': 'generated/' });
        const accepted = browser.location.href;
        for (const route of ['collection=content-types-and-schemas&sample=demo', 'collection=content-types-schema-authoring&sample=demo',
            'collection=unknown&sample=demo', 'collection=content-types-data&project=demo', 'collection=content-types-data&view=projects',
            'collection=content-types-data&project=', 'collection=content-types-data&view=']) {
            await service.openEntity(route);
            expect(service.snapshot().navigation.error.message).toMatch(/not in this samples folder|conflicting roots/u);
            expect(browser.location.href).toBe(accepted);
            expect(service.snapshot().navigation.selection.collection).toBe('content-types-data');
        }
    });

    it('keeps legacy URLs while revealing the first membership, reuses one project load and retains collapsed selection', async () => {
        const { service, browser, workbench, loadExample } = await fixture('https://studio.test/?sample=demo&file=article', false, catalog);
        expect(browser.location.searchParams.has('collection')).toBe(false);
        expect(service.snapshot().navigation.selection.key).toBe('sample=demo&file=article');
        expect(service.snapshot().navigation.trees.projects.selected).toBe('collection=conference&sample=demo');
        await service.openEntity('collection=content-types-data&sample=demo');
        await service.openEntity('collection=content-types-documents-ui&sample=demo&file=article');
        expect(loadExample).toHaveBeenCalledOnce();
        service.setNavigationExpanded('projects', []);
        service.setNavigationSection('projects', false);
        workbench.updateDraft('unrelated saved-project draft');
        expect(service.snapshot().navigation.trees.projects).toEqual({ selected: 'collection=content-types-documents-ui&sample=demo', expanded: '' });
        expect(service.snapshot().navigation.sections.projects).toBe(false);
        expect(service.snapshot().draft).toBe('unrelated saved-project draft');
        await service.openEntity('sample=demo');
        expect(browser.location.searchParams.has('collection')).toBe(false);
        expect(service.snapshot().navigation.selection.key).toBe('sample=demo');
        expect(service.snapshot().navigation.trees.projects.selected).toBe('collection=conference&sample=demo');
        expect(loadExample).toHaveBeenCalledOnce();
    });

    it('prepares selected CSV before publication and discards a superseded resource selection', async () => {
        const pending = [];
        const validator = { prepareResource: vi.fn(options => options.contentType === 'text/csv'
            ? new Promise(resolve => pending.push({ options, resolve })) : Promise.resolve()) };
        const { service, browser } = await fixture('https://studio.test/?sample=demo', false, { tabularSource: true, validator });
        const first = service.openEntity('sample=demo&file=article');
        await vi.waitFor(() => expect(pending).toHaveLength(1));
        expect(service.snapshot().navigation.selection.file).toBeUndefined();
        expect(service.snapshot().navigation.sourcePreview).toBeUndefined();
        expect(browser.location.searchParams.get('file')).toBeNull();
        expect(pending[0].options).toMatchObject({ uri: 'studio://demo/content/article.csv', contentType: 'text/csv',
            schema: 'https://cem.dev/ns/data/csv/1', projectId: 'demo', projectRevision: 1, resourceRevision: 1 });
        expect(new TextDecoder().decode(pending[0].options.bytes)).toBe('id,name\n1,Ada\n');

        await service.openEntity('sample=demo&file=other');
        expect(pending[0].options.signal.aborted).toBe(true);
        pending[0].resolve(); await first;
        expect(service.snapshot().navigation.selection.file).toBe('other');
        expect(browser.location.searchParams.get('file')).toBe('other');

        const accepted = service.openEntity('sample=demo&file=article');
        await vi.waitFor(() => expect(pending).toHaveLength(2));
        expect(service.snapshot().navigation.selection.file).toBe('other');
        expect(service.snapshot().navigation.contentType).toBe('text/markdown');
        pending[1].resolve(); await accepted;
        expect(service.snapshot().navigation).toMatchObject({ selection: { file: 'article' }, content: 'sample-file',
            contentType: 'text/csv', sourcePreview: { contentType: 'text/csv', text: 'id,name\n1,Ada\n' } });
        expect(browser.location.searchParams.get('file')).toBe('article');
    });

    for (const sourceBytes of [new Uint8Array([67, 69, 77, 0, 255]), new TextEncoder().encode('CEMPROJ\u0000\u0000\u0001\u0002')]) it(`keeps catalog binary resources out of text preparation and downloads their exact owned bytes (${sourceBytes.length} bytes)`, async () => {
        const prepareResource = vi.fn(async () => undefined);
        const { service } = await fixture('https://studio.test/?sample=demo&file=article', false, { binarySource: true, sourceBytes, validator: { prepareResource } });
        expect(service.snapshot().navigation).toMatchObject({ content: 'sample-file', sampleTextAvailable: false, sampleText: '' });
        expect(service.snapshot().navigation.sourcePreview).toBeUndefined(); expect(prepareResource).not.toHaveBeenCalled();
        const root = document.createElement('main'); document.body.append(root);
        const downloads = [], view = await mountCemStudioFeatureTourWorkbench({ root, workbench: service, download: async value => { downloads.push(value); } });
        onTestFinished(() => { view.dispose(); root.remove(); }); await view.whenSettled();
        const picker = root.querySelector('cem-studio-project-picker');
        expect(picker.querySelector('cem-source-editor')).toBeNull();
        const button = [...picker.querySelectorAll('button')].find(button => button.textContent.trim() === 'Download resource');
        expect(button).toBeDefined(); button.click(); await view.whenSettled();
        expect(downloads).toHaveLength(1);
        expect(downloads[0]).toMatchObject({ filename: 'projection.cem-bin', contentType: 'application/vnd.cem.ast+cem-bin' });
        expect(downloads[0].bytes).toEqual(sourceBytes);
    });

    for (const missing of [false, true]) it(`guards catalog text availability for ${missing ? 'unavailable bytes' : 'invalid UTF-8 beyond the preview prefix'}`, async () => {
        const bytes = new Uint8Array(300_000).fill(65); bytes[bytes.length - 1] = 255;
        const prepareResource = vi.fn(async () => undefined);
        const { service } = await fixture('https://studio.test/?sample=demo&file=article', false, {
            plaintextSource: true, sourceBytes: bytes, missingSource: missing, validator: { prepareResource },
        });
        expect(service.snapshot().navigation).toMatchObject({ content: 'sample-file', sampleTextAvailable: false, sampleAvailable: !missing, sampleText: '' });
        expect(service.snapshot().navigation.error).toBeUndefined(); expect(service.snapshot().navigation.sourcePreview).toBeUndefined();
        expect(prepareResource).not.toHaveBeenCalled();
        const save = vi.fn(async () => undefined);
        if (missing) { await expect(service.downloadSource(save)).rejects.toThrow('No owned sample resource bytes'); expect(save).not.toHaveBeenCalled(); }
        else { await service.downloadSource(save); expect(save.mock.calls[0][0].bytes).toEqual(bytes); }
    });

    const invalidCommandTail = new Uint8Array(300_000).fill(65); invalidCommandTail[invalidCommandTail.length - 1] = 255;
    for (const [name, options, reason] of [
        ['missing bytes', { missingCommand: true }, /unavailable/],
        ['invalid UTF-8', { commandBytes: new Uint8Array([255, 0]) }, /UTF-8/],
        ['invalid UTF-8 beyond the preview prefix', { commandBytes: invalidCommandTail }, /UTF-8/],
        ['binary identity with valid UTF-8 bytes', { commandContentType: 'application/vnd.cem.ast+cem-bin', commandBytes: new TextEncoder().encode('CEMPROJ') }, /Binary or unknown/],
        ['unknown identity', { commandContentType: 'application/octet-stream' }, /Binary or unknown/],
        ['oversized active markup', { commandContentType: 'text/html', commandBytes: new Uint8Array(262_145).fill(65) }, /inline preview limit/],
    ]) it(`ConfigurationTextAdmission refuses fabricated text for ${name}`, async () => {
        const prepareResource = vi.fn(async () => undefined);
        const { service, browser, workbench } = await fixture('https://studio.test/?collection=nested&sample=demo&configuration=build', false, { ...options, validator: { prepareResource } });
        const current = service.snapshot().navigation;
        expect(current).toMatchObject({ content: 'sample-file', selection: { collection: 'nested', sample: 'demo', configuration: 'build' }, sampleTextAvailable: false, sampleAvailable: !options.missingCommand, sampleText: '' });
        expect(current.sampleReason).toMatch(reason); expect(current.error).toBeUndefined();
        expect(current.sourcePreview).toBeUndefined(); expect(prepareResource).not.toHaveBeenCalled();
        expect(browser.location.searchParams.get('configuration')).toBe('build');
        workbench.updateDraft('retained saved-project draft');
        const save = vi.fn(async () => undefined);
        if (options.missingCommand) { await expect(service.downloadSource(save)).rejects.toThrow('No owned sample resource bytes'); expect(save).not.toHaveBeenCalled(); }
        else {
            await service.downloadSource(save);
            expect(save.mock.calls[0][0]).toMatchObject({ filename: 'build.cem', contentType: options.commandContentType ?? 'application/cem' });
            expect(save.mock.calls[0][0].bytes).toEqual(options.commandBytes ?? new TextEncoder().encode('{}'));
        }
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench: service });
        onTestFinished(() => { view.dispose(); root.remove(); }); await view.whenSettled();
        const picker = root.querySelector('cem-studio-project-picker');
        expect(picker.querySelector('cem-source-editor')).toBeNull();
        const button = [...picker.querySelectorAll('button')].find(button => button.textContent.trim() === 'Download resource');
        expect(button).toBeDefined(); expect(button.disabled).toBe(Boolean(options.missingCommand));
        expect(service.snapshot().draft).toBe('retained saved-project draft');
        expect(prepareResource).not.toHaveBeenCalled();
    });

    it.each(['', '{command | Café Noord 🌍}\r\n'])('ConfigurationTextAdmission retains exact admitted text %j', async text => {
        const prepareResource = vi.fn(async () => undefined), bytes = new TextEncoder().encode(text);
        const { service, browser } = await fixture('https://studio.test/?sample=demo&configuration=build', false, { commandBytes: bytes, validator: { prepareResource } });
        expect(service.snapshot().navigation).toMatchObject({ sampleTextAvailable: true, sampleAvailable: true, sampleText: text });
        expect(service.snapshot().navigation.sourcePreview).toBeUndefined(); expect(prepareResource).not.toHaveBeenCalled();
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench: service });
        onTestFinished(() => { view.dispose(); root.remove(); }); await view.whenSettled();
        const editor = root.querySelector('cem-studio-project-picker cem-source-editor[readonly] [role=textbox]');
        expect(editor).not.toBeNull(); expect(editor.value).toBe(text);
        await service.openEntity('sample=demo&file=command');
        expect(service.snapshot().navigation.sampleText).toBe(text);
        await service.openEntity('sample=demo&configuration=build');
        expect(service.snapshot().navigation.sampleText).toBe(text);
        expect(browser.location.searchParams.get('configuration')).toBe('build');
        const save = vi.fn(async () => undefined); await service.downloadSource(save);
        expect(save.mock.calls[0][0].bytes).toEqual(bytes);
    });

    it('ConfigurationTextAdmission refuses a missing configuration resource without losing selection or drafts', async () => {
        const { service, browser, workbench } = await fixture('https://studio.test/?sample=demo&file=article', false, { missingCommandResource: true });
        const selected = browser.location.href; workbench.updateDraft('retained draft');
        await service.openEntity('sample=demo&configuration=build');
        expect(service.snapshot().navigation.error.message).toMatch(/resource.*unavailable/);
        expect(browser.location.href).toBe(selected);
        expect(service.snapshot().navigation.selection.file).toBe('article');
        expect(service.snapshot().navigation.sampleText).toBe('# Article'); expect(service.snapshot().draft).toBe('retained draft');
    });

    it('keeps section openness and per-tree selection/expansion independent across publications and collapse', async () => {
        const { service, workbench } = await fixture();
        expect(service.snapshot().navigation).toMatchObject({ content: 'file', sections: { projects: true, structure: true, output: false } });
        const selected = service.snapshot().navigation.trees.structure.selected;
        expect(service.snapshot().navigation.trees.projects.selected).toBe('project=demo');
        expect(service.snapshot().navigation.trees.structure.expanded).toContain('folder=content%2F');
        service.setNavigationExpanded('structure', []);
        service.setNavigationSection('structure', false);
        workbench.updateDraft('# Draft');
        expect(service.snapshot().navigation.trees.structure).toEqual({ selected, expanded: '' });
        expect(service.snapshot().navigation.sections.structure).toBe(false);
        expect(service.snapshot().navigation.sourcePreview).toMatchObject({ text: '# Draft', draft: true, sourceUri: 'studio://demo/content/article.md' });
        await service.openEntity('project=demo');
        expect(service.snapshot().navigation.content).toBe('project');
        expect(service.snapshot().navigation.sections.structure).toBe(false);
        service.toggleNavigationExpanded('structure', 'project=demo');
        expect(service.snapshot().navigation.trees.structure.expanded).toBe('project=demo');
        service.toggleNavigationSection('output');
        expect(service.snapshot().navigation.sections.output).toBe(true);
        const sample = (await fixture('https://studio.test/?sample=demo&file=article')).service;
        expect(sample.snapshot().navigation.trees.projects.selected).toBe('collection=nested&sample=demo');
        expect(sample.snapshot().navigation.trees.projects.expanded).toContain('collection=nested');
        sample.setNavigationExpanded('projects', []);
        sample.setNavigationSection('projects', false);
        sample.setNavigationSection('output', true);
        expect(sample.snapshot().navigation.trees.projects.expanded).toBe('');
        const readonly = (await fixture('https://studio.test/?project=demo&file=article', true)).service;
        expect(readonly.snapshot().navigation.sourcePreview).toBeUndefined();
    });

    it('projects successful native output manifests and keeps active output routing independent from source selection', async () => {
        const { service, browser } = await fixture();
        expect(service.snapshot().navigation.output).toBeUndefined();
        await service.buildProjectPreview();
        expect(service.snapshot().projectPreview.selectedArtifact.path).toBe('generated/posts/article.html');
        let copied, downloaded;
        await service.copyPreview(async text => { copied = text; });
        await service.downloadPreview(async file => { downloaded = file; });
        expect(copied).toBe('<p>generated/posts/article.html</p>');
        expect(new TextDecoder().decode(downloaded.bytes)).toBe(copied);
        const sourceSelection = service.snapshot().navigation.trees.structure.selected;
        const root = service.snapshot().navigation.output;
        const generated = root.children[0], posts = generated.children.find(item => item.name === 'posts');
        expect(posts.children[0]).toMatchObject({ kind: 'output-file', path: 'generated/posts/article.html' });
        await service.openEntity(posts.key);
        expect(service.snapshot().navigation).toMatchObject({ content: 'output-folder', count: 1, canCreate: false, canClone: false, canDelete: false });
        expect(service.snapshot().projectPreview.preview).toBeUndefined();
        expect(service.snapshot().navigation.trees.structure.selected).toBe(sourceSelection);
        expect(browser.location.searchParams.get('selection')).toBe('output');
        expect(browser.location.searchParams.get('artifact')).toBeNull();
        await service.openEntity(posts.children[0].key);
        expect(service.snapshot().navigation.content).toBe('output-file');
        expect(service.snapshot().projectPreview.selectedOutput.text).toContain('generated/posts/article.html');
        expect(browser.location.searchParams.get('artifact')).toBe('generated/posts/article.html');
        await service.openEntity('project=demo&file=other');
        expect(service.snapshot().projectPreview.selectedArtifact).toBeUndefined();
        expect(service.snapshot().projectPreview.missingReason).toContain('No generated output');
        expect(browser.location.searchParams.get('artifact')).toBeNull();
    });

    it('preserves explicit legacy source-plus-artifact URLs and refuses unnormalized output directories', async () => {
        const { service, browser } = await fixture('https://studio.test/?project=demo&file=other&build=build&artifact=generated/index.html');
        await service.buildProjectPreview();
        expect(service.snapshot().navigation.content).toBe('file');
        expect(service.snapshot().projectPreview.selectedArtifact.path).toBe('generated/index.html');
        const accepted = browser.location.href;
        await service.openEntity('project=demo&selection=output&output-folder=../private/');
        expect(service.snapshot().navigation.error.message).toContain('normalized');
        expect(browser.location.href).toBe(accepted);
        expect(service.snapshot().navigation.selection.file).toBe('other');
        await service.openEntity('project=demo&selection=output&artifact=generated/index.html&output-folder=generated/');
        expect(service.snapshot().navigation.error.message).toContain('conflicting');
    });

    it('keeps output preview navigation canonical without switching source or project centers', async () => {
        const { service, browser } = await fixture();
        await service.buildProjectPreview();
        const generated = service.snapshot().navigation.output.children[0];
        const index = generated.children.find(item => item.path === 'generated/index.html');
        const article = generated.children.find(item => item.name === 'posts').children[0];
        await service.openEntity(index.key);
        await service.selectPreviewArtifact(article.path);
        expect(service.snapshot().navigation).toMatchObject({ content: 'output-file', path: article.path,
            selection: { selection: 'output', artifact: article.path }, trees: { output: { selected: article.key } } });
        expect(service.snapshot().navigation.title).toContain(article.path);
        expect(service.snapshot().projectPreview.selectedArtifact.path).toBe(article.path);
        expect(browser.location.searchParams.get('artifact')).toBe(article.path);
        await service.openEntity('project=demo&file=article');
        service.selectPreviewArtifact(index.path);
        expect(service.snapshot().navigation).toMatchObject({ content: 'file', path: 'content/article.md', selection: { file: 'article' } });
        expect(service.snapshot().navigation.selection.selection).toBeUndefined();
        expect(service.snapshot().projectPreview.selectedArtifact.path).toBe(index.path);
        expect(browser.location.searchParams.get('file')).toBe('article');
        expect(browser.location.searchParams.get('selection')).toBeNull();
        await service.openEntity('project=demo');
        service.selectPreviewArtifact(article.path);
        expect(service.snapshot().navigation).toMatchObject({ content: 'project', selection: { project: 'demo' } });
        expect(service.snapshot().navigation.selection.selection).toBeUndefined();
        expect(service.snapshot().navigation.selection.file).toBeUndefined();
        expect(service.snapshot().projectPreview.selectedArtifact.path).toBe(article.path);
        expect(browser.location.searchParams.get('file')).toBeNull();
        expect(browser.location.searchParams.get('selection')).toBeNull();
    });

    it('requires an explicit choice for ambiguous source outputs while preserving the configured whole-project entry', async () => {
        const { service, browser, workbench } = await fixture('https://studio.test/?project=demo&file=article', false, { ambiguousSource: true });
        await service.buildProjectPreview();
        expect(service.snapshot().projectPreview.relatedArtifacts.map(item => item.path)).toEqual(['generated/index.html', 'generated/posts/article.html']);
        expect(service.snapshot().projectPreview.selectedArtifact).toBeUndefined();
        expect(service.snapshot().projectPreview.preview).toBeUndefined();
        expect(service.snapshot().projectPreview.missingReason).toContain('Multiple generated outputs');
        expect(browser.location.searchParams.get('artifact')).toBeNull();
        await service.openEntity('project=demo');
        expect(service.snapshot().projectPreview.selectedArtifact.path).toBe('generated/index.html');
        await service.openEntity('project=demo&file=article');
        expect(service.snapshot().projectPreview.selectedArtifact).toBeUndefined();
        service.selectPreviewArtifact('generated/posts/article.html');
        expect(service.snapshot().projectPreview.selectedArtifact.path).toBe('generated/posts/article.html');
        expect(browser.location.searchParams.get('artifact')).toBe('generated/posts/article.html');
        workbench.updateDraft('# Edited article');
        expect(service.snapshot().projectPreview.selectedArtifact.path).toBe('generated/posts/article.html');
        await service.openEntity('project=demo&file=article');
        expect(service.snapshot().projectPreview.selectedArtifact.path).toBe('generated/posts/article.html');
    });

    it('preserves an already-admitted live CEM preview for its exact selected artifact', async () => {
        const source = '{p | Live Login}';
        const bytes = new TextEncoder().encode(source);
        const artifact = { id: 'live', path: 'view.cem', uri: 'studio://demo/view.cem', contentType: 'application/cem', provenance: [] };
        const live = { kind: 'live-cem', label: 'Live Login', source, sourceUri: artifact.uri, resourceSet: 'fixture-live' };
        const output = { uri: artifact.uri, bytes, contentType: artifact.contentType, text: source };
        const projection = { exitCode: 0, buildRun: { id: 'live-run', inputs: [], artifacts: [artifact] }, outputs: [output], output, preview: live };
        const project = { id: 'demo', resources: [{ id: 'command', schema: 'https://cem.dev/ns/cli/command/1' }], entries: [{ id: 'live', kind: 'transformation-graph', runConfigResourceId: 'command' }] };
        const service = createCemStudioProjectPreview({ repository: { query: async () => ({ value: [] }) }, openBuildWorkbench: async () => ({
            snapshot: () => ({ projection }), runPersistedOperation: async () => undefined, selectArtifact: () => undefined, dispose: () => undefined,
        }) }); services.push(service);
        service.setContext({ key: 'sample=live', project, bundle: { project, contents: {} } });
        await service.build();
        expect(service.snapshot().preview).toBe(live);
        expect(service.snapshot().preview.sourceUri).toBe(artifact.uri);
    });

    it('retains last-success output on failure/cancellation and does not reopen manually collapsed output', async () => {
        const { service, fail, delay } = await fixture();
        await service.buildProjectPreview();
        const projection = service.snapshot().projectPreview.projection;
        service.setNavigationSection('output', false);
        service.setNavigationExpanded('output', []);
        fail(); await service.buildProjectPreview();
        expect(service.snapshot().projectPreview).toMatchObject({ attempt: { status: 'failed' }, projection });
        expect(service.snapshot().navigation.output.children).toHaveLength(1);
        expect(service.snapshot().navigation.sections.output).toBe(false);
        expect(service.snapshot().navigation.trees.output.expanded).toBe('');
        let finish; delay(new Promise(resolve => { finish = resolve; }));
        const building = service.buildProjectPreview();
        service.cancelProjectPreview(); await building; finish();
        expect(service.snapshot().projectPreview).toMatchObject({ attempt: { status: 'cancelled' }, projection });
        expect(service.snapshot().navigation.sections.output).toBe(false);
    });
});


describe('Studio native per-file Markdown preview', () => {
    const descriptor = (resourceId, text) => ({ resourceId, sourceUri: `studio://demo/content/${resourceId}`, contentType: 'text/markdown; variant=CommonMark',
        schema: 'https://cem.dev/ns/data/markdown/1', text, projectId: 'demo', projectRootUri: 'studio://demo/', projectRevision: 1, revision: 1, sha256: resourceId, draft: false });
    const outcome = text => ({ output: { available: true, contentType: 'text/html', bytes: new TextEncoder().encode(`<h1>${text}</h1>`) } });
    it('runs only bounded native Markdown conversion, coalesces edits, cancels obsolete reads and rejects late results', async () => {
        const pending = [];
        const validator = { runResourceCommand: vi.fn(options => new Promise(resolve => pending.push({ options, resolve }))) };
        const service = createCemStudioSourcePreview({ validator, delay: 0 }); services.push(service);
        service.setSource(descriptor('first', '# First'));
        await vi.waitFor(() => expect(pending).toHaveLength(1));
        expect(pending[0].options.argv).toEqual(['convert', 'studio://demo/content/first', '--content-type', 'text/markdown; variant=CommonMark', '--schema', 'https://cem.dev/ns/data/markdown/1',
            '--to-content-type', 'text/html', '--to-schema', 'https://cem.dev/ns/data/html/1', '--output-color-type', 'none']);
        pending[0].resolve(outcome('First'));
        await vi.waitFor(() => expect(service.snapshot().status).toBe('ready'));
        service.setSource({ ...descriptor('first', '# Edited'), draft: true });
        expect(service.snapshot()).toMatchObject({ status: 'pending', stale: true, preview: { source: '<h1>First</h1>' } });
        await vi.waitFor(() => expect(pending).toHaveLength(2));
        service.setSource(descriptor('second', '# Second'));
        expect(pending[1].options.signal.aborted).toBe(true);
        expect(service.snapshot().preview).toBeUndefined();
        await vi.waitFor(() => expect(pending).toHaveLength(3));
        pending[2].resolve(outcome('Second')); await vi.waitFor(() => expect(service.snapshot().status).toBe('ready'));
        pending[1].resolve(outcome('Late first'));
        await Promise.resolve();
        expect(service.snapshot().preview.source).toBe('<h1>Second</h1>');
        service.setSource({ ...descriptor('other', '{}'), contentType: 'application/json' });
        expect(service.snapshot()).toEqual({ status: 'idle', stale: false });
        expect(validator.runResourceCommand).toHaveBeenCalledTimes(3);
    });

    it('surfaces native refusal without fabricating HTML and retains only same-context success as stale', async () => {
        let reject = false;
        const validator = { runResourceCommand: vi.fn(async () => { if (reject) throw new Error('native conversion refused'); return outcome('Saved'); }) };
        const service = createCemStudioSourcePreview({ validator, delay: 0 }); services.push(service);
        service.setSource(descriptor('article', '# Saved')); await vi.waitFor(() => expect(service.snapshot().status).toBe('ready'));
        reject = true;
        service.setSource({ ...descriptor('article', '# Invalid'), draft: true }); await vi.waitFor(() => expect(service.snapshot().status).toBe('error'));
        expect(service.snapshot()).toMatchObject({ stale: true, error: { message: 'native conversion refused' }, preview: { source: '<h1>Saved</h1>' } });
        service.setSource(descriptor('other', '# Invalid')); await vi.waitFor(() => expect(service.snapshot().status).toBe('error'));
        expect(service.snapshot().preview).toBeUndefined();
    });
    it('uses the shared native command service for a declared Markdown to HTML source projection', async () => {
        const { createCemStudioBrowserValidator } = await import('./feature-tour.js');
        const validator = await createCemStudioBrowserValidator();
        const service = createCemStudioSourcePreview({ validator, delay: 0 }); services.push(service);
        try {
            service.setSource(descriptor('article', '# Native preview\n\nRead **this**.'));
            await vi.waitFor(() => expect(service.snapshot().status).not.toBe('pending'), { timeout: 30_000 });
            expect(service.snapshot().error).toBeUndefined();
            expect(service.snapshot().preview).toMatchObject({ kind: 'sandboxed-html' });
            const html = new DOMParser().parseFromString(service.snapshot().preview.source, 'text/html');
            expect(html.querySelector('h1').textContent).toBe('Native preview');
            expect(html.querySelector('strong').textContent).toBe('this');
        } finally { service.dispose(); await validator.close(); }
    }, 40_000);

    it('renders ordinary README configuration tables through generic native Markdown conversion while preserving explicit CommonMark', async () => {
        const response = await fetch(new URL('../../../examples/projects/cemql-orders/README.md', import.meta.url));
        expect(response.ok).toBe(true);
        const readme = await response.text();
        const { createCemStudioBrowserValidator } = await import('./feature-tour.js');
        const validator = await createCemStudioBrowserValidator();
        const service = createCemStudioSourcePreview({ validator, delay: 0 }); services.push(service);
        try {
            const source = { ...descriptor('readme', readme), contentType: 'text/markdown; charset=utf-8' };
            service.setSource(source);
            await vi.waitFor(() => expect(service.snapshot().status).not.toBe('pending'), { timeout: 30_000 });
            expect(service.snapshot().error).toBeUndefined();
            const html = new DOMParser().parseFromString(service.snapshot().preview.source, 'text/html');
            const table = [...html.querySelectorAll('table')].find(table => table.querySelector('th')?.textContent === 'Configuration');
            expect(table).toBeDefined();
            expect([...table.querySelectorAll('thead th')].map(cell => cell.textContent)).toEqual(['Configuration', 'Typical use', 'Expected result']);
            expect(table.querySelectorAll('tbody tr')).toHaveLength(14);
            expect(table.querySelectorAll('tbody td code')[0].textContent).toBe('inspect-data');
            expect(table.textContent).toContain('report-ready');
            service.setSource({ ...source, contentType: 'text/markdown; charset=utf-8; variant=CommonMark' });
            await vi.waitFor(() => expect(service.snapshot().status).not.toBe('pending'), { timeout: 30_000 });
            expect(service.snapshot().error).toBeUndefined();
            const strict = new DOMParser().parseFromString(service.snapshot().preview.source, 'text/html');
            expect(strict.querySelector('table')).toBeNull();
            expect(strict.body.textContent).toContain('| Configuration | Typical use | Expected result |');
        } finally { service.dispose(); await validator.close(); }
    }, 70_000);

});


async function withCompactWorkspace(width, run) {
    const { page: browserPage, userEvent } = await import('vitest/browser');
    const originalViewport = { width: innerWidth, height: innerHeight };
    let root, theme, view, shell, cleanupPromise;
    const cleanup = () => cleanupPromise ??= (async () => {
        try { view?.dispose(); shell?.dispose(); }
        finally {
            root?.remove(); theme?.remove();
            await browserPage.viewport(originalViewport.width, originalViewport.height);
        }
    })();
    onTestFinished(cleanup);
    try {
        await browserPage.viewport(width, 400);
        theme = document.createElement('link');
        theme.rel = 'stylesheet'; theme.href = new URL('../../cem-theme/dist/lib/css/cem-combined.css', import.meta.url).href;
        const themeReady = new Promise((resolve, reject) => { theme.onload = resolve; theme.onerror = reject; });
        document.head.append(theme); await themeReady;
        // Geometry uses plain source; native Markdown and coloring have dedicated coverage.
        const { service, setState } = await fixture(undefined, false, { plaintextSource: true });
        root = document.createElement('main');
        root.style.cssText = 'inline-size:100%; block-size:100dvb;';
        document.body.append(root);
        const saved = new Map();
        shell = await mountCemStudioApplicationShell({ root, storage: { getItem: key => saved.get(key) ?? null, setItem: (key, value) => saved.set(key, value) } });
        view = await mountCemStudioFeatureTourWorkbench({ root: shell.root, workbench: service });
        await view.whenSettled();
        const center = root.querySelector('cem-studio-workbench-center');
        const file = center.querySelector('[part~=file]');
        const controls = file.querySelector('[part~=controls]');
        const inspector = file.querySelector('cem-expansion[label="Commands and results"]');
        const header = inspector.querySelector('[part~=header]');
        await run({ root, service, setState, view, userEvent, center, file, controls, inspector, header });
    } finally { await cleanup(); }
}

describe('Studio compact declarative layout', () => {
    it.each([900, 1280])('keeps source and command pointer targets separate in a short desktop workspace: %s', async width => {
        await withCompactWorkspace(width, async ({ root, service, setState, view, userEvent, center, file, controls, inspector, header }) => {
            const hittable = element => {
                const box = element.getBoundingClientRect();
                return element.contains(document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2));
            };
            await userEvent.click(header); await view.whenSettled();
            expect(inspector.hasAttribute('expanded')).toBe(true);
            const commandTab = [...inspector.querySelectorAll('[role=tab]')].find(tab => tab.textContent.trim() === 'Studio command');
            expect(commandTab).toBeTruthy();
            await userEvent.click(commandTab); await view.whenSettled();
            expect(commandTab.getAttribute('aria-selected')).toBe('true');
            expect(hittable(commandTab)).toBe(true);
            expect(hittable(header)).toBe(true);
            const rootBounds = root.getBoundingClientRect();
            const geometry = { viewport: [innerWidth, innerHeight], scroll: document.scrollingElement.scrollTop,
                chain: [root, root.querySelector('cem-studio-workbench-layout'), root.querySelector('cem-workspace'), root.querySelector('cem-workspace > [part~=root]'),
                    center.parentElement, center, center.querySelector('[part~=content]'), file, controls, inspector].map(element => {
                    const style = getComputedStyle(element), box = element.getBoundingClientRect();
                    return { tag: element.localName, part: element.getAttribute('part'), top: box.top, bottom: box.bottom, height: box.height,
                        display: style.display, block: style.blockSize, min: style.minBlockSize, max: style.maxBlockSize, flex: style.flex, overflow: style.overflow, rows: style.gridTemplateRows };
                }) };
            expect(file.getBoundingClientRect().bottom, JSON.stringify(geometry)).toBeLessThanOrEqual(Math.min(rootBounds.bottom, innerHeight) + 1);
            expect(inspector.getBoundingClientRect().bottom).toBeLessThanOrEqual(Math.min(rootBounds.bottom, innerHeight) + 1);
            expect(controls.getBoundingClientRect().bottom).toBeLessThanOrEqual(inspector.getBoundingClientRect().top + 1);
            expect(inspector.getBoundingClientRect().height).toBeLessThanOrEqual(file.getBoundingClientRect().height / 2 + 1);

            const editor = root.querySelector('cem-source-editor[data-cem-studio-editor]');
            const control = editor.querySelector('[role=textbox]');
            const draft = '# Retained short workspace draft';
            await userEvent.click(control); await userEvent.fill(control, draft);
            control.setSelectionRange(2, 7, 'backward'); await view.whenSettled();
            const box = control.getBoundingClientRect(), bounds = controls.getBoundingClientRect();
            const visibleHeight = Math.min(box.bottom, bounds.bottom, innerHeight) - Math.max(box.top, bounds.top, 0);
            const twoLines = 2 * parseFloat(getComputedStyle(control).lineHeight);
            expect(bounds.height).toBeGreaterThanOrEqual(twoLines - 1);
            expect(visibleHeight).toBeGreaterThanOrEqual(twoLines - 1);
            setState({ projectRevision: 2 }); await view.whenSettled();
            expect(root.querySelector('cem-source-editor[data-cem-studio-editor]')).toBe(editor);
            expect(document.activeElement).toBe(control);
            expect(control.value).toBe(draft);
            expect(service.snapshot().draft).toBe(draft);
            expect([control.selectionStart, control.selectionEnd, control.selectionDirection]).toEqual([2, 7, 'backward']);
            expect(inspector.hasAttribute('expanded')).toBe(true);
            await userEvent.click(header); await view.whenSettled();
            expect(inspector.hasAttribute('expanded')).toBe(false);
            expect(document.activeElement).toBe(header);
            expect(control.value).toBe(draft);
            expect([control.selectionStart, control.selectionEnd, control.selectionDirection]).toEqual([2, 7, 'backward']);
        });
    }, 30_000);

    it.each([900, 1280])('keeps long source scrolling and selection inside a short desktop editor: %s', async width => {
        await withCompactWorkspace(width, async ({ root, service, view, userEvent, file, controls, inspector, header }) => {
            await userEvent.click(header); await view.whenSettled();
            expect(inspector.hasAttribute('expanded')).toBe(true);
            const rootBounds = root.getBoundingClientRect();
            const editor = root.querySelector('cem-source-editor[data-cem-studio-editor]');
            const control = editor.querySelector('[role=textbox]');
            const draft = '# Retained short workspace draft';
            await userEvent.fill(control, draft); await view.whenSettled();
            control.setSelectionRange(2, 7, 'backward'); await view.whenSettled();
            const twoLines = 2 * parseFloat(getComputedStyle(control).lineHeight);
            const shortGeometry = { editorHeight: control.clientHeight, controlsHeight: controls.clientHeight, controlsContent: controls.scrollHeight };
            const longDraft = Array.from({ length: 200 }, (_, index) => `# Retained workspace line ${index + 1}`).join('\n');
            await userEvent.fill(control, longDraft); await view.whenSettled();
            control.setSelectionRange(2, 7, 'backward'); await view.whenSettled();
            expect(control.clientHeight).toBeGreaterThanOrEqual(twoLines - 1);
            expect(Math.abs(control.clientHeight - shortGeometry.editorHeight)).toBeLessThanOrEqual(1);
            expect(control.scrollHeight).toBeGreaterThan(control.clientHeight + twoLines);
            expect(Math.abs(controls.clientHeight - shortGeometry.controlsHeight)).toBeLessThanOrEqual(1);
            expect(controls.scrollHeight).toBeLessThanOrEqual(shortGeometry.controlsContent + 1);
            expect(root.querySelector('cem-source-editor[data-cem-studio-editor]')).toBe(editor);
            expect(document.activeElement).toBe(control);
            expect(control.value).toBe(longDraft);
            expect(service.snapshot().draft).toBe(longDraft);
            expect([control.selectionStart, control.selectionEnd, control.selectionDirection]).toEqual([2, 7, 'backward']);
            const controlsScroll = controls.scrollTop;
            control.scrollTop = control.scrollHeight;
            await view.whenSettled();
            expect(control.scrollTop).toBeGreaterThan(0);
            expect(Math.abs(controls.scrollTop - controlsScroll)).toBeLessThanOrEqual(1);
            expect(document.activeElement).toBe(control);
            expect([control.selectionStart, control.selectionEnd, control.selectionDirection]).toEqual([2, 7, 'backward']);
            expect(file.getBoundingClientRect().bottom).toBeLessThanOrEqual(Math.min(rootBounds.bottom, innerHeight) + 1);
            expect(inspector.hasAttribute('expanded')).toBe(true);
            await userEvent.click(header); await view.whenSettled();
            expect(inspector.hasAttribute('expanded')).toBe(false);
            expect(document.activeElement).toBe(header);
            expect(control.value).toBe(longDraft);
            expect([control.selectionStart, control.selectionEnd, control.selectionDirection]).toEqual([2, 7, 'backward']);
        });
    }, 30_000);

    it('admits only the active source invocation and preserves ambiguous and explicit-missing output choices', async () => {
        const { service, setState } = await fixture('https://studio.test/?project=demo&file=article', false, { ambiguousSource: true });
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench: service });
        const preview = () => root.querySelector('cem-studio-workbench-preview[section=preview] cem-media-preview');
        try {
            setState({ projection: invocation() }); await view.whenSettled();
            expect(preview().getAttribute('value')).toBe('Native invocation result');
            setState({ projection: invocation({ projectId: 'old-project' }) }); await view.whenSettled();
            expect(preview()).toBeNull();
            setState({ projection: invocation({ resourceId: 'other', configuration: 'article' }) }); await view.whenSettled();
            expect(preview()).toBeNull();
            setState({ projection: invocation({ configuration: 'other-configuration' }) }); await view.whenSettled();
            expect(preview()).toBeNull();
            setState({ projection: invocation() }); await view.whenSettled();
            expect(preview().getAttribute('value')).toBe('Native invocation result');
            await service.buildProjectPreview(); await view.whenSettled();
            expect(service.snapshot().projectPreview.relatedArtifacts).toHaveLength(2);
            expect(preview()).toBeNull();
            expect(root.querySelector('cem-studio-workbench-preview[section=preview]').textContent).toContain('Multiple generated outputs');
            service.selectPreviewArtifact('generated/posts/article.html'); await view.whenSettled();
            expect(preview().getAttribute('mode')).toBe('artifact-tree');
            service.selectPreviewArtifact('generated/missing.html'); await view.whenSettled();
            expect(preview()).toBeNull();
            expect(root.querySelector('cem-studio-workbench-preview[section=preview]').textContent).toContain('requested artifact is unavailable');
        } finally { view.dispose(); root.remove(); }
    }, 30_000);

    it('retains the exact active configuration live preview across stale source and unrelated project builds', async () => {
        const { service, setState } = await fixture('https://studio.test/?project=demo&configuration=build', false, { alternateBuild: true });
        const result = invocation({ configuration: 'build', live: true });
        setState({ entryId: 'build', resourceId: 'article', projection: result });
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench: service });
        const right = () => root.querySelector('cem-studio-workbench-preview[section=preview]');
        try {
            const live = right().querySelector('cem-media-preview[mode=live]');
            expect(live).not.toBeNull();
            expect(right().querySelector('cem-data-view')).toBeNull();
            await vi.waitFor(() => expect(live.querySelector('iframe')).not.toBeNull());
            const frame = live.querySelector('iframe');
            setState({ draft: '# Edited source', dirty: true, projection: { ...result, stale: true } }); await view.whenSettled();
            expect(right().querySelector('cem-media-preview[mode=live]')).toBe(live);
            expect(live.querySelector('iframe')).toBe(frame);
            await vi.waitFor(() => expect(right().querySelector('cem-alert')?.getAttribute('label')).toContain('last successful saved build'));
            await vi.waitFor(() => expect(right().querySelector('cem-alert [role=status]')?.textContent).toContain('last successful saved build'));
            service.selectPreviewConfiguration('alternate');
            await service.buildProjectPreview(); await view.whenSettled();
            expect(service.snapshot().projectPreview.projection.buildRun.configuration.id).toBe('alternate');
            expect(right().querySelector('cem-media-preview[mode=live]')).toBe(live);
            expect(live.querySelector('iframe')).toBe(frame);
            setState({ entryId: 'other' }); await view.whenSettled();
            expect(right().querySelector('cem-media-preview')).toBeNull();
        } finally { view.dispose(); root.remove(); }
    }, 30_000);

    async function loadTreeTheme() {
        const theme = document.createElement('link');
        theme.rel = 'stylesheet'; theme.href = new URL('../../cem-theme/dist/lib/css/cem-combined.css', import.meta.url).href;
        const ready = new Promise((resolve, reject) => { theme.onload = resolve; theme.onerror = reject; });
        document.head.append(theme); onTestFinished(() => theme.remove()); await ready;
    }
    function expectCondensedTree(tree) {
        expect(tree.getAttribute('data-cem-density')).toBe('super-compact');
        const row = tree.querySelector('[role=treeitem]'), style = getComputedStyle(row);
        const line = Number.parseFloat(style.lineHeight);
        expect(style.getPropertyValue('--cem-coupling-zone-min').trim()).toBe('0px');
        expect(style.paddingBlockStart).toBe('0px');
        expect(Math.abs(Number.parseFloat(style.minBlockSize) - line)).toBeLessThan(0.1);
        expect(Math.abs(row.getBoundingClientRect().height - line)).toBeLessThan(0.1);
        expect(row.getBoundingClientRect().height).toBeLessThan(48);
    }

    it('keeps the source editor and selected descendant when tree and accordion sections collapse', async () => {
        await loadTreeTheme();
        const { service } = await fixture();
        const root = document.createElement('main'); root.style.cssText = 'inline-size:1280px; block-size:640px;'; document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench: service });
        try {
            await view.whenSettled();
            expect(root.querySelector('cem-accordion[fit=fill]').getAttribute('label')).toBe('Navigation');
            expect(root.querySelector('cem-accordion[fit=fill]').querySelectorAll('cem-expansion[layout=panel]')).toHaveLength(3);
            const tree = root.querySelector('cem-tree[label="Project files"]');
            expectCondensedTree(tree);
            expectCondensedTree(root.querySelector('cem-tree[label=Projects]'));
            expect(tree.hasAttribute('separate-toggle')).toBe(true);
            const center = root.querySelector('cem-studio-workbench-center > [part~=content]');
            expect(getComputedStyle(center).display).toBe('flex');
            expect(getComputedStyle(center.querySelector('[part~=file]')).flexGrow).toBe('1');
            expect(getComputedStyle(center.querySelector('[part~=controls]')).flexGrow).toBe('1');
            const editor = root.querySelector('cem-source-editor[data-cem-studio-editor]');
            const control = editor.querySelector('[role=textbox]');
            control.focus();
            const branch = [...tree.querySelectorAll('[role=treeitem]')].find(item => item.getAttribute('data-tree-value')?.includes('folder=content%2F'));
            expect(branch.getAttribute('aria-expanded')).toBe('true');
            branch.querySelector('[part~=marker]').click(); await view.whenSettled();
            expect(service.snapshot().navigation.selection.file).toBe('article');
            expect(service.snapshot().navigation.trees.structure.selected).toBe('project=demo&file=article');
            expect(branch.getAttribute('aria-expanded')).toBe('false');
            root.querySelector('cem-expansion[value=structure] [part~=header]').click(); await view.whenSettled();
            expect(service.snapshot().navigation.sections.structure).toBe(false);
            expect(root.querySelector('cem-source-editor[data-cem-studio-editor]')).toBe(editor);
            expect(control.value).toBe('# Article');
            expect(service.snapshot().navigation.selection.file).toBe('article');
        } finally { view.dispose(); root.remove(); }
    }, 30_000);

    it('keeps the sample filename in the pane heading and accessible editor name without repeating it in the editor body', async () => {
        const { service } = await fixture('https://studio.test/?sample=demo&file=article');
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench: service });
        try {
            await view.whenSettled();
            expect(service.snapshot().navigation.content).toBe('sample-file');
            const editor = root.querySelector('cem-studio-project-picker cem-source-editor[readonly]');
            expect(editor).not.toBeNull();
            expect(editor.querySelector('[part=label]').textContent).toBe('');
            expect(editor.querySelector('[role=textbox]').getAttribute('aria-label')).toBe('content/article.md');
            expect(editor.value).toBe('# Article');
            expect(root.querySelector('[slot=editor-heading]').textContent.trim()).toBe('article.md');
        } finally { view.dispose(); root.remove(); }
    }, 30_000);

    it('renders project Build and readonly output centers from the last activated navigation item', async () => {
        await loadTreeTheme();
        const { service, browser } = await fixture();
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench: service });
        try {
            await service.openEntity('project=demo'); await view.whenSettled();
            const project = root.querySelector('section[aria-label="Project build"]');
            expect(project.hidden).toBe(false);
            expect(root.querySelector('section[aria-label="Selected file"]').hidden).toBe(true);
            expect(getComputedStyle(root.querySelector('section[aria-label="Selected file"]')).display).toBe('none');
            await service.buildProjectPreview(); await view.whenSettled();
            const output = service.snapshot().navigation.output.children[0].children.find(item => item.kind === 'output-file');
            await service.openEntity(output.key); await view.whenSettled();
            expect(service.snapshot().navigation.content).toBe('output-file');
            expect(project.hidden).toBe(true);
            expect(browser.location.searchParams.get('selection')).toBe('output');
            const readonly = root.querySelector('cem-studio-project-picker cem-source-editor[readonly]');
            expect(readonly).not.toBeNull();
            expect(readonly.value).toContain('generated/index.html');
            expect(readonly.querySelector('[part=label]').textContent).toBe('');
            expect(readonly.querySelector('[role=textbox]').getAttribute('aria-label')).toBe(output.path);
            expect(readonly.getAttribute('source-id')).toContain('output');
            expect(root.querySelectorAll('cem-tree[label="Output files"] [role=treeitem]').length).toBeGreaterThan(1);
            const trees = [...root.querySelectorAll('cem-studio-workbench-navigation cem-tree')];
            expect(trees).toHaveLength(3);
            trees.forEach(expectCondensedTree);
            const invalid = service.snapshot().navigation.output.children[0].children.find(item => item.path === 'generated/invalid.json');
            await service.openEntity(invalid.key); await view.whenSettled();
            expect(service.snapshot().projectPreview.preview.kind).toBe('download');
            expect(root.querySelector('cem-studio-workbench-preview[section=preview] cem-data-view')).toBeNull();
            expect(root.querySelector('cem-studio-workbench-preview[section=preview]').textContent).toContain('not valid UTF-8');
        } finally { view.dispose(); root.remove(); }
    }, 30_000);
});
