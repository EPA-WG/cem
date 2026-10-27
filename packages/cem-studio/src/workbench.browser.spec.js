import { parseCemMlCommandText } from '@epa-wg/cem-ml-cli/browser-commands';
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from 'vitest';

import { describeCemStudioOperationFields, describeCemStudioOperationGroups, editCemStudioOperationField, editCemStudioOperationFields, createCemStudioBrowserValidator } from './feature-tour.js';
import { createCemStudioFileSystemProvider, serializeCemStudioProjectBundle, parseCemStudioProjectBundle } from './file-system-provider.js';
import { createCemStudioNavigation, createCemStudioProjectPreview } from './navigation.js';
import { createCemStudioProjectRepository } from './repository.js';
import { CEM_STUDIO_LIMITS } from './preview.js';
import { createCemStudioProjectSession } from './project-session.js';
import { mountCemStudioApplicationShell } from './shell.js';
import {
    CEM_STUDIO_INSPECT_VIEWS,
    CEM_STUDIO_PARSE_PROJECTIONS,
    createCemStudioFeatureTourWorkbench,
    createCemStudioProjectWorkbench,
    createCemStudioSessionWorkbench,
    mountCemStudioFeatureTourWorkbench,
} from './workbench.js';

// Supply the same resource identity as the production page's import map.
beforeAll(() => {
    const map = document.createElement('script'); map.type = 'importmap';
    map.textContent = JSON.stringify({ imports: {
        '@epa-wg/cem-components/components.xhtml': new URL('../../cem-components/dist/components.xhtml', import.meta.url).href,
    } });
    document.head.append(map);
});

const repositories = [];
const workbenches = [];
const views = [];
const shells = [];

afterEach(async () => {
    for (const view of views.splice(0)) view.dispose();
    for (const shell of shells.splice(0)) shell.dispose();
    for (const workbench of workbenches.splice(0)) workbench.dispose();
    const names = [...new Set(repositories.map(({ databaseName }) => databaseName))];
    for (const repository of repositories.splice(0)) repository.close();
    for (const name of names) await deleteDatabase(name);
    document.body.replaceChildren();
});

const schemaSourceAdmissionCases = [
    { mediaType: 'application/xslt+xml', normalizedType: ' Application/XSLT+XML ; charset="UTF-8"',
        schema: 'https://cem.dev/ns/transform/xslt/1', path: 'templates/note.xsl',
        source: '<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template name="note"><p>Café note</p></xsl:template></xsl:stylesheet>\n' },
    { mediaType: 'application/schema+json', normalizedType: 'APPLICATION/SCHEMA+JSON; charset=utf-8',
        schema: 'https://cem.dev/ns/data/json-schema/1', path: 'schemas/note.schema.json',
        source: '{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"Café note","type":"object"}\n' },
    { mediaType: 'application/relax-ng+xml', normalizedType: 'application/relax-ng+xml; charset=UTF-8',
        schema: 'https://cem.dev/ns/data/relax-ng/1', path: 'schemas/note.rng',
        source: '<grammar xmlns="http://relaxng.org/ns/structure/1.0"><!-- Café note --><start><element name="note"><text/></element></start></grammar>\n' },
    { mediaType: 'application/relax-ng-compact-syntax', normalizedType: ' Application/Relax-NG-Compact-Syntax ; charset="UTF-8"',
        schema: 'https://cem.dev/ns/data/relax-ng/1', path: 'schemas/note.rnc',
        source: '# Café note\nstart = element note { text }\n' },
];


