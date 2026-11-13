import { afterEach, beforeAll, expect, it, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import themeCss from '@epa-wg/cem-theme/styles.css?raw';
import { createCemStudioBrowserValidator } from './feature-tour.js';
import { createCemStudioProjectRepository, CEM_STUDIO_REPOSITORY_ID } from './repository.js';
import { createCemStudioProjectSession } from './project-session.js';
import { createCemStudioProjectWorkbench, createCemStudioSessionWorkbench, mountCemStudioWorkbench } from './workbench.js';
import { createCemStudioNavigation } from './navigation.js';
import { createCemStudioProjectBootstrap } from './project-bootstrap.js';

beforeAll(() => {
    const map = document.createElement('script'); map.type = 'importmap';
    map.textContent = JSON.stringify({ imports: { '@epa-wg/cem-components/components.xhtml': new URL('../../cem-components/dist/components.xhtml', import.meta.url).href } });
    document.head.append(map);
});
let validator, repository, service, view, root, colors;
afterEach(async () => {
    view?.dispose(); service?.dispose();
    await validator?.close(); await repository?.deleteDatabase();
    root?.remove(); colors?.remove();
    validator = repository = service = view = root = colors = undefined;
    vi.restoreAllMocks();
});
const request = (operation, parameters = {}) => ({ protocolVersion: 1, repository: CEM_STUDIO_REPOSITORY_ID, requestRevision: 1, operation, parameters });
async function setup(mount = true) {
    validator = await createCemStudioBrowserValidator();
    repository = createCemStudioProjectRepository({ databaseName: `bootstrap-${crypto.randomUUID()}`, validateProject: (bundle, options) => validator.validateProject(bundle, options) });
    await repository.open();
    if (!mount) return;
    const session = createCemStudioProjectSession({ storage: { getItem: () => null, setItem() { /* Ephemeral fixture. */ }, removeItem() { /* Ephemeral fixture. */ } },
        openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
    const base = await createCemStudioSessionWorkbench({ repository, session });
    service = await createCemStudioNavigation({ repository, workbench: base, validator });
    colors = document.createElement('style'); colors.textContent = themeCss; document.head.append(colors);
    root = document.createElement('main'); root.style.cssText = 'inline-size:1280px; block-size:800px'; document.body.append(root);
    view = await mountCemStudioWorkbench({ root, workbench: service }); await view.whenSettled();
}
const dialog = () => root.querySelector('dialog[open]');
const button = command => root.querySelector(`cem-action[host-command="${command}"] button, cem-icon-button[host-command="${command}"] button`);
const field = name => dialog().querySelector(`cem-text-field[name="${name}"] input, cem-textarea[name="${name}"] textarea`);
async function click(command) { await userEvent.click(button(command)); await view.whenSettled(); }
async function choose(preset, name = 'My project') {
    await userEvent.fill(field('name'), name);
    const option = dialog().querySelector(`cem-select[name=preset] input[value="${preset}"]`);
    await userEvent.click(option);
    await userEvent.click(dialog().querySelector('form[host-command=bootstrap-type] button[type=submit]')); await view.whenSettled();
}
async function review() { await userEvent.click(dialog().querySelector('form[host-command=bootstrap-review] button[type=submit]')); await view.whenSettled(); }

it('creates a reviewed nested Markdown site with shared layout, navigation, CSS and static artifacts', async () => {
    await setup();
    await click('new-project');
    expect(dialog().textContent).toContain('Choose a project structure');
    expect(dialog().querySelectorAll('cem-select[name=preset] input[type=radio]')).toHaveLength(5);
    await choose('markdown-site', 'Nested site');
    await userEvent.fill(field('sourceFiles'), 'index.md\nabout.md\nguides/start.md\nguides/deep/next.md');
    await userEvent.fill(field('outputFolder'), 'public');
    await review();
    expect(dialog().querySelector('cem-tree[label="Planned source files"]').textContent).toContain('start.md');
    expect(dialog().querySelector('cem-tree[label="Planned target output files"]').textContent).toContain('next.html');
    expect(dialog().textContent).toContain('assets/css/site.css');
    expect(dialog().querySelectorAll('section[aria-label="Pipeline stages"] tbody tr').length).toBeGreaterThan(10);
    expect((await repository.query(request('list-projects'))).value).toHaveLength(0);
    for (const mode of ['light', 'dark', 'projector', 'contrast-light', 'contrast-dark', 'native']) {
        root.className = `cem-theme-${mode}`;
        expect(getComputedStyle(button('bootstrap-create')).visibility).toBe('visible');
        expect(button('bootstrap-create').disabled).toBe(false);
        expect(dialog().querySelector('[aria-label="Source hierarchy"]')).not.toBeNull();
    }
    await click('bootstrap-back-structure');
    expect(field('sourceFiles').value).toContain('guides/deep/next.md');
    await review();
    const importSpy = vi.spyOn(repository, 'execute');
    await click('bootstrap-create');
    expect(dialog()).toBeNull();
    const projects = (await repository.query(request('list-projects'))).value;
    expect(projects).toHaveLength(1); expect(projects[0].name).toBe('Nested site');
    expect(importSpy.mock.calls.filter(([r]) => r.operation === 'import-project')).toHaveLength(1);
    await service.bootstrapCreate();
    expect(importSpy.mock.calls.filter(([r]) => r.operation === 'import-project')).toHaveLength(1);
    const bundle = (await repository.query(request('export-project', { projectId: projects[0].id }))).value;
    expect(bundle.project.resources.map(r => r.path)).toEqual(expect.arrayContaining(['content/guides/deep/next.md', 'layouts/page.cemt', 'assets/css/site.css', 'static/logo.svg']));
    await service.buildProjectPreview(); await view.whenSettled();
    expect(service.snapshot().projectPreview.attempt.error).toBeUndefined();
    expect(service.snapshot().projectPreview.artifacts.map(a => a.path)).toEqual(expect.arrayContaining(['public/index.html', 'public/guides/deep/next.html', 'public/assets/css/site.css', 'public/static/logo.svg']));
}, 60_000);

it('repairs paired pipeline paths, reviews both trees and cancels without creating a project', async () => {
    await setup(); await click('create-selected'); await choose('transformation-pipeline', 'Pipeline');
    await userEvent.fill(field('sourceFiles'), 'sources/a.xml\nsources/nested/b.xml');
    await userEvent.fill(field('targetFiles'), 'target/a.html');
    await review();
    expect(dialog().textContent).toContain('one target path for each source');
    expect(field('sourceFiles').value).toContain('sources/nested/b.xml');
    await userEvent.fill(field('targetFiles'), 'target/a.html\ntarget/nested/b.html');
    await review();
    expect(dialog().querySelector('cem-tree[label="Planned source files"]').textContent).toContain('b.xml');
    expect(dialog().querySelector('cem-tree[label="Planned target output files"]').textContent).toContain('b.html');
    expect(dialog().querySelectorAll('section[aria-label="Pipeline stages"] tbody tr')).toHaveLength(6);
    await userEvent.keyboard('{Escape}'); await view.whenSettled();
    expect(dialog()).toBeNull();
    expect((await repository.query(request('list-projects'))).value).toHaveLength(0);
}, 60_000);

it('keeps simple input/output controls small, supports custom files and independent project identities', async () => {
    await setup(); await click('new-project'); await choose('simple-conversion', 'Convert');
    expect(field('sourceFiles').tagName).toBe('INPUT'); expect(field('targetFiles').tagName).toBe('INPUT');
    expect(dialog().querySelectorAll('cem-select[name=inputFormat] input[type=radio]')).toHaveLength(2);
    const firstId = service.snapshot().navigation.action.bootstrap.projectId;
    await review(); await click('cancel-action');
    await click('new-project'); await choose('custom', 'Custom');
    expect(service.snapshot().navigation.action.bootstrap.projectId).not.toBe(firstId);
    await userEvent.fill(field('sourceFiles'), 'README.md\nsrc/data.json\ntemplates/page.cemt\nstyles/site.css\nstatic/logo.svg');
    await review();
    expect(dialog().textContent).toContain('No target files yet');
    await click('bootstrap-create');
    expect((await repository.query(request('list-projects'))).value).toHaveLength(1);
}, 60_000);

it('refuses late generation, failed publication and corrupt bytes without partial project records', async () => {
    await setup(false);
    let entered, release, corrupt = false;
    const enteredPromise = new Promise(resolve => { entered = resolve; });
    const blocked = new Promise(resolve => { release = resolve; });
    const bridge = { bootstrapProject: async (request, options) => {
        if (request.action === 'generate') { entered(); await blocked; }
        const result = await validator.bootstrapProject(request, options);
        if (corrupt && result.bundle) result.bundle.contents[0].base64 = btoa('corrupt');
        return result;
    } };
    const workflow = createCemStudioProjectBootstrap({ validator: bridge, repository });
    try {
        await workflow.open(); await workflow.choose({ name: ['Cancelled'], preset: ['simple-conversion'] });
        await workflow.review({});
        const pending = workflow.create(); await enteredPromise; workflow.cancel(); release(); await pending;
        expect((await repository.query(request('list-projects'))).value).toHaveLength(0);
        await workflow.open(); await workflow.choose({ name: ['Failed'], preset: ['simple-conversion'] }); await workflow.review({});
        const execute = vi.spyOn(repository, 'execute').mockRejectedValueOnce(new DOMException('Storage full', 'QuotaExceededError'));
        await workflow.create(); expect(workflow.snapshot().error.message).toBe('Storage full'); execute.mockRestore();
        expect((await repository.query(request('list-projects'))).value).toHaveLength(0);
        corrupt = true;
        await workflow.create(); expect(workflow.snapshot().error).toBeDefined(); corrupt = false;
        expect((await repository.query(request('list-projects'))).value).toHaveLength(0);
        await workflow.create(); expect((await repository.query(request('list-projects'))).value).toHaveLength(1);
    } finally { workflow.dispose(); }
}, 60_000);

it('configures multiple validation inputs and creates them offline after startup', async () => {
    await setup(); await click('new-project');
    await userEvent.fill(field('name'), 'Validation');
    dialog().querySelector('cem-select[name=preset] input[value="schema-validation"]').focus();
    await userEvent.keyboard(' ');
    dialog().querySelector('form[host-command=bootstrap-type] button[type=submit]').focus();
    await userEvent.keyboard('{Enter}'); await view.whenSettled();
    expect(dialog().contains(document.activeElement)).toBe(true);
    const network = vi.spyOn(globalThis, 'fetch').mockRejectedValue(new TypeError('Offline'));
    await userEvent.fill(field('sourceFiles'), 'samples/a.xhtml\nsamples/nested/b.xhtml');
    await review();
    expect(dialog().textContent).toContain('No target files yet');
    expect(dialog().querySelector('cem-tree[label="Planned source files"]').textContent).toContain('b.xhtml');
    await click('bootstrap-create'); network.mockRestore();
    const projects = (await repository.query(request('list-projects'))).value;
    expect(projects).toHaveLength(1);
    const bundle = (await repository.query(request('export-project', { projectId: projects[0].id }))).value;
    expect(bundle.project.entries[0].kind).toBe('validation');
    expect(bundle.project.resources.filter(r => r.role === 'data').map(r => r.path)).toEqual(['samples/a.xhtml', 'samples/nested/b.xhtml']);
}, 60_000);
