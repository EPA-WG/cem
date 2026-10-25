import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { extname, resolve, sep } from 'node:path';
import { chromium } from 'playwright';
import { verifyMarkdownSite } from './verify-markdown-site.mjs';
import { verifyDataProjects } from './verify-data-projects.mjs';
import { verifyCsvInventory } from './verify-csv-inventory.mjs';
import { verifyXmlXhtmlProjects } from './verify-xml-xhtml-projects.mjs';
import { verifyHtmlCemCard } from './verify-html-cem-card.mjs';
import { verifySvgMathmlProjects } from './verify-svg-mathml-projects.mjs';
import { verifyStylesheetProjects } from './verify-stylesheet-projects.mjs';
import { verifyDataQueryProjects } from './verify-data-query-projects.mjs';
import { verifyTransformationProjects } from './verify-transformation-projects.mjs';
import { verifySchemaAuthoringProjects } from './verify-schema-authoring-projects.mjs';
import { verifyModuleMapProject } from './verify-module-map-project.mjs';
import { verifyPortableConfigProject } from './verify-portable-config-project.mjs';
import { verifyNativeProjectionProject } from './verify-native-projection-project.mjs';
import { verifyCatalogDocumentation } from './verify-catalog-documentation.mjs';

const projectRoot = resolve(import.meta.dirname, '..');
const csvOnly = process.argv.includes('--csv-only');
const documentsOnly = process.argv.includes('--documents-only');
const markdownOnly = process.argv.includes('--markdown-only');
const htmlCemOnly = process.argv.includes('--html-cem-only');
const svgMathmlOnly = process.argv.includes('--svg-mathml-only');
const stylesheetsOnly = process.argv.includes('--stylesheets-only');
const dataQueriesOnly = process.argv.includes('--data-queries-only');
const transformationsSchemasOnly = process.argv.includes('--transformations-schemas-only');
const moduleConfigsOnly = process.argv.includes('--module-configs-only');
const nativeProjectionsOnly = process.argv.includes('--native-projections-only');
const catalogDocsOnly = process.argv.includes('--catalog-docs-only');
const schemaAssets = process.argv.includes('--schema-assets');
if ([csvOnly, documentsOnly, markdownOnly, htmlCemOnly, svgMathmlOnly, stylesheetsOnly, dataQueriesOnly, transformationsSchemasOnly, moduleConfigsOnly, nativeProjectionsOnly, catalogDocsOnly].filter(Boolean).length > 1) throw new Error('Select one project verification scope.');
if (schemaAssets && !csvOnly) throw new Error('The optional profile replay requires --csv-only.');
const server = await startStaticServer(resolve(projectRoot, schemaAssets ? 'dist/schema-assets' : 'dist/static'));
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(30_000);
const errors = [];
page.on('pageerror', error => errors.push(error.message));
const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
const state = () => page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().navigation);
const preview = () => page.evaluate(() => {
    const current = globalThis.__cemStudioApplication.workbench.snapshot().projectPreview;
    return { configurationId: current.configurationId, artifactPath: current.artifactPath, runId: current.projection?.buildRun.id,
        stale: current.stale, attempt: current.attempt?.status, text: current.projection?.output.text };
});
const choose = async (name, option) => { await page.getByRole('combobox', { name, exact: true }).click(); await page.getByRole('option', { name: option, exact: true }).click(); await settle(); };
const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
const tree = async (name, owner = 'Projects') => { await page.getByRole('tree', { name: owner, exact: true }).getByRole('treeitem', { name, exact: true }).click(); await settle(); };
const treeKey = async key => { await page.getByRole('tree', { name: 'Projects', exact: true }).locator(`[role=treeitem][data-tree-value="${key}"]`).click(); await settle(); };
const submit = async (name, action) => { if (name) await page.getByRole('dialog').getByRole('textbox').fill(name); await page.getByRole('dialog').getByRole('button', { name: action, exact: true }).click(); await settle(); };
const heading = async () => assert.equal(await page.getByRole('heading', { level: 1 }).innerText(), await page.title());

