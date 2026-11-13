import { afterEach, expect, it } from 'vitest';
import { createCemStudioBrowserValidator } from './feature-tour.js';
import { parseCemStudioProjectBundle } from './file-system-provider.js';

let validator;
afterEach(async () => { await validator?.close(); validator = undefined; });
const presetRequest = preset => ({ action: 'preset', preset, projectId: 'bootstrap-browser', name: 'New project', createdAt: '2026-11-13T00:00:00Z' });

it('runs the shared native presets through Studio with stable addresses after path changes', async () => {
    validator = await createCemStudioBrowserValidator();
    const catalog = await validator.bootstrapProject({ action: 'catalog' });
    expect(catalog.presets.map(p => p.id)).toEqual(['markdown-site', 'transformation-pipeline', 'simple-conversion', 'schema-validation', 'custom']);
    expect(catalog.configurationSchema.properties.commonVersion.const).toBe(validator.commonVersion);
    for (const { id } of catalog.presets) {
        const { configuration } = await validator.bootstrapProject(presetRequest(id));
        for (const resource of configuration.resources) resource.path = `moved/${resource.id}.source`;
        const generated = await validator.bootstrapProject({ action: 'generate', configuration });
        const bundle = parseCemStudioProjectBundle(new TextEncoder().encode(JSON.stringify(generated.bundle)));
        expect(await validator.decodeProjectManifest(new TextEncoder().encode(generated.manifest))).toEqual(bundle.project);
        for (const entry of bundle.project.entries) {
            const source = bundle.project.resources.find(r => r.role === 'data');
            const outcome = await validator.executeAuthoredResourceCommand({
                bytes: bundle.contents[source.id], uri: `${bundle.project.rootUri}${source.referencePath}`,
                contentType: source.contentType, schema: source.schema, resourceScheme: 'studio',
                projectId: bundle.project.id, projectRootUri: bundle.project.rootUri,
                commandResource: bundle.contents[entry.runConfigResourceId],
                dependencies: bundle.project.resources.filter(r => r.id !== source.id).map(r => ({
                    path: `${bundle.project.rootUri}${r.referencePath}`, bytes: bundle.contents[r.id], contentType: r.contentType, schema: r.schema,
                })),
            });
            expect(outcome.result.exitCode, JSON.stringify(outcome.result.diagnostics.items)).toBe(0);
            expect(outcome.output.text).not.toContain('\u001b[');
            if (id === 'markdown-site') expect(outcome.output.text).toContain('Welcome</h1>');
            if (id === 'transformation-pipeline') {
                expect(outcome.output.text).toContain('CEM pipeline');
                expect(outcome.output.text).toContain('Hello from CEM');
            }
            if (id === 'simple-conversion') expect(outcome.output.text).toContain('Hello from CEM');
        }
    }
}, 60_000);

it('refuses cancelled, invalid and retired authoring without persisting a project', async () => {
    validator = await createCemStudioBrowserValidator();
    const { configuration } = await validator.bootstrapProject(presetRequest('simple-conversion'));
    const controller = new AbortController(); controller.abort();
    await expect(validator.bootstrapProject({ action: 'generate', configuration }, { signal: controller.signal })).rejects.toMatchObject({ name: 'AbortError' });
    const late = new AbortController();
    const pending = validator.bootstrapProject({ action: 'generate', configuration }, { signal: late.signal });
    late.abort();
    await expect(pending).rejects.toMatchObject({ name: 'AbortError' });
    const invalid = structuredClone(configuration); invalid.resources[0].path = '../outside';
    await expect(validator.bootstrapProject({ action: 'generate', configuration: invalid })).rejects.toMatchObject({ code: 'cem.studio_project.resource_path_invalid' });
    const badCommand = structuredClone(configuration);
    badCommand.resources.find(r => r.role === 'run-config').text = JSON.stringify({
        $schema: 'https://cem.dev/ns/cli/command/1', schemaVersion: 1, commandSchemaVersion: 1,
        commonVersion: validator.commonVersion, binaryName: 'cem-ml', argv: ['convert', '--unknown-flag'],
    });
    await expect(validator.bootstrapProject({ action: 'generate', configuration: badCommand })).rejects.toThrow();
    expect((await validator.bootstrapProject({ action: 'generate', configuration })).bundle.project.id).toBe('bootstrap-browser');
    const bootstrap = validator.bootstrapProject;
    await validator.close(); validator = undefined;
    await expect(bootstrap({ action: 'catalog' })).rejects.toThrow('cem.runtime.generation.retired');
}, 60_000);