describe('CEM Studio Feature Tour workbench', () => {
    it('validates the selected saved source with its authored namespace command', async () => {
        const response = await fetch(new URL('../generated/feature-tour/examples/xpath-catalog.cem-studio.json', import.meta.url));
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const repository = await repositoryWithSource('unrelated project');
        await repository.execute(request('import-project', { bundle }));
        const validator = validatorFor(validResult());
        validator.decodeResourceCommand = () => ({ commandPath: ['validate'], positionals: { inputs: ['queries/supplier-join.xpath'] } });
        validator.executeAuthoredResourceCommand = vi.fn(async () => validResult());
        const workbench = await createCemStudioProjectWorkbench({ repository, validator,
            selection: { projectId: bundle.project.id, entryId: 'validate-join' } });
        workbenches.push(workbench);
        await workbench.validatePersisted();
        expect(workbench.snapshot().status).toBe('valid');
        expect(validator.executeAuthoredResourceCommand).toHaveBeenCalledTimes(1);
        expect(validator.validateResource).not.toHaveBeenCalled();
        const invocation = validator.executeAuthoredResourceCommand.mock.calls[0][0];
        const command = JSON.parse(new TextDecoder().decode(invocation.commandResource));
        expect(command.argv).toContain('c=urn:example:catalog');
        expect(invocation.uri).toBe(`studio://${bundle.project.id}/queries/supplier-join.xpath`);
        expect(invocation.resourceRevision).toBe(workbench.snapshot().resourceRevision);
        const file = await createCemStudioProjectWorkbench({ repository, validator,
            selection: { projectId: bundle.project.id, resourceId: 'join-query' } });
        workbenches.push(file);
        await file.validatePersisted();
        expect(validator.validateResource).toHaveBeenCalledTimes(1);
        expect(validator.executeAuthoredResourceCommand).toHaveBeenCalledTimes(1);
    });

    it.each([
        ['login', 'login-html', 'text/html'],
        ['login', 'login-live-view', 'application/vnd.cem.element-template+cem'],
        ['supplier-pipeline', 'supplier-view', 'application/vnd.cem.template+cem'],
        ['markdown-site', 'assets-site-css', 'text/css'],
        ['css-selector', 'selector-source', 'application/vnd.cem.query-expression+css-selector'],
    ])('adopts shared source paint with readonly samples, URL identity and durable drafts: %s/%s', async (name, resourceId, contentType) => {
        const response = await fetch(new URL(`../generated/feature-tour/examples/${name}.cem-studio.json`, import.meta.url));
        expect(response.ok).toBe(true);
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const repository = await repositoryWithSource('unrelated saved project');
        const validator = validatorFor(validResult());
        const records = new Map();
        const storage = { getItem: key => records.get(key) ?? null, setItem: (key, value) => records.set(key, value) };
        const browser = new EventTarget(); browser.location = new URL(`https://studio.test/?sample=${name}&file=${resourceId}`);
        browser.history = { pushState: (_, __, url) => { browser.location = new URL(url); }, replaceState: (_, __, url) => { browser.location = new URL(url); } };
        const root = document.createElement('main'); document.body.append(root);
        let session, workbench, view;
        const mount = async () => {
            session = createCemStudioProjectSession({ storage, openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
            workbenches.push(session);
            const base = await createCemStudioSessionWorkbench({ repository, session });
            workbench = await createCemStudioNavigation({ repository, workbench: base,
                examples: [{ id: name, name, load: async () => bundle }], browser }); workbenches.push(workbench);
            view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        };
        const paintRanges = host => [...CSS.highlights.values()].flatMap(highlight => [...highlight]).filter(range => host.contains(range.startContainer));
        const assertPaint = async host => {
            await vi.waitFor(() => expect(paintRanges(host).length).toBeGreaterThan(0));
            expect(paintRanges(host).every(range => host.contains(range.endContainer))).toBe(true);
            const control = host.querySelector('[role=textbox]');
            expect(control.textContent).toBe(host.value);
            expect(control.children).toHaveLength(host.value.endsWith('\n') ? 1 : 0);
            for (const child of control.children) {
                expect(child).toBe(control.lastChild);
                expect(child.tagName).toBe('BR');
                expect(child.attributes).toHaveLength(0);
                expect(child.childNodes).toHaveLength(0);
            }
        };
        await mount();
        const sampleEditor = root.querySelector('cem-source-editor[readonly]');
        const original = new TextDecoder().decode(bundle.contents[resourceId]);
        expect(sampleEditor.value).toBe(original);
        expect(sampleEditor.getAttribute('content-type')).toBe(contentType);
        expect(sampleEditor.querySelector('[role=textbox]').contentEditable).toBe('false');
        await assertPaint(sampleEditor);
        const sampleIdentity = sampleEditor.getAttribute('source-id');
        await workbench.openEntity(`sample=${name}`); workbench.requestAction('clone');
        await workbench.submitAction({ name: [`${name} editor copy`] }); await view.whenSettled();
        const projectId = workbench.snapshot().navigation.selection.project;
        await workbench.openEntity(`project=${projectId}&file=${resourceId}`); await view.whenSettled();
        let host = root.querySelector('cem-source-editor[data-cem-studio-editor]');
        let control = host.querySelector('[role=textbox]');
        expect(host.getAttribute('source-id')).toBe(JSON.stringify([projectId, resourceId]));
        expect(host.getAttribute('source-id')).not.toBe(sampleIdentity);
        expect(host.getAttribute('content-type')).toBe(contentType);
        expect(new URL(browser.location).searchParams.get('file')).toBe(resourceId);
        expect(control.value).toBe(original); await assertPaint(host);
        const draft = original + '\n\t🧬 <unfinished\n';
        control.focus(); control.value = draft; control.setSelectionRange(2, 7);
        control.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText' })); await view.whenSettled();
        await assertPaint(host);
        expect(control.value).toBe(draft); expect([control.selectionStart, control.selectionEnd]).toEqual([2, 7]);
        expect(document.activeElement).toBe(control); expect(workbench.snapshot().dirty).toBe(true);
        const durable = (await repository.query(request('export-project', { projectId }))).value;
        expect(new TextDecoder().decode(durable.contents[resourceId])).toBe(original);
        expect(validator.runResourceCommand).not.toHaveBeenCalled(); // Paint never formats source.
        const journal = [...records.values()].find(value => value.includes('"draft"'));
        expect(JSON.parse(journal).draft).toBe(draft); expect(journal).not.toContain('cem-source-name');
        // Reopen the same URL using only the ordinary draft journal; paint is regenerated.
        view.dispose(); workbench.dispose(); session.dispose(); root.replaceChildren(); await mount();
        host = root.querySelector('cem-source-editor[data-cem-studio-editor]'); control = host.querySelector('[role=textbox]');
        expect(control.value).toBe(draft); expect(session.snapshot().draftRecovery.status).toBe('recovered');
        await assertPaint(host);
        await workbench.openEntity(`project=${projectId}&file=readme`); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('source');
        expect(control.value).toBe(draft); expect(host.getAttribute('source-id')).toBe(JSON.stringify([projectId, resourceId]));
        await workbench.resolveNavigation('cancel'); await view.whenSettled();
        expect(new URL(browser.location).searchParams.get('file')).toBe(resourceId);
        await workbench.saveSource(); await view.whenSettled();
        expect(new TextDecoder().decode((await repository.query(request('export-project', { projectId }))).value.contents[resourceId])).toBe(draft);
        workbench.updateDraft(draft + 'discard this'); await view.whenSettled();
        await workbench.openEntity(`project=${projectId}&file=readme`); await workbench.resolveNavigation('discard'); await view.whenSettled();
        expect(new URL(browser.location).searchParams.get('file')).toBe('readme');
        host = root.querySelector('cem-source-editor[data-cem-studio-editor]');
        expect(host.getAttribute('source-id')).toBe(JSON.stringify([projectId, 'readme']));
        expect(host.getAttribute('content-type')).toMatch(/^text\/markdown/);
        expect(paintRanges(host)).toHaveLength(0);
        expect(host.value).toBe(new TextDecoder().decode(bundle.contents.readme));
        expect(new TextDecoder().decode(bundle.contents[resourceId])).toBe(original);
    }, 30_000);

    it('opens a just-imported project before cached inventory receives its repository notification', async () => {
        const repository = await repositoryWithSource('saved content');
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        // Delay notification delivery without delaying the committed repository.
        const inventory = { query: input => repository.query(input), execute: input => repository.execute(input), subscribe: () => () => undefined };
        const workbench = await createCemStudioSessionWorkbench({ repository: inventory, session }); workbenches.push(workbench);
        const bundle = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        bundle.project = { ...bundle.project, id: 'just-imported', name: 'Just imported', rootUri: 'studio://just-imported/' };
        await repository.execute(request('import-project', { bundle, mode: 'create' }));
        expect(workbench.snapshot().workbenches.some(choice => choice.selection.projectId === 'just-imported')).toBe(false);
        await workbench.selectWorkbench(JSON.stringify({ projectId: 'just-imported', resourceId: 'source' }));
        expect(workbench.snapshot()).toMatchObject({ projectId: 'just-imported', resourceId: 'source', persistedText: 'saved content' });
        await expect(workbench.selectWorkbench(JSON.stringify({ projectId: 'missing', resourceId: 'source' }))).rejects.toThrow('unavailable');
        expect(workbench.snapshot().projectId).toBe('just-imported');
    });

    it('selects explicit build entries with URL precedence, missing-output reporting and stale admission guards', async () => {
        const repository = await repositoryWithSource('{main | saved}');
        const bundle = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const entry = { id: 'site', name: 'Site build', kind: 'transformation-graph', entryArtifact: 'generated/index.html' };
        bundle.project.entries = [entry];
        // The test transport exposes one native command resource as a named build.
        bundle.project.resources.push({ id: 'build-command', schema: 'https://cem.dev/ns/cli/command/1' }); entry.runConfigResourceId = 'build-command';
        let release;
        const candidates = [];
        const service = createCemStudioProjectPreview({ repository, openBuildWorkbench: async () => {
            const pending = deferred(); release = pending.resolve;
            const artifacts = ['generated/articles/index.html', 'generated/index.html', 'generated/other.html'].map(path => ({ id: path, path,
                uri: `studio://feature-tour/${path}`, contentType: 'text/html' }));
            const projection = { exitCode: 0, buildRun: { id: `run-${candidates.length}`, inputs: [], artifacts },
                outputs: artifacts.map(item => ({ uri: item.uri, bytes: new TextEncoder().encode('<h1>Built</h1>') })) };
            const candidate = { snapshot: () => ({ projection }), runPersistedOperation: () => pending.promise,
                selectArtifact: vi.fn(), dispose: vi.fn() };
            candidates.push(candidate); return candidate;
        } }); workbenches.push(service);
        const context = () => { const project = structuredClone(bundle.project); return { key: 'sample=site', project, bundle: { ...bundle, project } }; };
        const build = async () => { const pending = service.build(); await vi.waitFor(() => expect(release).toBeTypeOf('function')); release(); release = undefined; await pending; };
        service.setContext(context());
        expect(service.snapshot()).toMatchObject({ artifactPath: 'generated/index.html', entryArtifact: 'generated/index.html' });
        await build(); expect(service.snapshot().selectedArtifact.path).toBe('generated/index.html');
        service.setContext(context(), { artifact: 'generated/other.html?view=wide#part', reset: true }); await build();
        expect(service.snapshot().artifactPath).toBe('generated/other.html?view=wide#part');
        expect(service.snapshot().selectedArtifact.path).toBe('generated/other.html');
        entry.entryArtifact = 'generated/missing.html'; service.setContext(context());
        expect(service.snapshot().artifactPath).toBe('generated/other.html?view=wide#part');
        expect(service.snapshot().stale).toBe(true);
        service.selectConfiguration('site'); await build();
        expect(service.snapshot()).toMatchObject({ artifactPath: 'generated/missing.html', missingArtifact: true });
        expect(service.snapshot().selectedArtifact).toBeUndefined();
        service.selectArtifact('generated/articles/index.html'); await build();
        expect(service.snapshot().selectedArtifact.path).toBe('generated/articles/index.html');
        entry.entryArtifact = 'generated/index.html'; service.setContext(context(), { reset: true }); await build();
        const successful = service.snapshot().projection;
        const late = service.build(); await vi.waitFor(() => expect(release).toBeTypeOf('function'));
        entry.entryArtifact = 'generated/other.html'; service.setContext(context()); release(); release = undefined; await late;
        expect(service.snapshot()).toMatchObject({ stale: true, attempt: { status: 'stale' } });
        expect(service.snapshot().projection).toBe(successful); expect(candidates.at(-1).dispose).toHaveBeenCalled();
        delete entry.entryArtifact; service.setContext(context(), { reset: true }); await build();
        expect(service.snapshot().artifactPath).toBeUndefined(); // Two index documents remain ambiguous.
        entry.entryArtifact = 'generated/index.html'; service.setContext(context(), { artifact: 'gone.html', reset: true }); await build();
        expect(service.snapshot()).toMatchObject({ artifactPath: 'gone.html', missingArtifact: true });
        bundle.project.entries.push({ ...entry, id: 'other', entryArtifact: 'generated/other.html' });
        service.setContext(context(), { build: 'other', reset: true }); await build();
        expect(service.snapshot()).toMatchObject({ configurationId: 'other', artifactPath: 'generated/other.html' });
    });

    it('opens the ordinary Markdown site sample, edits and repairs builds, and runs a portable independent copy', async () => {
        const response = await fetch(new URL('../generated/feature-tour/examples/markdown-site.cem-studio.json', import.meta.url));
        expect(response.ok).toBe(true);
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const repository = await repositoryWithSource('unrelated saved resource');
        const validator = await createCemStudioBrowserValidator();
        repository.validateProject = validator.validateProject;
        try {
            const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) }); workbenches.push(session);
            const base = await createCemStudioSessionWorkbench({ repository, session });
            const browser = new EventTarget(); browser.location = new URL('https://studio.test/?sample=markdown-site&build=build-site');
            browser.history = { pushState: vi.fn((_, __, url) => { browser.location = new URL(url); }), replaceState: vi.fn((_, __, url) => { browser.location = new URL(url); }) };
            const workbench = await createCemStudioNavigation({ repository, validator, workbench: base,
                examples: [{ id: 'markdown-site', name: 'Markdown site', collections: ['amsterdam-2026'], load: async () => bundle }], browser }); workbenches.push(workbench);
            const root = document.createElement('main'); document.body.append(root);
            const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
            const preview = () => workbench.snapshot().projectPreview;
            const build = async () => { await workbench.buildProjectPreview(); await view.whenSettled();
                expect(preview().attempt, JSON.stringify(preview().attempt)).toMatchObject({ status: 'succeeded' }); };
            await build();
            expect((await repository.query(request('list-projects', {}))).value).toHaveLength(1);
            expect(preview().projection.buildRun.artifacts.map(item => item.path).sort()).toEqual([
                'generated/articles/first-build.html', 'generated/articles/index.html', 'generated/articles/shared-layouts.html',
                'generated/assets/site.css', 'generated/downloads/notes.txt', 'generated/images/harbor.svg', 'generated/index.html',
            ]);
            for (const output of preview().projection.outputs) {
                const path = output.uri.split('/generated/')[1];
                const expected = bundle.project.resources.find(resource => resource.path === (path.endsWith('.html') ? `expected/${path}` : path.startsWith('assets/') ? path : `static/${path}`));
                expect(output.bytes, path).toEqual(bundle.contents[expected.id]);
            }
            const frame = () => root.querySelector('cem-media-preview[mode="artifact-tree"] iframe');
            await vi.waitFor(() => expect(frame()?.contentDocument?.querySelector('h1')?.textContent).toBe('CEM field notes'));
            expect(getComputedStyle(frame().contentDocument.querySelector('nav')).display).toBe('flex');
            await vi.waitFor(() => expect(frame().contentDocument.querySelector('img').naturalWidth).toBe(480));
            [...frame().contentDocument.querySelectorAll('a')].find(link => link.textContent === 'a small build').click();
            await vi.waitFor(() => expect(preview().artifactPath).toBe('generated/articles/first-build.html')); await view.whenSettled();
            expect(new URL(browser.location).searchParams.get('artifact')).toBe('generated/articles/first-build.html');
            expect(workbench.snapshot().navigation.selection.sample).toBe('markdown-site');
            workbench.requestAction('clone'); await workbench.submitAction({ name: ['Site copy'] }); await view.whenSettled();
            const projectId = workbench.snapshot().navigation.selection.project;
            expect(projectId).toBeTruthy();
            expect(preview().entryArtifact).toBe('generated/index.html');
            root.querySelector('cem-expansion[label="Build entry settings"] [part=header]').click(); await view.whenSettled();
            const saveEntry = async path => {
                const input = root.querySelector('cem-text-field[label="Build entry artifact"] input');
                input.value = path; input.dispatchEvent(new Event('input', { bubbles: true })); await view.whenSettled();
                root.querySelector('form[host-command="save-build-entry"] button').click(); await view.whenSettled();
            };
            await saveEntry('../outside.html');
            expect(preview().entryError.message).toContain('entryArtifact');
            expect(preview().entryArtifact).toBe('generated/index.html');
            await saveEntry('generated/missing.html'); await build();
            expect(preview()).toMatchObject({ entryArtifact: 'generated/missing.html', missingArtifact: true });
            expect(root.querySelector('[data-cem-studio-preview]').textContent).toContain('requested artifact is unavailable');
            await saveEntry('generated/index.html');
            await workbench.editStructure({ kind: 'undo' });
            await vi.waitFor(() => expect(preview().entryArtifact).toBe('generated/missing.html'));
            await workbench.editStructure({ kind: 'redo' });
            await vi.waitFor(() => expect(preview().entryArtifact).toBe('generated/index.html')); await view.whenSettled();
            await saveEntry('');
            expect(preview().entryArtifact).toBeUndefined();
            expect((await repository.query(request('export-project', { projectId }))).value.project.entries.find(item => item.id === 'build-site').entryArtifact).toBeUndefined();
            await saveEntry('generated/index.html');
            await build();
            expect(preview().projection.buildRun.configuration.entryArtifact).toBe('generated/index.html');
            const articleId = 'content-articles-first-build-md';
            await workbench.openEntity(`project=${projectId}&file=${articleId}`); await view.whenSettled();
            const original = workbench.snapshot().draft;
            const editor = root.querySelector('[data-cem-studio-editor] [role="textbox"]');
            workbench.updateDraft(original.replaceAll('Build a small site', 'Build an edited site'));
            await workbench.saveSource(); await view.whenSettled(); await build();
            workbench.selectPreviewArtifact('generated/articles/first-build.html'); await view.whenSettled();
            expect(preview().projection.output.text).toContain('<title>Build an edited site</title>');
            expect(workbench.snapshot().resourceId).toBe(articleId);
            expect(root.querySelector('[data-cem-studio-editor] [role="textbox"]')).toBe(editor);
            const successful = preview().projection.buildRun, resourceSet = preview().preview.resourceSet;
            workbench.updateDraft(original.replace('title: Build a small site', 'title: [broken'));
            await workbench.saveSource(); await view.whenSettled(); await workbench.buildProjectPreview(); await view.whenSettled();
            expect(preview().attempt.status).toBe('failed');
            expect(preview().attempt.diagnostics).toEqual(expect.arrayContaining([expect.objectContaining({ code: 'cem.markdown.front_matter_invalid' })]));
            expect(preview().projection.buildRun).toBe(successful); expect(preview().preview.resourceSet).toBe(resourceSet);
            workbench.updateDraft(original); await workbench.saveSource(); await view.whenSettled(); await build();
            await workbench.openEntity(`project=${projectId}&file=layouts-partials-chrome-cemt`); await view.whenSettled();
            workbench.updateDraft(workbench.snapshot().draft.replace('Built from saved project resources.', 'Shared layout edited.'));
            await workbench.saveSource(); await view.whenSettled(); await build();
            const htmlOutputs = preview().projection.outputs.filter(output => output.contentType.startsWith('text/html'));
            expect(htmlOutputs).toHaveLength(4); expect(htmlOutputs.every(output => output.text.includes('Shared layout edited.'))).toBe(true);
            const article = preview().projection.buildRun.artifacts.find(item => item.path === 'generated/articles/first-build.html');
            expect(article.provenance).toEqual(expect.arrayContaining([
                expect.objectContaining({ uri: `studio://${projectId}/content/articles/first-build.md` }),
                expect.objectContaining({ uri: `studio://${projectId}/layouts/partials/chrome.cemt` }),
            ]));
            const exported = (await repository.query(request('export-project', { projectId }))).value;
            const portable = parseCemStudioProjectBundle(serializeCemStudioProjectBundle(exported));
            const copyId = 'markdown-portable';
            portable.project = { ...portable.project, id: copyId, name: 'Portable site', rootUri: `studio://${copyId}/` };
            await repository.execute(request('import-project', { bundle: portable, mode: 'create' }));
            await vi.waitFor(() => expect(workbench.snapshot().navigation.saved.children.some(item => item.route.project === copyId)).toBe(true));
            await workbench.openEntity(`project=${copyId}&file=${articleId}`); await view.whenSettled(); await build();
            expect(preview().projection.outputs.filter(output => output.contentType.startsWith('text/html')).every(output => output.text.includes('Shared layout edited.'))).toBe(true);
            expect(preview().projection.buildRun.artifacts.every(item => item.uri.startsWith(`studio://${copyId}/`)), JSON.stringify(preview().projection.buildRun.artifacts.map(item => item.uri))).toBe(true);
            root.querySelector('cem-expansion[label="Pinned builds"] [part="header"]').click(); await view.whenSettled();
            const toolbar = root.querySelector('cem-studio-workbench-preview[section=build] [part="build-controls"]');
            const previewRegion = root.querySelector('cem-studio-workbench-preview[section=preview] [part="preview"]');
            expect(getComputedStyle(toolbar).overflowY).toBe('auto');
            expect(toolbar.closest('cem-studio-workbench-preview').getAttribute('section')).toBe('build');
            expect(previewRegion.closest('cem-studio-workbench-preview').getAttribute('section')).toBe('preview');
            const pinButton = [...root.querySelectorAll('cem-action')].find(action => action.textContent.trim() === 'Pin successful build');
            pinButton.querySelector('button').click(); await view.whenSettled();
            expect(preview().pinError).toBeUndefined(); expect(preview().pins).toHaveLength(1);
            const pinId = preview().pins[0].key, pinnedRun = preview().projection.buildRun;
            const pinnedBytes = preview().projection.outputs.map(item => new Uint8Array(item.bytes));
            workbench.updateDraft(original.replaceAll('Build a small site', 'Only this copy'));
            await workbench.saveSource(); await view.whenSettled(); await build();
            const current = preview().projection, currentResourceSet = preview().preview.resourceSet, selectedFile = workbench.snapshot().navigation.selection.file;
            const expectCurrentPreview = () => {
                expect(preview().projection.buildRun).toBe(current.buildRun);
                expect(preview().projection.outputs).toBe(current.outputs);
                expect(preview().preview.resourceSet).toBe(currentResourceSet);
                expect(preview().selectedArtifact.path).toBe('generated/articles/first-build.html');
                expect(preview().preview.artifactPath).toBe('generated/articles/first-build.html');
            };
            workbench.updateDraft(workbench.snapshot().draft + '\nUnsaved draft retained.'); await view.whenSettled();
            const draft = workbench.snapshot().draft;
            const pinSelect = root.querySelector('cem-select[label="Preview build"]');
            pinSelect.value = pinId; pinSelect.dispatchEvent(new Event('change', { bubbles: true })); await view.whenSettled();
            expect(preview().pinError).toBeUndefined(); expect(preview().pinId).toBe(pinId);
            expect(preview().currentRunId).toBe(current.buildRun.id);
            const comparison = preview().comparison;
            expect(comparison.baselineRunId).toBe(pinnedRun.id); expect(comparison.currentRunId).toBe(current.buildRun.id);
            expect(comparison.counts).toEqual({ added: 0, removed: 0, changed: 1, unchanged: 6 });
            const comparedArticle = comparison.artifacts.find(item => item.path === 'generated/articles/first-build.html');
            expect(comparedArticle.bytesChanged).toBe(true);
            expect(comparedArticle.before.sha256).toBe(pinnedRun.artifacts.find(item => item.path === comparedArticle.path).sha256);
            expect(comparedArticle.after.sha256).toBe(current.buildRun.artifacts.find(item => item.path === comparedArticle.path).sha256);
            expect(comparedArticle.before.provenance.some(frame => frame.uri.endsWith('/content/articles/first-build.md'))).toBe(true);
            expect(comparedArticle.after.provenance.some(frame => frame.uri.endsWith('/layouts/partials/chrome.cemt'))).toBe(true);
            expect(comparison.inputs.find(item => item.resourceId === articleId).status).toBe('changed');
            root.querySelector('cem-expansion[label="Compare pin with current build"] [part="header"]').click(); await view.whenSettled();
            expect(root.querySelector('[aria-label="Artifact comparison"]').textContent).toContain('changed · generated/articles/first-build.html');
            expect(root.querySelector('[aria-label="Pinned native provenance"]').textContent).toContain('/content/articles/first-build.md');
            expect(root.querySelector('[aria-label="Current native provenance"]').textContent).toContain('/layouts/partials/chrome.cemt');
            expect(root.querySelector('cem-expansion[label="Compare pin with current build"]').textContent).toContain(comparedArticle.before.sha256);
            expect(root.querySelector('cem-expansion[label="Compare pin with current build"]').textContent).toContain(comparedArticle.after.sha256);
            const selectedPreviewPath = preview().artifactPath;
            expect(selectedPreviewPath).toBe('generated/articles/first-build.html');
            expect(preview().projection.buildRun.configuration.entryArtifact).toBe('generated/index.html');
            selectValue(root, '[label="Compare artifact"]', 'generated/images/harbor.svg'); await view.whenSettled();
            expect(root.querySelector('cem-select[label="Compare artifact"]').value).toBe('generated/images/harbor.svg');
            expect(preview().comparison).toBe(comparison); expect(preview().artifactPath).toBe(selectedPreviewPath);
            selectValue(root, '[label="Compare artifact"]', comparedArticle.path); await view.whenSettled();

            expect(preview().projection.outputs.map(item => new Uint8Array(item.bytes))).toEqual(pinnedBytes);
            expect(preview().projection.buildRun.inputs).toEqual(pinnedRun.inputs);
            expect(preview().projection.buildRun.artifacts.map(item => item.provenance)).toEqual(pinnedRun.artifacts.map(item => item.provenance));
            expect(workbench.snapshot().draft).toBe(draft); expect(workbench.snapshot().dirty).toBe(true);
            expect(workbench.snapshot().navigation.selection.file).toBe(selectedFile);
            await vi.waitFor(() => expect(frame()?.contentDocument?.querySelector('img')?.naturalWidth).toBe(480));
            expect(getComputedStyle(frame().contentDocument.querySelector('nav')).display).toBe('flex');
            workbench.selectPreviewArtifact('generated/articles/first-build.html'); await view.whenSettled();
            let copied, downloaded;
            await workbench.copyPreview(async text => { copied = text; });
            await workbench.downloadPreview(async file => { downloaded = file; });
            const pinnedArticle = preview().projection.output;
            expect(copied).toBe(new TextDecoder().decode(pinnedArticle.bytes));
            expect(downloaded.bytes).toEqual(pinnedArticle.bytes); expect(downloaded.filename).toBe('first-build.html');
            // A fresh preview owner reopens the complete build without invoking native execution.
            const storedProject = (await repository.query(request('get-project', { projectId: copyId }))).value;
            const reopen = createCemStudioProjectPreview({ repository }); workbenches.push(reopen);
            reopen.setContext({ key: `project=${copyId}`, project: storedProject });
            await vi.waitFor(() => expect(reopen.snapshot().pins).toHaveLength(1));
            await reopen.selectPin(pinId);
            expect(reopen.snapshot().comparison).toBeUndefined();
            expect(reopen.snapshot().pinId).toBe(pinId); expect(reopen.snapshot().attempt).toBeUndefined();
            expect(reopen.snapshot().projection.outputs.map(item => new Uint8Array(item.bytes))).toEqual(pinnedBytes);
            reopen.dispose();
            const pendingRead = deferred(); let readStarted = false;
            const delayed = createCemStudioProjectPreview({ repository: { query: async query => {
                if (query.operation === 'get-build-snapshot') { readStarted = true; await pendingRead.promise; }
                return repository.query(query);
            } } }); workbenches.push(delayed);
            delayed.setContext({ key: `project=${copyId}`, project: storedProject });
            const opening = delayed.selectPin(pinId); expect(readStarted).toBe(true);
            delayed.setContext(undefined); pendingRead.resolve(); await opening;
            expect(delayed.snapshot().pinId).toBeUndefined(); expect(delayed.snapshot().projection).toBeUndefined();
            const lateCopy = deferred(), copying = workbench.copyPreview(() => lateCopy.promise);
            await workbench.selectPinnedBuild(''); lateCopy.resolve(); await copying; await view.whenSettled();
            expectCurrentPreview(); expect(preview().comparison).toBeUndefined(); expect(workbench.snapshot().draft).toBe(draft);
            await workbench.selectPinnedBuild(pinId); await view.whenSettled();
            const deleteButton = [...root.querySelectorAll('cem-action')].find(action => action.textContent.trim() === 'Delete pinned build');
            deleteButton.querySelector('button').click(); await view.whenSettled();
            expect(preview().pinId).toBeUndefined(); expectCurrentPreview(); expect(preview().pins).toHaveLength(0);
            expect(workbench.snapshot().draft).toBe(draft); expect(workbench.snapshot().navigation.selection.file).toBe(selectedFile);
            // An unreadable pin remains selected for deletion while the current view stays intact.
            await workbench.pinProjectBuild(); const damagedId = preview().pins[0].key;
            await new Promise((resolve, reject) => {
                const transaction = repository.database.transaction('resultSnapshots', 'readwrite'), store = transaction.objectStore('resultSnapshots');
                const read = store.get(damagedId);
                read.onsuccess = () => { const record = read.result; new Uint8Array(record.outputs[0].bytes)[0] ^= 1; store.put(record); };
                transaction.oncomplete = resolve; transaction.onabort = () => reject(transaction.error);
            });
            await workbench.selectPinnedBuild(damagedId); await view.whenSettled();
            expect(preview().comparison).toBeUndefined();
            expect(preview().pinError.message).toContain('SHA-256'); expect(preview().pinSelection).toBe(damagedId);
            expect(preview().pinId).toBeUndefined(); expectCurrentPreview();
            const damagedDelete = [...root.querySelectorAll('cem-action')].find(action => action.textContent.trim() === 'Delete pinned build');
            expect(damagedDelete.querySelector('button').disabled).toBe(false);
            damagedDelete.querySelector('button').click(); await view.whenSettled();
            expect(preview().pins).toHaveLength(0); expect(preview().pinSelection).toBeUndefined();
            expectCurrentPreview(); expect(workbench.snapshot().draft).toBe(draft);
            const unchanged = (await repository.query(request('export-project', { projectId }))).value;
            expect(new TextDecoder().decode(unchanged.contents[articleId])).toBe(original);
            expect(bundle.contents[articleId]).toEqual(new Uint8Array(unchanged.contents[articleId]));
            const references = project => project.resources.map(resource => [resource.id, resource.referencePath]).sort(([left], [right]) => left.localeCompare(right));
            expect(references(portable.project)).toEqual(references(bundle.project));
        } finally { await validator.close(); }
    }, 90_000);

    it('builds native site metadata and source provenance and retains the last success on invalid front matter', async () => {
        // Literal asset URLs let Vite serve the shared native fixture verbatim.
        const [graph, layout, partial, site] = await Promise.all([
            new URL('../../cem_ml_transform_cem_ql/tests/fixtures/site-metadata/graphs/build.cem', import.meta.url),
            new URL('../../cem_ml_transform_cem_ql/tests/fixtures/site-metadata/layouts/page.cemt', import.meta.url),
            new URL('../../cem_ml_transform_cem_ql/tests/fixtures/site-metadata/layouts/partials/content.cemt', import.meta.url),
            new URL('../../cem_ml_transform_cem_ql/tests/fixtures/site-metadata/data/site.json', import.meta.url),
        ].map(async url => { const response = await fetch(url); expect(response.ok, url.href).toBe(true); return response.text(); }));
        const repository = await repositoryWithSource('unrelated saved source');
        const exported = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const first = '---\r\ntitle: Page A\r\ndescription: Café\r\n---\r\n# Body A\r\n';
        const command = { $schema: 'https://cem.dev/ns/cli/command/1', schemaVersion: 1, commandSchemaVersion: 1,
            commonVersion: '0.1.0', binaryName: 'cem-ml', argv: ['transform', '--config', 'graphs/build.cem',
                '--config-content-type', 'application/cem', '--config-schema', 'https://cem.dev/ns/cli/transform-config/1'] };
        const definitions = [
            ['home', 'content/index.md', 'text/markdown; charset=utf-8', first],
            ['next', 'content/posts/next.md', 'text/markdown; charset=utf-8', '---\ntitle: Page B\n---\n# Body B\n'],
            ['site', 'data/site.json', 'application/json', site],
            ['layout', 'layouts/page.cemt', 'application/vnd.cem.transform+cem', layout],
            ['partial', 'layouts/partials/content.cemt', 'application/vnd.cem.transform+cem', partial],
            ['graph', 'graphs/build.cem', 'application/cem', graph, 'https://cem.dev/ns/cli/transform-config/1'],
            ['command', 'config/build.command.json', 'application/vnd.cem.cli-command+json', JSON.stringify(command), 'https://cem.dev/ns/cli/command/1'],
        ];
        const contents = {}, resources = [];
        for (const [id, path, contentType, content, schema] of definitions) {
            const bytes = new TextEncoder().encode(content); contents[id] = bytes;
            resources.push({ id, path, referencePath: path, contentType, schema, revision: 1, sourceKind: 'project-file',
                role: id === 'graph' ? 'transform-config' : id === 'command' ? 'run-config' : 'data', sha256: await digest(bytes) });
        }
        const bundle = { project: { ...exported.project, id: 'metadata-site', name: 'Metadata site', rootUri: 'studio://metadata-site/', resources,
            entries: [{ id: 'build', name: 'Build site', kind: 'transformation-graph', runConfigResourceId: 'command', resourceIds: ['graph', 'command', ...resources.filter(item => !['graph', 'command'].includes(item.id)).map(item => item.id)] }] }, contents };
        await repository.execute(request('import-project', { bundle }));
        const validator = await createCemStudioBrowserValidator();
        try {
            const preview = createCemStudioProjectPreview({ repository, validator }); workbenches.push(preview);
            const refresh = async () => {
                const current = (await repository.query(request('export-project', { projectId: 'metadata-site' }))).value;
                preview.setContext({ key: 'project=metadata-site', project: current.project });
            };
            await refresh(); await preview.build();
            expect(preview.snapshot().attempt, JSON.stringify(preview.snapshot().attempt)).toMatchObject({ status: 'succeeded' });
            const successful = preview.snapshot().projection.buildRun;
            const resourceSet = preview.snapshot().preview.resourceSet;
            expect(successful.artifacts.map(item => item.path).sort()).toEqual(['generated/index.html', 'generated/posts/next.html']);
            for (const [path, title, heading, source] of [['index.html', 'Page A', 'Body A', 'content/index.md'], ['posts/next.html', 'Page B', 'Body B', 'content/posts/next.md']]) {
                const output = preview.snapshot().projection.outputs.find(item => item.uri.endsWith(`/generated/${path}`));
                const html = new DOMParser().parseFromString(output.text, 'text/html');
                expect(html.title).toBe(title); expect(html.querySelector('h1').textContent).toBe(heading);
                expect(html.querySelector('footer').textContent).toBe('Shared footer');
                expect([...html.querySelectorAll('nav a')].map(item => item.textContent)).toEqual(['Home', 'Next']);
                const artifact = successful.artifacts.find(item => item.uri === output.uri);
                expect(artifact.provenance).toEqual(expect.arrayContaining([
                    expect.objectContaining({ uri: `studio://metadata-site/${source}` }),
                    expect.objectContaining({ uri: 'studio://metadata-site/layouts/partials/content.cemt' }),
                ]));
            }
            await repository.execute(request('save-resource', { projectId: 'metadata-site', resourceId: 'home', expectedProjectRevision: 1,
                expectedResourceRevision: 1, content: '---\ntitle: [broken\n---\n# Body A\n' }));
            await refresh(); await preview.build();
            expect(preview.snapshot().attempt.status).toBe('failed');
            expect(preview.snapshot().attempt.diagnostics).toEqual(expect.arrayContaining([expect.objectContaining({ code: 'cem.markdown.front_matter_invalid' })]));
            expect(preview.snapshot().projection.buildRun).toBe(successful);
            expect(preview.snapshot().preview.resourceSet).toBe(resourceSet);
            await repository.execute(request('save-resource', { projectId: 'metadata-site', resourceId: 'home', expectedProjectRevision: 2,
                expectedResourceRevision: 2, content: first.replace('Page A', 'Repaired page') }));
            await refresh(); await preview.build();
            expect(preview.snapshot().attempt, JSON.stringify(preview.snapshot().attempt)).toMatchObject({ status: 'succeeded' });
            expect(preview.snapshot().projection.outputs.find(item => item.uri.endsWith('/generated/index.html')).text).toContain('<title>Repaired page</title>');
            expect(preview.snapshot().projection.buildRun).not.toBe(successful);
        } finally { await validator.close(); }
    }, 60_000);

    it('builds rooted native content and mixed static assets in the ordinary project preview', async () => {
        const repository = await repositoryWithSource('unrelated saved source');
        const exported = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const encode = text => new TextEncoder().encode(text);
        const graph = `{run |
            {import @id=content @src="../content/**/*.html" @root="../content" @content-type=text/html @opaque=true |
                {export @id=pages @out="../generated/{path}" @content-type=text/html}}
            {import @id=markdown @src="../content/**/*.md" @root="../content" @content-type="text/markdown; charset=utf-8; variant=CommonMark" @schema=https://cem.dev/ns/data/markdown/1 |
                {convert @id=html @content-type=text/html @schema=https://cem.dev/ns/data/html/1 @converter=markdown-to-html-rust |
                    {export @id=article @out="../generated/{dir}/{stem}.html" @content-type=text/html}}}
            {import @id=assets @src="../static/**/*" @root="../static" @opaque=true |
                {export @id=publish @out="../generated/{path}"}}
        }`;
        const command = { $schema: 'https://cem.dev/ns/cli/command/1', schemaVersion: 1, commandSchemaVersion: 1,
            commonVersion: '0.1.0', binaryName: 'cem-ml', argv: ['transform', '--config', 'graphs/build.cem',
                '--config-content-type', 'application/cem', '--config-schema', 'https://cem.dev/ns/cli/transform-config/1'] };
        const definitions = [
            ['home', 'content/index.html', 'text/html', '<main><link rel="stylesheet" href="assets/site.css"><h1>Native folder build</h1><a href="posts/intro.html">Intro</a><img src="assets/logo.svg" alt="Logo"></main>', 'https://cem.dev/ns/data/html/1'],
            ['article', 'content/posts/intro.html', 'text/html', '<main><link rel="stylesheet" href="../assets/site.css"><h1>Nested article</h1><a href="../index.html">Home</a><img src="../assets/logo.svg" alt="Logo"></main>', 'https://cem.dev/ns/data/html/1'],
            ['markdown', 'content/docs/guide.md', 'text/markdown; charset=utf-8; variant=CommonMark', '# Guide\n', 'https://cem.dev/ns/data/markdown/1'],
            ['css', 'static/assets/site.css', 'text/css', 'h1 { color: rgb(19, 38, 57); }'],
            ['logo', 'static/assets/logo.svg', 'image/svg+xml', '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"><rect width="12" height="8" fill="red"/></svg>'],
            ['binary', 'static/downloads/raw.bin', 'application/octet-stream', new Uint8Array([0, 255, 128, 13, 10])],
            ['graph', 'graphs/build.cem', 'application/cem', graph, 'https://cem.dev/ns/cli/transform-config/1'],
            ['command', 'config/build.command.json', 'application/vnd.cem.cli-command+json', JSON.stringify(command), 'https://cem.dev/ns/cli/command/1'],
        ];
        const contents = {}, resources = [];
        for (const [id, path, contentType, content, schema] of definitions) {
            const bytes = typeof content === 'string' ? encode(content) : content;
            contents[id] = bytes;
            resources.push({ id, path, referencePath: path, contentType, schema, revision: 1, sourceKind: 'project-file',
                role: id === 'graph' ? 'transform-config' : id === 'command' ? 'run-config' : 'data', sha256: await digest(bytes) });
        }
        const bundle = { project: { ...exported.project, id: 'folder-site', name: 'Folder site', rootUri: 'studio://folder-site/',
            resources, entries: [{ id: 'build', name: 'Build site', kind: 'transformation-graph', runConfigResourceId: 'command', resourceIds: ['graph', 'command', ...resources.filter(item => !['graph', 'command'].includes(item.id)).map(item => item.id)] }] }, contents };
        await repository.execute(request('import-project', { bundle }));
        const validator = await createCemStudioBrowserValidator();
        try {
            const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) }); workbenches.push(session);
            const base = await createCemStudioSessionWorkbench({ repository, session });
            const browser = new EventTarget(); browser.location = new URL('https://studio.test/?project=folder-site&file=home&build=build&artifact=generated/index.html');
            browser.history = { pushState: vi.fn((_, __, url) => { browser.location = new URL(url); }), replaceState: vi.fn((_, __, url) => { browser.location = new URL(url); }) };
            const workbench = await createCemStudioNavigation({ repository, validator, workbench: base, browser }); workbenches.push(workbench);
            const root = document.createElement('main'); document.body.append(root);
            const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
            const build = async () => { await workbench.buildProjectPreview(); await view.whenSettled(); };
            await build();
            const preview = () => workbench.snapshot().projectPreview;
            expect(preview().attempt).toMatchObject({ status: 'succeeded' });
            const first = preview().projection.buildRun, resourceSet = preview().preview.resourceSet;
            expect(first.artifacts.map(item => item.path).sort()).toEqual(['generated/assets/logo.svg', 'generated/assets/site.css',
                'generated/docs/guide.html', 'generated/downloads/raw.bin', 'generated/index.html', 'generated/posts/intro.html']);
            expect(new DOMParser().parseFromString(preview().projection.outputs.find(item => item.uri.endsWith('/docs/guide.html')).text, 'text/html').querySelector('h1').textContent).toBe('Guide');
            const binary = preview().projection.outputs.find(item => item.uri.endsWith('/downloads/raw.bin'));
            expect(binary.bytes).toEqual(contents.binary);
            expect(binary.contentType).toBe('application/octet-stream');
            expect(binary.sha256).toBe(await digest(contents.binary));
            expect(preview().projection.outputs.find(item => item.uri.endsWith('/assets/logo.svg')).contentType).toBe('image/svg+xml');
            expect(first.inputs).toEqual(expect.arrayContaining([expect.objectContaining({ resourceId: 'article', revision: 1, sha256: resources.find(item => item.id === 'article').sha256 })]));
            const frame = () => root.querySelector('[part~=region][part~=preview] iframe');
            const ready = async title => { await vi.waitFor(() => expect(frame()?.contentDocument?.querySelector('h1')?.textContent).toBe(title));
                await vi.waitFor(() => expect(frame().contentWindow.getComputedStyle(frame().contentDocument.querySelector('h1')).color).toBe('rgb(19, 38, 57)'));
                await vi.waitFor(() => expect(frame().contentDocument.querySelector('img').naturalWidth).toBe(12)); };
            await ready('Native folder build');
            const editor = root.querySelector('[data-cem-studio-editor] [role=textbox]');
            frame().contentDocument.querySelector('a').click(); await view.whenSettled(); await ready('Nested article');
            expect(new URL(browser.location).searchParams.get('artifact')).toBe('generated/posts/intro.html');
            expect(new URL(browser.location).searchParams.get('file')).toBe('home');
            expect(root.querySelector('[data-cem-studio-editor] [role=textbox]')).toBe(editor);
            await repository.execute(request('save-resource', { projectId: 'folder-site', resourceId: 'graph', expectedProjectRevision: 1,
                expectedResourceRevision: 1, content: graph.replace('@root="../static"', '@root="../missing"') }));
            await build();
            expect(preview().attempt.status).toBe('failed');
            expect(preview().projection.buildRun).toBe(first);
            expect(preview().preview.resourceSet).toBe(resourceSet);
            await ready('Nested article');
            expect(preview().attempt.error.message).toContain('import_root_outside');
        } finally { await validator.close(); }
    }, 60_000);

    it('navigates one immutable output tree without changing the source file and atomically replaces complete runs', async () => {
        const repository = await repositoryWithSource('{main | Saved source}');
        const bundle = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) }); workbenches.push(session);
        const base = await createCemStudioSessionWorkbench({ repository, session });
        const browser = new EventTarget(); browser.location = new URL('https://studio.test/?project=feature-tour&file=source&artifact=index.html');
        browser.history = { pushState: vi.fn((_, __, url) => { browser.location = new URL(url); }), replaceState: vi.fn((_, __, url) => { browser.location = new URL(url); }) };
        let runs = 0, fail = false;
        const workbench = await createCemStudioNavigation({ repository, workbench: base, browser, openBuildWorkbench: async () => {
            const run = ++runs, encode = text => new TextEncoder().encode(text);
            const pages = [ ['index.html', 'text/html', `<link rel="stylesheet" href="assets/site.css"><h1>Run ${run}</h1><a href="posts/intro.html?mode=read#intro">Intro</a><a href="gone.html">Missing</a>`],
                ['posts/intro.html', 'text/html', `<h2 id="intro">Intro ${run}</h2><a href="../index.html">Home</a>`],
                ...(run === 1 ? [['assets/site.css', 'text/css', 'h1 { color:rgb(13,26,39); }']] : []) ];
            const outputs = pages.map(([path, contentType, text]) => ({ uri: `studio://feature-tour/${path}`, contentType, text, bytes: encode(text), available: true }));
            const artifacts = pages.map(([path, contentType]) => ({ id: `${run}:${path}`, path, contentType, uri: `studio://feature-tour/${path}` }));
            let projection = { exitCode: fail ? 1 : 0, outputs, output: outputs[0], preview: { kind: 'sandboxed-html', label: 'Built site', source: outputs[0].text },
                buildRun: { id: `run-${run}`, projectRevision: 1, inputs: [{ resourceId: 'source', revision: 1, sha256: bundle.project.resources.find(item => item.id === 'source').sha256 }], artifacts } };
            return { snapshot: () => ({ projection }), runPersistedOperation: async () => undefined, dispose: vi.fn(), selectArtifact(id) {
                const index = artifacts.findIndex(item => item.id === id); projection = { ...projection, output: outputs[index], preview: { ...projection.preview, source: outputs[index].text } };
            } };
        } }); workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const build = async () => { await workbench.buildProjectPreview(); await view.whenSettled(); };
        const frame = () => root.querySelector('[part~="preview"][part~="region"] iframe');
        const ready = async selector => { await vi.waitFor(() => expect(frame()?.contentDocument?.querySelector(selector)).toBeTruthy()); };
        await build(); await ready('h1');
        const firstSet = workbench.snapshot().projectPreview.preview.resourceSet;
        const source = root.querySelector('[data-cem-studio-editor] [role=textbox]');
        await vi.waitFor(() => expect(frame().contentWindow.getComputedStyle(frame().contentDocument.querySelector('h1')).color).toBe('rgb(13, 26, 39)'));
        frame().contentDocument.querySelector('a').click(); await view.whenSettled(); await ready('h2');
        expect(workbench.snapshot().projectPreview.artifactPath).toBe('posts/intro.html?mode=read#intro');
        expect(new URL(browser.location).searchParams.get('artifact')).toBe('posts/intro.html?mode=read#intro');
        expect(new URL(browser.location).searchParams.get('file')).toBe('source');
        expect(root.querySelector('[data-cem-studio-editor] [role=textbox]')).toBe(source);
        expect(root.querySelector('[label="Preview artifact"]').getAttribute('value')).toBe('posts/intro.html');
        frame().contentDocument.querySelector('a').click(); await view.whenSettled(); await ready('h1');
        fail = true; await build(); await ready('h1');
        expect(workbench.snapshot().projectPreview.preview.resourceSet).toBe(firstSet);
        expect(frame().contentDocument.querySelector('h1').textContent).toBe('Run 1');
        fail = false; await build(); await ready('h1');
        expect(frame().contentDocument.querySelector('h1').textContent).toBe('Run 3');
        expect(workbench.snapshot().projectPreview.preview.resourceSet).not.toBe(firstSet);
        expect(root.querySelector('cem-media-preview [role=status]').textContent).toContain('Missing artifact: assets/site.css');
        frame().contentDocument.querySelectorAll('a')[1].click(); await view.whenSettled();
        expect(workbench.snapshot().projectPreview).toMatchObject({ artifactPath: 'gone.html', missingArtifact: true });
        expect(root.querySelector('[part~="preview"][part~="region"]').textContent).toContain('requested artifact is unavailable');
        expect(new URL(browser.location).searchParams.get('file')).toBe('source');
    });

    it('builds an ordinary single-document sample without a graph and reopens its independent preview URL', async () => {
        const repository = await repositoryWithSource('{main | Saved document}');
        const exported = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const bundle = { ...exported, project: { ...exported.project, entries: [], resources: exported.project.resources.filter(item => item.id === 'source') } };
        const validator = await createCemStudioBrowserValidator();
        try {
            const examples = [{ id: 'document', name: 'Document', load: async () => bundle }];
            const browser = new EventTarget(); browser.location = new URL('https://studio.test/?sample=document&file=source');
            browser.history = { pushState: vi.fn((_, __, url) => { browser.location = new URL(url); }), replaceState: vi.fn((_, __, url) => { browser.location = new URL(url); }) };
            const open = async () => {
                const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) }); workbenches.push(session);
                const base = await createCemStudioSessionWorkbench({ repository, session });
                const workbench = await createCemStudioNavigation({ repository, validator, workbench: base, examples, browser }); workbenches.push(workbench);
                return workbench;
            };
            const workbench = await open();
            const root = document.createElement('main'); document.body.append(root);
            let copied;
            const view = await mountCemStudioFeatureTourWorkbench({ root, workbench, clipboard: { writeText: async text => { copied = text; } } }); views.push(view);
            expect(workbench.snapshot().projectPreview.configurationId).toBe('html-v1:source');
            await workbench.buildProjectPreview(); await view.whenSettled();
            expect(workbench.snapshot().projectPreview.projection.output.text).toContain('<main>Saved document</main>');
            expect(root.querySelector('[part~=region][part~=preview] iframe').srcdoc).toContain('Saved document');
            root.querySelector('[host-command="copy-preview"] button').click(); await view.whenSettled();
            expect(copied).toBe(workbench.snapshot().projectPreview.projection.output.text);
            const url = new URL(browser.location);
            expect(url.searchParams.get('file')).toBe('source');
            expect(url.searchParams.get('build')).toBe('html-v1:source');
            expect(url.searchParams.get('artifact')).toBe('cem-stdio://stdout');
            view.dispose(); workbench.dispose();
            const reopened = await open();
            expect(reopened.snapshot().projectPreview).toMatchObject({ configurationId: 'html-v1:source', artifactPath: 'cem-stdio://stdout', projection: undefined });
            expect(reopened.snapshot().navigation.selection.file).toBe('source');
            expect((await repository.query(request('list-projects', {}))).value).toHaveLength(1);
        } finally { await validator.close(); }
    }, 30_000);

    it('owns the project preview across source navigation, saved builds, URL selection and failed repair', async () => {
        const response = await fetch(new URL('../generated/feature-tour/examples/supplier-pipeline.cem-studio.json', import.meta.url));
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const repository = await repositoryWithSource('unrelated');
        await repository.execute(request('import-project', { bundle }));
        const validator = await createCemStudioBrowserValidator();
        try {
            const examples = [{ id: 'supplier', name: 'Supplier', load: async () => bundle }];
            const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) }); workbenches.push(session);
            const base = await createCemStudioSessionWorkbench({ repository, session });
            const browser = new EventTarget(); browser.location = new URL('https://studio.test/?sample=supplier&folder=templates%2F&build=availability&artifact=generated%2Favailability.json');
            browser.history = { pushState: vi.fn((_, __, url) => { browser.location = new URL(url); }), replaceState: vi.fn((_, __, url) => { browser.location = new URL(url); }) };
            const workbench = await createCemStudioNavigation({ repository, validator, workbench: base, examples, browser }); workbenches.push(workbench);
            const root = document.createElement('main'); document.body.append(root);
            const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
            const pane = root.querySelector('[part~="preview"][part~="region"]');
            const build = async () => { await workbench.buildProjectPreview(); await view.whenSettled(); };
            const preview = () => workbench.snapshot().projectPreview;
            expect(preview()).toMatchObject({ configurationId: 'availability', previewContext: 'none' });
            expect(pane.textContent).toContain('Select a generated output or a source resource to preview.');
            expect(root.querySelector('[aria-label="Folder contents"]').textContent).toContain('view.cemt');
            const projectCount = (await repository.query(request('list-projects', {}))).value.length;
            await build();
            expect(preview().projection.output.contentType).toBe('application/json');
            expect(JSON.parse(preview().projection.output.text)[0].quantity).toBe(7);
            const first = preview().projection;
            expect(preview().preview).toBeUndefined(); // A source directory has no unrelated right preview.
            await workbench.openEntity('sample=supplier&file=supplier-view'); await view.whenSettled();
            expect(preview().projection.buildRun).toBe(first.buildRun);
            expect(preview().projection.outputs).toBe(first.outputs);
            expect(preview().selectedArtifact.provenance.some(frame => frame.uri.endsWith('/templates/view.cemt'))).toBe(true);
            expect(new URL(browser.location).searchParams.get('artifact')).toBe('generated/availability.html');
            expect((await repository.query(request('list-projects', {}))).value).toHaveLength(projectCount);
            await workbench.openEntity(`project=${bundle.project.id}&file=supplier-stock`); await view.whenSettled();
            selectValue(root, '[label="Build configuration"]', 'availability'); await view.whenSettled();
            const editor = root.querySelector('[data-cem-studio-editor] [role="textbox"]');
            const original = workbench.snapshot().draft;
            workbench.updateDraft(original.replace('SKU-A,7', 'SKU-A,0')); await view.whenSettled();
            editor.focus(); editor.setSelectionRange(2, 4);
            await build();
            expect(workbench.snapshot()).toMatchObject({ dirty: true, resourceId: 'supplier-stock' });
            expect(root.querySelector('[data-cem-studio-editor] [role="textbox"]')).toBe(editor);
            expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 4]);
            expect(preview().stale).toBe(true);
            expect(preview().projection.output.text).toContain('Supplier availability');
            await workbench.saveSource(); await view.whenSettled(); await build();
            const successful = preview().projection;
            expect(preview().stale).toBe(false);
            workbench.selectPreviewArtifact('generated/availability.json'); await view.whenSettled();
            expect(JSON.parse(preview().projection.output.text)[0].quantity).toBe(0);
            workbench.updateDraft(original.replace('SKU-A,7', 'SKU-A,many')); await workbench.saveSource(); await view.whenSettled(); await build();
            expect(preview().attempt.status).toBe('failed');
            expect(preview().projection.buildRun).toBe(successful.buildRun);
            expect(JSON.parse(preview().projection.output.text)[0].quantity).toBe(0);
            expect(pane.textContent).toContain('Showing output from the last successful saved build');
            expect(root.querySelector('cem-studio-workbench-preview[section=build] [aria-label="Build diagnostics"]').textContent).toContain('invalid data number');
            workbench.updateDraft(original); await workbench.saveSource(); await view.whenSettled(); await build();
            expect(JSON.parse(preview().projection.output.text)[0].quantity).toBe(7);
            await workbench.openEntity(`project=${bundle.project.id}&folder=templates%2F`); await view.whenSettled();
            expect(root.querySelector('[aria-label="Folder contents"]').textContent).toContain('view.cemt');
            expect(preview().projection.output.contentType).toBe('application/json');
            browser.location = new URL(`https://studio.test/?project=${bundle.project.id}&file=supplier-stock&build=availability&artifact=missing.html`);
            browser.dispatchEvent(new Event('popstate'));
            await vi.waitFor(() => expect(preview().missingArtifact).toBe(true)); await view.whenSettled();
            expect(pane.querySelector('cem-media-preview')).toBeNull();
            expect(pane.textContent).toContain('requested artifact is unavailable');
            expect(workbench.snapshot().navigation.selection.file).toBe('supplier-stock');
        } finally { await validator.close(); }
    }, 60_000);

    it('cancels and supersedes build attempts without replacing the last admitted success', async () => {
        const repository = await repositoryWithSource('{main | saved}');
        const bundle = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const wait = [], candidates = [];
        const service = createCemStudioProjectPreview({ repository, openBuildWorkbench: async () => {
            const pending = deferred(); wait.push(pending);
            const id = `run-${wait.length}`, artifact = { id, path: 'index.html', uri: 'studio://feature-tour/index.html', contentType: 'text/html' };
            const candidate = { snapshot: () => ({ projection: { exitCode: 0, stale: false,
                buildRun: { id, inputs: [{ resourceId: 'source', revision: 1, sha256: bundle.project.resources.find(item => item.id === 'source').sha256 }], artifacts: [artifact] },
                output: { uri: artifact.uri }, outputs: [{ uri: artifact.uri, bytes: new TextEncoder().encode('<p>Run</p>') }] } }), runPersistedOperation: () => pending.promise, selectArtifact: vi.fn(), dispose: vi.fn() };
            candidates.push(candidate); return candidate;
        } }); workbenches.push(service);
        const context = { key: 'project=feature-tour', project: bundle.project };
        service.setContext(context);
        const first = service.build(); await vi.waitFor(() => expect(wait).toHaveLength(1)); wait[0].resolve(); await first;
        expect(service.snapshot().projection.buildRun.id).toBe('run-1');
        const late = service.build(); await vi.waitFor(() => expect(wait).toHaveLength(2)); service.cancel(); await late;
        expect(service.snapshot()).toMatchObject({ busy: false, attempt: { status: 'cancelled' } });
        const latest = service.build(); await vi.waitFor(() => expect(wait).toHaveLength(3)); wait[2].resolve(); await latest;
        wait[1].resolve(); await vi.waitFor(() => expect(candidates[1].dispose).toHaveBeenCalled());
        expect(service.snapshot().projection.buildRun.id).toBe('run-3');
        const stale = service.build(); await vi.waitFor(() => expect(wait).toHaveLength(4));
        await repository.execute(request('save-resource', { projectId: 'feature-tour', resourceId: 'source', expectedProjectRevision: 1, expectedResourceRevision: 1, content: '{main | newer}' }));
        wait[3].resolve(); await stale;
        expect(service.snapshot()).toMatchObject({ stale: true, attempt: { status: 'stale' }, projection: { buildRun: { id: 'run-3' } } });
        service.setContext({ ...context, key: 'sample=other' });
        expect(service.snapshot().projection).toBeUndefined();
        expect(candidates[2].dispose).toHaveBeenCalled();
    });

    it('cancels a pending build from the declarative toolbar while source navigation remains available', async () => {
        const repository = await repositoryWithSource('{main | saved}');
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) }); workbenches.push(session);
        const base = await createCemStudioSessionWorkbench({ repository, session });
        const browser = new EventTarget(); browser.location = new URL('https://studio.test/?project=feature-tour&file=source');
        browser.history = { pushState: vi.fn((_, __, url) => { browser.location = new URL(url); }), replaceState: vi.fn((_, __, url) => { browser.location = new URL(url); }) };
        const pending = deferred(), dispose = vi.fn(), run = vi.fn(() => pending.promise);
        const workbench = await createCemStudioNavigation({ repository, workbench: base, browser,
            openBuildWorkbench: async () => ({ snapshot: () => ({}), runPersistedOperation: run, dispose }) }); workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        await workbench.openEntity('project=feature-tour'); await view.whenSettled();
        root.querySelector('[host-command="build-project"] button').click();
        await vi.waitFor(() => expect(run).toHaveBeenCalledTimes(1));
        await vi.waitFor(() => expect(root.querySelector('[host-command="cancel-build"] button').disabled).toBe(false));
        root.querySelector('cem-tree[label="Project files"] [data-tree-value="project=feature-tour&file=source"] [part~=label]').click();
        await vi.waitFor(() => expect(workbench.snapshot().navigation.content).toBe('file'));
        await vi.waitFor(() => expect(root.querySelector('section[aria-label="Selected file"]').hidden).toBe(false));
        await vi.waitFor(() => expect(root.querySelector('[data-cem-studio-editor] [role="textbox"]')).not.toBeNull());
        // Complete the first navigation's public busy/count projection before
        // returning to the project; the separate build is still pending.
        const actions = root.querySelector('cem-studio-workbench-navigation[section="actions"]');
        await vi.waitFor(() => expect(actions.querySelector('[part~=count]').textContent.trim()).toBe('1 file'));
        await vi.waitFor(() => expect(actions.querySelector('[aria-label="Clone selected"] button').disabled).toBe(false));
        await vi.waitFor(() => expect(root.querySelector('cem-tree[label="Project files"] [data-tree-value="project=feature-tour&file=source"]').getAttribute('aria-selected')).toBe('true'));
        root.querySelector('cem-tree[label="Project files"] [data-tree-value="project=feature-tour"] [part~=label]').click();
        await vi.waitFor(() => expect(root.querySelector('section[aria-label="Project build"]').hidden).toBe(false));
        root.querySelector('[host-command="cancel-build"] button').click(); await view.whenSettled();
        expect(workbench.snapshot().projectPreview).toMatchObject({ busy: false, attempt: { status: 'cancelled' }, projection: undefined });
        expect(root.querySelector('[host-command="build-project"] button').disabled).toBe(false);
        pending.resolve(); await vi.waitFor(() => expect(dispose).toHaveBeenCalled());
        expect(workbench.snapshot().projectPreview.attempt.status).toBe('cancelled');
    });

    it('captures saved build inputs and selects exact native Supplier artifacts through declarative controls', async () => {
        const response = await fetch(new URL('../generated/feature-tour/examples/supplier-pipeline.cem-studio.json', import.meta.url));
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const repository = await repositoryWithSource('unrelated');
        await repository.execute(request('import-project', { bundle }));
        const validator = await createCemStudioBrowserValidator();
        try {
            const workbench = await createCemStudioProjectWorkbench({ repository, validator,
                selection: { projectId: bundle.project.id, entryId: 'availability' } });
            workbenches.push(workbench);
            let copied, downloaded;
            const root = document.createElement('main'); document.body.append(root);
            const view = await mountCemStudioFeatureTourWorkbench({ root, workbench,
                clipboard: { writeText: async text => { copied = text; } }, download: async file => { downloaded = file; } }); views.push(view);
            await workbench.runPersistedOperation(); await view.whenSettled();
            const projection = workbench.snapshot().projection;
            const run = projection.buildRun;
            expect(run).toMatchObject({ id: projection.requestId, projectId: bundle.project.id, projectRevision: 1,
                configuration: { id: 'availability', kind: 'transformation-graph', resourceId: 'command-availability', revision: 1 },
                executionIdentity: { resolverPolicyStamp: 'browser-explicit-v1' }, status: 'succeeded' });
            expect(run.inputs).toEqual(expect.arrayContaining([expect.objectContaining({ resourceId: 'supplier-stock', revision: 1,
                sha256: bundle.project.resources.find(resource => resource.id === 'supplier-stock').sha256 })]));
            expect(run.artifacts.map(artifact => artifact.path).sort()).toEqual([
                'generated/availability.html', 'generated/availability.json', 'generated/availability.yaml',
            ]);
            const sourceId = workbench.snapshot().resourceId;
            for (const artifact of run.artifacts) {
                selectValue(root, '[label="Output artifact"]', artifact.id); await view.whenSettled();
                const selected = workbench.snapshot().projection;
                expect(selected.selectedArtifactId).toBe(artifact.id);
                expect(selected.output.uri).toBe(artifact.uri);
                expect(selected.output.sha256).toBe(artifact.sha256);
                const metadata = root.querySelector('[data-cem-studio-projection-metadata]');
                expect(metadata.textContent).toContain(artifact.uri);
                expect(metadata.textContent).toContain(artifact.sha256);
                expect(artifact.available).toBe(true);
                expect(artifact.handleId).toBeDefined();
                if (artifact.path.endsWith('.html')) {
                    expect(artifact.provenance).toEqual(expect.arrayContaining([expect.objectContaining({
                        uri: `${bundle.project.rootUri}templates/view.cemt`, outputUri: artifact.uri,
                    })]));
                }
                // Native data formatters currently publish no character-level
                // lineage for these constructed records; never invent it here.
                if (artifact.path.endsWith('.json')) expect(JSON.parse(selected.output.text))
                    .toEqual(JSON.parse(new TextDecoder().decode(bundle.contents['supplier-expected-json'])));
                root.querySelector('[data-cem-studio-projection-copy] button').click(); await view.whenSettled();
                root.querySelector('[data-cem-studio-projection-download] button').click(); await view.whenSettled();
                expect(copied).toBe(selected.output.text);
                expect(downloaded.bytes).toEqual(new Uint8Array(selected.output.bytes));
                expect(downloaded.filename).toBe(artifact.path.split('/').pop());
                const digest = await crypto.subtle.digest('SHA-256', downloaded.bytes);
                expect([...new Uint8Array(digest)].map(byte => byte.toString(16).padStart(2, '0')).join('')).toBe(artifact.sha256);
                expect(workbench.snapshot().resourceId).toBe(sourceId);
                expect(workbench.snapshot().projection.buildRun).toBe(run);
            }
            expect(() => workbench.selectArtifact('missing')).toThrow(/unavailable/);
            workbench.updateDraft('unsaved graph');
            expect(workbench.snapshot().projection.stale).toBe(true);
            expect(workbench.snapshot().projection.buildRun).toBe(run);
            await repository.execute(request('save-resource', { projectId: bundle.project.id, resourceId: 'supplier-stock',
                expectedProjectRevision: 1, expectedResourceRevision: 1,
                content: new TextDecoder().decode(bundle.contents['supplier-stock']).replace('SKU-A,7', 'SKU-A,many') }));
            await workbench.runPersistedOperation(); await view.whenSettled();
            const failure = workbench.snapshot().projection;
            expect(failure).toMatchObject({ exitCode: 1, output: { available: false }, buildRun: { status: 'failed', artifacts: [] } });
            expect(failure.diagnostics.length).toBeGreaterThan(0);
            expect(root.querySelector('[data-cem-studio-projection-copy]').hasAttribute('disabled')).toBe(true);
        } finally { await validator.close(); }
    }, 60_000);

    it('downloads binary artifacts exactly and does not attach a late transfer to a different selection', async () => {
        const repository = await repositoryWithSource('source');
        const validator = validatorFor(validResult());
        const outcome = portableOperationOutcome(['convert']);
        const binary = { uri: 'studio://feature-tour/generated/asset.bin', available: true, contentType: 'application/octet-stream',
            bytes: new Uint8Array([0, 255, 128, 1]), byteLength: 4, sha256: 'binary-fixture', text: '\u0000��\u0001' };
        outcome.outputs = [outcome.output, binary];
        validator.runResourceCommand.mockResolvedValueOnce(outcome);
        const workbench = await createWorkbench(repository, validator);
        await workbench.selectWorkbench('conversion'); await workbench.runPersistedOperation();
        const run = workbench.snapshot().projection.buildRun;
        const pending = deferred();
        const copying = workbench.copyProjection(() => pending.promise);
        workbench.selectArtifact(run.artifacts[1].id);
        pending.resolve(); await copying;
        expect(workbench.snapshot().projection.transfer).toBeUndefined();
        await expect(workbench.copyProjection(vi.fn())).rejects.toThrow(/downloaded as exact bytes/);
        const save = vi.fn(async () => undefined);
        await workbench.downloadProjection(save);
        expect(save).toHaveBeenCalledWith(expect.objectContaining({ filename: 'asset.bin', bytes: binary.bytes }));
        expect(workbench.snapshot().projection.preview.kind).toBe('download');
        expect(workbench.snapshot().projection.buildRun).toBe(run);
    });

    it('admits a one-document HTML build without an authored transformation graph', async () => {
        const repository = await repositoryWithSource('{main | One document}');
        const validator = await createCemStudioBrowserValidator();
        try {
            const seed = featureTourSeed();
            seed.catalog.workbenches = [{ ...seed.catalog.workbenches[0], id: 'html', name: 'HTML build', kind: 'conversion', operation: 'convert',
                commandArguments: ['convert', '$input', '--content-type', 'application/cem', '--schema', 'https://cem.dev/ns/cem-ml/1', '--to-content-type', 'text/html', '--to-schema', 'https://cem.dev/ns/data/html/1'], expectedSummary: undefined }];
            const workbench = await createCemStudioFeatureTourWorkbench({ repository, validator, seed, projectId: 'feature-tour' }); workbenches.push(workbench);
            await workbench.selectWorkbench('html');
            await workbench.runPersistedOperation();
            const { projection } = workbench.snapshot();
            expect(projection.buildRun).toMatchObject({ configuration: { id: 'html' }, status: 'succeeded',
                artifacts: [expect.objectContaining({ contentType: 'text/html', available: true })] });
            expect(projection.output.text).toContain('<main>One document</main>');
            expect(projection.preview.kind).toBe('sandboxed-html');
            expect(projection.selectedArtifactId).toBe(projection.buildRun.artifacts[0].id);
            let copied, downloaded;
            await workbench.copyProjection(async text => { copied = text; });
            await workbench.downloadProjection(async file => { downloaded = file; });
            expect(copied).toBe(projection.output.text);
            expect(downloaded.filename).toMatch(/\.html$/);
            expect(downloaded.bytes).toEqual(new Uint8Array(projection.output.bytes));
        } finally { await validator.close(); }
    }, 60_000);

    it('selects the innermost retained diagnostic span instead of the document frame', async () => {
        const source = '{main | {form @action=session}}';
        const outcome = invalidResult(0, source.length);
        outcome.result.diagnostics.items[0].sourceMap.frames.push({ source_id: 7,
            span: { kind: 'Single', ranges: { start: source.indexOf('{form'), len: 5 } }, transform: { kind: 'CemAstBuilder' } });
        const repository = await repositoryWithSource(source);
        const workbench = await createWorkbench(repository, validatorFor(outcome));
        await workbench.validatePersisted(); workbench.navigateDiagnostic(0);
        const selection = workbench.snapshot().selection;
        expect(source.slice(selection.start, selection.end)).toBe('{form');
        expect(selection.byteLength).toBe(5);
        expect(workbench.snapshot().validation.diagnostics[0].sourceMap.frames).toHaveLength(2);
    });

    it('retains the mounted workspace through shell theme and status updates', async () => {
        const repository = await repositoryWithSource('original source');
        const workbench = await createWorkbench(repository, validatorFor(validResult()));
        await workbench.selectWorkbench(featureTourSeed().catalog.workbenches[0].id);
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        await shell.whenSettled(); await view.whenSettled();
        const workspace = root.querySelector('cem-workspace');
        const editor = root.querySelector('[data-cem-studio-editor] [role="textbox"]');
        workbench.updateDraft('retained unsaved source'); await view.whenSettled();
        editor.focus(); editor.setSelectionRange(2, 8);
        for (const mode of ['cem-theme-dark', 'cem-theme-contrast-light']) {
            shell.theme.setMode(mode); shell.view.update({ storageState: mode });
            await shell.whenSettled(); await view.whenSettled();
            expect(view.root.isConnected).toBe(true);
            expect(root.querySelector('cem-workspace')).toBe(workspace);
            expect(root.querySelector('[data-cem-studio-workspace-mount]').contains(workspace)).toBe(true);
            expect(root.querySelector('[data-cem-studio-editor] [role="textbox"]')).toBe(editor);
            expect(editor.value).toBe('retained unsaved source');
            expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 8]);
            expect(document.activeElement).toBe(editor);
        }
    });

    it('stages native graph attributes with stale guards and bounded reversible drafts', async () => {
        const { workbench, repository, validator } = await graphWorkbench();
        const original = workbench.snapshot().draft;
        const fields = (attribute, value) => ({ node: ['result'], attribute: [attribute], value: [value], mode: ['set'], version: [workbench.snapshot().graphConfiguration.version] });
        const first = fields('out', 'café output.svg');
        await workbench.updateGraphAttribute(first);
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        const changed = workbench.snapshot().draft;
        expect(changed).not.toBe(original);
        expect(workbench.snapshot()).toMatchObject({ dirty: true, projectRevision: 1, graphConfiguration: { canUndo: true, canRedo: false } });
        expect(validator.runResourceCommand).not.toHaveBeenCalled();
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value.project.revision).toBe(1);
        await workbench.updateGraphAttribute(first);
        expect(workbench.snapshot().graphConfiguration.error.message).toContain('graph changed');
        expect(workbench.snapshot().draft).toBe(changed);
        await workbench.updateGraphAttribute(fields('style-policy', 'invented'));
        expect(workbench.snapshot().draft).toBe(changed);
        await workbench.updateGraphAttribute({ ...fields('out', 'ignored'), extra: ['no'] });
        expect(workbench.snapshot().graphConfiguration.error.message).toContain('Unexpected');
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).toBe(original);
        workbench.redoGraphDraft(); expect(workbench.snapshot().draft).toBe(changed);
        await workbench.saveSource();
        expect(workbench.snapshot()).toMatchObject({ dirty: false, projectRevision: 2, resourceRevision: 2 });
        workbench.undoGraphDraft(); expect(workbench.snapshot()).toMatchObject({ dirty: true, draft: original });
        workbench.redoGraphDraft(); expect(workbench.snapshot().dirty).toBe(false);
        for (let index = 0; index < 24; index++) await workbench.updateGraphAttribute(fields('out', `output-${index}.svg`));
        let undoCount = 0;
        while (workbench.snapshot().graphConfiguration.canUndo) { workbench.undoGraphDraft(); undoCount++; }
        expect(undoCount).toBe(20);
        await workbench.reload();
        expect(workbench.snapshot().draft).toBe(changed);
        expect(workbench.snapshot().graphConfiguration).toMatchObject({ canUndo: false, canRedo: false });
    });

    it('retries a transient package-publication refusal without changing the graph draft', async () => {
        const { workbench, validator } = await graphWorkbench();
        const original = validator.authorGraph;
        validator.authorGraph = vi.fn().mockImplementationOnce(() => { throw Object.assign(new Error('cem.schema_asset.busy'), { code: 'cem.schema_asset.busy' }); })
            .mockImplementation(original);
        // Changing the source invalidates the cached authoring result. A later
        // state publication must retry a refused preparation for the same draft.
        workbench.updateDraft(`${workbench.snapshot().draft}\n`);
        const source = workbench.snapshot().draft;
        expect(workbench.snapshot().graphConfiguration.error.code).toBe('cem.schema_asset.busy');
        await workbench.validatePersisted();
        expect(workbench.snapshot().draft).toBe(source);
        expect(workbench.snapshot().graphConfiguration.nodes.length).toBeGreaterThan(0);
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        expect(validator.authorGraph).toHaveBeenCalledTimes(2);
    });

    it('stages native graph parameters with literal empty values, revision guards and bounded history', async () => {
        const { workbench, repository, validator } = await graphWorkbench(parameterGraphSource);
        const original = workbench.snapshot().draft;
        const fields = (name, value, mode = 'set') => ({ node: ['view'], name: [name], value: [value], mode: [mode], version: [workbench.snapshot().graphConfiguration.version] });
        const parameters = () => workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'view').parameters;
        const first = fields('title', 'Café {stem}');
        await workbench.updateGraphParameter(first);
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        const added = workbench.snapshot().draft;
        expect(parameters()).toContainEqual({ name: 'title', value: 'Café {stem}', editable: true });
        expect(workbench.snapshot()).toMatchObject({ dirty: true, projectRevision: 1, resourceRevision: 1 });
        expect(validator.runResourceCommand).not.toHaveBeenCalled();
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value.project.revision).toBe(1);
        await workbench.updateGraphParameter(first);
        expect(workbench.snapshot().graphConfiguration.error.message).toContain('graph changed');
        expect(workbench.snapshot().draft).toBe(added);
        const missingValue = fields('title', 'invalid'); delete missingValue.value;
        for (const invalid of [
            missingValue,
            { ...fields('title', 'invalid'), value: ['one', 'two'] },
            { ...fields('title', 'invalid'), extra: ['unexpected'] },
            { ...fields('title', 'invalid'), name: [] },
            { ...fields('title', 'invalid'), mode: ['reset'] },
            { ...fields('title', 'invalid'), node: ['result'] },
            { ...fields('title', 'invalid'), version: ['foreign-workbench'] },
            { ...fields('', 'invalid') },
        ]) {
            await workbench.updateGraphParameter(invalid);
            expect(workbench.snapshot().graphConfiguration.error).toBeDefined();
            expect(workbench.snapshot().draft).toBe(added);
        }
        await workbench.updateGraphParameter(fields('title', ''));
        expect(parameters()).toContainEqual({ name: 'title', value: '', editable: true });
        const empty = workbench.snapshot().draft;
        const version = workbench.snapshot().graphConfiguration.version;
        await workbench.updateGraphParameter(fields('title', ''));
        expect(workbench.snapshot().draft).toBe(empty);
        expect(workbench.snapshot().graphConfiguration.version).toBe(version);
        await workbench.updateGraphParameter(fields('title', '', 'remove'));
        expect(parameters().some(parameter => parameter.name === 'title')).toBe(false);
        expect(parameters()).toContainEqual({ name: 'locale', value: 'en-US', editable: true });
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).toBe(empty);
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).toBe(added);
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).toBe(original);
        expect(workbench.snapshot().graphConfiguration.canUndo).toBe(false);
        workbench.redoGraphDraft(); expect(workbench.snapshot().draft).toBe(added);
        await workbench.saveSource(); await workbench.reload();
        expect(workbench.snapshot()).toMatchObject({ draft: added, dirty: false, projectRevision: 2, resourceRevision: 2 });
        for (let index = 0; index < 24; index++) await workbench.updateGraphParameter(fields('title', `Title ${index}`));
        let undoCount = 0;
        while (workbench.snapshot().graphConfiguration.canUndo) { workbench.undoGraphDraft(); undoCount++; }
        expect(undoCount).toBe(20);
        await workbench.reload();
        expect(workbench.snapshot()).toMatchObject({ draft: added, graphConfiguration: { canUndo: false, canRedo: false } });
    });

    it('composes literal graph parameter selection, native reset and empty-value removal declaratively', async () => {
        const { workbench } = await graphWorkbench(parameterGraphSource);
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view); await view.whenSettled();
        const form = () => root.querySelector('form[host-command="edit-graph-parameter"]');
        const choose = async (name, value) => { selectValue(name === 'mode' ? form() : root, `[name="${name}"]`, value); await view.whenSettled(); };
        const action = name => [...root.querySelectorAll('button')].find(button => button.textContent.trim() === name);
        await choose('graph-parameter-node', 'view'); await choose('graph-parameter', 'locale');
        const name = () => form().querySelector('cem-text-field[name=name] input');
        const value = () => form().querySelector('cem-textarea[name=value] textarea');
        const set = (input, text) => { input.value = text; input.dispatchEvent(new Event('input', { bubbles: true })); };
        expect(root.querySelectorAll('form[host-command="edit-graph-parameter"]')).toHaveLength(1);
        expect(root.querySelector('cem-table[label="Graph parameter map"]').getAttribute('data-cem-density')).toBe('super-compact');
        expect(name().value).toBe('locale'); expect(name().required).toBe(true); expect(value().value).toBe('en-US');
        set(name(), 'staged'); set(value(), 'staged text');
        action('Reset staged parameter change').click(); await view.whenSettled();
        expect(name().value).toBe('locale'); expect(value().value).toBe('en-US');
        expect(workbench.snapshot().dirty).toBe(false);
        set(value(), 'Français\nSeconde ligne'); action('Apply graph parameter').click(); await view.whenSettled();
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        expect(value().value).toBe('Français\nSeconde ligne');
        expect(workbench.snapshot().projectRevision).toBe(1);
        await choose('graph-parameter', '');
        expect(name().value).toBe(''); expect(value().value).toBe('');
        set(name(), 'note'); action('Apply graph parameter').click(); await view.whenSettled();
        expect(workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'view').parameters).toContainEqual({ name: 'note', value: '', editable: true });
        await choose('graph-parameter', 'note'); await choose('mode', 'remove');
        action('Apply graph parameter').click(); await view.whenSettled();
        expect(workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'view').parameters.some(parameter => parameter.name === 'note')).toBe(false);
        expect(form()).not.toBeNull(); expect(name().value).toBe(''); expect(value().value).toBe('');
        action('Undo graph edit').click(); await view.whenSettled();
        expect(name().value).toBe('note'); expect(value().value).toBe('');
        action('Redo graph edit').click(); await view.whenSettled();
        expect(name().value).toBe('');
        await choose('graph-parameter-node', ''); expect(form()).toBeNull();
    }, 30_000);

    it('discards unsubmitted graph parameter staging when equal-valued entries or owners change', async () => {
        const source = '{run | {import @id=data @src=data.xml | {transform @id=view @src=view.cemt | {param @name=locale @value=en-US} {param @name=duplicate @value=en-US} {export @id=result @out=result.html}} {transform @id=second @src=view.cemt | {param @name=locale @value=en-US} {export @id=second-result @out=second.html}}}}';
        const { workbench } = await graphWorkbench(source);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view); await view.whenSettled();
        const form = () => root.querySelector('form[host-command="edit-graph-parameter"]');
        const choose = async (name, value) => { selectValue(name === 'mode' ? form() : root, `[name="${name}"]`, value); await view.whenSettled(); };
        const name = () => form().querySelector('cem-text-field[name=name] input');
        const value = () => form().querySelector('cem-textarea[name=value] textarea');
        const mode = () => form().querySelector('cem-select[name=mode]');
        const set = (input, text) => { input.value = text; input.dispatchEvent(new Event('input', { bubbles: true })); };
        await choose('graph-parameter-node', 'view'); await choose('graph-parameter', 'locale');
        set(value(), 'Unsubmitted edit'); await choose('mode', 'remove');
        await choose('graph-parameter', 'duplicate');
        expect(name().value, JSON.stringify({ selected: root.querySelector('cem-select[name="graph-parameter"]').value, nameDefault: form().querySelector('cem-text-field[name=name]').getAttribute('value'), valueDefault: form().querySelector('cem-textarea[name=value]').getAttribute('value'), request: form().querySelector('cem-text-field[name=name]').getAttribute('value-request'), parameters: workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'view').parameters })).toBe('duplicate');
        expect(value().value).toBe('en-US'); expect(mode().value).toBe('set');
        await choose('graph-parameter', ''); set(name(), 'title'); set(value(), 'Staged for view');
        await choose('mode', 'remove');
        await choose('graph-parameter-node', 'second');
        expect(name().value).toBe(''); expect(value().value).toBe(''); expect(mode().value).toBe('set');
        await choose('graph-parameter-node', 'view');
        expect(name().value).toBe(''); expect(value().value).toBe(''); expect(mode().value).toBe('set');
        expect(workbench.snapshot()).toMatchObject({ draft: source, dirty: false, projectRevision: 1, graphConfiguration: { canUndo: false } });
    }, 30_000);

    it('disables graph parameter forms and rejects parameter changes during validation', async () => {
        const { workbench, validator } = await graphWorkbench(parameterGraphSource);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view); await view.whenSettled();
        selectValue(root, '[name="graph-parameter-node"]', 'view'); await view.whenSettled();
        const pending = deferred(); validator.validateResource.mockImplementationOnce(() => pending.promise);
        const saving = workbench.saveAndValidate();
        await vi.waitFor(() => expect(workbench.snapshot().status).toBe('validating')); await view.whenSettled();
        try {
            expect(root.querySelector('cem-fieldset[label="Graph parameters"] fieldset').disabled).toBe(true);
            expect(root.querySelector('cem-select[name="graph-parameter-node"]').hasAttribute('disabled')).toBe(true);
            const draft = workbench.snapshot().draft;
            await workbench.updateGraphParameter({ node: ['view'], name: ['locale'], value: ['blocked'], mode: ['set'], version: [workbench.snapshot().graphConfiguration.version] });
            expect(workbench.snapshot().draft).toBe(draft);
            expect(workbench.snapshot().graphConfiguration.error.message).toContain('graph changed');
        } finally { pending.resolve(validResult()); await saving; await view.whenSettled(); }
        expect(root.querySelector('cem-fieldset[label="Graph parameters"] fieldset').disabled).toBe(false);
    }, 30_000);

    it('creates native named graph nodes with revision guards and reversible unsaved drafts', async () => {
        const { workbench, repository, validator } = await graphWorkbench();
        const original = workbench.snapshot().draft;
        const fields = () => ({ parent: ['data'], kind: ['export'], id: ['copy'], 'attribute.out': ['café copy.svg'], version: [workbench.snapshot().graphConfiguration.version] });
        const first = fields();
        await workbench.createGraphNode(first);
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        expect(workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'copy')).toMatchObject({ kind: 'export' });
        const draft = workbench.snapshot().draft;
        expect(workbench.snapshot()).toMatchObject({ dirty: true, projectRevision: 1, resourceRevision: 1 });
        expect(validator.runResourceCommand).not.toHaveBeenCalled();
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value.project.revision).toBe(1);
        await workbench.createGraphNode(first);
        expect(workbench.snapshot().graphConfiguration.error.message).toContain('graph changed');
        for (const invalid of [
            { ...fields(), id: ['duplicate', 'again'] }, { ...fields(), 'attribute.id': ['renamed'] },
            { ...fields(), parent: ['result'] }, { ...fields(), id: ['result'] }, { ...fields(), 'attribute.out': [''] },
        ]) {
            await workbench.createGraphNode(invalid);
            expect(workbench.snapshot().graphConfiguration.error).toBeDefined();
            expect(workbench.snapshot().draft).toBe(draft);
        }
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).toBe(original);
        workbench.redoGraphDraft(); expect(workbench.snapshot().draft).toBe(draft);
        await workbench.saveSource(); await workbench.reload();
        expect(workbench.snapshot()).toMatchObject({ dirty: false, draft, projectRevision: 2 });
    });

    it('composes schema-backed graph creation forms with native reset and pending disabling', async () => {
        const { workbench, validator } = await graphWorkbench();
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        await view.whenSettled();
        const choose = async (name, value) => {
            const select = root.querySelector(`cem-select[name="${name}"]`);
            expect(select).not.toBeNull(); select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); await view.whenSettled();
        };
        const action = name => [...root.querySelectorAll('button')].find(button => button.textContent.trim() === name);
        await choose('graph-create-parent', 'data'); await choose('graph-create-kind', 'export');
        let form = root.querySelector('form[host-command="create-graph-node"]');
        expect(root.querySelectorAll('form[host-command="create-graph-node"]')).toHaveLength(1);
        const set = (name, value) => {
            const input = form.querySelector(`cem-text-field[name="${name}"] input`);
            input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); return input;
        };
        set('id', 'copy'); const output = set('attribute.out', 'copy.svg');
        expect(output.required).toBe(true);
        action('Reset new graph node').click(); await view.whenSettled();
        expect(output.value).toBe(''); expect(workbench.snapshot().dirty).toBe(false);
        set('id', 'copy'); set('attribute.out', 'copy.svg');
        action('Create graph node').click(); await view.whenSettled();
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        expect(workbench.snapshot().graphConfiguration.nodes.some(node => node.id === 'copy')).toBe(true);
        expect(workbench.snapshot().projectRevision).toBe(1);
        action('Undo graph edit').click(); await view.whenSettled();
        expect(workbench.snapshot().dirty).toBe(false);
        action('Redo graph edit').click(); await view.whenSettled();
        expect(workbench.snapshot().dirty).toBe(true);
        const pending = deferred(); validator.validateResource.mockImplementationOnce(() => pending.promise);
        const saving = workbench.saveAndValidate();
        await vi.waitFor(() => expect(workbench.snapshot().status).toBe('validating')); await view.whenSettled();
        form = root.querySelector('form[host-command="create-graph-node"]');
        expect(form.querySelector('fieldset').disabled).toBe(true);
        const saved = workbench.snapshot().draft;
        await workbench.createGraphNode({ parent: ['data'], kind: ['export'], id: ['blocked'], 'attribute.out': ['blocked.svg'], version: [workbench.snapshot().graphConfiguration.version] });
        expect(workbench.snapshot().draft).toBe(saved);
        pending.resolve(validResult()); await saving; await view.whenSettled();
        expect(root.querySelector('form[host-command="create-graph-node"] fieldset').disabled).toBe(false);
        await choose('graph-create-parent', '');
        expect(root.querySelector('form[host-command="create-graph-node"]')).toBeNull();
        await choose('graph-create-kind', 'import');
        expect(root.querySelector('form[host-command="create-graph-node"] cem-text-field[name="attribute.src"]')).not.toBeNull();
        await choose('graph-create-parent', 'data'); await choose('graph-create-kind', 'join');
        expect(root.querySelector('form[host-command="create-graph-node"] cem-select[name="attribute.mode"]')).not.toBeNull();
    });

    it('stages native graph edges with cycle, foreign, repeated and stale guards', async () => {
        const { workbench, repository, validator } = await graphWorkbench(edgeGraphSource);
        const original = workbench.snapshot().draft;
        const fields = (role, source, name) => ({ node: ['view'], role: [role], source: [source], ...(name === undefined ? {} : { name: [name] }), version: [workbench.snapshot().graphConfiguration.version] });
        const first = fields('input', 'other');
        await workbench.updateGraphEdge(first);
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        const input = workbench.snapshot().draft;
        expect(workbench.snapshot()).toMatchObject({ dirty: true, projectRevision: 1 });
        await workbench.updateGraphEdge(first);
        expect(workbench.snapshot().graphConfiguration.error.message).toContain('graph changed');
        for (const invalid of [fields('input', 'result'), { ...fields('input', 'other'), version: ['foreign'] }, { ...fields('input', 'other'), source: ['other', 'data'] }, { ...fields('input', 'other'), extra: ['no'] }]) {
            await workbench.updateGraphEdge(invalid); expect(workbench.snapshot().draft).toBe(input);
            expect(workbench.snapshot().graphConfiguration.error).toBeDefined();
        }
        await workbench.updateGraphEdge(fields('input', ''));
        expect(workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'view').edges.find(field => field.id === 'input').source).toBeNull();
        await workbench.updateGraphEdge(fields('with', 'other', 'palette'));
        expect(workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'view').edges.find(field => field.id === 'with').bindings).toEqual([{ name: 'palette', source: 'other' }]);
        await workbench.updateGraphEdge(fields('parent', 'other'));
        const changed = workbench.snapshot().draft;
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).not.toBe(changed);
        workbench.redoGraphDraft(); expect(workbench.snapshot().draft).toBe(changed);
        expect(validator.runResourceCommand).not.toHaveBeenCalled();
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value.project.revision).toBe(1);
        await workbench.saveSource(); await workbench.reload();
        expect(workbench.snapshot()).toMatchObject({ dirty: false, draft: changed, projectRevision: 2 });
        expect(changed).not.toBe(original);
    });

    it('composes graph edge forms declaratively with native reset, bindings and pending disabling', async () => {
        const { workbench, validator } = await graphWorkbench(edgeGraphSource);
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view); await view.whenSettled();
        const choose = async (name, value) => { const host = root.querySelector(`cem-select[name="${name}"]`); expect(host).not.toBeNull(); host.value = value; host.dispatchEvent(new Event('change', { bubbles: true })); await view.whenSettled(); };
        const action = name => [...root.querySelectorAll('button')].find(button => button.textContent.trim() === name);
        await choose('graph-edge-node', 'view'); await choose('graph-edge-role', 'input');
        expect(root.querySelectorAll('form[host-command="edit-graph-edge"]')).toHaveLength(1);
        await choose('source', 'other'); action('Reset staged edge change').click(); await view.whenSettled();
        expect(root.querySelector('form[host-command="edit-graph-edge"] cem-select[name=source]').value).toBe('');
        expect(workbench.snapshot().dirty).toBe(false);
        await choose('source', 'other'); action('Apply graph edge').click(); await view.whenSettled();
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        action('Undo graph edit').click(); await view.whenSettled(); expect(workbench.snapshot().dirty).toBe(false);
        action('Redo graph edit').click(); await view.whenSettled(); expect(workbench.snapshot().dirty).toBe(true);
        await choose('graph-edge-role', 'with');
        const binding = root.querySelector('form[host-command="edit-graph-edge"] cem-text-field[name=name] input');
        binding.value = 'palette'; binding.dispatchEvent(new Event('input', { bubbles: true }));
        await choose('source', 'other'); action('Apply graph edge').click(); await view.whenSettled();
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        expect(root.querySelector('cem-fieldset[label="Graph edge"]').textContent).toContain('palette');
        const pending = deferred(); validator.validateResource.mockImplementationOnce(() => pending.promise);
        const saving = workbench.saveAndValidate(); await vi.waitFor(() => expect(workbench.snapshot().status).toBe('validating')); await view.whenSettled();
        expect(root.querySelector('cem-fieldset[label="Graph edge"] fieldset').disabled).toBe(true);
        const saved = workbench.snapshot().draft;
        await workbench.updateGraphEdge({ node: ['view'], role: ['input'], source: ['data'], version: [workbench.snapshot().graphConfiguration.version] });
        expect(workbench.snapshot().draft).toBe(saved);
        pending.resolve(validResult()); await saving; await view.whenSettled();
        expect(root.querySelector('cem-fieldset[label="Graph edge"] fieldset').disabled).toBe(false);
        await choose('graph-edge-role', 'parent');
        expect(root.querySelector('form[host-command="edit-graph-edge"] cem-select[name=source]').value).toBe('data');
    });

    it('bounds graph draft history by UTF-8 bytes during undo and redo', async () => {
        const { workbench } = await graphWorkbench();
        const small = `${workbench.snapshot().draft}// Retained small draft\n`;
        workbench.updateDraft(small);
        const large = '🙂'.repeat(2 * 1024 * 1024);
        workbench.updateDraft(large);
        workbench.undoGraphDraft();
        expect(workbench.snapshot().draft).toBe(small);
        expect(workbench.snapshot().graphConfiguration).toMatchObject({ canUndo: false, canRedo: true });
        workbench.redoGraphDraft(); expect(workbench.snapshot().draft).toBe(large);
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).toBe(small);
        workbench.updateDraft(`${large}🙂`);
        workbench.undoGraphDraft();
        expect(workbench.snapshot().draft).toBe(small);
        expect(workbench.snapshot().graphConfiguration).toMatchObject({ canUndo: false, canRedo: false });
    });

    it('composes one graph attribute form declaratively with native reset and pending disabling', async () => {
        const { workbench, validator } = await graphWorkbench();
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        await view.whenSettled();
        const choose = async (name, value) => {
            const select = root.querySelector(`cem-select[name="${name}"]`);
            expect(select).not.toBeNull(); select.value = value; select.dispatchEvent(new Event('change', { bubbles: true }));
            await view.whenSettled();
        };
        const action = name => [...root.querySelectorAll('button')].find(button => button.textContent.trim() === name);
        await choose('graph-node', 'result'); await choose('graph-attribute', 'out');
        const group = root.querySelector('cem-fieldset[label="Graph configuration"] fieldset');
        expect(group.querySelector('legend').textContent.trim()).toBe('Graph configuration');
        expect(root.querySelectorAll('form[host-command="edit-graph"]')).toHaveLength(1);
        const input = group.querySelector('cem-text-field[name=value] input');
        expect(input.value).toBe('result.svg');
        input.value = 'staged.svg'; input.dispatchEvent(new Event('input', { bubbles: true }));
        action('Reset staged graph change').click(); await view.whenSettled();
        expect(input.value).toBe('result.svg');
        input.value = 'changed.svg'; input.dispatchEvent(new Event('input', { bubbles: true }));
        action('Set graph attribute').click(); await view.whenSettled();
        expect(workbench.snapshot().draft).toContain('changed.svg');
        expect(workbench.snapshot().projectRevision).toBe(1);
        action('Undo graph edit').click(); await view.whenSettled();
        expect(workbench.snapshot().dirty).toBe(false);
        action('Redo graph edit').click(); await view.whenSettled();
        expect(workbench.snapshot().dirty).toBe(true);
        const pending = deferred(); validator.validateResource.mockImplementationOnce(() => pending.promise);
        const saving = workbench.saveAndValidate();
        await vi.waitFor(() => expect(workbench.snapshot().status).toBe('validating')); await view.whenSettled();
        expect(group.disabled).toBe(true);
        const saved = workbench.snapshot().draft;
        await workbench.updateGraphAttribute({ node: ['result'], attribute: ['out'], value: ['pending.svg'], mode: ['set'], version: [workbench.snapshot().graphConfiguration.version] });
        expect(workbench.snapshot().draft).toBe(saved);
        pending.resolve(validResult()); await saving; await view.whenSettled();
        expect(root.querySelector('cem-fieldset[label="Graph configuration"] fieldset').disabled).toBe(false);
        await choose('graph-attribute', 'style-policy');
        expect(root.querySelectorAll('form[host-command="edit-graph"]')).toHaveLength(1);
        expect(root.querySelector('cem-fieldset[label="Graph configuration"] cem-select[name=value] [role=combobox]')).not.toBeNull();
    });

    it('uses native metadata to edit enum, boolean and literal fields without dropping other options', () => {
        const text = 'cem-ml parse studio://alpha/source.cem --namespace a=urn:a --namespace b=urn:b';
        const fields = describeCemStudioOperationFields(text);
        expect(fields.find(field => field.id === 'format')).toMatchObject({ type: 'enum', value: 'ast', defaultValue: 'ast' });
        expect(fields.find(field => field.id === 'preserve_source_offsets').type).toBe('boolean');
        let edited = editCemStudioOperationField(text, 'format', 'events');
        edited = editCemStudioOperationField(edited, 'preserve_source_offsets', 'true');
        const parsed = parseCemMlCommandText(edited);
        expect(parsed.options).toMatchObject({ format: 'events', preserve_source_offsets: true, namespaces: ['a=urn:a', 'b=urn:b'] });
        expect(parseCemMlCommandText(editCemStudioOperationField(edited, 'format', '', true)).options.format).toBe('ast');
        expect(() => editCemStudioOperationField(text, 'format', 'invented')).toThrow('Choose a value');
        expect(() => editCemStudioOperationField(text, 'input', 'file:///elsewhere')).toThrow('not available');
        expect(() => editCemStudioOperationField(text, 'preserve_source_offsets', 'yes')).toThrow('true or false');
        const expression = 'concat("café", "\\", "$(literal)")';
        const query = editCemStudioOperationField('cem-ml query studio://alpha/source.cem --query . --query-content-type application/vnd.cem.query-expression+xpath', 'query', expression);
        expect(parseCemMlCommandText(query).options.query).toBe(expression);
        for (const operation of ['parse', 'inspect', 'convert', 'query', 'transform', 'trace']) {
            const argv = operation === 'query' ? '--query . --query-content-type application/vnd.cem.query-expression+xpath' : operation === 'transform' ? '--template-expression .' : '';
            expect(describeCemStudioOperationFields(`cem-ml ${operation} studio://alpha/source.cem ${argv}`).length).toBeGreaterThan(0);
        }
    });

    it('validates grouped option batches together with the native grammar', () => {
        const source = 'cem-ml parse studio://alpha/source.cem --namespace a=urn:a --namespace b=urn:b --from-format cem';
        const group = describeCemStudioOperationGroups(source).find(group => group.id === 'operation-options');
        expect(group.fields.map(field => field.id)).toEqual(['format', 'fail_level', 'preserve_source_offsets']);
        expect(describeCemStudioOperationGroups(source).every(group => group.fields.length >= 2 && group.fields.length <= 3)).toBe(true);
        const changed = editCemStudioOperationFields(source, [
            { id: 'format', value: 'events' }, { id: 'preserve_source_offsets', value: 'true' },
        ]);
        expect(parseCemMlCommandText(changed).options).toMatchObject({
            format: 'events', preserve_source_offsets: true, from_format: 'cem', namespaces: ['a=urn:a', 'b=urn:b'],
        });
        expect(editCemStudioOperationFields(source, [])).toBe(source);
        for (const changes of [
            [{ id: 'format', value: 'events' }, { id: 'fail_level', value: 'invented' }],
            [{ id: 'format', value: 'ast' }, { id: 'format', value: 'events' }],
            [{ id: 'input', value: 'file:///elsewhere' }],
            [{ id: 'preserve_source_offsets', value: 'yes' }],
        ]) expect(() => editCemStudioOperationFields(source, changes)).toThrow();
        const query = 'cem-ml query studio://alpha/source.cem --query . --query-content-type application/vnd.cem.query-expression+xpath';
        expect(describeCemStudioOperationGroups(query).find(group => group.id === 'query_source').fields.map(field => field.id)).toEqual(['query', 'query_file']);
        expect(() => editCemStudioOperationField(query, 'query_file', 'studio://alpha/query.xpath')).toThrow();
        const replaced = editCemStudioOperationFields(query, [
            { id: 'query', value: '', reset: true }, { id: 'query_file', value: 'studio://alpha/query.xpath' },
        ]);
        expect(parseCemMlCommandText(replaced).options).toMatchObject({ query_file: 'studio://alpha/query.xpath', query_content_type: 'application/vnd.cem.query-expression+xpath' });
        expect(parseCemMlCommandText(replaced).options.query).toBeUndefined();
        expect(() => editCemStudioOperationFields(query, [{ id: 'query', value: '', reset: true }])).toThrow();
        const literal = 'concat("café", "\\", "$(literal)")';
        const updated = editCemStudioOperationFields(query, [{ id: 'query', value: literal }, { id: 'output', value: 'cem' }]);
        expect(parseCemMlCommandText(updated).options).toMatchObject({ query: literal, output: 'cem' });
    });

    it('commits grouped drafts once and preserves them after invalid, stale or failed batches', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        const other = await createWorkbench(repository, validator);
        const before = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const initial = workbench.snapshot().command;
        const changes = [{ id: 'format', value: 'events' }, { id: 'preserve_source_offsets', value: 'true' }];
        await workbench.updateCommandFields(changes, other.snapshot().command.formVersion);
        expect(workbench.snapshot().command.draftText).toBe(initial.draftText);
        expect(workbench.snapshot().command.formError.message).toContain('command changed');
        await workbench.updateCommandFields([...changes, { id: 'format', value: 'ast' }], workbench.snapshot().command.formVersion);
        expect(workbench.snapshot().command.draftText).toBe(initial.draftText);
        validator.previewResourceCommand.mockRejectedValueOnce(new Error('Grouped preview unavailable'));
        await workbench.updateCommandFields(changes, workbench.snapshot().command.formVersion);
        expect(workbench.snapshot().command.draftText).toBe(initial.draftText);
        expect(workbench.snapshot().command.status).toBe(initial.status);
        expect(workbench.snapshot().command.formError.message).toBe('Grouped preview unavailable');
        const observed = []; const unsubscribe = workbench.subscribe(state => observed.push(state.command.draftText));
        await workbench.updateCommandFields(changes, workbench.snapshot().command.formVersion);
        unsubscribe();
        const committed = workbench.snapshot().command;
        expect(parseCemMlCommandText(committed.draftText).options).toMatchObject({ format: 'events', preserve_source_offsets: true });
        expect(new Set(observed)).toEqual(new Set([initial.draftText, committed.draftText]));
        expect(committed.formError).toBeUndefined();
        await workbench.updateCommandFields([{ id: 'format', value: '', reset: true }], initial.formVersion);
        expect(workbench.snapshot().command.draftText).toBe(committed.draftText);
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        expect(validator.validateResource).not.toHaveBeenCalled();
    });

    it('rejects repeated pending batches and ignores a superseded grouped preview', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        const initial = workbench.snapshot().command;
        const pending = deferred();
        validator.previewResourceCommand.mockImplementationOnce(async options => { await pending.promise; return commandPreviewOutcome(options); });
        const version = initial.formVersion;
        const change = [{ id: 'format', value: 'events' }, { id: 'preserve_source_offsets', value: 'true' }];
        const previews = validator.previewResourceCommand.mock.calls.length;
        const update = workbench.updateCommandFields(change, version);
        try {
            await expect.poll(() => workbench.snapshot().command.status).toBe('checking');
            await expect.poll(() => validator.previewResourceCommand.mock.calls.length).toBe(previews + 1);
            expect(workbench.snapshot().command.draftText).toBe(initial.draftText);
            await workbench.updateCommandFields(change, version);
            expect(workbench.snapshot().command.formError.message).toContain('command changed');
            await workbench.updateCommandDraft(initial.draftText.replace('--format ast', '--format events'));
        } finally { pending.resolve(); }
        await update;
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options.preserve_source_offsets).not.toBe(true);
        expect(workbench.snapshot().command.formError).toBeUndefined();
    });

    it('persists grouped options only through one explicit Apply and reopens stable resources', async () => {
        const native = await createCemStudioBrowserValidator();
        try {
            const validator = { ...validatorFor(validResult()), previewResourceCommand: native.previewResourceCommand,
                serializeResourceCommand: native.serializeResourceCommand, decodeResourceCommand: native.decodeResourceCommand };
            const repository = await repositoryWithSource('{article | Native grouped options}');
            const workbench = await createWorkbench(repository, validator);
            await workbench.parsePersisted('ast');
            const before = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
            await workbench.updateCommandFields([{ id: 'format', value: 'events' }, { id: 'preserve_source_offsets', value: 'true' }], workbench.snapshot().command.formVersion);
            expect(workbench.snapshot().command.parsed.options).toMatchObject({ format: 'events', preserve_source_offsets: true });
            expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
            workbench.setCommandTarget({ mode: 'new', name: 'Grouped inspection', parentId: 'schema-package-examples' });
            await workbench.applyCommand();
            const applied = workbench.snapshot().command.application;
            expect(applied.status).toBe('applied');
            const after = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
            expect(after.project.revision).toBe(before.project.revision + 1);
            for (const resource of before.project.resources) {
                expect(after.project.resources.find(item => item.id === resource.id)).toEqual(resource);
                expect(after.contents[resource.id]).toEqual(before.contents[resource.id]);
            }
            const committed = native.decodeResourceCommand(after.contents[applied.result.commandResource.id]);
            expect(committed.options).toMatchObject({ format: 'events', preserve_source_offsets: true });
            const reopened = await createCemStudioProjectWorkbench({ repository, validator, selection: { projectId: 'feature-tour', entryId: applied.result.entry.id } });
            workbenches.push(reopened);
            expect(reopened.snapshot().command.parsed.options).toMatchObject({ format: 'events', preserve_source_offsets: true });
            expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        } finally { await native.close(); }
    }, 30_000);

    it('rejects malformed grouped form capture without changing the command', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        await workbench.parsePersisted('ast');
        const original = workbench.snapshot().command;
        const fields = { group: ['operation-options'], version: [original.formVersion],
            'mode.format': ['keep'], 'mode.fail_level': ['keep'], 'mode.preserve_source_offsets': ['keep'] };
        await workbench.updateCommandGroup('operation-options', fields, original.formVersion);
        expect(workbench.snapshot().command.formVersion).toBe(original.formVersion);
        for (const invalid of [
            { ...fields, 'mode.format': ['set', 'reset'] },
            { ...fields, 'mode.format': ['invented'] },
            { ...fields, 'mode.format': ['set'] },
            { ...fields, 'value.query': ['unexpected'] },
        ]) {
            await workbench.updateCommandGroup('operation-options', invalid, original.formVersion);
            expect(workbench.snapshot().command.draftText).toBe(original.draftText);
            expect(workbench.snapshot().command.formError).toBeDefined();
        }
        await workbench.updateCommandGroup('operation-options', fields, original.formVersion);
        expect(workbench.snapshot().command.formError).toBeUndefined();
        expect(workbench.snapshot().command.formVersion).toBe(original.formVersion);
    });

    it('guards typed field versions and edits drafts without Save, Apply or Run', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        const before = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const other = await createWorkbench(repository, validator);
        await workbench.updateCommandField('format', 'events', other.snapshot().command.formVersion);
        expect(workbench.snapshot().command.formError.message).toContain('command changed');
        const version = workbench.snapshot().command.formVersion;
        await workbench.updateCommandField('format', 'events', version);
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options.format).toBe('events');
        const edited = workbench.snapshot().command.draftText;
        await workbench.updateCommandField('format', 'ast', version);
        expect(workbench.snapshot().command.formError.message).toContain('command changed');
        expect(workbench.snapshot().command.draftText).toBe(edited);
        await workbench.updateCommandField('format', 'invalid', workbench.snapshot().command.formVersion);
        expect(workbench.snapshot().command.draftText).toBe(edited);
        await workbench.updateCommandField('format', '', workbench.snapshot().command.formVersion, true);
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options.format).toBe('ast');
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        expect(validator.validateResource).not.toHaveBeenCalled();
    });

    it('saves and reloads exact revisions, retains native validation data, and navigates with CEM controls', async () => {
        const invalid = '@doc cem-ml 1\n\n{article |\n    {h1 | Missing article close}\n';
        const rangeStart = new TextEncoder().encode(invalid.slice(0, invalid.indexOf('{article'))).byteLength;
        const validator = validatorFor(invalidResult(rangeStart, 10));
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root });
        shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench });
        views.push(view);

        const editor = root.querySelector('cem-source-editor[data-cem-studio-editor] [role="textbox"]');
        expect(root.querySelector('cem-studio-workbench-controls')).not.toBeNull();
        expect(editor.value).toBe('original');
        editor.focus();
        editor.value = invalid;
        editor.setSelectionRange(2, 5);
        editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(workbench.snapshot()).toMatchObject({ status: 'dirty', dirty: true });
        await view.whenSettled();
        expect(root.querySelector('cem-source-editor[data-cem-studio-editor] [role="textbox"]')).toBe(editor);
        expect(document.activeElement).toBe(editor);
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 5]);

        root.querySelector('cem-action[data-cem-studio-save] button').click();
        await view.whenSettled();
        const snapshot = workbench.snapshot();
        expect(snapshot).toMatchObject({
            status: 'invalid',
            projectRevision: 2,
            resourceRevision: 2,
            dirty: false,
        });
        expect(snapshot.validation).toMatchObject({
            hardViolationCount: 1,
            stale: false,
            reportSummary: { inputCount: 1, errorCount: 1, hardViolationCount: 1 },
            diagnostics: [{ code: 'cem.tokenizer.unterminated_node' }],
            provenance: [{ transform: 'CemTokenizer', range: { start: rangeStart, len: 10 } }],
        });
        expect(validator.validateResource).toHaveBeenCalledWith(
            expect.objectContaining({
                projectId: 'feature-tour',
                projectRevision: 2,
                resourceRevision: 2,
            }),
        );
        const exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(new TextDecoder().decode(exported.value.contents.source)).toBe(invalid);
        expect(exported.value.project.revision).toBe(2);
        expect(exported.value.project.resources.find(({ id }) => id === 'source').revision).toBe(2);

        expect(root.querySelector('cem-tabs [role="tablist"]')).not.toBeNull();
        expect(root.querySelector('cem-table [role="table"]')).not.toBeNull();
        expect(root.querySelector('cem-list')).not.toBeNull();
        const reportRows = [...root.querySelectorAll('[data-cem-studio-report] [role="row"]')]
            .map(row => [...row.querySelectorAll('[role="cell"]')].map(cell => cell.textContent.trim()));
        expect(reportRows).toEqual(expect.arrayContaining([['Inputs', '1'], ['Warnings', '0'], ['Hard violations', '1']]));
        expect(
            root.querySelectorAll('button:not(cem-action button):not(cem-icon-button button):not(cem-select button):not(cem-tabs button):not(cem-expansion button):not(cem-tree button)'),
        ).toHaveLength(0);

        const paneTabs = [...root.querySelector('cem-tabs[label="Workbench panes"]').querySelectorAll('[role="tab"]')]
            .filter(tab => tab.closest('cem-tabs').getAttribute('label') === 'Workbench panes');
        const diagnostics = root.querySelector('cem-list[data-diagnostic-list] select');
        for (let attempt = 0; attempt < 2; attempt++) {
            paneTabs.find(tab => tab.textContent === 'Results').click(); await view.whenSettled();
            expect(editor.closest('[role="tabpanel"]')).toBeNull();
            expect(editor.checkVisibility()).toBe(true);
            diagnostics.value = '0'; diagnostics.dispatchEvent(new Event('change', { bubbles: true }));
            await view.whenSettled();
            expect(editor.closest('[role="tabpanel"]')).toBeNull();
            expect(editor.checkVisibility()).toBe(true);
            expect(document.activeElement).toBe(editor);
            expect(editor.selectionStart).toBe(workbench.snapshot().selection.start);
        }
        diagnostics.value = '0';
        diagnostics.dispatchEvent(new Event('change', { bubbles: true }));
        await view.whenSettled();
        expect(editor.selectionStart).toBe(workbench.snapshot().selection.start);
        expect(workbench.snapshot().selection).toMatchObject({ kind: 'diagnostic', byteStart: rangeStart });
        expect(document.activeElement).toBe(editor);
        editor.setSelectionRange(0, 0);
        root.querySelector('cem-expansion[label="Editor details"] button').click(); await view.whenSettled();
        const reload = root.querySelector('[data-cem-studio-reload] button');
        reload.focus();
        await workbench.copyCommand(async () => undefined);
        await view.whenSettled();
        expect(document.activeElement).toBe(reload);
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([0, 0]);
        workbench.navigateDiagnostic(0);
        await view.whenSettled();
        expect(document.activeElement).toBe(editor);
        expect(editor.selectionStart).toBe(workbench.snapshot().selection.start);
        const provenance = root.querySelector('cem-list[data-provenance-list] select');
        provenance.value = '0';
        provenance.dispatchEvent(new Event('change', { bubbles: true }));
        await view.whenSettled();
        expect(workbench.snapshot().selection).toMatchObject({ kind: 'provenance', byteStart: rangeStart });

        const parseSelect = root.querySelector('cem-select[data-cem-studio-parse-projection]');
        parseSelect.value = 'events';
        root.querySelector('cem-action[data-cem-studio-parse] button').click();
        await view.whenSettled();
        expect(workbench.snapshot()).toMatchObject({
            status: 'projected',
            projection: {
                kind: 'parse',
                mode: 'events',
                revision: { projectRevision: 2, resourceRevision: 2 },
                output: { contentType: 'application/cem', text: 'parse events projection\n' },
                nativeResult: { operation: 'parse' },
                stale: false,
            },
        });
        expect(root.querySelector('[data-cem-studio-preview] pre').textContent).toBe('parse events projection\n');

        const inspectSelect = root.querySelector('cem-select[data-cem-studio-inspect-view]');
        inspectSelect.value = 'tree';
        root.querySelector('cem-action[data-cem-studio-inspect] button').click();
        await view.whenSettled();
        expect(workbench.snapshot().projection).toMatchObject({
            kind: 'inspect',
            mode: 'tree',
            output: { text: 'inspect tree projection\n' },
        });
        expect(root.querySelector('cem-table[label="Projection execution"] [role="table"]')).not.toBeNull();
    }, 30_000);

    it('groups the active option declaratively and disables it during an atomic draft update', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        await workbench.parsePersisted('ast');
        const before = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        selectValue(root, '[name="operation-option"]', 'format'); await view.whenSettled();
        await expect.poll(() => root.querySelector('form[host-command="edit-field"] cem-fieldset fieldset')).toBeTruthy();
        const form = root.querySelector('form[host-command="edit-field"]');
        const host = form.querySelector('cem-fieldset'), group = host.querySelector('fieldset');
        expect(group.querySelector('legend').textContent).toBe('Edit format');
        expect(group.getAttribute('role')).toBeNull();
        expect(group.getAttribute('aria-describedby')).toBeTruthy();
        const choice = form.querySelector('cem-select[name="value"]');
        choice.value = 'events'; choice.dispatchEvent(new Event('change', { bubbles: true }));
        await view.whenSettled();
        const pending = deferred();
        validator.previewResourceCommand.mockImplementationOnce(async options => { await pending.promise; return commandPreviewOutcome(options); });
        const submit = form.querySelector('cem-action[value="set"] button');
        const previews = validator.previewResourceCommand.mock.calls.length;
        submit.click();
        try {
            await expect.poll(() => group.disabled).toBe(true);
            expect(choice.querySelector('button').matches(':disabled')).toBe(true);
            await expect.poll(() => validator.previewResourceCommand.mock.calls.length).toBe(previews + 1);
            submit.click();
            expect(validator.previewResourceCommand.mock.calls.length).toBe(previews + 1);
        } finally { pending.resolve(); }
        await view.whenSettled();
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options.format).toBe('events');
        expect(form.querySelector('fieldset')).toBe(group);
        expect(group.disabled).toBe(false);
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        form.querySelector('cem-action[value="reset"] button').click(); await view.whenSettled();
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options.format).toBe('ast');
        expect(root.querySelectorAll('form[host-command="edit-field"]')).toHaveLength(1);
    }, 30_000);

    it('stages a bounded option group declaratively and submits one native form', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        await workbench.parsePersisted('ast');
        const before = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        selectValue(root, '[name="operation-option"]', 'group:operation-options'); await view.whenSettled();
        const form = root.querySelector('form[host-command="edit-group"]');
        expect(form).not.toBeNull();
        expect(root.querySelectorAll('form[host-command="edit-field"]')).toHaveLength(0);
        expect(form.querySelector('legend').textContent).toBe('Edit Operation options together');
        expect(form.querySelectorAll('cem-select[name^="mode."]')).toHaveLength(3);
        const button = form.querySelector('cem-action[type="submit"] button');
        const original = workbench.snapshot().command;
        selectValue(form, '[name="value.format"]', 'events'); await view.whenSettled();
        button.click(); await view.whenSettled();
        expect(workbench.snapshot().command.draftText).toBe(original.draftText);
        expect(workbench.snapshot().command.formVersion).toBe(original.formVersion);
        form.querySelector('cem-action[type="reset"] button').click(); await view.whenSettled();
        expect(form.querySelector('cem-select[name="value.format"]').value).toBe('ast');
        selectValue(form, '[name="mode.format"]', 'set');
        selectValue(form, '[name="value.format"]', 'events');
        selectValue(form, '[name="mode.preserve_source_offsets"]', 'set');
        selectValue(form, '[name="value.preserve_source_offsets"]', 'true');
        await view.whenSettled();
        const pending = deferred(); const previews = validator.previewResourceCommand.mock.calls.length;
        validator.previewResourceCommand.mockImplementationOnce(async options => { await pending.promise; return commandPreviewOutcome(options); });
        button.click();
        try {
            await expect.poll(() => form.querySelector('fieldset').disabled).toBe(true);
            expect(workbench.snapshot().command.draftText).toBe(original.draftText);
            expect(form.querySelector('cem-select[name="value.format"]').value).toBe('events');
            button.click();
            await expect.poll(() => validator.previewResourceCommand.mock.calls.length).toBe(previews + 1);
        } finally { pending.resolve(); }
        await view.whenSettled();
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options).toMatchObject({ format: 'events', preserve_source_offsets: true });
        expect(root.querySelector('form[host-command="edit-group"]')).toBe(form);
        expect(form.querySelector('fieldset').disabled).toBe(false);
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        selectValue(form, '[name="mode.format"]', 'reset');
        selectValue(form, '[name="mode.preserve_source_offsets"]', 'keep');
        button.click(); await view.whenSettled();
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options).toMatchObject({ format: 'ast', preserve_source_offsets: true });
    }, 30_000);

    it('replaces a coupled query source through the grouped declarative form', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        await workbench.updateCommandDraft('cem-ml query studio://feature-tour/data/cem-ml/basic.cem --query . --query-content-type application/vnd.cem.query-expression+xpath');
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        selectValue(root, '[name="operation-option"]', 'group:query_source'); await view.whenSettled();
        const form = root.querySelector('form[host-command="edit-group"]');
        expect(form.querySelector('legend').textContent).toBe('Edit Query source together');
        expect(form.querySelectorAll('cem-select[name^="mode."]')).toHaveLength(2);
        selectValue(form, '[name="mode.query"]', 'reset');
        selectValue(form, '[name="mode.query_file"]', 'set');
        const file = form.querySelector('cem-text-field[name="value.query_file"] input');
        file.value = 'studio://feature-tour/queries/source.xpath';
        file.dispatchEvent(new InputEvent('input', { bubbles: true }));
        form.querySelector('cem-action[type="submit"] button').click(); await view.whenSettled();
        const parsed = parseCemMlCommandText(workbench.snapshot().command.draftText);
        expect(parsed.options.query).toBeUndefined();
        expect(parsed.options.query_file).toBe('studio://feature-tour/queries/source.xpath');
        expect(parsed.options.query_content_type).toBe('application/vnd.cem.query-expression+xpath');
        expect(workbench.snapshot().command.formError).toBeUndefined();
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
    }, 30_000);

    it('retains declarative editor state through reload and rejects repeated pending commands', async () => {
        let finishValidation;
        const result = new Promise(resolve => { finishValidation = resolve; });
        const validator = validatorFor(validResult());
        validator.validateResource.mockImplementationOnce(() => result);
        const repository = await repositoryWithSource('durable');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const editor = root.querySelector('[data-cem-studio-editor] [role="textbox"]');
        editor.value = 'saved draft'; editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
        const save = root.querySelector('[data-cem-studio-save] button');
        save.click(); save.click();
        await vi.waitFor(() => expect(validator.validateResource).toHaveBeenCalledTimes(1));
        finishValidation(validResult());
        await view.whenSettled();
        expect(workbench.snapshot()).toMatchObject({ resourceRevision: 2, dirty: false, draft: 'saved draft' });
        expect(root.querySelector('[data-cem-studio-workbench-alert]').textContent).toContain('0 diagnostics; 0 hard violations.');
        expect(root.querySelector('[data-cem-studio-save]').hasAttribute('disabled')).toBe(true);
        editor.value = 'discard me'; editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
        root.querySelector('[data-cem-studio-reload] button').click();
        await view.whenSettled();
        expect(root.querySelector('[data-cem-studio-editor] [role="textbox"]')).toBe(editor);
        expect(editor.value).toBe('saved draft');
        expect(root.querySelector('[data-cem-studio-workbench-status]').getAttribute('label')).toBe('Persisted revision loaded');
        expect(root.querySelector('[data-cem-studio-reload]').getAttribute('variant')).toBe('contextual');
        expect(root.querySelector('[data-cem-studio-parse]').getAttribute('variant')).toBe('alternate');
        view.dispose();
        editor.value = 'disposed'; editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(workbench.snapshot().draft).toBe('saved draft');
    });

    it('executes every CEM-ML parse projection and inspect view against the durable revision', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Durable projection}\n');
        const workbench = await createWorkbench(repository, validator);

        for (const projection of CEM_STUDIO_PARSE_PROJECTIONS) {
            const snapshot = await workbench.parsePersisted(projection);
            expect(snapshot.projection).toMatchObject({
                kind: 'parse',
                mode: projection,
                revision: { projectRevision: 1, resourceRevision: 1 },
                output: { text: `parse ${projection} projection\n` },
                stale: false,
            });
        }
        for (const view of CEM_STUDIO_INSPECT_VIEWS) {
            const snapshot = await workbench.inspectPersisted(view);
            expect(snapshot.projection).toMatchObject({
                kind: 'inspect',
                mode: view,
                revision: { projectRevision: 1, resourceRevision: 1 },
                output: { text: `inspect ${view} projection\n` },
                stale: false,
            });
        }

        expect(validator.parseResource).toHaveBeenCalledTimes(CEM_STUDIO_PARSE_PROJECTIONS.length);
        expect(validator.inspectResource).toHaveBeenCalledTimes(CEM_STUDIO_INSPECT_VIEWS.length);
        expect(validator.inspectResource).toHaveBeenLastCalledWith(
            expect.objectContaining({
                projectId: 'feature-tour',
                projectRevision: 1,
                resourceRevision: 1,
                view: 'tree',
            }),
        );
    });

    it('runs a portable operation and exposes expected, copy, and download evidence with CEM controls', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Portable conversion}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        let copied;
        let downloaded;
        const view = await mountCemStudioFeatureTourWorkbench({
            root,
            workbench,
            clipboard: {
                writeText: async (text) => {
                    copied = text;
                },
            },
            download: async (file) => {
                downloaded = file;
            },
        });
        views.push(view);

        await workbench.selectWorkbench('conversion');
        await view.whenSettled();
        root.querySelector('cem-action[data-cem-studio-operation-run] button').click();
        await view.whenSettled();

        const projection = workbench.snapshot().projection;
        expect(projection).toMatchObject({
            kind: 'convert',
            mode: 'conversion',
            summary: { kind: 'convert', outputCount: 1 },
            expected: { kind: 'convert', outputCount: 1 },
            expectedMatches: true,
            stale: false,
        });
        expect(validator.runResourceCommand).toHaveBeenCalledTimes(1);
        expect(validator.runResourceCommand.mock.calls[0][0].argv).toEqual(
            expect.arrayContaining([
                'convert',
                'studio://feature-tour/data/cem-ml/basic.cem',
                '--to-format',
                'dom-json',
            ]),
        );
        root.querySelector('cem-action[data-cem-studio-projection-copy] button').click();
        await view.whenSettled();
        root.querySelector('cem-action[data-cem-studio-projection-download] button').click();
        await view.whenSettled();
        expect(copied).toBe(projection.output.text);
        expect([...downloaded.bytes]).toEqual([...projection.output.bytes]);
        expect(root.querySelector('[data-cem-studio-projection-expected] cem-table [role="table"]')).not.toBeNull();
        expect(root.querySelector('[data-cem-studio-projection-expected]').textContent).toContain('Native result matches the pinned expectation.');
        expect(root.querySelector('[data-cem-studio-projection-metadata]').textContent).toContain('wasm-browser-worker');
        const metadataRows = [...root.querySelectorAll('[data-cem-studio-projection-metadata] [role="row"]')]
            .map(row => [...row.querySelectorAll('[role="cell"]')].map(cell => cell.textContent.trim()));
        expect(metadataRows).toEqual(expect.arrayContaining([['Diagnostics', '0'], ['Source-map frames', '0']]));
        expect(root.querySelector('[data-cem-studio-projection-trace]').textContent).toContain('No operation trace records');
        expect(root.querySelector('[data-cem-studio-projection-graph]').textContent).toContain('No transformation graph execution overlay records');
        expect(root.querySelector('[data-cem-studio-diagnostics]').textContent).toContain('No diagnostics');
        expect(root.querySelector('[data-cem-studio-provenance]').textContent).toContain('No source-map provenance');
        expect(
            root.querySelectorAll(
                '[data-cem-studio-workbench] button:not(cem-action button):not(cem-icon-button button):not(cem-select button):not(cem-tabs button):not(cem-expansion button):not(cem-tree button)',
            ),
        ).toHaveLength(0);
    });

    it('retains declarative result owners and excludes duplicate exact output transfers', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Result ownership}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main'); document.body.append(root);
        const copied = Promise.withResolvers(), downloaded = Promise.withResolvers();
        const clipboard = { writeText: vi.fn(() => copied.promise) };
        const download = vi.fn(() => downloaded.promise);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench, clipboard, download }); views.push(view);
        expect(root.querySelector('cem-studio-workbench-layout')).not.toBeNull();
        expect(root.querySelector('cem-studio-workbench-results')).not.toBeNull();
        expect(root.querySelector('[data-cem-studio-preview] cem-alert').getAttribute('tone')).toBe('info');
        await workbench.parsePersisted('events'); await view.whenSettled();
        root.querySelector('cem-tabs[label="Workbench panes"]').selectedIndex = 1; await view.whenSettled();
        const output = root.querySelector('[data-cem-studio-preview] cem-media-preview pre');
        expect(output.textContent).toBe(workbench.snapshot().projection.output.text);
        output.focus(); workbench.updateDraft('a newer draft'); await view.whenSettled();
        expect(root.querySelector('[data-cem-studio-preview] pre')).toBe(output);
        expect(document.activeElement).toBe(output);
        const copy = root.querySelector('[data-cem-studio-projection-copy] button');
        const save = root.querySelector('[data-cem-studio-projection-download] button');
        copy.click(); copy.click(); save.click();
        expect(clipboard.writeText).toHaveBeenCalledTimes(1);
        expect(clipboard.writeText).toHaveBeenCalledWith(workbench.snapshot().projection.output.text);
        expect(download).not.toHaveBeenCalled();
        copied.resolve(); await view.whenSettled();
        save.click(); save.click(); expect(download).toHaveBeenCalledTimes(1);
        expect([...download.mock.calls[0][0].bytes]).toEqual([...workbench.snapshot().projection.output.bytes]);
        downloaded.resolve(); await view.whenSettled();
        const outcome = projectionOutcome('parse', 'ast');
        const source = '<p>Isolated operation result</p>';
        const bytes = [...new TextEncoder().encode(source)];
        validator.parseResource.mockResolvedValueOnce({ ...outcome, output: { ...outcome.output,
            contentType: 'text/html', text: source, bytes, byteLength: bytes.length } });
        await workbench.parsePersisted('ast'); await view.whenSettled();
        const frame = root.querySelector('[data-cem-studio-preview] iframe');
        expect(frame.getAttribute('sandbox')).toBe(''); expect(frame.srcdoc).toContain(source);
        const changes = [];
        const observer = new MutationObserver(records => changes.push(...records));
        observer.observe(frame, { attributes: true, attributeFilter: ['srcdoc'] });
        workbench.updateDraft('another draft'); await view.whenSettled();
        changes.push(...observer.takeRecords()); observer.disconnect();
        expect(root.querySelector('[data-cem-studio-preview] iframe')).toBe(frame);
        expect(changes).toHaveLength(0);
        view.dispose(); copy.click(); save.click();
        expect(clipboard.writeText).toHaveBeenCalledTimes(1); expect(download).toHaveBeenCalledTimes(1);
    });

    it('renders bounded native trace and graph values without parsing their text as markup', async () => {
        const validator = validatorFor(validResult());
        const outcome = projectionOutcome('transform', 'graph');
        outcome.result.result.value = { kind: 'transform', value: { value: {
            artifacts: [], events: [false, null, '🚀'.repeat(241)], nodes: ['<node>'],
        } } };
        validator.runResourceCommand.mockResolvedValueOnce(outcome);
        const repository = await repositoryWithSource('{main | Graph values}\n');
        const seed = featureTourSeed();
        seed.catalog.workbenches.push({ ...seed.catalog.workbenches[0], id: 'graph',
            operation: 'transform', kind: 'transformation-graph', name: 'Graph',
            commandArguments: ['transform', '$input', '--format', 'json'],
            expectedSummary: { kind: 'transform', mode: 'graph', artifactCount: 0, itemCount: 0 } });
        const workbench = await createCemStudioFeatureTourWorkbench({ repository, validator, seed, projectId: 'feature-tour' });
        workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        await workbench.selectWorkbench('graph'); await workbench.runPersistedOperation(); await view.whenSettled();
        const trace = root.querySelector('[data-cem-studio-projection-trace]');
        const graph = root.querySelector('[data-cem-studio-projection-graph]');
        const values = [...trace.querySelectorAll('[role="cell"]')].map(cell => cell.textContent.trim());
        expect(values).toEqual(expect.arrayContaining(['false', 'null', `${'🚀'.repeat(239)}…`]));
        expect(graph.textContent).toContain('<node>'); expect(graph.querySelector('node')).toBeNull();
        expect(root.querySelector('[data-cem-studio-projection-expected]').textContent)
            .toContain('Native result matches the pinned expectation.');
        const traceOutcome = projectionOutcome('trace', 'trace');
        const events = Array.from({ length: 105 }, (_, index) => ({ kind: 'value', value: `event ${index}` }));
        traceOutcome.result.result.value = { kind: 'trace', value: { body: { events } } };
        validator.runResourceCommand.mockResolvedValueOnce(traceOutcome);
        seed.catalog.workbenches.push({ ...seed.catalog.workbenches[0], id: 'native-trace', operation: 'trace',
            kind: 'trace', name: 'Native trace', commandArguments: ['trace', '$input'],
            expectedSummary: { kind: 'trace', hasEvents: true } });
        const traceWorkbench = await createCemStudioFeatureTourWorkbench({ repository, validator, seed, projectId: 'feature-tour' });
        workbenches.push(traceWorkbench); view.dispose();
        const traceView = await mountCemStudioFeatureTourWorkbench({ root, workbench: traceWorkbench }); views.push(traceView);
        await traceWorkbench.selectWorkbench('native-trace'); await traceWorkbench.runPersistedOperation(); await traceView.whenSettled();
        expect(traceWorkbench.snapshot().projection.trace).toHaveLength(100);
        const traceRows = root.querySelectorAll('[data-cem-studio-projection-trace] [role="row"]');
        expect(traceRows).toHaveLength(101);
        expect(traceRows[1].textContent).toContain('value.body.events.0');
        expect(traceRows[1].textContent).toContain(JSON.stringify(events[0]));
        expect(traceRows[100].textContent).toContain(JSON.stringify(events[99]));
    });

    it('round trips the Studio command, copies displayed text, and previews semantic changes without mutation', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Command source}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        const clipboard = { writeText: vi.fn(async () => undefined) };
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench, clipboard });
        views.push(view);

        const original = workbench.snapshot().command.current.text;
        expect(root.querySelector('[data-cem-studio-command-changes]').textContent)
            .toContain('The command matches the active normalized run plan');
        const commandEditor = root.querySelector('cem-textarea[data-cem-studio-command-editor] textarea');
        expect(commandEditor.value).toBe(original);
        expect(workbench.snapshot().command).toMatchObject({
            projection: 'studio',
            status: 'current',
            changes: [],
            revision: { projectRevision: 1, resourceRevision: 1 },
        });

        const changed = original.replace('--format ast', '--format events');
        validator.previewResourceCommand.mockImplementationOnce(async options => {
            const result = commandPreviewOutcome(options);
            return { ...result, semantic: { ...result.semantic, configuration: {
                label: '🚀'.repeat(241), enabled: false, nothing: null,
            } } };
        });
        commandEditor.value = changed;
        commandEditor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
        expect(workbench.snapshot().command).toMatchObject({
            status: 'changed',
            draftText: changed,
            preview: { parsed: { commandPath: ['parse'], options: { format: 'events' } } },
        });
        expect(workbench.snapshot().command.changes).toEqual(
            expect.arrayContaining([expect.objectContaining({ category: 'operation', kind: 'changed' })]),
        );
        expect(root.querySelector('[data-cem-studio-command-alert]').textContent)
            .toContain(`${workbench.snapshot().command.changes.length} semantic changes`);
        expect(root.querySelector('cem-table[label="CLI Command semantic changes"] [role="table"]')).not.toBeNull();
        const cells = [...root.querySelectorAll('cem-table[label="CLI Command semantic changes"] [role="cell"]')]
            .map(cell => cell.textContent.trim());
        expect(cells).toEqual(expect.arrayContaining(['ast', 'events', '—', 'false', 'null', `${'🚀'.repeat(239)}…`]));

        root.querySelector('cem-action[data-cem-studio-command-copy] button').click();
        await view.whenSettled();
        expect(clipboard.writeText).toHaveBeenCalledWith(changed);
        expect(workbench.snapshot().command.copy).toMatchObject({ status: 'success' });

        commandEditor.value = `${changed} --unknown-studio-option`;
        commandEditor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
        expect(workbench.snapshot().command).toMatchObject({
            status: 'invalid',
            diagnostic: { code: 'cem.command.unknown_option' },
        });
        const exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(new TextDecoder().decode(exported.value.contents.source)).toBe('{main | Command source}\n');
        expect(exported.value.project.revision).toBe(1);

        root.querySelector('cem-action[data-cem-studio-command-reset] button').click();
        await view.whenSettled();
        expect(workbench.snapshot().command).toMatchObject({ status: 'current', draftText: original, changes: [] });
        expect(
            root.querySelectorAll(
                '[data-cem-studio-workbench] button:not(cem-action button):not(cem-icon-button button):not(cem-select button):not(cem-tabs button):not(cem-expansion button):not(cem-tree button)',
            ),
        ).toHaveLength(0);
    });

    it('retains command edits across pending validation and excludes duplicate clipboard commands', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Command editing}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main'); document.body.append(root);
        const copied = Promise.withResolvers();
        const clipboard = { writeText: vi.fn(() => copied.promise) };
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench, clipboard }); views.push(view);
        expect(root.querySelector('cem-studio-workbench-command')).not.toBeNull();
        const editor = root.querySelector('[data-cem-studio-command-editor] textarea');
        const original = editor.value;
        const first = Promise.withResolvers();
        validator.previewResourceCommand.mockImplementationOnce(async options => {
            await first.promise; return commandPreviewOutcome(options);
        });
        editor.focus(); editor.value = original.replace('--format ast', '--format events');
        editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(workbench.snapshot().command).toMatchObject({ status: 'checking', draftText: editor.value });
        await expect.poll(() => validator.previewResourceCommand.mock.calls.some(([options]) => options.text === editor.value)).toBe(true);
        editor.value = original; editor.setSelectionRange(2, 7);
        editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        workbench.updateDraft('unrelated source edit');
        await expect.poll(() => workbench.snapshot().command.status).toBe('current');
        first.resolve(); await view.whenSettled();
        expect(workbench.snapshot().command.draftText).toBe(original);
        expect(root.querySelector('[data-cem-studio-command-editor] textarea')).toBe(editor);
        expect(editor.value).toBe(original);
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 7]);
        expect(document.activeElement).toBe(editor);
        const copy = root.querySelector('[data-cem-studio-command-copy] button');
        copy.click(); copy.click();
        expect(clipboard.writeText).toHaveBeenCalledTimes(1); // Same synchronous activation stack.
        expect(clipboard.writeText).toHaveBeenCalledWith(original);
        copied.resolve(); await view.whenSettled();
        view.dispose(); copy.click();
        expect(clipboard.writeText).toHaveBeenCalledTimes(1);
    });

    it('applies named new and compatible existing pages through CEM controls without executing', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Apply only}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench });
        views.push(view);

        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'ready',
            target: { mode: 'new', parentId: 'schema-package-examples' },
            targets: { current: { id: 'validate-cem-ml', kind: 'validation', compatible: false } },
        });
        const name = root.querySelector('cem-text-field[data-cem-studio-command-target-name] input');
        name.focus();
        name.value = 'Applied inspection';
        name.setSelectionRange(2, 6);
        name.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
        expect(root.querySelector('[data-cem-studio-command-target-name] input')).toBe(name);
        expect(document.activeElement).toBe(name);
        expect([name.selectionStart, name.selectionEnd]).toEqual([2, 6]);
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();

        const application = workbench.snapshot().command.application;
        expect(application).toMatchObject({
            status: 'applied',
            target: { mode: 'current', entryId: 'applied-inspection' },
            result: {
                disposition: 'created',
                projectRevision: 2,
                resourceRevision: 1,
                entry: { id: 'applied-inspection', kind: 'inspection', name: 'Applied inspection' },
            },
        });
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        const exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        const commandResourceId = application.result.commandResource.id;
        expect(new TextDecoder().decode(exported.value.contents[commandResourceId])).toBe(
            new TextDecoder().decode(application.result.commandBytes),
        );
        expect(root.querySelector('cem-alert[data-cem-studio-command-apply-alert]').getAttribute('tone')).toBe(
            'success',
        );
        selectValue(root, '[data-cem-studio-command-target-mode]', 'existing');
        await view.whenSettled();
        expect(workbench.snapshot().command.application.target).toEqual({
            mode: 'existing',
            entryId: 'applied-inspection',
        });
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();
        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'applied',
            result: {
                disposition: 'updated',
                projectRevision: 3,
                resourceRevision: 2,
                entry: { id: 'applied-inspection' },
            },
        });
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        expect(
            root.querySelectorAll(
                '[data-cem-studio-command-application] button:not(cem-action button):not(cem-icon-button button):not(cem-select button)',
            ),
        ).toHaveLength(0);
    });

    it('recommends a new page and confirmation-gates incompatible replacement in a CEM dialog', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Confirm replacement}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench });
        views.push(view);

        selectValue(root, '[data-cem-studio-command-target-mode]', 'current');
        await view.whenSettled();
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();
        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'confirmation-required',
            confirmation: { target: { mode: 'current', entryId: 'validate-cem-ml' }, runAfterApply: false },
        });
        const dialog = root.querySelector('cem-dialog[data-cem-studio-command-replace-dialog] dialog');
        await expect.poll(() => dialog.open).toBe(true);
        expect(dialog.matches(':modal')).toBe(true);
        const entry = document.activeElement;
        workbench.updateDraft('unrelated edit while confirming');
        await view.whenSettled();
        expect(root.querySelector('[data-cem-studio-command-replace-dialog] dialog')).toBe(dialog);
        expect(document.activeElement).toBe(entry);
        dialog.querySelector('cem-text').dispatchEvent(new CustomEvent('cem-close', { bubbles: true }));
        expect(workbench.snapshot().command.application.status).toBe('confirmation-required');
        await expect.element(dialog).toHaveAccessibleName('Confirm incompatible configuration replacement');
        expect(dialog.querySelector('[data-cem-studio-command-replace-confirm]').getAttribute('variant')).toBe('destructive');
        let exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(exported.value.project.revision).toBe(1);

        const dismissed = new Promise((resolve) => {
            root.addEventListener('cem-close', resolve, { once: true });
        });
        dialog.close();
        await dismissed;
        await view.whenSettled();
        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'ready',
            target: { mode: 'current' },
            confirmation: undefined,
        });
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();
        const { userEvent } = await import('vitest/browser');
        await userEvent.keyboard('{Escape}');
        await expect.poll(() => workbench.snapshot().command.application.status).toBe('ready');
        await view.whenSettled();
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();
        dialogAction(root, 'Use new configuration').click();
        await view.whenSettled();
        expect(workbench.snapshot().command.application).toMatchObject({ status: 'ready', target: { mode: 'new' } });

        selectValue(root, '[data-cem-studio-command-target-mode]', 'current');
        await view.whenSettled();
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();
        expect(workbench.snapshot().command.application.status).toBe('confirmation-required');
        dialogAction(root, 'Replace selected configuration').click();
        await view.whenSettled();
        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'applied',
            result: { disposition: 'updated', entry: { id: 'validate-cem-ml', kind: 'inspection' } },
        });
        exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(exported.value.project.revision).toBe(2);
        expect(exported.value.project.entries.find(({ id }) => id === 'validate-cem-ml')).toMatchObject({
            kind: 'inspection',
            runConfigResourceId: 'run-cem-ml',
        });
    });

    it('runs exactly the command bytes and revisions returned by Apply', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Apply and run}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench });
        views.push(view);

        const applyRun = root.querySelector('cem-action[data-cem-studio-command-apply-run] button');
        applyRun.click(); applyRun.click();
        await view.whenSettled();
        const snapshot = workbench.snapshot();
        expect(snapshot.command.application).toMatchObject({
            status: 'ran',
            result: { projectRevision: 2, resourceRevision: 1 },
            execution: { projectRevision: 2, resourceRevision: 1, stale: false },
        });
        expect(snapshot.projection).toMatchObject({
            kind: 'parse',
            mode: 'ast',
            revision: { projectRevision: 2, resourceRevision: 1 },
            stale: false,
        });
        expect(validator.executeAuthoredResourceCommand).toHaveBeenCalledTimes(1);
        const execution = validator.executeAuthoredResourceCommand.mock.calls[0][0];
        expect(execution.projectRevision).toBe(snapshot.command.application.result.projectRevision);
        expect([...execution.commandResource]).toEqual([
            ...new Uint8Array(snapshot.command.application.result.commandBytes),
        ]);
    });

    it('marks Apply & Run stale when the repository advances during exact-revision execution', async () => {
        const validator = validatorFor(validResult());
        let releaseExecution;
        let startedExecution;
        const started = new Promise((resolve) => {
            startedExecution = resolve;
        });
        validator.executeAuthoredResourceCommand.mockImplementation(
            (options) =>
                new Promise((resolve) => {
                    releaseExecution = () => resolve(authoredCommandOutcome(options));
                    startedExecution();
                }),
        );
        const repository = await repositoryWithSource('{main | Before run}\n');
        const workbench = await createWorkbench(repository, validator);
        const running = workbench.applyAndRun();
        await started;
        await repository.execute(
            request('save-resource', {
                projectId: 'feature-tour',
                resourceId: 'source',
                expectedProjectRevision: 2,
                expectedResourceRevision: 1,
                content: '{main | Concurrent revision}\n',
            }),
        );
        releaseExecution();
        await running;

        expect(workbench.snapshot()).toMatchObject({
            status: 'projection-stale',
            projection: { stale: true, revision: { projectRevision: 2, resourceRevision: 1 } },
            command: {
                application: {
                    status: 'run-stale',
                    result: { projectRevision: 2, resourceRevision: 1 },
                    execution: { stale: true },
                },
            },
        });
    });

    it('surfaces a stale project conflict without applying the command draft', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Loaded revision}\n');
        const workbench = await createWorkbench(repository, validator);
        await repository.execute(
            request('save-resource', {
                projectId: 'feature-tour',
                resourceId: 'source',
                expectedProjectRevision: 1,
                expectedResourceRevision: 1,
                content: '{main | Newer repository revision}\n',
            }),
        );

        await expect(workbench.applyCommand()).rejects.toMatchObject({
            code: 'cem.studio.repository.revision_conflict',
        });
        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'conflict',
            error: { code: 'cem.studio.repository.revision_conflict' },
            result: undefined,
        });
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        const exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(exported.value.project.entries).toHaveLength(2);
        expect(exported.value.project.revision).toBe(2);
    });

    it('marks an in-flight saved-revision result stale when the draft advances', async () => {
        let releaseValidation;
        let startedValidation;
        const started = new Promise((resolve) => {
            startedValidation = resolve;
        });
        const validator = validatorFor(
            () =>
                new Promise((resolve) => {
                    releaseValidation = () => resolve(validResult());
                    startedValidation();
                }),
        );
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        workbench.updateDraft('saved revision');
        const saving = workbench.saveAndValidate();
        await started;
        workbench.updateDraft('new unsaved draft');
        releaseValidation();
        await saving;

        expect(workbench.snapshot()).toMatchObject({
            status: 'stale',
            dirty: true,
            persistedText: 'saved revision',
            draft: 'new unsaved draft',
            validation: { stale: true, revision: { projectRevision: 2, resourceRevision: 2 } },
        });
    });

    it('marks an in-flight persisted projection stale when the draft advances', async () => {
        let releaseProjection;
        let startedProjection;
        const started = new Promise((resolve) => {
            startedProjection = resolve;
        });
        const validator = validatorFor(validResult());
        validator.parseResource.mockImplementation(
            () =>
                new Promise((resolve) => {
                    releaseProjection = () => resolve(projectionOutcome('parse', 'ast'));
                    startedProjection();
                }),
        );
        const repository = await repositoryWithSource('{main | Persisted}\n');
        const workbench = await createWorkbench(repository, validator);
        const projecting = workbench.parsePersisted('ast');
        await started;
        workbench.updateDraft('{main | New draft}\n');
        releaseProjection();
        await projecting;

        expect(workbench.snapshot()).toMatchObject({
            status: 'projection-stale',
            dirty: true,
            projection: {
                kind: 'parse',
                mode: 'ast',
                revision: { projectRevision: 1, resourceRevision: 1 },
                stale: true,
            },
        });
    });
});