try {
    if (catalogDocsOnly) {
        const catalogDocumentation = await verifyCatalogDocumentation({ page, context, url: server.url });
        assert.deepEqual(errors, []);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/catalog-documentation.json'), `${JSON.stringify({ schemaVersion: 1, catalogDocumentation }, null, 2)}\n`);
        console.log('Studio catalog documentation: all original README views, source-first starter, independent saved copies and offline draft retention passed.');
    } else if (nativeProjectionsOnly) {
        const nativeProjectionProject = await verifyNativeProjectionProject({ page, context, url: server.url });
        assert.deepEqual(errors, []);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/native-projection-projects.json'), `${JSON.stringify({ schemaVersion: 1, nativeProjectionProject }, null, 2)}\n`);
        console.log('Studio native projections: owned source views, explicit codec exports, repairs and independent offline workflows passed.');
    } else if (moduleConfigsOnly) {
        const portableContext = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
        const portablePage = await portableContext.newPage(); portablePage.setDefaultTimeout(30_000);
        portablePage.on('pageerror', error => errors.push(error.message));
        const [moduleMapProject, portableConfigProject] = await Promise.all([
            verifyModuleMapProject({ page, context, url: server.url }),
            verifyPortableConfigProject({ page: portablePage, context: portableContext, url: server.url }).catch(async error => {
                await portablePage.screenshot({ path: '/tmp/cem-studio-portable-config-failure.png', fullPage: true }).catch(() => undefined);
                throw error;
            }),
        ]);
        assert.deepEqual(errors, []);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/module-config-projects.json'), `${JSON.stringify({ schemaVersion: 1, moduleMapProject, portableConfigProject }, null, 2)}\n`);
        console.log('Studio module maps and portable configurations: native deployments, identities, host import/export and independent offline workflows passed.');
    } else if (transformationsSchemasOnly) {
        const transformationProjects = await verifyTransformationProjects({ page, context, url: server.url });
        const schemaAuthoringProjects = await verifySchemaAuthoringProjects({ page, context, url: server.url });
        assert.deepEqual(errors, []);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/transformations-schemas.json'), `${JSON.stringify({ schemaVersion: 1, transformationProjects, schemaAuthoringProjects }, null, 2)}\n`);
        console.log('Studio transformations and schemas: native artifacts, source/instance contracts, repairs and independent offline workflows passed.');
    } else if (dataQueriesOnly) {
        const dataQueryProjects = await verifyDataQueryProjects({ page, context, url: server.url });
        assert.deepEqual(errors, []);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/data-query-projects.json'), `${JSON.stringify({ schemaVersion: 1, dataQueryProjects }, null, 2)}\n`);
        console.log('Studio data queries: native CEM-QL/XPath, parameter maps, original provenance and independent offline workflows passed.');
    } else if (stylesheetsOnly) {
        const stylesheetProjects = await verifyStylesheetProjects({ page, context, url: server.url });
        assert.deepEqual(errors, []);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/stylesheet-projects.json'), `${JSON.stringify({ schemaVersion: 1, stylesheetProjects }, null, 2)}\n`);
        console.log('Studio stylesheets: native CSS/SCSS, selector results, source repairs and independent offline previews passed.');
    } else if (svgMathmlOnly) {
        const svgMathmlProjects = await verifySvgMathmlProjects({ page, context, url: server.url });
        assert.deepEqual(errors, []);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/svg-mathml-projects.json'), `${JSON.stringify({ schemaVersion: 1, svgMathmlProjects }, null, 2)}\n`);
        console.log('Studio SVG/MathML projects: native families, namespace repairs, exact foreign-content previews and independent offline builds passed.');
    } else if (htmlCemOnly) {
        const htmlCemCard = await verifyHtmlCemCard({ page, context, url: server.url });
        assert.deepEqual(errors, []);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/html-cem-card.json'), `${JSON.stringify({ schemaVersion: 1, htmlCemCard }, null, 2)}\n`);
        console.log('Studio HTML/CEM card: shared component, native contracts, policy repair, saved data and offline live/build previews passed.');
    } else if (markdownOnly) {
        const markdownSite = await verifyMarkdownSite({ page, context, url: server.url });
        assert.deepEqual(errors, []);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/markdown-site.json'), `${JSON.stringify({ schemaVersion: 1, markdownSite }, null, 2)}\n`);
        console.log('Studio Markdown site: catalog, article commands, metadata repair, native source navigation, exact assets and offline saved builds passed.');
    } else if (documentsOnly) {
        const xmlXhtmlProjects = await verifyXmlXhtmlProjects({ page, context, url: server.url });
        assert.deepEqual(errors, []);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/xml-xhtml-projects.json'), `${JSON.stringify({ schemaVersion: 1, xmlXhtmlProjects }, null, 2)}\n`);
        console.log('Studio XML/XHTML projects: catalog, native namespaces, source repair, saved edits, generated previews and offline rebuilding passed.');
    } else if (csvOnly) {
        const csvInventory = await verifyCsvInventory({ page, context, url: server.url });
        assert.deepEqual(errors, []);
        const packages = await page.evaluate(() => globalThis.__cemStudioApplication.packageStatus());
        if (schemaAssets) {
            assert.equal(packages.state, 'idle'); assert.equal(packages.demandCount, 1); assert.ok(packages.runtimeCount >= 4);
        }
        const report = resolve(projectRoot, `../../dist/reports/cem-studio/csv-inventory-${schemaAssets ? 'schema-assets' : 'default'}.json`);
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(report, `${JSON.stringify({ schemaVersion: 1, deploymentProfile: schemaAssets ? 'schema-assets' : 'default', csvInventory, packages }, null, 2)}\n`);
        console.log(`Studio CSV inventory: ${schemaAssets ? 'optional schema-assets' : 'default'} deployment source policies, saved edits and offline reports passed.`);
    } else {
    await page.goto(server.url); await ready();
    assert.equal((await state()).selection.key, 'samples'); await heading();
    const categories = ['Data', 'Documents and UI', 'Queries and transformations', 'Schema authoring'];
    await tree('Content types and schemas');
    assert.equal((await state()).selection.collection, 'content-types-and-schemas');
    assert.equal(new URL(page.url()).searchParams.get('collection'), 'content-types-and-schemas');
    assert.deepEqual((await state()).children.map(item => item.name), categories);
    const listing = page.getByRole('list', { name: 'Folder contents' });
    for (const item of (await state()).children) {
        assert.ok(item.description);
        assert.ok(await listing.getByRole('button', { name: item.name, exact: true }).isVisible());
        assert.ok((await listing.innerText()).includes(item.description));
    }
    await heading();
    await page.screenshot({ path: '/tmp/cem-studio-content-type-folders.png', fullPage: true });
    const catalogKeys = await page.evaluate(() => {
        const keys = [];
        const visit = item => { keys.push(item.key); item.children.forEach(visit); };
        visit(globalThis.__cemStudioApplication.workbench.snapshot().navigation.samples);
        return keys;
    });
    assert.equal(new Set(catalogKeys).size, catalogKeys.length, 'Every catalog occurrence has a unique tree value');
    await treeKey('collection=content-types-data');
    assert.equal((await state()).selection.collection, 'content-types-data');
    assert.ok(await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: 'Supplier pipeline', exact: true }).isVisible());
    await treeKey('collection=content-types-data&sample=supplier-pipeline');
    assert.equal((await state()).content, 'project');
    assert.equal(new URL(page.url()).searchParams.get('collection'), 'content-types-data');
    const contextualSampleUrl = page.url();
    await tree('data', 'Project files');
    const sampleFile = (await state()).children.find(item => item.name === 'stock.csv');
    assert.ok(sampleFile);
    await listing.getByRole('button', { name: sampleFile.name, exact: true }).click(); await settle();
    assert.equal((await state()).content, 'sample-file');
    assert.equal(new URL(page.url()).searchParams.get('collection'), 'content-types-data');
    assert.equal(new URL(page.url()).searchParams.get('file'), sampleFile.resourceId);
    const readonlySource = page.locator('cem-source-editor[readonly] [role=textbox]');
    const sampleText = await readonlySource.evaluate(element => element.value);
    assert.ok(sampleText.includes('quantity'));
    assert.equal(await readonlySource.getAttribute('aria-readonly'), 'true');
    const readonlyIdentity = await readonlySource.elementHandle();
    const catalogBranch = page.getByRole('tree', { name: 'Projects', exact: true }).locator('[role=treeitem][data-tree-value="collection=content-types-and-schemas"]');
    await catalogBranch.locator('[part~=marker]').click(); await settle();
    assert.equal(await catalogBranch.getAttribute('aria-expanded'), 'false');
    assert.equal((await state()).selection.file, sampleFile.resourceId);
    assert.equal((await state()).trees.projects.selected, 'collection=content-types-data&sample=supplier-pipeline');
    assert.ok(await readonlyIdentity.evaluate(element => element.isConnected));
    assert.equal(await readonlySource.evaluate(element => element.value), sampleText);
    await page.goto(`${server.url}/?sample=supplier-pipeline&file=${sampleFile.resourceId}`); await ready();
    assert.equal((await state()).content, 'sample-file');
    assert.equal(new URL(page.url()).searchParams.get('collection'), null);
    assert.equal(await readonlySource.evaluate(element => element.value), sampleText);
    await page.goto(contextualSampleUrl); await ready();
    await click('Build project');
    const artifactFrame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
    await artifactFrame.waitFor();
    await page.waitForFunction(() => document.querySelector('cem-media-preview[mode="artifact-tree"] [role=status]')?.textContent === 'Artifact view ready');
    assert.equal(await artifactFrame.getAttribute('sandbox'), 'allow-same-origin');
    assert.equal(await artifactFrame.evaluate(frame => frame.contentDocument.body.textContent.includes('Supplier availability')), true);
    await choose('Preview artifact', 'generated/availability.json');
    assert.equal(JSON.parse((await preview()).text)[0].quantity, 7);
    assert.equal(new URL(page.url()).searchParams.get('artifact'), 'generated/availability.json');
    assert.equal(await page.evaluate(async () => (await globalThis.__cemStudioApplication.repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation: 'list-projects', parameters: {} })).value.length), 0);
    await click('Clone selected'); await submit('Navigator Supplier', 'Clone');
    const projectId = (await state()).selection.project;
    assert.ok(projectId); await heading();
    const projectUrl = page.url();
    assert.ok(new URL(projectUrl).searchParams.get('project') === projectId);
    await tree('data', 'Project files');
    assert.equal((await state()).selection.folder, 'data/');
    const file = (await state()).children.find(item => item.name === 'stock.csv');
    await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: 'stock.csv', exact: true }).click(); await settle();
    assert.equal(new URL(page.url()).searchParams.get('file'), file.resourceId); await heading();
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    await page.screenshot({ path: '/tmp/cem-studio-navigator.png', fullPage: true });
    const original = await editor.evaluate(element => element.value);
    assert.equal(original, sampleText);
    await editor.fill(`${original}\n`); await settle();
    await page.evaluate(() => globalThis.__cemStudioApplication.workbench.buildProjectPreview()); await settle();
    await page.evaluate(() => globalThis.__cemStudioApplication.workbench.selectPreviewArtifact('generated/availability.json')); await settle();
    assert.equal((await preview()).stale, true);
    assert.equal(await editor.evaluate(element => element.value), `${original}\n`);
    const runId = (await preview()).runId;
    const editorIdentity = await editor.elementHandle();
    for (const mode of ['cem-theme-dark', 'cem-theme-contrast-light', 'cem-theme-contrast-dark', 'cem-theme-native', 'cem-theme-light']) {
        await page.evaluate(mode => globalThis.__cemStudioApplication.shell.theme.setMode(mode), mode); await settle();
        assert.ok(await editorIdentity.evaluate(element => element.isConnected)); await heading();
        const palette = await page.locator('cem-workspace [part~=region][part~=editor]').evaluate(element => { const style = getComputedStyle(element); return [style.color, style.backgroundColor]; });
        assert.notEqual(palette[0], palette[1]);
    }
    // A folder change is guarded by the same source admission as a file change.
    await page.getByRole('tree', { name: 'Project files', exact: true }).getByRole('treeitem', { name: 'data', exact: true }).click(); await settle();
    assert.ok(await page.getByRole('dialog', { name: 'Resolve drafts before navigation' }).isVisible());
    await click('Cancel navigation');
    assert.equal((await state()).selection.file, file.resourceId);
    assert.equal(await editor.evaluate(element => element.value), `${original}\n`);
    assert.equal((await preview()).runId, runId);
    const draftRoute = new URL(page.url()).searchParams.toString();
    await treeKey('collection=content-types-and-schemas');
    if (await page.getByRole('dialog', { name: 'Resolve drafts before navigation' }).isVisible()) {
        await click('Cancel navigation');
    } else {
        assert.equal((await state()).selection.collection, 'content-types-and-schemas');
        assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().dirty), true);
        await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(route), draftRoute); await settle();
    }
    assert.equal((await state()).selection.file, file.resourceId);
    assert.equal(await editor.evaluate(element => element.value), `${original}\n`, 'Browsing the catalog preserves the existing draft');
    await click('Save and validate');
    await page.reload(); await ready();
    assert.equal((await state()).selection.file, file.resourceId); await heading();
    assert.equal(await page.getByRole('textbox', { name: 'CEM source', exact: true }).evaluate(element => element.value), `${original}\n`);
    assert.equal((await preview()).artifactPath, 'generated/availability.json');
    assert.equal((await preview()).runId, undefined);
    const bookmarked = page.url();
    await page.goto(projectUrl); await ready();
    await click('Create'); await submit('notes.cem', 'Create');
    assert.equal((await state()).selected.name, 'notes.cem');
    await click('Clone selected'); await submit('notes-copy.cem', 'Clone');
    assert.equal((await state()).selected.name, 'notes-copy.cem');
    await click('Delete selected'); await submit(undefined, 'Delete');
    assert.equal((await state()).content, 'project');
    await page.goto(bookmarked); await ready();
    const fileUrl = page.url();
    const examplesRoot = page.getByRole('tree', { name: 'Projects', exact: true }).locator('[role=treeitem][data-tree-value="samples"]');
    assert.equal(await examplesRoot.getAttribute('aria-label'), 'Examples');
    await examplesRoot.click(); await settle();
    assert.equal((await state()).selection.key, 'samples');
    await page.goBack(); await settle();
    assert.equal(page.url(), fileUrl); assert.equal((await state()).selection.file, file.resourceId);
    await page.goForward(); await settle(); assert.equal((await state()).selection.key, 'samples');
    await page.reload(); await ready(); assert.equal((await state()).selection.key, 'samples');
    await tree('Content types and schemas'); await treeKey('collection=content-types-data');
    await treeKey('collection=content-types-data&sample=supplier-pipeline');
    await click('Clone selected'); await submit('Navigator Supplier independent', 'Clone');
    const independentProjectId = (await state()).selection.project;
    assert.notEqual(independentProjectId, projectId);
    await tree('data', 'Project files');
    await listing.getByRole('button', { name: 'stock.csv', exact: true }).click(); await settle();
    assert.equal(await editor.evaluate(element => element.value), original, 'A second clone starts from the unchanged original');
    const copies = await page.evaluate(async ({ firstId, secondId, resourceId }) => {
        const query = async projectId => (await globalThis.__cemStudioApplication.repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation: 'export-project', parameters: { projectId } })).value;
        const first = await query(firstId), second = await query(secondId);
        return { firstRoot: first.project.rootUri, secondRoot: second.project.rootUri,
            firstText: new TextDecoder().decode(first.contents[resourceId]), secondText: new TextDecoder().decode(second.contents[resourceId]) };
    }, { firstId: projectId, secondId: independentProjectId, resourceId: file.resourceId });
    assert.notEqual(copies.firstRoot, copies.secondRoot);
    assert.equal(copies.firstText, `${original}\n`);
    assert.equal(copies.secondText, original);
    await tree('Content types and schemas'); await treeKey('collection=content-types-data');
    await treeKey('collection=content-types-data&sample=supplier-pipeline');
    await tree('data', 'Project files');
    await listing.getByRole('button', { name: 'stock.csv', exact: true }).click(); await settle();
    assert.equal(await readonlySource.evaluate(element => element.value), original, 'Editing saved clones never changes the original sample');
    const inset = await page.locator('cem-studio-shell').evaluate(shell => {
        shell.style.setProperty('--cem-inset-container', '23px');
        const region = shell.querySelector('cem-workspace [part~=editor] > [part=content]');
        const value = getComputedStyle(region).paddingInlineStart;
        shell.style.removeProperty('--cem-inset-container');
        return value;
    });
    assert.equal(inset, '0px'); // Explicit shared flush appearance owns zero content inset.
    const headerInset = await page.locator('cem-studio-shell').evaluate(shell => {
        shell.style.setProperty('--cem-control-padding-x', '23px');
        const value = getComputedStyle(shell.querySelector('cem-workspace [part~=editor] > [part~=heading]')).paddingInlineStart;
        shell.style.removeProperty('--cem-control-padding-x'); return value;
    });
    assert.equal(headerInset, '23px');
    const layouts = [];
    for (const width of [1440, 900, 390, 320]) {
        await page.setViewportSize({ width, height: 1000 });
        const geometry = await page.locator('cem-workspace').evaluate(host => {
            const root = host.querySelector('[part=root]');
            const rect = part => host.querySelector(`[part~="${part}"]`).getBoundingClientRect().toJSON();
            return { width: innerWidth, pageWidth: document.documentElement.scrollWidth, navigation: rect('navigation'), editor: rect('editor'), columns: getComputedStyle(root).gridTemplateColumns.split(' ').length };
        });
        assert.equal(geometry.width, geometry.pageWidth); assert.equal(geometry.columns, width >= 1200 ? 3 : width >= 900 ? 2 : 1); layouts.push(geometry);
    }
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.emulateMedia({ forcedColors: 'active' });
    const paint = await page.locator('cem-workspace').evaluate(host => [...host.querySelectorAll('[part~=region]')].filter(element => element.checkVisibility()).map(element => { const style = getComputedStyle(element); return { color: style.color, background: style.backgroundColor, border: style.borderTopWidth }; }));
    assert.equal(paint.length, 3); for (const region of paint) { assert.notEqual(region.color, region.background); assert.notEqual(region.border, '0px'); }
    await page.emulateMedia({ forcedColors: 'none' });
    // Run the ordinary Supplier configuration through the new file/configuration route.
    await page.goto(`${projectUrl}&configuration=availability`); await ready();
    assert.equal((await state()).error, undefined);
    await page.locator('cem-expansion[label="Commands and results"] > [part~=root] [part~=header]').first().click(); await settle();
    await click('Run configuration');
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().status), 'projected');
    const provenance = await page.evaluate(async () => {
        const { workbench } = globalThis.__cemStudioApplication;
        const index = workbench.snapshot().projection.provenance.findIndex(frame => frame.uri?.endsWith('/templates/view.cemt'));
        if (index < 0) throw new Error('Supplier source provenance is missing');
        await workbench.navigateProvenance(index);
        return { resourceId: workbench.snapshot().resourceId, selection: workbench.snapshot().selection };
    });
    await settle();
    assert.equal(new URL(page.url()).searchParams.get('file'), provenance.resourceId);
    assert.equal(provenance.selection.kind, 'provenance'); await heading();
    await page.goto(`${projectUrl}&configuration=availability`); await ready();
    await page.evaluate(async () => { await navigator.serviceWorker.ready; });
    await context.setOffline(true); await page.reload(); await ready();
    assert.equal((await state()).content, 'file');
    await page.evaluate(() => globalThis.__cemStudioApplication.workbench.buildProjectPreview()); await settle();
    assert.equal((await preview()).attempt, 'succeeded');
    await page.locator('cem-expansion[label="Commands and results"] > [part~=root] [part~=header]').first().click(); await settle();
    await click('Run configuration');
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().status), 'projected');
    await page.goto(`${server.url}/?collection=content-types-and-schemas`); await ready();
    assert.equal((await state()).content, 'folder');
    assert.deepEqual((await state()).children.map(item => item.name), categories);
    for (const name of categories) assert.ok(await listing.getByRole('button', { name, exact: true }).isVisible());
    await treeKey('collection=content-types-data');
    await listing.getByRole('button', { name: 'Supplier pipeline', exact: true }).click(); await settle();
    assert.equal(new URL(page.url()).searchParams.get('collection'), 'content-types-data');
    await tree('data', 'Project files');
    await listing.getByRole('button', { name: 'stock.csv', exact: true }).click(); await settle();
    await page.reload(); await ready();
    assert.equal((await state()).content, 'sample-file');
    assert.equal((await state()).selection.collection, 'content-types-data');
    assert.equal((await state()).selection.file, sampleFile.resourceId);
    assert.equal(await readonlySource.evaluate(element => element.value), sampleText);
    await context.setOffline(false);
    const markdownSite = await verifyMarkdownSite({ page, context, url: server.url });
    const dataProjects = await verifyDataProjects({ page, context, url: server.url });
    const csvInventory = await verifyCsvInventory({ page, context, url: server.url });
    const xmlXhtmlProjects = await verifyXmlXhtmlProjects({ page, context, url: server.url });
    const htmlCemCard = await verifyHtmlCemCard({ page, context, url: server.url });
    const svgMathmlProjects = await verifySvgMathmlProjects({ page, context, url: server.url });
    const stylesheetProjects = await verifyStylesheetProjects({ page, context, url: server.url });
    const dataQueryProjects = await verifyDataQueryProjects({ page, context, url: server.url });
    const transformationProjects = await verifyTransformationProjects({ page, context, url: server.url });
    const schemaAuthoringProjects = await verifySchemaAuthoringProjects({ page, context, url: server.url });
    const moduleMapProject = await verifyModuleMapProject({ page, context, url: server.url });
    const portableConfigProject = await verifyPortableConfigProject({ page, context, url: server.url });
    const nativeProjectionProject = await verifyNativeProjectionProject({ page, context, url: server.url });
    const catalogDocumentation = await verifyCatalogDocumentation({ page, context, url: server.url });
    assert.deepEqual(errors, []);
    await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
    await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/navigation.json'), `${JSON.stringify({ schemaVersion: 1, urlSelection: true, history: true, modelTrees: true, sampleBrowsing: true, nestedCatalog: true, uniqueCatalogOccurrences: true, contextualAndLegacySampleUrls: true, collapsedCatalogSelection: true, readonlyOriginals: true, independentClones: true, catalogDraftRetention: true, offlineCatalog: true, createCloneDelete: true, guardedDraft: true, themeRetention: true, bookmarkReload: true, supplierNativeRun: true, markdownSite, dataProjects, csvInventory, xmlXhtmlProjects, htmlCemCard, svgMathmlProjects, stylesheetProjects, dataQueryProjects, transformationProjects, schemaAuthoringProjects, moduleMapProject, portableConfigProject, nativeProjectionProject, catalogDocumentation, offline: true, layouts, forcedColors: paint }, null, 2)}\n`);
    console.log('Studio navigation: nested catalog, contextual/legacy URLs, readonly originals, independent clones, drafts, themes, responsive layout and offline catalog/Supplier/Markdown site/JSON and YAML report/CSV inventory/XML and XHTML builds passed.');
    }
} catch (error) {
    console.error('Studio navigation verification failed:', error);
    let timer;
    const failure = await Promise.race([
        state().catch(() => ({})),
        new Promise(resolveFailure => { timer = setTimeout(() => resolveFailure({}), 5_000); }),
    ]).finally(() => clearTimeout(timer));
    console.error('Navigation failure:', JSON.stringify({ selection: failure.selection, title: failure.title, error: failure.error, status: failure.status }));
    await page.screenshot({ path: '/tmp/cem-studio-navigation-failure.png', fullPage: true, timeout: 5_000 }).catch(() => undefined);
    throw error;
} finally { await browser.close(); await server.close(); }

