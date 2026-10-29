import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseCemStudioProjectBundle } from '../src/file-system-provider.js';
import { auditCatalog, inventoryWorkflows, catalogCoverageMarkdown } from './catalog-audit.mjs';

/** Inspect explicit catalog/control metadata; never import or parse project data. */
export async function collectCatalogAudit(workspaceRoot) {
    const json = async path => JSON.parse(await readFile(resolve(workspaceRoot, path), 'utf8'));
    const generated = 'packages/cem-studio/generated/feature-tour';
    const catalog = await json(`${generated}/catalog.json`), index = await json(`${generated}/index.json`);
    const definitions = await Promise.all((await readdir(resolve(workspaceRoot, 'examples/projects'), { withFileTypes: true }))
        .filter(item => item.isDirectory()).sort((a, b) => a.name.localeCompare(b.name))
        .map(item => json(`examples/projects/${item.name}/example.json`)));
    const projects = await Promise.all(definitions.map(async definition => {
        const source = definition.sources.find(source => source.path.toLowerCase() === 'readme.md');
        const text = source && await readFile(resolve(workspaceRoot, source.source), 'utf8');
        const record = index.projects.find(record => record.id === definition.id);
        assert.ok(record, `Catalog has no authored project ${definition.id}`);
        const archive = parseCemStudioProjectBundle(new Uint8Array(await readFile(resolve(workspaceRoot, generated, record.archive.slice(2)))));
        if (source) {
            assert.equal(new TextDecoder('utf-8', { fatal: true }).decode(archive.contents[source.id]), text, `${definition.id} archived README drifted`);
            assert.equal(archive.project.resources.find(resource => resource.id === source.id)?.sha256,
                createHash('sha256').update(text).digest('hex'), `${definition.id} README hash drifted`);
        }
        assert.deepEqual(archive.project.entries.map(entry => entry.id), definition.configurations.map(configuration => configuration.id));
        const missingSources = (await Promise.all(definition.sources.map(async source => {
            const exists = await stat(resolve(workspaceRoot, source.source)).then(value => value.isFile(), () => false);
            return exists ? undefined : source.source;
        }))).filter(Boolean);
        return { definition, readme: source && { id: source.id, text }, missingSources, bundleResourceCount: archive.project.resources.length };
    }));
    const schema = index.projects.find(project => project.id === 'schema-examples');
    const bundle = async record => parseCemStudioProjectBundle(new Uint8Array(await readFile(resolve(workspaceRoot, generated, record.archive.slice(2)))));
    const [schemaBundle, starterBundle, seed] = await Promise.all([bundle(schema), bundle(index.starter), json(`${generated}/feature-tour.project.json`)]);
    const commands = project => [
        ...catalog.examples.map(example => ({ entryId: project.entries.find(entry => entry.runConfigResourceId === example.runConfigResourceId)?.id,
            command: 'validate', source: example.resourceId })),
        ...catalog.workbenches.map(workbench => ({ entryId: workbench.entryId, command: workbench.commandArguments[0], source: workbench.resourceId })),
    ];
    const documentation = (project, contents) => {
        const resource = project.resources.find(resource => resource.path.toLowerCase() === 'readme.md');
        const bytes = resource && contents[resource.id];
        return resource && bytes && { id: resource.id, text: new TextDecoder('utf-8', { fatal: true }).decode(bytes) };
    };
    const schemaReadme = documentation(schemaBundle.project, schemaBundle.contents);
    const seedDoc = catalog.documentation?.find(document => document.path.toLowerCase() === 'readme.md');
    let seedReadme;
    if (seedDoc) {
        const bytes = await readFile(resolve(workspaceRoot, seedDoc.source));
        assert.equal(createHash('sha256').update(bytes).digest('hex'), seedDoc.sha256, 'Feature Tour README source hash drifted');
        assert.equal(seed.resources.find(resource => resource.id === seedDoc.resourceId)?.sha256, seedDoc.sha256);
        assert.deepEqual(new Uint8Array(bytes), schemaBundle.contents[seedDoc.resourceId], 'Generated README copies drifted');
        seedReadme = { id: seedDoc.resourceId, text: bytes.toString('utf8'), source: seedDoc.source };
        schemaReadme.source = seedDoc.source;
    }
    const starterReadme = documentation(starterBundle.project, starterBundle.contents);
    if (starterReadme) {
        starterReadme.source = 'examples/projects/starter.md';
        const bytes = await readFile(resolve(workspaceRoot, starterReadme.source));
        assert.deepEqual(new Uint8Array(bytes), starterBundle.contents[starterReadme.id], 'Starter README source drifted');
        assert.equal(starterBundle.project.resources.find(resource => resource.id === starterReadme.id)?.sha256,
            createHash('sha256').update(bytes).digest('hex'));
    }
    const servedFiles = catalog.examples.map(example => ({ path: example.path, asset: example.asset.replace(/^\.\//u, '') }));
    const generatedProjects = [
        { id: 'feature-tour', project: seed, readme: seedReadme, commands: commands(seed), servedFiles },
        { id: 'schema-examples', ...schemaBundle, readme: schemaReadme, commands: commands(schemaBundle.project), servedFiles },
        { id: 'starter', ...starterBundle, readme: starterReadme },
    ];
    const runtime = await json('packages/cem-ml-npm/dist/cem-ml-runtime.json');
    return auditCatalog({ projects, generatedProjects,
        registry: await json('packages/cem_ml/schema/schema-assets-registry-v1.json'),
        operations: runtime.capabilities.browser.operations,
        workflows: inventoryWorkflows(await readFile(resolve(workspaceRoot, 'docs/studio-content-types.md'), 'utf8')) });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
    const workspaceRoot = resolve(import.meta.dirname, '../../..');
    const report = await collectCatalogAudit(workspaceRoot);
    const markdown = catalogCoverageMarkdown(report), path = resolve(workspaceRoot, 'docs/studio-catalog-coverage.md');
    if (process.argv.includes('--write')) await writeFile(path, markdown);
    else assert.equal(await readFile(path, 'utf8'), markdown, 'Catalog coverage documentation drifted; run audit-catalog.mjs --write and review the changes.');
    const directory = resolve(workspaceRoot, 'dist/reports/cem-studio');
    await mkdir(directory, { recursive: true });
    await writeFile(resolve(directory, 'catalog-audit.json'), `${JSON.stringify(report, null, 2)}\n`);
    console.log(`Catalog audit: ${report.counts.authoredProjects} authored projects, ${report.counts.authoredConfigurations} configurations, ${report.counts.documentedWorkflows} documented workflows, ${report.counts.registeredPackages} native packages.`);
}