describe('Project-owned workbench loading', () => {
    async function fixture(id = 'alpha', empty = false, extra = false, extraText = 'additional text', extraPath = 'extra.cem') {
        const repository = await repositoryWithSource('project source');
        const bundle = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        bundle.project.id = id;
        bundle.project.name = id;
        bundle.project.rootUri = `studio://${id}/`;
        if (empty) { bundle.project.entries = []; bundle.project.resources = []; bundle.contents = {}; }
        if (extra) {
            const bytes = new TextEncoder().encode(extraText);
            bundle.project.resources.push({ ...bundle.project.resources.find(item => item.id === 'source'), id: 'extra', role: 'data', path: extraPath, referencePath: extraPath, sha256: await digest(bytes) });
            bundle.contents.extra = bytes;
        }
        await repository.execute(request('import-project', { bundle }));
        return repository;
    }

    async function provenanceFixture(uri = 'studio://alpha/extra.cem', range = { start: 9, len: 11 }, kind = 'SourceBoundary') {
        const source = '// café\n{p | target}';
        const repository = await fixture('alpha', false, true, source);
        const validator = validatorFor(validResult());
        validator.parseResource.mockImplementation(async () => {
            const outcome = projectionOutcome('parse', 'ast');
            outcome.result.sourceMaps.items.push({ sourceMapId: 'named-origin', sourceMap: { storage: 'inline', value: { frames: [
                { source_id: 1, span: { kind: 'Single', ranges: range }, transform: kind === 'ScssOrigin'
                    ? { kind, module_uri: uri, origin_kind: 'definition', name: '$accent' } : { kind, uri } },
            ] } } });
            return outcome;
        });
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', resourceId: 'source' });
        const base = await createCemStudioSessionWorkbench({ repository, session });
        const workbench = await createCemStudioNavigation({ workbench: base, repository }); workbenches.push(workbench);
        await workbench.openEntity('project=alpha&file=source');
        return { source, repository, session, workbench };
    }

    async function queryProvenanceFixture({ frames, stacks, diagnosticFrames, range = { start: 9, len: 11 } } = {}) {
        const source = '// café\n{p | target}', uri = 'studio://alpha/queries/report.xpath';
        const repository = await fixture('query-seed', false, true, source, 'queries/report.xpath');
        const bundle = (await repository.query(request('export-project', { projectId: 'query-seed' }))).value;
        bundle.project.id = 'alpha'; bundle.project.name = 'alpha'; bundle.project.rootUri = 'studio://alpha/';
        const query = bundle.project.resources.find(resource => resource.id === 'extra');
        Object.assign(query, { role: 'query', contentType: 'application/vnd.cem.xpath', schema: 'https://cem.dev/ns/query/xpath/1' });
        const command = bundle.project.resources.find(resource => resource.id === 'run-cem-ml');
        const commandBytes = new TextEncoder().encode(JSON.stringify({ $schema: 'https://cem.dev/ns/cli/command/1', schemaVersion: 1,
            commandSchemaVersion: 1, commonVersion: '0.1.0', binaryName: 'cem-ml',
            argv: ['query', 'data/cem-ml/basic.cem', '--query-file', 'queries/report.xpath', '--query-content-type', 'application/vnd.cem.xpath', '--output', 'json'] }));
        Object.assign(command, { schema: 'https://cem.dev/ns/cli/command/1', sha256: await digest(commandBytes) });
        bundle.contents[command.id] = commandBytes;
        bundle.project.entries = [{ id: 'query-origin', kind: 'query', name: 'Query origins',
            resourceIds: ['source', 'extra', command.id], runConfigResourceId: command.id }];
        await repository.execute(request('import-project', { bundle }));
        const fine = { source_id: 7, span: { kind: 'Single', ranges: range },
            transform: { kind: 'ContentTypeTransform', content_type: 'application/vnd.cem.xpath' } };
        const boundary = { source_id: 7, span: { kind: 'Single', ranges: { start: 0, len: new TextEncoder().encode(source).byteLength } },
            transform: { kind: 'SourceBoundary', uri } };
        const originalStacks = stacks ?? [frames ?? [fine, boundary]];
        const validator = validatorFor(validResult());
        validator.decodeResourceCommand = vi.fn(() => ({ commandPath: ['query'], options: {}, positionals: { input: 'data/cem-ml/basic.cem' } }));
        validator.executeAuthoredResourceCommand.mockImplementation(async () => {
            const outcome = projectionOutcome('query', 'query');
            outcome.parsed = { commandPath: ['query'], options: { query_content_type: 'application/vnd.cem.xpath' }, positionals: { input: 'data/cem-ml/basic.cem' } };
            outcome.result.sourceMaps.items = originalStacks.map((frames, index) => ({ sourceMapId: `query-stack-${index}`,
                sourceMap: { storage: 'inline', value: { frames } } }));
            outcome.result.sourceMaps.originalCount = originalStacks.length;
            if (diagnosticFrames) outcome.result.diagnostics.items = [{ severity: 'warning', code: 'query.fixture.origin',
                message: 'Native query origin fixture', sourceMap: { frames: diagnosticFrames } }];
            outcome.result.diagnostics.originalCount = diagnosticFrames ? 1 : 0;
            return outcome;
        });
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', entryId: 'query-origin', resourceId: 'source' });
        const base = await createCemStudioSessionWorkbench({ repository, session });
        const workbench = await createCemStudioNavigation({ workbench: base, repository }); workbenches.push(workbench);
        await workbench.openEntity('project=alpha&configuration=query-origin'); await workbench.runPersistedOperation();
        return { source, uri, repository, session, workbench, originalStacks, fine, boundary };
    }

    it('binds QueryIdentity fine query frames to their explicit source without changing native ranges or indices', async () => {
        const { source, uri, session, workbench, originalStacks } = await queryProvenanceFixture();
        expect(workbench.snapshot().projection.provenance[0]).toMatchObject({ sourceId: 7, uri, transform: 'ContentTypeTransform',
            range: { start: 9, len: 11 }, sourceMapId: 'query-stack-0', frameIndex: 0 });
        expect(originalStacks[0][0].transform).toEqual({ kind: 'ContentTypeTransform', content_type: 'application/vnd.cem.xpath' });
        await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError).toBeUndefined();
        expect(session.snapshot().selection.resourceId).toBe('extra');
        expect(workbench.snapshot().path).toBe('queries/report.xpath');
        const selected = workbench.snapshot().selection;
        expect(selected).toMatchObject({ byteStart: 9, byteLength: 11 });
        expect(source.slice(selected.start, selected.end)).toBe('{p | target');
    });

    it.each(['missing', 'different-id', 'ambiguous'])('refuses QueryIdentity input fallback for a query frame with %s source identity', async reason => {
        const frame = { source_id: 7, span: { kind: 'Single', ranges: { start: 1, len: 3 } }, transform: { kind: 'Query' } };
        const named = (source_id, uri) => ({ source_id, span: frame.span, transform: { kind: 'SourceBoundary', uri } });
        const frames = [frame, ...(reason === 'missing' ? [] : reason === 'different-id'
            ? [named(8, 'studio://alpha/queries/report.xpath')]
            : [named(7, 'studio://alpha/queries/report.xpath'), named(7, 'studio://alpha/data/cem-ml/basic.cem')])];
        const { session, workbench } = await queryProvenanceFixture({ frames });
        expect(workbench.snapshot().projection.provenance[0]).toMatchObject({ requiresSourceIdentity: true, uri: undefined });
        expect(() => session.activeWorkbench().navigateProvenance(0)).toThrow(/source identity/i);
        await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError.message).toMatch(/source identity/i);
        expect(session.snapshot().selection.entryId).toBe('query-origin'); expect(workbench.snapshot().resourceId).toBe('source');
        expect(workbench.snapshot().selection).toBeUndefined();
    });

    it('keeps QueryIdentity bindings within one source-map stack and native source-ID domain', async () => {
        const fine = { source_id: 7, span: { kind: 'Single', ranges: { start: 9, len: 11 } }, transform: { kind: 'Query' } };
        const named = (source_id, uri) => ({ source_id, span: { kind: 'Single', ranges: { start: 0, len: 1 } }, transform: { kind: 'SourceBoundary', uri } });
        const uri = 'studio://alpha/queries/report.xpath';
        const { workbench } = await queryProvenanceFixture({ stacks: [[fine, named(7, uri), named(7, uri), named(8, 'studio://else/other.xpath')], [fine]] });
        const frames = workbench.snapshot().projection.provenance;
        expect(frames[0]).toMatchObject({ uri, sourceId: 7, range: { start: 9, len: 11 } });
        expect(frames[4]).toMatchObject({ uri: undefined, requiresSourceIdentity: true, sourceMapId: 'query-stack-1', frameIndex: 0 });
    });

    it.each(['', 123])('refuses QueryIdentity inheritance across a known source witness with invalid URI %j', async invalidUri => {
        const fine = { source_id: 7, span: { kind: 'Single', ranges: { start: 9, len: 11 } }, transform: { kind: 'Query' } };
        const uri = 'studio://alpha/queries/report.xpath';
        const { session, workbench } = await queryProvenanceFixture({ frames: [fine,
            { ...fine, transform: { kind: 'SourceBoundary', uri: invalidUri } }, { ...fine, transform: { kind: 'SourceBoundary', uri } }] });
        expect(workbench.snapshot().projection.provenance[0]).toMatchObject({ uri: undefined, requiresSourceIdentity: true });
        expect(workbench.snapshot().projection.provenance[2].uri).toBe(uri);
        await workbench.navigateProvenance(0); expect(workbench.snapshot().navigationError.message).toMatch(/source identity/i);
        expect(session.snapshot().selection.entryId).toBe('query-origin'); expect(workbench.snapshot().resourceId).toBe('source');
    });

    it('honors direct QueryIdentity frame URIs even when sibling source IDs are ambiguous', async () => {
        const uri = 'studio://alpha/queries/report.xpath';
        const named = uri => ({ source_id: 7, span: { kind: 'Single', ranges: { start: 9, len: 11 } }, transform: { kind: 'SourceBoundary', uri } });
        const { session, workbench } = await queryProvenanceFixture({ frames: [named(uri), named('studio://other/source.xpath')] });
        expect(workbench.snapshot().projection.provenance[0].uri).toBe(uri);
        await workbench.navigateProvenance(0); expect(workbench.snapshot().navigationError).toBeUndefined();
        expect(session.snapshot().selection.resourceId).toBe('extra');
    });

    it('binds QueryIdentity diagnostic stacks independently from published source-map references', async () => {
        const fine = { source_id: 7, span: { kind: 'Single', ranges: { start: 9, len: 11 } }, transform: { kind: 'Query' } };
        const named = uri => ({ source_id: 7, span: fine.span, transform: { kind: 'SourceBoundary', uri } });
        const uri = 'studio://alpha/queries/report.xpath';
        const { session, workbench } = await queryProvenanceFixture({ diagnosticFrames: [fine, named(uri)],
            stacks: [[fine, named('studio://alpha/data/cem-ml/basic.cem')]] });
        const frames = workbench.snapshot().projection.provenance;
        expect(frames[0]).toMatchObject({ uri, diagnosticIndex: 0, frameIndex: 0 });
        expect(frames[2]).toMatchObject({ uri: 'studio://alpha/data/cem-ml/basic.cem', sourceMapId: 'query-stack-0' });
        await workbench.navigateProvenance(0); expect(workbench.snapshot().navigationError).toBeUndefined();
        expect(session.snapshot().selection.resourceId).toBe('extra');
    });

    it.each(['studio://other/queries/report.xpath', 'cem-query://inline/expression'])('refuses unowned QueryIdentity source %s', async uri => {
        const fine = { source_id: 7, span: { kind: 'Single', ranges: { start: 1, len: 3 } }, transform: { kind: 'Query' } };
        const { session, workbench } = await queryProvenanceFixture({ frames: [fine, { ...fine, transform: { kind: 'SourceBoundary', uri } }] });
        expect(workbench.snapshot().projection.provenance[0].uri).toBe(uri);
        await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError.message).toContain('not an available resource');
        expect(session.snapshot().selection.entryId).toBe('query-origin'); expect(workbench.snapshot().resourceId).toBe('source');
    });

    it.each(['cancel', 'discard', 'save'])('preserves QueryIdentity draft transitions: %s', async decision => {
        const { session, workbench } = await queryProvenanceFixture();
        workbench.updateDraft('retained source draft'); await workbench.navigateProvenance(0);
        expect(session.snapshot().transition.stage).toBe('source');
        await workbench.resolveNavigation(decision);
        if (decision === 'cancel') {
            expect(session.snapshot().selection.entryId).toBe('query-origin'); expect(workbench.snapshot().resourceId).toBe('source'); expect(workbench.snapshot().draft).toBe('retained source draft');
        } else {
            expect(session.snapshot().selection.resourceId).toBe('extra');
            if (decision === 'discard') expect(workbench.snapshot().selection).toMatchObject({ kind: 'provenance', byteStart: 9, byteLength: 11 });
            else { expect(workbench.snapshot().selection).toBeUndefined(); expect(workbench.snapshot().navigationError.message).toContain('stale'); }
        }
    });

    it('preserves QueryIdentity captured revision refusal', async () => {
        const { repository, session, workbench } = await queryProvenanceFixture();
        await repository.execute(request('save-resource', { projectId: 'alpha', resourceId: 'extra', expectedProjectRevision: 1, expectedResourceRevision: 1, content: 'changed' }));
        await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError.message).toContain('stale'); expect(session.snapshot().selection.entryId).toBe('query-origin'); expect(workbench.snapshot().resourceId).toBe('source');
        await workbench.refreshNavigation();
    });

    it.each([{ start: 100, len: 2 }, { start: 6, len: 1 }, { start: 9, len: -1 }])('preserves QueryIdentity original UTF-8 bounds: %j', async range => {
        const { workbench } = await queryProvenanceFixture({ range }); await workbench.navigateProvenance(0);
        expect(workbench.snapshot().selection).toBeUndefined(); expect(workbench.snapshot().navigationError.message).toMatch(/outside|UTF-8/);
    });

    it('preserves the QueryIdentity row cap while resolving a boundary beyond the presentation limit', async () => {
        const fine = { source_id: 7, span: { kind: 'Single', ranges: { start: 9, len: 11 } }, transform: { kind: 'Query' } };
        const uri = 'studio://alpha/queries/report.xpath';
        const frames = [...Array.from({ length: CEM_STUDIO_LIMITS.structuredRows + 5 }, () => fine), { ...fine, transform: { kind: 'SourceBoundary', uri } }];
        const { workbench } = await queryProvenanceFixture({ frames });
        expect(workbench.snapshot().projection.provenance).toHaveLength(CEM_STUDIO_LIMITS.structuredRows);
        expect(workbench.snapshot().projection.provenance.every(frame => frame.uri === uri)).toBe(true);
        expect(frames[0].transform).toEqual({ kind: 'Query' });
    });

    it.each(['parse', 'inspect'])('preserves legacy %s provenance input fallback outside QueryIdentity mode', async kind => {
        const repository = await repositoryWithSource('legacy source text'); const validator = validatorFor(validResult());
        const outcome = projectionOutcome(kind, 'ast');
        outcome.result.sourceMaps.items.push({ sourceMapId: 'legacy-input', sourceMap: { storage: 'inline', value: { frames: [
            { source_id: 1, span: { kind: 'Single', ranges: { start: 1, len: 3 } }, transform: { kind: 'CemTokenizer' } },
        ] } } });
        if (kind === 'parse') validator.parseResource.mockResolvedValue(outcome); else validator.inspectResource.mockResolvedValue(outcome);
        const workbench = await createWorkbench(repository, validator);
        if (kind === 'parse') await workbench.parsePersisted('ast'); else await workbench.inspectPersisted('ast');
        expect(workbench.snapshot().projection.provenance[0].requiresSourceIdentity).toBeUndefined();
        workbench.navigateProvenance(0); expect(workbench.snapshot().selection).toMatchObject({ byteStart: 1, byteLength: 3 });
    });

    it('preserves ScssOrigin module frames from a real native CSS graph without changing their kind or ranges', async () => {
        const response = await fetch(new URL('../generated/feature-tour/examples/scss-stylesheet.cem-studio.json', import.meta.url));
        expect(response.ok).toBe(true);
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const module = bundle.project.resources.find(resource => resource.id === 'scss-tokens');
        const original = `// café module\n${new TextDecoder().decode(bundle.contents[module.id])}`;
        const bytes = new TextEncoder().encode(original);
        bundle.contents[module.id] = bytes; module.sha256 = await digest(bytes);
        const repository = await repositoryWithSource('unrelated');
        await repository.execute(request('import-project', { bundle }));
        const validator = await createCemStudioBrowserValidator();
        try {
            const workbench = await createCemStudioProjectWorkbench({ repository, validator,
                selection: { projectId: bundle.project.id, entryId: 'build-html' } }); workbenches.push(workbench);
            await workbench.runPersistedOperation();
            const artifact = workbench.snapshot().projection.buildRun.artifacts.find(item => item.path.endsWith('.css'));
            const frames = artifact.provenance.filter(frame => frame.uri === `${bundle.project.rootUri}${module.path}`);
            expect(frames.length).toBeGreaterThan(0);
            expect(artifact.provenance.length).toBeLessThanOrEqual(CEM_STUDIO_LIMITS.structuredRows);
            for (const frame of frames) {
                expect(frame.transform).toBe('ScssOrigin');
                expect(frame.originKind).toBeTruthy();
                expect(frame.outputUri).toBe(artifact.uri);
                expect(frame.outputRange.start + frame.outputRange.len).toBeLessThanOrEqual(artifact.byteLength);
                expect(frame.range.start + frame.range.len).toBeLessThanOrEqual(bytes.byteLength);
                expect(() => new TextDecoder('utf-8', { fatal: true }).decode(bytes.slice(frame.range.start, frame.range.start + frame.range.len))).not.toThrow();
            }
            expect(frames.some(frame => frame.originKind === 'definition'
                && new TextDecoder().decode(bytes.slice(frame.range.start, frame.range.start + frame.range.len)).includes('$accent'))).toBe(true);
            expect(workbench.snapshot().projection.buildRun.inputs).toEqual(expect.arrayContaining([
                expect.objectContaining({ resourceId: module.id, revision: 1, sha256: module.sha256 }),
            ]));
        } finally { await validator.close(); }
    }, 30_000);

    it('normalizes ScssOrigin source-map identity and follows its stable module address after a rename', async () => {
        const { source, session, workbench } = await provenanceFixture('studio://alpha/extra.cem', { start: 9, len: 11 }, 'ScssOrigin');
        await workbench.editStructure({ kind: 'rename-resource', id: 'extra', path: 'styles/_tokens.scss' });
        await workbench.parsePersisted('ast');
        expect(workbench.snapshot().projection.provenance[0]).toMatchObject({ transform: 'ScssOrigin', uri: 'studio://alpha/extra.cem',
            originKind: 'definition', name: '$accent', range: { start: 9, len: 11 } });
        await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError).toBeUndefined();
        expect(session.snapshot().selection).toEqual({ projectId: 'alpha', resourceId: 'extra' });
        expect(workbench.snapshot().path).toBe('styles/_tokens.scss');
        const selected = workbench.snapshot().selection;
        expect(selected).toMatchObject({ kind: 'provenance', byteStart: 9, byteLength: 11 });
        expect(source.slice(selected.start, selected.end)).toBe('{p | target');
    });

    it.each(['cancel', 'discard', 'save'])('retains source-draft guards for ScssOrigin navigation: %s', async decision => {
        const { session, workbench } = await provenanceFixture('studio://alpha/extra.cem', { start: 9, len: 11 }, 'ScssOrigin');
        await workbench.parsePersisted('ast'); workbench.updateDraft('retained graph draft');
        await workbench.navigateProvenance(0);
        expect(session.snapshot().transition.stage).toBe('source');
        await workbench.resolveNavigation(decision);
        if (decision === 'cancel') {
            expect(session.snapshot().selection.resourceId).toBe('source'); expect(workbench.snapshot().draft).toBe('retained graph draft');
        } else {
            expect(session.snapshot().selection.resourceId).toBe('extra');
            if (decision === 'discard') expect(workbench.snapshot().selection?.kind).toBe('provenance');
            else { expect(workbench.snapshot().selection).toBeUndefined(); expect(workbench.snapshot().navigationError.message).toContain('stale'); }
        }
    });

    it('retains project ownership and stale-revision refusals for ScssOrigin module references', async () => {
        const { repository, session, workbench } = await provenanceFixture('studio://other/extra.cem', { start: 9, len: 11 }, 'ScssOrigin');
        await workbench.parsePersisted('ast'); await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError.message).toContain('not an available resource');
        expect(session.snapshot().selection.resourceId).toBe('source');
        await repository.execute(request('save-resource', { projectId: 'alpha', resourceId: 'extra', expectedProjectRevision: 1, expectedResourceRevision: 1, content: 'changed' }));
        await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError.message).toContain('stale');
        expect(session.snapshot().selection.resourceId).toBe('source'); await workbench.refreshNavigation();
    });

    it.each([{ start: 100, len: 2 }, { start: 6, len: 1 }, { start: 9, len: -1 }])('retains ScssOrigin original UTF-8 range bounds: %j', async range => {
        const { workbench } = await provenanceFixture('studio://alpha/extra.cem', range, 'ScssOrigin');
        await workbench.parsePersisted('ast'); await workbench.navigateProvenance(0);
        expect(workbench.snapshot().selection).toBeUndefined(); expect(workbench.snapshot().navigationError.message).toMatch(/outside|UTF-8/);
    });

    it('follows named source provenance using stable resolver addresses after a display-path rename', async () => {
        const { source, session, workbench } = await provenanceFixture();
        await workbench.editStructure({ kind: 'rename-resource', id: 'extra', path: 'moved/view.cemt' });
        await workbench.parsePersisted('ast');
        await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError).toBeUndefined();
        expect(session.snapshot().selection).toEqual({ projectId: 'alpha', resourceId: 'extra' });
        expect(workbench.snapshot().path).toBe('moved/view.cemt');
        expect(workbench.snapshot().navigation.selection.file).toBe('extra');
        expect(workbench.snapshot().navigation.title).toBe('alpha · moved/view.cemt - CEM Studio');
        const selected = workbench.snapshot().selection;
        expect(selected).toMatchObject({ kind: 'provenance', byteStart: 9, byteLength: 11 });
        expect(source.slice(selected.start, selected.end)).toBe('{p | target');
    });

    it.each(['cancel', 'discard', 'save'])('guards provenance navigation with a source draft: %s', async decision => {
        const { session, workbench } = await provenanceFixture();
        await workbench.parsePersisted('ast');
        workbench.updateDraft('retained graph draft');
        await workbench.navigateProvenance(0);
        expect(session.snapshot().transition.stage).toBe('source');
        await workbench.resolveNavigation(decision);
        if (decision === 'cancel') {
            expect(session.snapshot().selection.resourceId).toBe('source');
            expect(workbench.snapshot().draft).toBe('retained graph draft');
        } else {
            expect(session.snapshot().selection.resourceId).toBe('extra');
            expect(workbench.snapshot().navigation.selection.file).toBe('extra');
            if (decision === 'discard') expect(workbench.snapshot().selection?.kind).toBe('provenance');
            else {
                expect(workbench.snapshot().selection).toBeUndefined();
                expect(workbench.snapshot().navigationError.message).toContain('stale');
            }
        }
    });

    it('stops a disposed navigation refresh before it can reopen project storage', async () => {
        const { repository, workbench } = await provenanceFixture();
        const listed = await repository.query(request('list-projects', {}));
        const pending = deferred();
        const query = vi.spyOn(repository, 'query').mockImplementationOnce(() => pending.promise);
        const refreshing = workbench.refreshNavigation();
        workbench.dispose();
        pending.resolve(listed); await refreshing;
        expect(query).toHaveBeenCalledTimes(1);
        query.mockRestore();
    });

    it('refuses foreign resource identities and stale generated-source revisions', async () => {
        const { repository, session, workbench } = await provenanceFixture('studio://other/extra.cem');
        await workbench.parsePersisted('ast'); await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError.message).toContain('not an available resource');
        expect(session.snapshot().selection.resourceId).toBe('source');
        await repository.execute(request('save-resource', { projectId: 'alpha', resourceId: 'extra', expectedProjectRevision: 1, expectedResourceRevision: 1, content: 'changed' }));
        await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError.message).toContain('stale');
        expect(session.snapshot().selection.resourceId).toBe('source');
        // Finish the save-triggered navigation read before closing this fixture's database.
        await workbench.refreshNavigation();
    });

    it('preserves a native zero-length source location as an editor caret at EOF', async () => {
        const { source, workbench } = await provenanceFixture('studio://alpha/extra.cem', { start: 21, len: 0 });
        await workbench.parsePersisted('ast'); await workbench.navigateProvenance(0);
        expect(workbench.snapshot().navigationError).toBeUndefined();
        expect(workbench.snapshot().selection).toMatchObject({ kind: 'provenance', start: source.length, end: source.length, byteStart: 21, byteLength: 0 });
    });

    it.each([{ start: 100, len: 2 }, { start: 6, len: 1 }, { start: 9, len: -1 }])('refuses unavailable or split UTF-8 provenance ranges: %j', async range => {
        const { workbench } = await provenanceFixture('studio://alpha/extra.cem', range);
        await workbench.parsePersisted('ast'); await workbench.navigateProvenance(0);
        expect(workbench.snapshot().selection).toBeUndefined();
        expect(workbench.snapshot().navigationError.message).toMatch(/outside|UTF-8/);
    });

    it('refreshes a renamed resource from the dataset while retaining selection and its resolver address', async () => {
        const repository = await fixture();
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        const before = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        await workbench.editStructure({ kind: 'rename-resource', id: 'source', path: 'moved/renamed.cem' });
        expect(workbench.snapshot().structure.error).toBeUndefined();
        expect(workbench.snapshot()).toMatchObject({ resourceId: 'source', path: 'moved/renamed.cem' });
        expect(session.snapshot().selection).toEqual({ projectId: 'alpha', resourceId: 'source' });
        const after = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        expect(after.contents).toEqual(before.contents);
        expect(after.project.resources.find(item => item.id === 'source').referencePath)
            .toBe(before.project.resources.find(item => item.id === 'source').referencePath);
        expect(workbench.snapshot().explorer.nodes.flatMap(project => project.children ?? []).length).toBeGreaterThan(0);
        await workbench.editStructure({ kind: 'undo' });
        expect(workbench.snapshot().path).toBe(before.project.resources.find(item => item.id === 'source').path);
    });

    it('edits structure with bounded undo/redo and guards drafts and divergent revisions', async () => {
        const repository = await fixture();
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        workbench.updateDraft('retained draft');
        await workbench.editStructure({ kind: 'rename-project', name: 'Must not commit' });
        expect(workbench.snapshot().structure.error.message).toContain('Save/Apply');
        expect((await repository.query(request('get-project', { projectId: 'alpha' }))).value.name).toBe('alpha');
        for (const kind of ['rename-resource', 'delete-resource']) {
            await workbench.editStructure({ kind, id: 'source', path: 'moved.cem' });
            expect(workbench.snapshot().structure.error.message).toContain('Save/Apply');
            expect(workbench.snapshot().draft).toBe('retained draft');
        }
        expect((await repository.query(request('get-project', { projectId: 'alpha' }))).value.revision).toBe(1);
        await workbench.reload();
        await workbench.editStructure({ kind: 'rename-project', name: 'Edited Alpha' });
        expect(workbench.snapshot().structure).toMatchObject({ name: 'Edited Alpha', canUndo: true });
        expect(workbench.snapshot().projectRevision).toBe(2);
        await workbench.editStructure({ kind: 'create-resource', path: 'data/new.cem', content: '{p | New source}' });
        expect(workbench.snapshot().structure.resources.some(resource => resource.path === 'data/new.cem')).toBe(true);
        await workbench.editStructure({ kind: 'undo' });
        expect(workbench.snapshot().structure.resources.some(resource => resource.path === 'data/new.cem')).toBe(false);
        await workbench.editStructure({ kind: 'redo' });
        expect(workbench.snapshot().structure.resources.some(resource => resource.path === 'data/new.cem')).toBe(true);
        const selection = workbench.snapshot().workbenches.find(item => item.selection.projectId === 'feature-tour' && item.selection.resourceId === 'source');
        await workbench.selectWorkbench(selection.id);
        expect(workbench.snapshot().structure.canUndo).toBe(false);
        await workbench.selectWorkbench(JSON.stringify({ projectId: 'alpha', resourceId: 'source' }));
        expect(workbench.snapshot().structure.canUndo).toBe(true);
        workbench.updateDraft('new durable content'); await workbench.saveSource();
        expect(workbench.snapshot().structure.canUndo).toBe(false);
        await workbench.editStructure({ kind: 'undo' });
        expect(workbench.snapshot().structure.error.message).toContain('History is unavailable');
        expect(workbench.snapshot().persistedText).toBe('new durable content');
    });

    it('retains edits made during a structural commit and bounds undo history to twenty revisions', async () => {
        const repository = await fixture();
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        const execute = repository.execute.bind(repository);
        const spy = vi.spyOn(repository, 'execute').mockImplementationOnce(async request => {
            const result = await execute(request);
            workbench.updateDraft('typed during commit');
            return result;
        });
        await workbench.editStructure({ kind: 'rename-project', name: 'Committed name' });
        expect(session.snapshot().transition.stage).toBe('source');
        expect(workbench.snapshot().draft).toBe('typed during commit');
        await workbench.resolveNavigation('cancel');
        expect(workbench.snapshot().draft).toBe('typed during commit');
        spy.mockRestore();
        await workbench.reload();
        for (let index = 0; index < 21; index++) await workbench.editStructure({ kind: 'rename-project', name: `Name ${index}` });
        for (let index = 0; index < 20; index++) await workbench.editStructure({ kind: 'undo' });
        expect(workbench.snapshot().structure).toMatchObject({ name: 'Name 0', canUndo: false, canRedo: true });
    });

    it('recovers admitted identities and exact bytes, retaining drafts through external trash and restore', async () => {
        const repository = await fixture();
        const validator = validatorFor(validResult());
        const records = new Map();
        const storage = { getItem: key => records.get(key) ?? null, setItem: (key, value) => { records.set(key, value); } };
        const openWorkbench = selection => createCemStudioProjectWorkbench({ repository, validator, selection });
        const session = createCemStudioProjectSession({ openWorkbench, storage }); workbenches.push(session);
        await session.recover({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        const exact = 'saved bytes\r\n{div | café}\n';
        workbench.updateDraft(exact); await workbench.saveSource();
        workbench.updateDraft('uncommitted');
        await session.select({ projectId: 'feature-tour', resourceId: 'source' });
        session.cancelTransition(session.snapshot().transition.id);
        expect(JSON.parse(records.get('cem-studio-selection-v1'))).toEqual({ projectId: 'alpha', resourceId: 'source' });
        const restored = createCemStudioProjectSession({ openWorkbench, storage }); workbenches.push(restored);
        await restored.recover();
        expect(restored.activeWorkbench().snapshot().persistedText).toBe(exact);
        expect(restored.activeWorkbench().snapshot()).toMatchObject({ dirty: true, draft: 'uncommitted' });
        await workbench.searchProjects('alpha');
        expect(workbench.snapshot().explorer.search.results.length).toBeGreaterThan(0);
        const remote = createCemStudioProjectRepository({ databaseName: repository.databaseName, validateProject: async bundle => bundle });
        repositories.push(remote); await remote.open();
        const revision = workbench.snapshot().projectRevision;
        await remote.execute(request('trash-project', { projectId: 'alpha', expectedRevision: revision }));
        await vi.waitFor(() => expect(workbench.snapshot().navigationError?.code).toBe('cem.studio.selection.unavailable'));
        await vi.waitFor(() => expect(workbench.snapshot().explorer.search.status).toBe('ready'));
        expect(workbench.snapshot().draft).toBe('uncommitted');
        expect(workbench.snapshot().navigationError.code).toBe('cem.studio.selection.unavailable');
        expect(workbench.snapshot().explorer.search.results).toHaveLength(0);
        await expect(workbench.saveSource()).rejects.toMatchObject({ code: 'cem.studio.repository.project_trashed' });
        const fallback = createCemStudioProjectSession({ openWorkbench, storage }); workbenches.push(fallback);
        await fallback.recover({ projectId: 'feature-tour', resourceId: 'source' });
        expect(fallback.snapshot().selection.projectId).toBe('feature-tour');
        expect(fallback.snapshot().recoveryError.message).toContain('Trash');
        await workbench.setProjectTrashed('alpha', false);
        expect(workbench.snapshot().navigationError.code).toBe('cem.studio.selection.changed');
        expect(workbench.snapshot().draft).toBe('uncommitted');
        expect(workbench.snapshot().explorer.search.results.length).toBeGreaterThan(0);
        await expect(workbench.saveSource()).rejects.toMatchObject({ code: 'cem.studio.repository.revision_conflict' });
        expect(workbench.snapshot().draft).toBe('uncommitted');
        const exported = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        expect(new TextDecoder().decode(exported.contents.source)).toBe(exact);
        records.set('cem-studio-selection-v1', JSON.stringify({ projectId: 'missing', resourceId: 'source' }));
        const missing = createCemStudioProjectSession({ openWorkbench, storage }); workbenches.push(missing);
        await missing.recover({ projectId: 'alpha', resourceId: 'source' });
        expect(missing.snapshot().selection.projectId).toBe('alpha');
    });

    async function navigatorFixture(url = 'https://studio.test/') {
        const repository = await fixture('alpha', false, true, 'nested source', 'templates/café forms/view.cemt');
        const bundle = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        const examples = [{ id: 'login', name: 'Login', collections: ['amsterdam', 'examples'], description: 'Semantic login forms', load: vi.fn(async () => structuredClone(bundle)) }];
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator: validatorFor(validResult()), selection }) });
        workbenches.push(session);
        const base = await createCemStudioSessionWorkbench({ repository, session, home: { starter: bundle, examples } });
        const browser = new EventTarget();
        browser.location = new URL(url);
        browser.history = { pushState: vi.fn((_, __, url) => { browser.location = new URL(url); }), replaceState: vi.fn((_, __, url) => { browser.location = new URL(url); }) };
        const workbench = await createCemStudioNavigation({ workbench: base, repository, examples, collections: [{ id: 'amsterdam', name: 'Amsterdam', description: 'Presentation examples' }], browser });
        workbenches.push(workbench);
        return { repository, session, workbench, browser, examples };
    }

    it('models default samples, project folders and read-only sample files without creating saved projects', async () => {
        const { repository, workbench, browser, examples } = await navigatorFixture();
        expect(workbench.snapshot().navigation).toMatchObject({ title: 'Examples - CEM Studio', selection: { key: 'samples' }, count: 2, countKind: 'folder', content: 'folder' });
        expect(workbench.snapshot().navigation.children[0]).toMatchObject({ name: 'Amsterdam', description: 'Presentation examples', icon: 'folder' });
        const before = (await repository.query(request('list-projects', {}))).value;
        await workbench.openEntity('collection=amsterdam');
        expect(workbench.snapshot().navigation).toMatchObject({ count: 1, countKind: 'project' });
        await workbench.openEntity('sample=login');
        expect(examples[0].load).toHaveBeenCalledOnce();
        expect(workbench.snapshot().navigation.files.name).toBe('alpha');
        await workbench.openEntity('sample=login&folder=templates%2Fcaf%C3%A9+forms%2F');
        expect(workbench.snapshot().navigation.children[0]).toMatchObject({ name: 'view.cemt', resourceId: 'extra', kind: 'file' });
        await workbench.openEntity('sample=login&file=extra');
        expect(workbench.snapshot().navigation).toMatchObject({ content: 'sample-file', sampleText: 'nested source', count: 1, countKind: 'file', title: 'alpha · templates/café forms/view.cemt - CEM Studio', canDelete: false });
        expect(new URL(browser.location).searchParams.get('file')).toBe('extra');
        expect((await repository.query(request('list-projects', {}))).value).toEqual(before);
        expect(examples[0].load).toHaveBeenCalledOnce();
    });

    it('admits URL resource IDs, preserves Unicode folder paths and cancels history navigation without losing drafts', async () => {
        const { workbench, browser, session } = await navigatorFixture('https://studio.test/?project=alpha&file=extra');
        expect(session.snapshot().selection).toEqual({ projectId: 'alpha', resourceId: 'extra' });
        expect(workbench.snapshot().navigation.expanded).toContain('folder=templates%2F');
        workbench.updateDraft('retain this source');
        const accepted = browser.location.href;
        browser.location = new URL('https://studio.test/?project=alpha&file=source');
        browser.dispatchEvent(new Event('popstate'));
        await vi.waitFor(() => expect(session.snapshot().transition?.stage).toBe('source'));
        expect(workbench.snapshot().navigation.selection.file).toBe('extra');
        await workbench.resolveNavigation('cancel');
        expect(browser.location.href).toBe(accepted);
        expect(workbench.snapshot().draft).toBe('retain this source');
        await workbench.openEntity('project=alpha&file=source');
        await workbench.resolveNavigation('discard');
        expect(workbench.snapshot().navigation.selection.file).toBe('source');
        expect(new URL(browser.location).searchParams.get('file')).toBe('source');
        await workbench.openEntity('project=alpha&file=missing');
        expect(workbench.snapshot().navigation.error.message).toContain('unavailable');
        expect(workbench.snapshot().navigation.selection.file).toBe('source');
        expect(workbench.snapshot().draft).toBe('project source');
    });

    it('renders model-owned trees and folder listings, clones samples and creates/deletes files through the action bar', async () => {
        const { repository, workbench, session } = await navigatorFixture();
        const root = document.createElement('main');
        root.style.cssText = '--cem-gap-related: 8px; --cem-gap-group: 16px; --cem-layout-stack-gap: 16px; --cem-coupling-guard-min: 8px';
        document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const treeNode = label => [...root.querySelectorAll('[role=treeitem]')].find(item => item.getAttribute('aria-label') === label);
        const actions = root.querySelector('[aria-label="Selection actions"]');
        expect(actions.closest('[part~=heading]').querySelector('h2').textContent).toBe('Navigation');
        const count = actions.querySelector('[aria-live="polite"]');
        const buttons = [...actions.querySelectorAll('cem-icon-button')];
        expect(actions.firstElementChild).toBe(count);
        expect(count.textContent.trim()).toBe('2 folders');
        expect(buttons.every(button => button.getAttribute('variant') === 'contextual')).toBe(true);
        expect(getComputedStyle(actions).columnGap).toBe('8px');
        for (let index = 1; index < buttons.length; index++) {
            expect(buttons[index].getBoundingClientRect().left - buttons[index - 1].getBoundingClientRect().right).toBeGreaterThanOrEqual(8);
        }
        expect(root.querySelectorAll('[host-command="create-selected"]')).toHaveLength(1);
        const { userEvent } = await import('vitest/browser');
        await userEvent.click(treeNode('Amsterdam')); await view.whenSettled();
        expect(root.querySelector('[aria-label="Folder contents"]').textContent).toContain('Login');
        await userEvent.click(treeNode('Login')); await view.whenSettled();
        expect(workbench.snapshot().navigation.selection.sample).toBe('login');
        const filesTree = root.querySelector('cem-tree[label="Project files"]');
        const assertFileTreeLabels = item => {
            const label = item.name;
            const rendered = [...filesTree.querySelectorAll('[role=treeitem]')].find(element => element.getAttribute('data-tree-value') === item.key);
            expect(rendered.getAttribute('aria-label')).toBe(label);
            expect(rendered.querySelector('[part~=label]').textContent).toBe(label);
            expect(rendered.querySelector('[part~=icon]').textContent).toBe(item.kind === 'folder' || item.kind === 'project' ? '📁' : item.kind === 'configuration' ? '▷' : '▤');
            item.children.forEach(assertFileTreeLabels);
        };
        assertFileTreeLabels(workbench.snapshot().navigation.files);
        expect(treeNode('Login')).toBeDefined();
        root.querySelector('[host-command="clone-selected"] button').click(); await view.whenSettled();
        expect(root.querySelector('dialog[open] h2').textContent).toBe('clone · alpha');
        const form = root.querySelector('form[host-command="submit-action"]');
        const field = form.querySelector('cem-text-field');
        const formActions = form.querySelector('[part~="form-actions"]');
        expect(formActions.getBoundingClientRect().top - field.getBoundingClientRect().bottom).toBeGreaterThanOrEqual(15);
        expect(getComputedStyle(formActions.querySelector('[part~=root]')).columnGap).toBe('8px');
        const submit = async name => {
            const input = root.querySelector('cem-dialog cem-text-field input');
            if (input) { input.value = name; input.dispatchEvent(new InputEvent('input', { bubbles: true })); }
            root.querySelector('form[host-command="submit-action"]').requestSubmit(); await view.whenSettled();
        };
        await submit('My Login');
        const id = workbench.snapshot().navigation.selection.project;
        expect(id).toMatch(/^project-/); expect(workbench.snapshot().navigation.title).toBe('My Login - CEM Studio');
        root.querySelector('[host-command="create-selected"] button').click(); await view.whenSettled();
        expect(root.querySelector('dialog[open] h2').textContent).toBe('create · My Login');
        await submit('new.cem');
        expect(workbench.snapshot().navigation).toMatchObject({ content: 'file', selected: { name: 'new.cem' } });
        const resourceId = session.snapshot().selection.resourceId;
        expect(workbench.snapshot().navigationError).toBeUndefined();
        expect(root.querySelector('[data-cem-studio-editor] [role="textbox"]')).not.toBeNull();
        root.querySelector('[host-command="delete-selected"] button').click(); await view.whenSettled();
        expect(root.querySelector('dialog[open] h2').textContent).toBe('delete · new.cem');
        await submit();
        expect(workbench.snapshot().navigation.content).toBe('project');
        expect(workbench.snapshot().navigationError).toBeUndefined();
        expect((await repository.query(request('get-project', { projectId: id }))).value.resources.some(item => item.id === resourceId)).toBe(false);
        expect((await repository.query(request('get-project', { projectId: 'alpha' }))).value.resources).toHaveLength(3);
    });

    it('ignores a superseded sample load failure and keeps the accepted URL and model selection', async () => {
        const { workbench, browser, examples } = await navigatorFixture();
        const pending = deferred();
        examples[0].load.mockImplementationOnce(async () => { await pending.promise; throw new Error('Late sample failure'); });
        const opening = workbench.openEntity('sample=login');
        await workbench.openEntity('view=projects');
        pending.resolve(); await opening;
        expect(workbench.snapshot().navigation).toMatchObject({ selection: { view: 'projects' }, content: 'folder' });
        expect(workbench.snapshot().navigation.error).toBeUndefined();
        expect(new URL(browser.location).searchParams.get('view')).toBe('projects');
    });

    it('rejects invalid startup links while retaining the samples root and a recoverable error', async () => {
        const { workbench } = await navigatorFixture('https://studio.test/?project=missing&file=unknown');
        expect(workbench.snapshot().navigation).toMatchObject({ selection: { key: 'samples' }, content: 'folder', error: { message: 'The requested project is missing or in Trash.' } });
        await workbench.openEntity('samples');
        expect(workbench.snapshot().navigation.error).toBeUndefined();
    });

    it('composes Home creation and archive import with guarded project admission', async () => {
        const repository = await fixture();
        const starter = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        starter.project.entries = [];
        const archive = structuredClone(starter); archive.project.id = 'imported'; archive.project.rootUri = 'studio://imported/';
        const selectImport = vi.fn(async () => serializeCemStudioProjectBundle(archive));
        const provider = createCemStudioFileSystemProvider({ repository });
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session, home: { starter, provider, selectImport } });
        workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        await workbench.createProject('My project'); await view.whenSettled();
        expect(workbench.snapshot().home.error).toBeUndefined();
        const created = workbench.snapshot().home.projects.find(item => item.name === 'My project');
        expect(created).toBeDefined();
        expect(JSON.parse(created.id).projectId).not.toBe('alpha');
        await workbench.setProjectTrashed(JSON.parse(created.id).projectId, true); await view.whenSettled();
        expect(workbench.snapshot().home.trash.some(item => item.id === JSON.parse(created.id).projectId)).toBe(true);
        await workbench.setProjectTrashed(JSON.parse(created.id).projectId, false); await view.whenSettled();
        expect(workbench.snapshot().home.trash).toHaveLength(0);
        expect(workbench.snapshot().home.projects.some(item => item.id === created.id)).toBe(true);
        workbench.updateDraft('keep this draft');
        await workbench.selectWorkbench(created.id); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('source');
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        expect(workbench.snapshot().draft).toBe('keep this draft');
        await workbench.importProject(); await view.whenSettled();
        expect(workbench.snapshot().home.projects.some(item => JSON.parse(item.id).projectId === 'imported')).toBe(true);
        const before = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        await workbench.importProject();
        expect(workbench.snapshot().home.error).toBeDefined();
        await workbench.createProject('  ');
        expect(workbench.snapshot().home.error.message).toContain('project name');
        const execute = vi.spyOn(repository, 'execute').mockRejectedValueOnce(new DOMException('Storage quota exceeded', 'QuotaExceededError'));
        await workbench.createProject('Quota failure'); execute.mockRestore();
        expect(workbench.snapshot().home.error.message).toContain('quota');
        expect((await repository.query(request('export-project', { projectId: 'alpha' }))).value).toEqual(before);
        selectImport.mockResolvedValueOnce(new TextEncoder().encode('invalid archive'));
        await workbench.importProject(); expect(workbench.snapshot().home.error).toBeDefined();
        selectImport.mockRejectedValueOnce(new DOMException('Cancelled', 'AbortError'));
        await workbench.importProject(); expect(workbench.snapshot().home.error).toBeUndefined();
        expect(workbench.snapshot().draft).toBe('keep this draft');
        const imported = workbench.snapshot().home.projects.find(item => JSON.parse(item.id).projectId === 'imported');
        await workbench.selectWorkbench(imported.id);
        await workbench.resolveNavigation('discard');
        expect(session.snapshot().selection.projectId).toBe('imported');
    });

    it('creates independent example project copies declaratively without replacing active drafts', async () => {
        const repository = await fixture();
        const original = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        const load = vi.fn(async () => original);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session, home: { examples: [{ id: 'arbitrary-project', name: 'Example project', load }] } });
        workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        workbench.updateDraft('keep this draft'); await view.whenSettled();
        const validate = [...root.querySelectorAll('button')].find(button => button.textContent.trim() === 'Validate persisted revision');
        expect(validate).toBeDefined(); validate.click(); await view.whenSettled();
        expect(workbench.snapshot().validation.stale).toBe(true);
        expect(workbench.snapshot().draft).toBe('keep this draft');
        expect((await repository.query(request('export-project', { projectId: 'alpha' }))).value.contents).toEqual(original.contents);
        await workbench.createExampleProject('arbitrary-project'); await view.whenSettled();
        expect(workbench.snapshot().home.error).toBeUndefined();
        const first = workbench.snapshot().home.projects.find(project => project.name === 'Example project');
        expect(first).toBeDefined();
        expect(workbench.snapshot().draft).toBe('keep this draft');
        expect(session.snapshot().selection.projectId).toBe('alpha');
        const copied = (await repository.query(request('export-project', { projectId: JSON.parse(first.id).projectId }))).value;
        expect(copied.contents).toEqual(original.contents);
        expect(copied.project.resources.map(resource => resource.id)).toEqual(original.project.resources.map(resource => resource.id));
        await workbench.createExampleProject('arbitrary-project'); await view.whenSettled();
        const second = workbench.snapshot().home.projects.find(project => project.name === 'Example project (2)');
        expect(second).toBeDefined(); expect(second.id).not.toBe(first.id);
        await workbench.createExampleProject('missing');
        expect(workbench.snapshot().home.error).toBeDefined();
        await workbench.selectWorkbench(first.id); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('source');
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        expect(workbench.snapshot().draft).toBe('keep this draft');
        await workbench.selectWorkbench(first.id); await workbench.resolveNavigation('discard');
        const resource = copied.project.resources.find(resource => resource.id === 'source');
        await workbench.selectWorkbench(JSON.stringify({ projectId: copied.project.id, resourceId: resource.id }));
        workbench.updateDraft('sample copy edit'); await workbench.saveSource();
        const saved = (await repository.query(request('export-project', { projectId: copied.project.id }))).value;
        expect(new TextDecoder().decode(saved.contents.source)).toBe('sample copy edit');
        expect((await repository.query(request('export-project', { projectId: JSON.parse(second.id).projectId }))).value.contents).toEqual(original.contents);
        expect((await repository.query(request('export-project', { projectId: 'alpha' }))).value.contents).toEqual(original.contents);
        expect(load).toHaveBeenCalledTimes(2);
    });

    it('guards repeated example project creation and reports loading failures without creating a project', async () => {
        const repository = await fixture();
        const original = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        const projects = (await repository.query(request('list-projects', {}))).value;
        const pending = deferred(); const load = vi.fn(async () => { await pending.promise; throw new Error('Sample unavailable'); });
        const session = createCemStudioProjectSession({ openWorkbench: async () => { throw new Error('unused'); } });
        workbenches.push(session);
        const workbench = await createCemStudioSessionWorkbench({ repository, session, home: { examples: [{ id: 'arbitrary-project', name: 'Example project', load }] } });
        workbenches.push(workbench);
        const creating = workbench.createExampleProject('arbitrary-project');
        await workbench.createExampleProject('arbitrary-project');
        expect(load).toHaveBeenCalledTimes(1);
        pending.resolve(); await creating;
        expect(workbench.snapshot().home.error.message).toBe('Sample unavailable');
        expect((await repository.query(request('list-projects', {}))).value).toEqual(projects);
        expect((await repository.query(request('export-project', { projectId: 'alpha' }))).value).toEqual(original);
    });

    it('presents an empty Home and unavailable storage without requiring an active project', async () => {
        const repository = createCemStudioProjectRepository({ databaseName: `cem-home-${crypto.randomUUID()}`, validateProject: async bundle => bundle });
        repositories.push(repository);
        const session = createCemStudioProjectSession({ openWorkbench: async () => { throw new Error('unused'); } });
        workbenches.push(session);
        const workbench = await createCemStudioSessionWorkbench({ repository, session, home: {} });
        workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        expect(workbench.snapshot().home.projects).toHaveLength(0);
        const query = vi.spyOn(repository, 'query').mockRejectedValueOnce(new Error('Storage unavailable'));
        await expect(workbench.refreshNavigation()).rejects.toThrow('Storage unavailable'); query.mockRestore();
        await view.whenSettled(); expect(root.textContent).toContain('Storage unavailable');
    });

    it('retains editor panes and admits recursive explorer activation independently of focus and expansion', async () => {
        const repository = await fixture('project with spaces', false, true);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'project with spaces', resourceId: 'source' });
        const base = await createCemStudioSessionWorkbench({ repository, session });
        const workbench = await createCemStudioNavigation({ workbench: base, repository }); workbenches.push(workbench);
        await workbench.openEntity('project=project+with+spaces&file=source');
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const tree = root.querySelector('cem-tree[label="Project files"]');
        const node = key => [...tree.querySelectorAll('[role=treeitem]')].find(item => item.dataset.treeValue === key);
        const current = 'project=project+with+spaces&file=source';
        const target = 'project=project+with+spaces&file=extra';
        const project = node('project=project+with+spaces');
        project.focus();
        const { userEvent } = await import('vitest/browser');
        await userEvent.keyboard('{ArrowRight}');
        expect(document.activeElement).not.toBe(project);
        expect(tree.getAttribute('selected-values')).toBe(current);
        workbench.updateDraft('draft with retained selection'); await view.whenSettled();
        const editor = root.querySelector('[data-cem-studio-editor] [role="textbox"]');
        editor.focus(); editor.setSelectionRange(2, 7);
        const tabs = root.querySelector('cem-tabs[label="Workbench panes"]');
        const buttons = [...tabs.querySelectorAll(':scope > [role="tablist"] [role="tab"], :scope > div > [role="tablist"] [role="tab"]')];
        const paneButtons = buttons.length ? buttons : [...tabs.querySelectorAll('[role="tab"]')].filter(item => item.closest('cem-tabs') === tabs);
        paneButtons.find(item => item.textContent === 'Results').click(); await view.whenSettled();
        expect(editor.closest('[role="tabpanel"]')).toBeNull();
        expect(editor.checkVisibility()).toBe(true);
        paneButtons.find(item => item.textContent === 'Source and command').click(); await view.whenSettled();
        expect(root.querySelector('[data-cem-studio-editor] [role="textbox"]')).toBe(editor);
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 7]);
        const launcher = node(target); launcher.focus(); launcher.click(); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('source');
        expect(tree.getAttribute('selected-values')).toBe(current);
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        expect(workbench.snapshot().draft).toBe('draft with retained selection');
        expect(node(target)).toBe(launcher);
        expect(document.activeElement).toBe(launcher);
        launcher.click(); await view.whenSettled();
        dialogAction(root, 'Discard draft').click(); await view.whenSettled();
        expect(session.snapshot().selection.resourceId).toBe('extra');
        expect(tree.getAttribute('selected-values')).toBe(target);
        expect([...tree.querySelectorAll('[role="treeitem"]')].every(item => !/\s/.test(item.dataset.treeValue))).toBe(true);
    }, 30000);

    it('searches repository records deterministically, ignores stale completions and guards result activation', async () => {
        const repository = await fixture('beta', false, true);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) }); workbenches.push(session);
        await session.select({ projectId: 'beta', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        await workbench.searchProjects('additional'); await view.whenSettled();
        const found = workbench.snapshot().explorer.search.results;
        expect(found).toHaveLength(1); expect(found[0].entityId).toBe('extra');
        await workbench.searchProjects('additional'); expect(workbench.snapshot().explorer.search.results).toEqual(found);
        workbench.updateDraft('search keeps draft'); await view.whenSettled();
        await workbench.openEntity(found[0].key); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('source');
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        let resolveOld;
        const original = repository.query.bind(repository);
        const spy = vi.spyOn(repository, 'query').mockImplementationOnce(() => new Promise(resolve => { resolveOld = resolve; }));
        const old = workbench.searchProjects('old');
        await workbench.searchProjects('additional');
        resolveOld({ value: [] }); await old;
        expect(workbench.snapshot().explorer.search.results).toEqual(found);
        spy.mockRestore();
        await workbench.searchProjects('zzzznotfound'); await view.whenSettled();
        expect(workbench.snapshot().explorer.search.results).toHaveLength(0);
        const failure = vi.spyOn(repository, 'query').mockRejectedValueOnce(new Error('Search unavailable'));
        await workbench.searchProjects('additional'); failure.mockRestore(); await view.whenSettled();
        expect(workbench.snapshot().explorer.search.error.message).toBe('Search unavailable');
        expect(workbench.snapshot().draft).toBe('search keeps draft');
        expect((await original(request('get-project', { projectId: 'beta' }))).value.id).toBe('beta');
    }, 30000);

    it('refreshes explorer and search after a native command creates a durable page', async () => {
        const repository = await fixture();
        const validator = await createCemStudioBrowserValidator();
        try {
            const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) }); workbenches.push(session);
            await session.select({ projectId: 'alpha', resourceId: 'source' });
            const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
            await workbench.searchProjects('New inspection');
            await workbench.parsePersisted('events');
            workbench.setCommandTarget({ mode: 'new', name: 'New inspection', parentId: 'schema-package-examples' });
            await workbench.applyCommand();
            const entryId = workbench.snapshot().command.application.result.entry.id;
            const key = encodeURIComponent(JSON.stringify({ projectId: 'alpha', entryId }));
            const flatten = nodes => nodes.flatMap(node => [node, ...flatten(node.children)]);
            expect(flatten(workbench.snapshot().explorer.nodes).some(node => node.key === key)).toBe(true);
            expect(workbench.snapshot().explorer.search.results.some(result => result.key === key)).toBe(true);
            await workbench.openEntity(key);
            expect(workbench.snapshot()).toMatchObject({ entryId, resourceId: 'source', runnable: true });
        } finally { await validator.close(); }
    }, 30000);

    it.each(schemaSourceAdmissionCases.flatMap(item => [
        [item.mediaType, item], [item.normalizedType, item],
    ]))('SchemaSourceAdmission saves and reopens native schema/template source for %s', async (contentType, item) => {
        const bytes = new TextEncoder().encode(item.source);
        const repository = await schemaSourceAdmissionRepository(contentType, bytes, item);
        const validator = await createCemStudioBrowserValidator();
        try {
            const file = await createCemStudioProjectWorkbench({ repository, validator,
                selection: { projectId: 'schema-source-admission', resourceId: 'source' } }); workbenches.push(file);
            expect(file.snapshot()).toMatchObject({ status: 'loaded', editable: true, runnable: false,
                contentType, persistedText: item.source, draft: item.source });
            const edited = item.source.replace('Café note', 'Saved note'); expect(edited).not.toBe(item.source);
            file.updateDraft(edited); await file.saveSource();
            expect(file.snapshot()).toMatchObject({ dirty: false, persistedText: edited });
            await expect(file.runPersistedOperation()).rejects.toThrow(/persisted executable command/);
            const configuration = await createCemStudioProjectWorkbench({ repository, validator,
                selection: { projectId: 'schema-source-admission', entryId: 'validate-source' } }); workbenches.push(configuration);
            expect(configuration.snapshot()).toMatchObject({ editable: true, runnable: true, resourceId: 'source', contentType, persistedText: edited });
            await configuration.runPersistedOperation();
            expect(configuration.snapshot()).toMatchObject({ status: 'projected', projection: { kind: 'validate', exitCode: 0, stale: false } });
            expect(configuration.snapshot().projection.diagnostics).toEqual([]);
            expect(configuration.snapshot().projection.sourceByteLength).toBe(new TextEncoder().encode(edited).byteLength);
            expect(configuration.snapshot().projection.revision.resourceRevision).toBe(configuration.snapshot().resourceRevision);
            const exported = (await repository.query(request('export-project', { projectId: 'schema-source-admission' }))).value;
            const storedBytes = new Uint8Array(exported.contents.source);
            expect(storedBytes).toEqual(new TextEncoder().encode(edited));
            let downloaded; await configuration.downloadSource(async value => { downloaded = value; });
            expect(downloaded.contentType).toBe(contentType); expect(downloaded.bytes).toEqual(storedBytes);
        } finally { await validator.close(); }
    }, 30_000);

    it.each(schemaSourceAdmissionCases.flatMap(item => [
        [item.mediaType, 'invalid UTF-8', new Uint8Array([0x3c, 0xff, 0x3e]), item],
        [item.mediaType, 'oversized UTF-8', new Uint8Array(CEM_STUDIO_LIMITS.sourceBytes + 1).fill(0x61), item],
        [item.mediaType, 'cached URL source', new TextEncoder().encode(item.source), { ...item, sourceKind: 'url' }],
    ]).concat([
        ...['ast', 'dom', 'events', 'query-artifact', 'xpath-artifact', 'xslt-bundle'].map(name => [
            `application/vnd.cem.${name}+cem-bin`, 'compiled/binary selector with UTF-8-compatible bytes', new TextEncoder().encode('CEM binary bytes'), schemaSourceAdmissionCases[0],
        ]),
        [' Application/Vnd.Cem.Query-Artifact+Cem-Bin ; version=1', 'normalized compiled selector', new TextEncoder().encode('CEM binary bytes'), schemaSourceAdmissionCases[0]],
        ['application/octet-stream', 'unknown binary media', new TextEncoder().encode('UTF-8 is insufficient'), schemaSourceAdmissionCases[0]],
        ['image/png', 'image media', new TextEncoder().encode('UTF-8 is insufficient'), schemaSourceAdmissionCases[0]],
    ]))('SchemaSourceAdmission refuses editing but retains exact downloads for %s: %s', async (contentType, _reason, bytes, item) => {
        const repository = await schemaSourceAdmissionRepository(contentType, bytes, item);
        const validator = validatorFor(validResult());
        for (const selection of [{ projectId: 'schema-source-admission', resourceId: 'source' },
            { projectId: 'schema-source-admission', entryId: 'validate-source', resourceId: 'source' }]) {
            const workbench = await createCemStudioProjectWorkbench({ repository, validator, selection }); workbenches.push(workbench);
            expect(workbench.snapshot()).toMatchObject({ status: 'read-only', editable: false, runnable: false,
                contentType, byteLength: bytes.byteLength, draft: '', persistedText: '' });
            expect(workbench.updateDraft).toBeUndefined(); expect(workbench.runPersistedOperation).toBeUndefined();
            let downloaded; await workbench.downloadSource(async value => { downloaded = value; });
            expect(downloaded.contentType).toBe(contentType); expect(downloaded.bytes.byteLength).toBe(bytes.byteLength);
            expect(await digest(downloaded.bytes)).toBe(await digest(bytes));
        }
        expect(validator.previewResourceCommand).not.toHaveBeenCalled();
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
    });

    it.each([
        ['Application/MathML+XML; charset=utf-8', '<mi>x</mi>'],
        [' application/mathml-presentation+xml ; charset="UTF-8"', '<mi>x</mi>'],
        ['APPLICATION/MATHML-CONTENT+XML; profile=content; charset=utf-8', '<ci>x</ci>'],
    ])('admits MathML text editing and explicit commands for normalized media essence: %s', async (contentType, token) => {
        const source = `<math xmlns="http://www.w3.org/1998/Math/MathML" alttext="Café formula">${token}</math>\n`;
        const repository = await mathmlAdmissionRepository(contentType, new TextEncoder().encode(source));
        const validator = await createCemStudioBrowserValidator();
        try {
            const file = await createCemStudioProjectWorkbench({ repository, validator,
                selection: { projectId: 'mathml-admission', resourceId: 'source' } }); workbenches.push(file);
            expect(file.snapshot()).toMatchObject({ status: 'loaded', editable: true, runnable: false,
                contentType, persistedText: source, draft: source });
            const edited = source.replace('>x<', '>y<');
            file.updateDraft(edited); expect(file.snapshot().dirty).toBe(true);
            await file.saveSource();
            expect(file.snapshot()).toMatchObject({ dirty: false, persistedText: edited });
            await expect(file.runPersistedOperation()).rejects.toThrow(/persisted executable command/);
            const configuration = await createCemStudioProjectWorkbench({ repository, validator,
                selection: { projectId: 'mathml-admission', entryId: 'inspect-mathml' } }); workbenches.push(configuration);
            expect(configuration.snapshot()).toMatchObject({ editable: true, runnable: true, resourceId: 'source', contentType, persistedText: edited });
            await configuration.runPersistedOperation();
            expect(configuration.snapshot()).toMatchObject({ status: 'projected', projection: { kind: 'inspect', exitCode: 0, stale: false } });
            expect(configuration.snapshot().projection.sourceByteLength).toBe(new TextEncoder().encode(edited).byteLength);
            expect(configuration.snapshot().projection.revision.resourceRevision).toBe(configuration.snapshot().resourceRevision);
        } finally { await validator.close(); }
    }, 30_000);

    it.each([
        'application/mathml+xml',
        'application/mathml-presentation+xml',
        'application/mathml-content+xml',
    ].flatMap(contentType => [
        [contentType, 'invalid UTF-8', new Uint8Array([0x3c, 0xff, 0x3e])],
        [contentType, 'oversized UTF-8', new Uint8Array(CEM_STUDIO_LIMITS.sourceBytes + 1).fill(0x61)],
    ]).concat([['application/octet-stream', 'binary media type', new TextEncoder().encode('<math/>')]]))(
        'keeps MathML editing bounds and exact downloads for %s: %s', async (contentType, _reason, bytes) => {
            const repository = await mathmlAdmissionRepository(contentType, bytes);
            const validator = validatorFor(validResult());
            for (const selection of [{ projectId: 'mathml-admission', resourceId: 'source' },
                { projectId: 'mathml-admission', entryId: 'inspect-mathml', resourceId: 'source' }]) {
                const workbench = await createCemStudioProjectWorkbench({ repository, validator, selection }); workbenches.push(workbench);
                expect(workbench.snapshot()).toMatchObject({ status: 'read-only', editable: false, runnable: false,
                    contentType, byteLength: bytes.byteLength, draft: '', persistedText: '' });
                expect(workbench.updateDraft).toBeUndefined(); expect(workbench.runPersistedOperation).toBeUndefined();
                let downloaded; await workbench.downloadSource(async value => { downloaded = value; });
                expect(downloaded.contentType).toBe(contentType); expect(downloaded.bytes.byteLength).toBe(bytes.byteLength);
                expect(await digest(downloaded.bytes)).toBe(await digest(bytes));
            }
            expect(validator.previewResourceCommand).not.toHaveBeenCalled();
            expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        });

    it('loads nested durable identities without a catalog and never infers a missing command', async () => {
        const repository = await fixture();
        const validator = validatorFor(validResult());
        const workbench = await createCemStudioProjectWorkbench({ repository, validator,
            selection: { projectId: 'alpha', entryId: 'validate-cem-ml', resourceId: 'source' } });
        workbenches.push(workbench);
        expect(workbench.snapshot()).toMatchObject({ projectId: 'alpha', entryId: 'validate-cem-ml',
            resourceId: 'source', draft: 'project source', editable: true });
        expect(workbench.snapshot().command).toBeUndefined();
        expect(validator.previewResourceCommand).not.toHaveBeenCalled();
        await expect(workbench.runPersistedOperation()).rejects.toThrow(/command/i);
        workbench.updateDraft('saved independently');
        await workbench.saveSource();
        expect(workbench.snapshot()).toMatchObject({ dirty: false, persistedText: 'saved independently' });
        await expect(createCemStudioProjectWorkbench({ repository, validator,
            selection: { projectId: 'alpha', entryId: 'missing' } })).rejects.toThrow(/missing/);
    });

    it('reopens a native command-created page from repository bytes and runs its exact command', async () => {
        const repository = await fixture('alpha', false, true);
        const validator = await createCemStudioBrowserValidator();
        try {
            const source = await createCemStudioProjectWorkbench({ repository, validator, selection: { projectId: 'alpha', resourceId: 'source' } });
            workbenches.push(source);
            await source.parsePersisted('events');
            source.setCommandTarget({ mode: 'new', name: 'Native events', parentId: 'schema-package-examples' });
            await source.applyCommand();
            const entryId = source.snapshot().command.application.result.entry.id;
            const reopened = await createCemStudioProjectWorkbench({ repository, validator, selection: { projectId: 'alpha', entryId } });
            workbenches.push(reopened);
            expect(reopened.snapshot()).toMatchObject({ entryId, resourceId: 'source', runnable: true });
            expect(reopened.snapshot().command.parsed.options.format).toBe('events');
            await reopened.runPersistedOperation();
            expect(reopened.snapshot().projection).toMatchObject({ kind: 'parse', mode: 'events', stale: false });
            const reordered = await repository.execute(request('apply-command-page', {
                projectId: 'alpha', expectedProjectRevision: reopened.snapshot().projectRevision,
                target: { mode: 'new', name: 'Reordered resources' },
                commandResource: validator.serializeResourceCommand(reopened.snapshot().command.parsed),
                referencedResourceIds: ['extra', 'source'],
            }));
            const ordered = await createCemStudioProjectWorkbench({ repository, validator,
                selection: { projectId: 'alpha', entryId: reordered.value.entry.id } });
            workbenches.push(ordered);
            expect(ordered.snapshot().resourceId).toBe('source');
        } finally { await validator.close(); }
    }, 30000);

    it('uses page-owned dependency identities and preserves binary downloads exactly', async () => {
        const repository = await fixture();
        const bundle = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        bundle.project.id = 'gamma'; bundle.project.rootUri = 'studio://gamma/';
        const bytes = new Uint8Array([0, 255, 1, 2]);
        bundle.project.resources.push({ id: 'binary', role: 'data', sourceKind: 'project-file', path: 'assets/picture.bin',
            contentType: 'application/octet-stream', schema: 'https://cem.dev/ns/data/xml/1', revision: 1, sha256: await digest(bytes) });
        bundle.contents.binary = bytes;
        bundle.project.entries.find(item => item.id === 'validate-cem-ml').resourceIds.push('binary');
        await repository.execute(request('import-project', { bundle }));
        const validator = validatorFor(validResult());
        const source = await createCemStudioProjectWorkbench({ repository, validator, selection: { projectId: 'gamma', entryId: 'validate-cem-ml' } });
        workbenches.push(source);
        await source.parsePersisted();
        expect(validator.parseResource.mock.calls[0][0].dependencies).toEqual([expect.objectContaining({ resourceId: 'binary', path: 'studio://gamma/assets/picture.bin', bytes })]);
        const binary = await createCemStudioProjectWorkbench({ repository, validator, selection: { projectId: 'gamma', resourceId: 'binary' } });
        workbenches.push(binary);
        expect(binary.snapshot()).toMatchObject({ status: 'read-only', editable: false, resourceId: 'binary', byteLength: 4 });
        let downloaded; await binary.downloadSource(async value => { downloaded = value; });
        expect(downloaded.bytes).toEqual(bytes);
    });

    it('keeps both drafts on failed Save and stale Apply through the mounted guard', async () => {
        const repository = await fixture('beta', true);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'feature-tour', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session });
        await workbench.parsePersisted();
        workbench.updateDraft('saved source');
        await workbench.updateCommandDraft(workbench.snapshot().command.current.text.replace('--format ast', '--format events'));
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const target = workbench.snapshot().workbenches.find(item => item.selection.projectId === 'beta');
        await workbench.selectWorkbench(target.id); await view.whenSettled();
        const execute = vi.spyOn(repository, 'execute'); execute.mockRejectedValueOnce(new Error('Save unavailable'));
        dialogAction(root, 'Save source').click(); await view.whenSettled();
        expect(session.snapshot()).toMatchObject({ status: 'failed', transition: { stage: 'source' }, workbench: { dirty: true } });
        expect(root.textContent).toContain('Save unavailable');
        dialogAction(root, 'Save source').click(); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('command');
        const loaded = workbench.snapshot();
        await repository.execute(request('save-resource', { projectId: loaded.projectId, resourceId: loaded.resourceId,
            expectedProjectRevision: loaded.projectRevision, expectedResourceRevision: loaded.resourceRevision, content: 'external change' }));
        dialogAction(root, 'Apply command').click(); await view.whenSettled();
        expect(session.snapshot().selection.projectId).toBe('feature-tour');
        expect(session.snapshot().transition.error).toBeDefined();
        expect(workbench.snapshot().command.draftText).toContain('--format events');
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        expect(session.snapshot().transition).toBeUndefined();
    });

    it('persists both drafts through the dialog, then admits the requested project', async () => {
        const repository = await fixture('beta', true);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'feature-tour', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session });
        await workbench.parsePersisted();
        workbench.updateDraft('durable source');
        await workbench.updateCommandDraft(workbench.snapshot().command.current.text.replace('--format ast', '--format events'));
        workbench.setCommandTarget({ mode: 'existing', entryId: 'validate-cem-ml' });
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const target = workbench.snapshot().workbenches.find(item => item.selection.projectId === 'beta');
        await workbench.selectWorkbench(target.id); await view.whenSettled();
        dialogAction(root, 'Save source').click(); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('command');
        dialogAction(root, 'Apply command').click(); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('confirmation');
        expect(root.querySelectorAll('dialog[open]')).toHaveLength(1);
        dialogAction(root, 'Replace configuration').click(); await view.whenSettled();
        expect(session.snapshot().selection.projectId).toBe('beta');
        const bundle = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        expect(new TextDecoder().decode(bundle.contents.source)).toBe('durable source');
        expect(bundle.project.entries.some(item => item.kind === 'inspection')).toBe(true);
        await workbench.refreshNavigation();
        expect(workbench.snapshot().workbenches.some(item => item.operation === 'inspection')).toBe(true);
    });

    it('cancels a late candidate from the native dialog without discarding the active draft', async () => {
        const repository = await fixture('beta', true);
        const validator = validatorFor(validResult());
        const gate = deferred();
        const session = createCemStudioProjectSession({ openWorkbench: async selection => {
            const candidate = await createCemStudioProjectWorkbench({ repository, validator, selection });
            if (selection.projectId === 'beta') await gate.promise;
            return candidate;
        } });
        workbenches.push(session);
        await session.select({ projectId: 'feature-tour', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session });
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        workbench.updateDraft('retain late draft');
        const target = workbench.snapshot().workbenches.find(item => item.selection.projectId === 'beta');
        await workbench.selectWorkbench(target.id); await view.whenSettled();
        dialogAction(root, 'Discard draft').click();
        await vi.waitFor(() => expect(session.snapshot().status).toBe('opening'));
        dialogAction(root, 'Cancel navigation').click();
        gate.resolve(); await view.whenSettled();
        expect(session.snapshot()).toMatchObject({ selection: { projectId: 'feature-tour' }, workbench: { draft: 'retain late draft', dirty: true } });
        expect(root.querySelector('dialog[open]')).toBeNull();
    });

    it('admits an empty second project and guards mounted source navigation with native dialogs', async () => {
        const repository = await fixture('beta', true);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection =>
            createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'feature-tour', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session });
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        workbench.updateDraft('retain me');
        const target = workbench.snapshot().workbenches.find(item => item.selection.projectId === 'beta');
        await workbench.selectWorkbench(target.id);
        await view.whenSettled();
        expect(root.querySelector('dialog[open]')).not.toBeNull();
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        expect(session.snapshot().workbench.draft).toBe('retain me');
        const selector = root.querySelector('[data-cem-studio-editor] [role="textbox"]');
        selector.focus();
        await vi.waitFor(() => expect(document.activeElement).toBe(selector));
        await workbench.selectWorkbench(target.id); await view.whenSettled();
        const dialog = root.querySelector('dialog[open]');
        dialog.requestClose(); await view.whenSettled();
        expect(session.snapshot().transition).toBeUndefined();
        await vi.waitFor(() => expect(document.activeElement).toBe(selector));
        await workbench.selectWorkbench(target.id); await view.whenSettled();
        dialogAction(root, 'Discard draft').click(); await view.whenSettled();
        expect(session.snapshot()).toMatchObject({ status: 'ready', selection: { projectId: 'beta' },
            workbench: { status: 'empty', editable: false } });
        expect(root.querySelector('dialog[open]')).toBeNull();
        expect(root.textContent).toContain('This project has no resources');
    });
});