async function startStaticServer(root) {
    const security = JSON.parse(await readFile(resolve(root, 'security-headers.json'), 'utf8'));
    return new Promise((resolveServer, reject) => {
        const server = createServer(async (request, response) => {
            try {
                const requestPath = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
                const candidate = resolve(root, `.${requestPath === '/' ? '/index.html' : requestPath}`);
                const rootPrefix = root.endsWith(sep) ? root : `${root}${sep}`;
                if (candidate !== root && !candidate.startsWith(rootPrefix)) {
                    response.writeHead(403).end('Forbidden');
                    return;
                }
                const metadata = await stat(candidate);
                const path = metadata.isDirectory() ? resolve(candidate, 'index.html') : candidate;
                const bytes = await readFile(path);
                response.writeHead(200, {
                    ...security.headers,
                    'content-type': contentType(path),
                    'cache-control': 'no-store',
                    'service-worker-allowed': '/',
                });
                response.end(bytes);
            } catch {
                response.writeHead(404).end('Not found');
            }
        });
        server.on('error', reject);
        server.listen(0, '127.0.0.1', () => {
            const address = server.address();
            if (!address || typeof address === 'string') {
                reject(new Error('static server did not expose a TCP address'));
                return;
            }
            resolveServer({
                url: `http://127.0.0.1:${address.port}`,
                close: () =>
                    new Promise((resolveClose, rejectClose) => {
                        server.close((error) => (error ? rejectClose(error) : resolveClose()));
                    }),
            });
        });
    });
}

function contentType(path) {
    return (
        {
            '.css': 'text/css; charset=utf-8',
            '.html': 'text/html; charset=utf-8',
            '.js': 'text/javascript; charset=utf-8',
            '.json': 'application/json; charset=utf-8',
            '.svg': 'image/svg+xml',
            '.wasm': 'application/wasm',
            '.webmanifest': 'application/manifest+json',
        }[extname(path)] ?? 'application/octet-stream'
    );
}