describe('Studio project session admission', () => {
    const first = { projectId: 'feature-tour', entryId: 'validate-cem-ml', resourceId: 'source' };
    const second = { ...first, projectId: 'user-project' };

    async function fixture(storage) {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const exported = await repository.query(request('export-project', { projectId: first.projectId }));
        await repository.execute(request('import-project', { bundle: {
            ...exported.value,
            project: { ...exported.value.project, id: second.projectId, rootUri: 'studio://user-project/' },
        } }));
        const openWorkbench = vi.fn(async selection => {
            const workbench = await createCemStudioFeatureTourWorkbench({
                repository, validator, seed: featureTourSeed(), projectId: selection.projectId,
            });
            workbenches.push(workbench);
            return workbench;
        });
        const session = createCemStudioProjectSession({ openWorkbench, storage });
        workbenches.push(session);
        await session.select(first);
        return { repository, validator, session, openWorkbench, workbench: session.activeWorkbench() };
    }

    async function commandDraft(workbench, mode = 'events') {
        await workbench.updateCommandDraft(workbench.snapshot().command.current.text.replace('--format ast', `--format ${mode}`));
    }

    async function resolve(session, decision) {
        return session.resolveTransition(session.snapshot().transition.id, decision);
    }

    function draftStorage() {
        const records = new Map();
        return { records, getItem: key => records.get(key) ?? null,
            setItem: vi.fn((key, value) => records.set(key, value)) };
    }

    async function reopen(openWorkbench, storage, selection = first) {
        const session = createCemStudioProjectSession({ openWorkbench, storage });
        workbenches.push(session);
        await session.select(selection);
        return session;
    }

    it.each(['', 'draft 🙂\nsecond line'])('recovers exact source and command drafts without Save, Apply or Run: %s', async draft => {
        const storage = draftStorage();
        const { session, workbench, openWorkbench, repository, validator } = await fixture(storage);
        const before = await repository.query(request('export-project', { projectId: first.projectId }));
        workbench.updateDraft(draft);
        await commandDraft(workbench);
        const expected = workbench.snapshot().command.draftText;
        expect(session.snapshot().draftRecovery.status).toBe('saved');
        session.dispose();
        const restored = await reopen(openWorkbench, storage);
        expect(restored.snapshot().workbench).toMatchObject({ draft, dirty: true, command: { draftText: expected } });
        expect(restored.snapshot().draftRecovery.status).toBe('recovered');
        const after = await repository.query(request('export-project', { projectId: first.projectId }));
        expect(after.value).toEqual(before.value);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        await restored.activeWorkbench().saveSource();
        restored.dispose();
        const commandOnly = await reopen(openWorkbench, storage);
        expect(commandOnly.activeWorkbench().snapshot()).toMatchObject({ draft, dirty: false, command: { draftText: expected } });
        await commandOnly.activeWorkbench().resetCommandDraft();
        expect(commandOnly.draftBackup()).toBe('');
    });

    it('recovers an accepted grouped draft with every option and no persisted command mutation', async () => {
        const storage = draftStorage();
        const { session, workbench, openWorkbench, repository } = await fixture(storage);
        const before = (await repository.query(request('export-project', { projectId: first.projectId }))).value;
        await workbench.updateCommandFields([{ id: 'format', value: 'events' }, { id: 'preserve_source_offsets', value: 'true' }], workbench.snapshot().command.formVersion);
        const expected = workbench.snapshot().command.draftText;
        session.dispose();
        const restored = await reopen(openWorkbench, storage);
        expect(restored.snapshot().draftRecovery.status).toBe('recovered');
        expect(restored.activeWorkbench().snapshot().command.draftText).toBe(expected);
        expect(parseCemMlCommandText(expected).options).toMatchObject({ format: 'events', preserve_source_offsets: true });
        expect((await repository.query(request('export-project', { projectId: first.projectId }))).value).toEqual(before);
    });

    it('retains conflicting and corrupt records for download until explicit clearing', async () => {
        const storage = draftStorage();
        const { session, workbench, openWorkbench } = await fixture(storage);
        workbench.updateDraft('recover me');
        session.dispose();
        const other = await openWorkbench(first);
        other.updateDraft('new durable source');
        await other.saveSource();
        const restored = await reopen(openWorkbench, storage);
        expect(restored.snapshot().draftRecovery).toMatchObject({ status: 'conflict', blocked: true });
        expect(restored.activeWorkbench().snapshot().draft).toBe('new durable source');
        expect(restored.draftBackup()).toContain('recover me');
        restored.clearDraftRecovery();
        expect(restored.draftBackup()).toBe('');
        restored.dispose();
        const key = [...storage.records.keys()].find(key => key.includes(':draft-v1:'));
        storage.records.set(key, '{broken');
        const corrupt = await reopen(openWorkbench, storage);
        expect(corrupt.snapshot().draftRecovery.status).toBe('error');
        expect(corrupt.draftBackup()).toContain('{broken');
        expect(storage.records.get(key)).toBe('{broken');
    });

    it('retains latest in-memory text after quota failure and protects changed journal records', async () => {
        const storage = draftStorage();
        const { session, workbench, openWorkbench } = await fixture(storage);
        storage.setItem.mockImplementation(() => { throw new DOMException('Quota exceeded', 'QuotaExceededError'); });
        workbench.updateDraft('first draft');
        workbench.updateDraft('latest draft');
        expect(session.snapshot().draftRecovery.status).toBe('error');
        expect(JSON.parse(session.draftBackup()).current.draft).toBe('latest draft');
        storage.setItem.mockImplementation((key, value) => storage.records.set(key, value));
        session.clearDraftRecovery();
        const competitor = await reopen(openWorkbench, storage);
        competitor.activeWorkbench().updateDraft('other tab draft');
        workbench.updateDraft('local draft');
        expect(session.snapshot().draftRecovery.blocked).toBe(true);
        expect(session.draftBackup()).toContain('other tab draft');
        expect(JSON.parse(session.draftBackup()).current.draft).toBe('local draft');
    });

    it('clears discarded drafts only on admission and isolates project journals', async () => {
        const storage = draftStorage();
        const { session, workbench, openWorkbench } = await fixture(storage);
        workbench.updateDraft('alpha draft');
        await session.select(second);
        session.cancelTransition(session.snapshot().transition.id);
        expect(session.draftBackup()).toContain('alpha draft');
        openWorkbench.mockRejectedValueOnce(new Error('Unavailable destination'));
        await session.select(second);
        await resolve(session, 'discard');
        expect(session.snapshot().status).toBe('failed');
        expect(session.draftBackup()).toContain('alpha draft');
        await resolve(session, 'retry');
        session.activeWorkbench().updateDraft('beta draft');
        session.dispose();
        const alpha = await reopen(openWorkbench, storage);
        expect(alpha.activeWorkbench().snapshot().draft).toBe('original');
        const beta = await reopen(openWorkbench, storage, second);
        expect(beta.activeWorkbench().snapshot().draft).toBe('beta draft');
    });

    it('retries an initial open failure without committing a selection', async () => {
        const { openWorkbench } = await fixture();
        const opener = vi.fn(openWorkbench).mockRejectedValueOnce(new Error('Project temporarily unavailable'));
        const session = createCemStudioProjectSession({ openWorkbench: opener });
        workbenches.push(session);
        await session.select(second);
        expect(session.snapshot()).toMatchObject({ status: 'failed', selection: undefined, workbench: undefined });
        await resolve(session, 'retry');
        expect(session.snapshot()).toMatchObject({ status: 'ready', selection: second });
    });

    it.each(['projectId', 'entryId', 'resourceId'])('rejects a candidate with the wrong %s', async field => {
        const { session, openWorkbench } = await fixture();
        const realOpen = openWorkbench.getMockImplementation();
        let candidate;
        openWorkbench.mockImplementationOnce(async selection => {
            const actual = await realOpen(selection);
            const snapshot = actual.snapshot();
            const wrong = field === 'entryId'
                ? { ...snapshot, command: { ...snapshot.command,
                    application: { ...snapshot.command.application, currentEntryId: 'wrong-entry' } } }
                : { ...snapshot, [field]: 'wrong-identity' };
            candidate = { ...actual, snapshot: () => wrong, dispose: vi.fn(actual.dispose) };
            return candidate;
        });
        await session.select(second);
        expect(session.snapshot()).toMatchObject({ status: 'failed', selection: first });
        expect(candidate.dispose).toHaveBeenCalledTimes(1);
    });

    it('requires both draft decisions and saves before Apply without running validation', async () => {
        const { repository, validator, session, workbench } = await fixture();
        workbench.updateDraft('saved source');
        await commandDraft(workbench);
        await session.select(second);
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'source' } });
        const id = session.snapshot().transition.id;
        await session.select(second);
        expect(session.snapshot().transition.id).toBe(id);
        await resolve(session, 'discard');
        expect(session.snapshot().transition.stage).toBe('command');
        session.cancelTransition(id);
        expect(session.snapshot()).toMatchObject({ selection: first, transition: undefined });
        expect(workbench.snapshot()).toMatchObject({ draft: 'saved source', dirty: true });
        expect(workbench.snapshot().command.draftText).toContain('--format events');

        await session.select(second);
        await resolve(session, 'save');
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'command' } });
        expect(workbench.snapshot().dirty).toBe(false);
        expect(validator.validateResource).not.toHaveBeenCalled();
        await resolve(session, 'apply');
        expect(session.snapshot()).toMatchObject({ status: 'ready', selection: second, transition: undefined });
        const saved = await repository.query(request('export-project', { projectId: first.projectId }));
        expect(new TextDecoder().decode(saved.value.contents.source)).toBe('saved source');
        expect(saved.value.project.revision).toBe(3);
        expect(saved.value.project.entries).toHaveLength(3);
        expect(workbench.snapshot().command).toMatchObject({ status: 'current', application: { status: 'applied' } });
        expect(workbench.snapshot().command.draftText).toBe(workbench.snapshot().command.current.text);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        expect(saved.value.project).not.toHaveProperty('selection');
    });

    it('defers both discards until a candidate opens, retaining drafts after failure or cancellation', async () => {
        const { session, workbench, openWorkbench } = await fixture();
        workbench.updateDraft('unsaved');
        await commandDraft(workbench);
        await session.select(second);
        await resolve(session, 'discard');
        openWorkbench.mockRejectedValueOnce(new Error('Project unavailable'));
        await resolve(session, 'discard');
        expect(session.snapshot()).toMatchObject({ status: 'failed', selection: first });
        expect(workbench.snapshot()).toMatchObject({ draft: 'unsaved', dirty: true });
        expect(workbench.snapshot().command.draftText).toContain('--format events');
        session.cancelTransition(session.snapshot().transition.id);
        await session.select(second);
        await resolve(session, 'discard');
        await resolve(session, 'discard');
        expect(session.snapshot().selection).toEqual(second);
    });

    it('persists source independently of native command preview availability', async () => {
        const { session, workbench, validator } = await fixture();
        workbench.updateDraft('durable without a native result');
        await commandDraft(workbench);
        await session.select(second);
        validator.previewResourceCommand.mockRejectedValueOnce(new Error('Worker unavailable'));
        await resolve(session, 'save');
        expect(workbench.snapshot()).toMatchObject({ dirty: false, persistedText: 'durable without a native result' });
        expect(workbench.snapshot().command.draftText).toContain('--format events');
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'command' } });
        expect(validator.validateResource).not.toHaveBeenCalled();
        await workbench.updateCommandDraft(workbench.snapshot().command.draftText);
        await resolve(session, 'apply');
        expect(session.snapshot()).toMatchObject({ status: 'ready', selection: second });
    });

    it('retains the applied baseline when an older projection finishes', async () => {
        const { workbench, validator } = await fixture();
        await commandDraft(workbench);
        const projection = deferred();
        validator.parseResource.mockImplementation(() => projection.promise);
        const projecting = workbench.parsePersisted('ast');
        await vi.waitFor(() => expect(validator.parseResource).toHaveBeenCalledTimes(1));
        await workbench.applyCommand();
        projection.resolve(projectionOutcome('parse', 'ast'));
        await projecting;
        expect(workbench.snapshot().command.current.text).toContain('--format events');
        expect(workbench.snapshot().command.draftText).toBe(workbench.snapshot().command.current.text);
        expect(workbench.snapshot().command.application.status).toBe('applied');
    });

    it('retains command edits when a background native projection completes', async () => {
        const { session, workbench, validator } = await fixture();
        const projection = deferred();
        validator.parseResource.mockImplementation(() => projection.promise);
        const projecting = workbench.parsePersisted('ast');
        await vi.waitFor(() => expect(validator.parseResource).toHaveBeenCalledTimes(1));
        await commandDraft(workbench);
        await session.select(second);
        projection.resolve(projectionOutcome('parse', 'ast'));
        await projecting;
        expect(workbench.snapshot().command.draftText).toContain('--format events');
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'command' } });
    });

    it('retains the newest command edit through a delayed validation preview refresh', async () => {
        const { workbench, validator } = await fixture();
        const preview = deferred();
        validator.previewResourceCommand.mockImplementationOnce(() => preview.promise);
        const validating = workbench.validatePersisted();
        await vi.waitFor(() => expect(validator.previewResourceCommand).toHaveBeenCalledTimes(2));
        await commandDraft(workbench);
        preview.resolve(commandPreviewOutcome({ operation: 'parse', projection: 'ast', uri: 'data/cem-ml/basic.cem',
            projectId: first.projectId, contentType: 'application/cem', schema: 'https://cem.dev/ns/cem-ml/1',
            projectRevision: 1, resourceRevision: 1 }));
        await validating;
        expect(workbench.snapshot().command.draftText).toContain('--format events');
    });

    it('rechecks admission after both drafts are reverted', async () => {
        const { session, workbench, repository } = await fixture();
        workbench.updateDraft('unsaved');
        await commandDraft(workbench);
        await session.select(second);
        workbench.updateDraft('original');
        await workbench.resetCommandDraft();
        await resolve(session, 'discard');
        expect(session.snapshot()).toMatchObject({ status: 'ready', selection: second });
        const saved = await repository.query(request('export-project', { projectId: first.projectId }));
        expect(saved.value.project.revision).toBe(1);
    });

    it('keeps drafts and the original selection on a failed source save', async () => {
        const { session, workbench, repository } = await fixture();
        workbench.updateDraft('not durable');
        await commandDraft(workbench);
        vi.spyOn(repository, 'execute').mockRejectedValueOnce(new Error('Quota exceeded'));
        await session.select(second);
        await resolve(session, 'save');
        expect(session.snapshot()).toMatchObject({ status: 'failed', selection: first,
            transition: { stage: 'source', error: { message: 'Quota exceeded' } } });
        expect(workbench.snapshot()).toMatchObject({ draft: 'not durable', persistedText: 'original', dirty: true });
        expect(workbench.snapshot().command.draftText).toContain('--format events');
    });

    it('deduplicates Save, ignores cancelled admission, and preserves edits made during persistence', async () => {
        const { session, workbench, repository } = await fixture();
        const gate = deferred();
        const execute = repository.execute.bind(repository);
        const spy = vi.spyOn(repository, 'execute').mockImplementation(async (...args) => {
            await gate.promise;
            // Persistence can complete despite cancellation; admission must still be revoked.
            return execute(args[0]);
        });
        workbench.updateDraft('committing');
        await session.select(second);
        const id = session.snapshot().transition.id;
        const saving = session.resolveTransition(id, 'save');
        await vi.waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
        const duplicate = session.resolveTransition(id, 'save');
        workbench.updateDraft('newer draft');
        session.cancelTransition(id);
        await session.select(second);
        expect(session.snapshot().selection).toEqual(first);
        gate.resolve();
        await Promise.all([saving, duplicate]);
        expect(spy).toHaveBeenCalledTimes(1);
        expect(session.snapshot()).toMatchObject({ selection: first, transition: undefined });
        expect(workbench.snapshot()).toMatchObject({ persistedText: 'committing', draft: 'newer draft', dirty: true });
        await session.select(second);
        expect(session.snapshot().transition.stage).toBe('source');
        await session.resolveTransition(id, 'discard');
        expect(session.snapshot().selection).toEqual(first);
    });

    it('retains a newer command edit when Apply completes and asks again', async () => {
        const { session, workbench, repository } = await fixture();
        await commandDraft(workbench);
        const gate = deferred();
        const execute = repository.execute.bind(repository);
        const spy = vi.spyOn(repository, 'execute').mockImplementation(async (...args) => {
            await gate.promise;
            return execute(...args);
        });
        await session.select(second);
        const applying = resolve(session, 'apply');
        await vi.waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
        const duplicate = resolve(session, 'apply');
        await workbench.updateCommandDraft(workbench.snapshot().command.draftText.replace('--format events', '--format ast'));
        gate.resolve();
        await Promise.all([applying, duplicate]);
        expect(spy).toHaveBeenCalledTimes(1);
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'command' } });
        expect(workbench.snapshot().command).toMatchObject({ application: { status: 'stale' } });
        expect(workbench.snapshot().command.current.text).toContain('--format events');
        expect(workbench.snapshot().command.draftText).toContain('--format ast');
    });

    it('keeps stale Apply conflicts and explicit replacement confirmation inside the guard', async () => {
        const { session, workbench, repository } = await fixture();
        await commandDraft(workbench);
        workbench.setCommandTarget({ mode: 'existing', entryId: first.entryId });
        await session.select(second);
        await resolve(session, 'apply');
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'confirmation' } });
        session.cancelTransition(session.snapshot().transition.id);
        expect(workbench.snapshot().command.draftText).toContain('--format events');
        await session.select(second);
        await resolve(session, 'apply');
        await repository.execute(request('save-resource', { projectId: first.projectId, resourceId: first.resourceId,
            expectedProjectRevision: 1, expectedResourceRevision: 1, content: 'concurrent save' }));
        await resolve(session, 'confirm-replacement');
        expect(session.snapshot()).toMatchObject({ status: 'failed', selection: first,
            transition: { stage: 'command', error: { code: 'cem.studio.repository.revision_conflict' } } });
        expect(workbench.snapshot().command.draftText).toContain('--format events');
    });

    it('admits an explicitly confirmed compatible replacement after Apply', async () => {
        const { session, workbench } = await fixture();
        await commandDraft(workbench);
        workbench.setCommandTarget({ mode: 'existing', entryId: first.entryId });
        await session.select(second);
        await resolve(session, 'apply');
        expect(session.snapshot().transition.stage).toBe('confirmation');
        await resolve(session, 'confirm-replacement');
        expect(session.snapshot().selection).toEqual(second);
        expect(workbench.snapshot().command.application.result.entry.id).toBe(first.entryId);
    });

    it('disposes a cancelled candidate and ignores late results from the former active workbench', async () => {
        const { session, workbench, openWorkbench, validator } = await fixture();
        const projection = deferred();
        validator.parseResource.mockImplementation(() => projection.promise);
        const projecting = workbench.parsePersisted('ast');
        await vi.waitFor(() => expect(validator.parseResource).toHaveBeenCalledTimes(1));
        const opening = deferred();
        const realOpen = openWorkbench.getMockImplementation();
        let candidate;
        openWorkbench.mockImplementationOnce(async selection => {
            candidate = await realOpen(selection);
            candidate = { ...candidate, dispose: vi.fn(candidate.dispose) };
            await opening.promise;
            return candidate;
        });
        const selecting = session.select(second);
        await vi.waitFor(() => expect(candidate).toBeDefined());
        const id = session.snapshot().transition.id;
        session.cancelTransition(id);
        await session.select(second);
        expect(session.snapshot().selection).toEqual(second);
        const admitted = session.snapshot().workbench;
        opening.resolve();
        await selecting;
        projection.resolve(projectionOutcome('parse', 'ast'));
        await projecting;
        expect(session.snapshot().selection).toEqual(second);
        expect(session.snapshot().workbench).toBe(admitted);
        expect(candidate.dispose).toHaveBeenCalledTimes(1);
    });

    it('rechecks discard consent if a draft changes while the candidate is opening', async () => {
        const { session, workbench, openWorkbench } = await fixture();
        workbench.updateDraft('discarded version');
        const opening = deferred();
        const realOpen = openWorkbench.getMockImplementation();
        openWorkbench.mockImplementationOnce(async selection => {
            const candidate = await realOpen(selection);
            await opening.promise;
            return candidate;
        });
        await session.select(second);
        const selecting = resolve(session, 'discard');
        await vi.waitFor(() => expect(openWorkbench).toHaveBeenCalledTimes(2));
        workbench.updateDraft('keep this version');
        opening.resolve();
        await selecting;
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'source' } });
        expect(workbench.snapshot().draft).toBe('keep this version');
    });
});

let nativeGraphAuthor, nativeGraphValidator;
afterAll(async () => { await nativeGraphValidator?.close(); });
const edgeGraphSource = '{run | {import @id=other @src=other.xml} {import @id=data @src=data.xml | {transform @id=view @src=view.xpath @template-content-type=application/vnd.cem.xpath | {export @id=result @out=result.xml}}}}';
const parameterGraphSource = '{run | {import @id=data @src=data.xml | {transform @id=view @src=view.cem @template-content-type=application/vnd.cem.template+cem | {param @name=locale @value=en-US} {export @id=result @out=result.html}}}}';
async function graphWorkbench(source = '{run | {import @id=data @src=asset.svg @opaque=true @content-type=image/svg+xml | {export @id=result @out=result.svg}}}\n') {
    if (!nativeGraphAuthor) {
        const native = await createCemStudioBrowserValidator();
        nativeGraphValidator = native; nativeGraphAuthor = native.authorGraph;
    }
    const repository = await repositoryWithSource(source);
    const validator = validatorFor(validResult()); validator.authorGraph = nativeGraphAuthor;
    const seed = featureTourSeed();
    seed.catalog.workbenches = [{ ...seed.catalog.workbenches[0], id: 'graph', operation: 'transform', kind: 'transformation-graph', commandArguments: ['transform', '--config', '$input'] }];
    const workbench = await createCemStudioFeatureTourWorkbench({ repository, validator, seed, projectId: 'feature-tour' });
    workbenches.push(workbench); await workbench.selectWorkbench('graph');
    return { workbench, repository, validator };
}

function deferred() {
    let resolve;
    const promise = new Promise(complete => { resolve = complete; });
    return { promise, resolve };
}

function selectValue(root, selector, value) {
    const select = root.querySelector(`cem-select${selector}`);
    select.value = value;
    select.dispatchEvent(new Event('change', { bubbles: true }));
}

function dialogAction(root, label) {
    const buttons = [...root.ownerDocument.querySelectorAll('button')];
    const action = buttons.find((button) => button.textContent.trim() === label);
    if (!action) throw new Error(`Dialog action ${label} is unavailable`);
    return action;
}

async function createWorkbench(repository, validator) {
    const workbench = await createCemStudioFeatureTourWorkbench({
        repository,
        validator,
        seed: featureTourSeed(),
        projectId: 'feature-tour',
    });
    workbenches.push(workbench);
    return workbench;
}

function validatorFor(outcome) {
    return {
        validateProject: async (bundle) => bundle,
        previewResourceCommand: vi.fn(async (options) => commandPreviewOutcome(options)),
        serializeResourceCommand: vi.fn((parsed) => serializeCommandResource(parsed)),
        executeAuthoredResourceCommand: vi.fn(async (options) => authoredCommandOutcome(options)),
        parseResource: vi.fn(async ({ projection = 'ast' }) => projectionOutcome('parse', projection)),
        inspectResource: vi.fn(async ({ view = 'summary' }) => projectionOutcome('inspect', view)),
        runResourceCommand: vi.fn(async ({ argv }) => portableOperationOutcome(argv)),
        validateResource: vi.fn(async (options) => {
            const value = typeof outcome === 'function' ? await outcome(options) : outcome;
            if (value.result.exitCode === 0) return value;
            const error = new Error('invalid saved revision');
            error.result = value.result;
            error.presentation = value.presentation;
            throw error;
        }),
    };
}

function serializeCommandResource(parsed) {
    const operation = parsed.commandPath[0];
    const uri = parsed.positionals.inputs;
    const argv =
        operation === 'parse'
            ? ['parse', uri, '--format', parsed.options.format ?? 'ast']
            : ['inspect', uri, '--show', parsed.options.show ?? 'summary', '--format', parsed.options.format ?? 'cem'];
    return `${JSON.stringify(
        {
            $schema: 'https://cem.dev/ns/cli/command/1',
            schemaVersion: 1,
            commandSchemaVersion: 1,
            commonVersion: '0.1.0',
            binaryName: 'cem-ml',
            argv,
        },
        null,
        2,
    )}\n`;
}

function authoredCommandOutcome(options) {
    const resource = JSON.parse(new TextDecoder().decode(new Uint8Array(options.commandResource)));
    const operation = resource.argv[0];
    const mode =
        operation === 'parse'
            ? resource.argv[resource.argv.indexOf('--format') + 1]
            : resource.argv[resource.argv.indexOf('--show') + 1];
    const outcome = projectionOutcome(operation, mode);
    const authored = {
        resource,
        command: commandPreviewOutcome({
            operation,
            projection: operation === 'parse' ? mode : undefined,
            view: operation === 'inspect' ? mode : undefined,
            uri: 'data/cem-ml/basic.cem',
            projectId: options.projectId,
            contentType: options.contentType,
            schema: options.schema,
            projectRevision: options.projectRevision,
            resourceRevision: options.resourceRevision,
        }).parsed,
    };
    return {
        ...outcome,
        parsed: authored.command,
    };
}

function commandPreviewOutcome(options) {
    if (options.text?.includes('--unknown-studio-option')) {
        const error = new Error('unknown CEM-ML option `--unknown-studio-option`');
        error.code = 'cem.command.unknown_option';
        throw error;
    }
    const operation = options.text?.match(/\b(parse|inspect|validate)\b/u)?.[1] ?? options.operation ?? 'parse';
    const format =
        options.text?.match(/--format\s+([^\s]+)/u)?.[1] ??
        (operation === 'parse' ? (options.projection ?? 'ast') : operation === 'inspect' ? 'cem' : 'json');
    const view = options.text?.match(/--show\s+([^\s]+)/u)?.[1] ?? options.view ?? 'summary';
    const uri = /^[a-z][a-z0-9+.-]*:/iu.test(options.uri)
        ? options.uri
        : `studio://${options.projectId}/${options.uri}`;
    const argv =
        operation === 'parse'
            ? ['parse', uri, '--format', format]
            : operation === 'inspect'
              ? ['inspect', uri, '--show', view, '--format', format]
              : ['validate', uri, '--format', format];
    const text =
        options.text ?? `cem-ml ${argv.join(' ')} --content-type ${options.contentType} --schema ${options.schema}`;
    const parsed = Object.freeze({
        schemaVersion: 1,
        commonVersion: '0.1.0',
        commandPath: Object.freeze([operation]),
        globalOptions: Object.freeze({}),
        options: Object.freeze({ format, ...(operation === 'inspect' ? { show: view } : {}) }),
        positionals: Object.freeze({ inputs: uri }),
    });
    return Object.freeze({
        projection: 'studio',
        binaryName: 'cem-ml',
        commonVersion: '0.1.0',
        argv: Object.freeze(argv),
        text,
        parsed,
        semantic: Object.freeze({
            operation: Object.freeze({ kind: operation, ...(operation === 'parse' ? { projection: format } : {}) }),
            inputs: Object.freeze({ normalized: Object.freeze([{ uri }]) }),
            identity: Object.freeze({ contentType: options.contentType, schema: options.schema }),
            configuration: Object.freeze({ view }),
            outputs: Object.freeze({ format }),
            scope: Object.freeze({
                projectRevision: options.projectRevision,
                resourceRevision: options.resourceRevision,
            }),
        }),
    });
}

function projectionOutcome(kind, mode) {
    const text = `${kind} ${mode} projection\n`;
    const bytes = [...new TextEncoder().encode(text)];
    return {
        result: {
            protocolVersion: 1,
            requestId: `workbench-${kind}-${mode}`,
            operation: kind,
            exitCode: 0,
            result: { storage: 'inline', value: { kind, value: { projection: mode } } },
            diagnostics: { items: [], originalCount: 0 },
            sourceMaps: { items: [], originalCount: 0 },
            identity: { runtime: 'wasm-browser-worker' },
        },
        presentation: { writes: [] },
        output: {
            uri: 'cem-stdio://stdout',
            contentType: 'application/cem',
            byteLength: bytes.length,
            sha256: 'fixture-output-sha256',
            bytes,
            text,
        },
    };
}

function portableOperationOutcome(argv) {
    const outcome = projectionOutcome('convert', 'conversion');
    return {
        ...outcome,
        result: {
            ...outcome.result,
            operation: argv[0],
            result: {
                storage: 'inline',
                value: {
                    kind: 'convert',
                    value: { outputs: { items: [{ response: { primary: { kind: 'dom' } } }] } },
                },
            },
        },
    };
}

function invalidResult(start, len) {
    const diagnostic = {
        uri: 'cem-studio://feature-tour/data/cem-ml/basic.cem',
        line: 3,
        column: 1,
        byteOffset: start,
        code: 'cem.tokenizer.unterminated_node',
        severity: 'error',
        message: 'node scope must close before end of input',
        node: 'article',
        details: null,
        sourceMap: {
            frames: [
                {
                    source_id: 7,
                    span: { kind: 'Single', ranges: { start, len } },
                    transform: { kind: 'CemTokenizer' },
                },
            ],
        },
    };
    const report = {
        generatedAt: '2026-08-21T00:00:00Z',
        inputs: ['data/cem-ml/basic.cem'],
        summary: {
            inputCount: 1,
            infoCount: 0,
            warningCount: 0,
            errorCount: 1,
            fatalCount: 0,
            hardViolationCount: 1,
        },
        options: {},
        diagnostics: [diagnostic],
        reportAst: {},
    };
    return commandOutcome(1, report, [diagnostic]);
}

function validResult() {
    return commandOutcome(
        0,
        {
            generatedAt: '2026-08-21T00:00:00Z',
            inputs: ['data/cem-ml/basic.cem'],
            summary: {
                inputCount: 1,
                infoCount: 0,
                warningCount: 0,
                errorCount: 0,
                fatalCount: 0,
                hardViolationCount: 0,
            },
            options: {},
            diagnostics: [],
            reportAst: {},
        },
        [],
    );
}

function commandOutcome(exitCode, report, diagnostics) {
    return {
        result: {
            protocolVersion: 1,
            requestId: 'workbench-test',
            exitCode,
            result: { storage: 'inline', value: { kind: 'validate', value: { report } } },
            diagnostics: { items: diagnostics, originalCount: diagnostics.length },
            sourceMaps: { items: [], originalCount: 0 },
            identity: { runtime: 'wasm-browser-worker' },
        },
        presentation: { writes: [] },
    };
}

async function repositoryWithSource(content) {
    const repository = createCemStudioProjectRepository({
        databaseName: `cem-studio-workbench-${crypto.randomUUID()}`,
        validateProject: async (bundle) => bundle,
        now: () => '2026-08-21T00:00:00Z',
    });
    repositories.push(repository);
    const bytes = new TextEncoder().encode(content);
    const sha256 = await digest(bytes);
    const runConfigBytes = new TextEncoder().encode('{}\n');
    const runConfigSha256 = await digest(runConfigBytes);
    await repository.execute(
        request('import-project', {
            bundle: {
                project: {
                    $schema: 'https://cem.dev/ns/studio/project/1',
                    schemaVersion: 1,
                    id: 'feature-tour',
                    name: 'Feature Tour',
                    rootUri: 'studio://feature-tour/',
                    revision: 1,
                    createdAt: '2026-08-21T00:00:00Z',
                    updatedAt: '2026-08-21T00:00:00Z',
                    entries: [
                        {
                            id: 'schema-package-examples',
                            kind: 'subproject',
                            name: 'Schema package examples',
                        },
                        {
                            id: 'validate-cem-ml',
                            parentId: 'schema-package-examples',
                            kind: 'validation',
                            name: 'CEM ML: Basic',
                            runConfigResourceId: 'run-cem-ml',
                            resourceIds: ['source', 'run-cem-ml'],
                        },
                    ],
                    resources: [
                        {
                            id: 'source',
                            role: 'data',
                            sourceKind: 'project-file',
                            path: 'data/cem-ml/basic.cem',
                            contentType: 'application/cem',
                            schema: 'https://cem.dev/ns/cem-ml/1',
                            revision: 1,
                            sha256,
                        },
                        {
                            id: 'run-cem-ml',
                            role: 'run-config',
                            sourceKind: 'project-file',
                            path: 'config/cem-ml.validate.json',
                            contentType: 'application/json',
                            schema: 'https://cem.dev/ns/cli/run-config/1',
                            revision: 1,
                            sha256: runConfigSha256,
                        },
                    ],
                },
                contents: { source: bytes, 'run-cem-ml': runConfigBytes },
            },
        }),
    );
    return repository;
}


async function schemaSourceAdmissionRepository(contentType, bytes, item) {
    const repository = await repositoryWithSource('fixture seed');
    const bundle = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
    bundle.project.id = 'schema-source-admission'; bundle.project.rootUri = 'studio://schema-source-admission/';
    const resource = bundle.project.resources.find(item => item.id === 'source');
    Object.assign(resource, { path: item.path, referencePath: item.path, contentType, schema: item.schema, sourceKind: item.sourceKind ?? 'project-file', sha256: await digest(bytes) });
    if (resource.sourceKind === 'url') resource.url = 'https://example.test/schema-source';
    const commandBytes = new TextEncoder().encode(JSON.stringify({ $schema: 'https://cem.dev/ns/cli/command/1',
        schemaVersion: 1, commandSchemaVersion: 1, commonVersion: '0.1.0', binaryName: 'cem-ml',
        argv: ['validate', resource.path, '--content-type', item.mediaType, '--schema', item.schema, '--format', 'json', '--no-color'] }));
    const command = bundle.project.resources.find(item => item.id === 'run-cem-ml');
    Object.assign(command, { schema: 'https://cem.dev/ns/cli/command/1', sha256: await digest(commandBytes) });
    bundle.project.entries = [{ id: 'validate-source', name: 'Validate schema source', kind: 'validation',
        resourceIds: ['source', 'run-cem-ml'], runConfigResourceId: 'run-cem-ml' }];
    bundle.contents = { source: bytes, 'run-cem-ml': commandBytes };
    await repository.execute(request('import-project', { bundle }));
    return repository;
}

async function mathmlAdmissionRepository(contentType, bytes) {
    const repository = await repositoryWithSource('fixture seed');
    const bundle = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
    bundle.project.id = 'mathml-admission'; bundle.project.rootUri = 'studio://mathml-admission/';
    const resource = bundle.project.resources.find(item => item.id === 'source');
    Object.assign(resource, { path: 'data/formula.mml', contentType, schema: 'https://cem.dev/ns/data/mathml/1', sha256: await digest(bytes) });
    const commandBytes = new TextEncoder().encode(JSON.stringify({ $schema: 'https://cem.dev/ns/cli/command/1',
        schemaVersion: 1, commandSchemaVersion: 1, commonVersion: '0.1.0', binaryName: 'cem-ml',
        argv: ['inspect', resource.referencePath ?? resource.path, '--content-type', contentType, '--show', 'summary', '--format', 'cem'] }));
    const command = bundle.project.resources.find(item => item.id === 'run-cem-ml');
    Object.assign(command, { schema: 'https://cem.dev/ns/cli/command/1', sha256: await digest(commandBytes) });
    bundle.project.entries = [{ id: 'inspect-mathml', name: 'Inspect MathML', kind: 'inspection',
        resourceIds: ['source', 'run-cem-ml'], runConfigResourceId: 'run-cem-ml' }];
    bundle.contents = { source: bytes, 'run-cem-ml': commandBytes };
    await repository.execute(request('import-project', { bundle }));
    return repository;
}

function featureTourSeed() {
    return {
        catalog: {
            examples: [
                {
                    packageId: 'cem-ml',
                    resourceId: 'source',
                    runConfigResourceId: 'run-cem-ml',
                    path: 'data/cem-ml/basic.cem',
                    contentType: 'application/cem',
                    schema: 'https://cem.dev/ns/cem-ml/1',
                    dependencies: [],
                },
            ],
            workbenches: [
                {
                    id: 'conversion',
                    operation: 'convert',
                    kind: 'conversion',
                    name: 'CEM-ML conversion',
                    resourceId: 'source',
                    runConfigResourceId: 'run-cem-ml',
                    path: 'data/cem-ml/basic.cem',
                    contentType: 'application/cem',
                    schema: 'https://cem.dev/ns/cem-ml/1',
                    dependencies: [],
                    expectedSummary: { kind: 'convert', outputCount: 1 },
                    commandArguments: [
                        'convert',
                        '$input',
                        '--content-type',
                        'application/cem',
                        '--schema',
                        'https://cem.dev/ns/cem-ml/1',
                        '--to-format',
                        'dom-json',
                    ],
                },
            ],
        },
    };
}

function request(operation, parameters) {
    return {
        protocolVersion: 1,
        repository: 'studio-projects',
        operation,
        requestRevision: 1,
        parameters,
    };
}

async function digest(bytes) {
    const result = await crypto.subtle.digest('SHA-256', bytes);
    return [...new Uint8Array(result)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function deleteDatabase(name) {
    return new Promise((resolve, reject) => {
        const request = indexedDB.deleteDatabase(name);
        request.onsuccess = () => resolve(undefined);
        request.onerror = () => reject(request.error);
        request.onblocked = () => reject(new Error(`database ${name} remained open`));
    });
}
