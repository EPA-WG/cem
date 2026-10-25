import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { parseCemStudioProjectBundle } from './file-system-provider.js';

import {
    CEM_STUDIO_FEATURE_TOUR_SEED_ID,
    createCemStudioBrowserValidator,
    installCemStudioFeatureTour,
} from './feature-tour.js';
import { CEM_STUDIO_REPOSITORY_ID, createCemStudioProjectRepository } from './repository.js';
let invalidLogin, loginSchema, loginPackage, loginHtml, loginCem, semanticsQuery, semanticsContract, semanticsCemql, viewTemplate, viewExpected, viewGraph;
beforeAll(async () => {
    [invalidLogin, loginSchema, loginPackage, loginHtml, loginCem, semanticsQuery, semanticsContract, semanticsCemql, viewTemplate, viewExpected, viewGraph] = await Promise.all([
        new URL('../../../examples/projects/login/login-invalid.cem', import.meta.url),
        new URL('../../../examples/projects/login/login-schema.cem', import.meta.url),
        new URL('../../../examples/projects/login/login-schema-package.cem', import.meta.url),
        new URL('../../../examples/semantic/login.html', import.meta.url),
        new URL('../../../examples/cem-ml/login.cem', import.meta.url),
        new URL('../../../examples/projects/login/queries/semantics.xpath', import.meta.url),
        new URL('../../../examples/projects/login/expected/semantics.json', import.meta.url),
        new URL('../../../examples/projects/login/queries/semantics.cemql', import.meta.url),
        new URL('../../../examples/projects/login/templates/view.cemt', import.meta.url),
        new URL('../../../examples/projects/login/expected/view.html', import.meta.url),
        new URL('../../../examples/projects/login/graphs/view.cem', import.meta.url),
    ].map(async url => {
        const response = await fetch(url);
        if (!response.ok) throw new Error(`Example resource is unavailable: ${url}`);
        return response.text();
    }));
});

const repositories = [];
const validators = [];

afterEach(async () => {
    await Promise.all(validators.splice(0).map((validator) => validator.close()));
    await Promise.all(repositories.splice(0).map(async (repository) => {
        repository.close();
        await repository.deleteDatabase();
    }));
});

describe('CEM Studio Feature Tour copies', () => {
    for (const projectId of ['cem-transform', 'xslt-transform', 'cem-schema', 'json-schema', 'relax-ng', 'module-maps', 'portable-config']) {
        it(`runs ${projectId} authored native configurations and repairs original sources`, async () => {
            const { bundle, definition, decode } = await authoredProject(projectId);
            const validator = await createCemStudioBrowserValidator(); validators.push(validator);
            for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph')) {
                const entry = bundle.project.entries.find(item => item.id === configuration.id);
                const options = projectCommandOptions(bundle, entry, `${projectId}-source-copy`);
                const commandResource = bundle.contents[entry.runConfigResourceId], command = JSON.parse(decode(entry.runConfigResourceId));
                const run = overrides => command.argv
                    ? validator.executeAuthoredResourceCommand({ ...options, ...overrides, commandResource })
                    : validator.validateResource({ ...options, ...overrides });
                const outcome = await run().catch(error => error);
                if (!outcome.result) throw new Error(`${projectId}/${configuration.id}: ${outcome.message}`, { cause: outcome });
                expect(outcome.result.exitCode, JSON.stringify(outcome.result.diagnostics.items)).toBe(configuration.expectedExit ?? 0);
                if ((configuration.expectedExit ?? 0) === 0) expect(outcome.result.diagnostics.items.filter(item => ['error', 'fatal'].includes(item.severity))).toEqual([]);
                if (configuration.expectedOutput) expect(outcome.output.text).toBe(decode(configuration.expectedOutput));
                for (const text of configuration.expectedText ?? []) expect(outcome.output.text).toContain(text);
                const expected = configuration.expectedValidation;
                if (expected) {
                    expect(outcome.result.diagnostics.items.filter(item => item.severity === 'error')).toHaveLength(expected.errorCount);
                    expect(outcome.result.diagnostics.items.filter(item => item.severity === 'warning')).toHaveLength(expected.warningCount);
                    if (configuration.repair) {
                        const source = bundle.project.resources.find(item => item.id === (expected.source ?? configuration.source));
                        const diagnostic = outcome.result.diagnostics.items.find(item => item.code === (expected.errorCode ?? expected.warningCode));
                        expect(diagnostic).toMatchObject({ severity: expected.errorCode ? 'error' : 'warning', uri: `studio://${projectId}-source-copy/${source.path}` });
                        for (const [key, value] of Object.entries(expected.location ?? {})) expect(diagnostic[key] ?? null).toBe(value);
                        const original = decode(configuration.source), repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                        expect(repaired).not.toBe(original);
                        const fixed = await run({ bytes: new TextEncoder().encode(repaired) });
                        expect(fixed.result.exitCode).toBe(0);
                        expect(fixed.result.diagnostics.items.filter(item => ['error', 'fatal', 'warning'].includes(item.severity))).toEqual([]);
                        expect(decode(configuration.source)).toBe(original);
                    }
                }
            }
        }, 60_000);
    }

    for (const projectId of ['module-maps', 'portable-config']) {
        it(`builds ${projectId} native artifacts with owned identities and independent roots`, async () => {
            const { bundle, definition, decode } = await authoredProject(projectId);
            const validator = await createCemStudioBrowserValidator(); validators.push(validator);
            for (const configuration of definition.configurations.filter(item => item.kind === 'transformation-graph')) {
                const entry = bundle.project.entries.find(item => item.id === configuration.id), command = JSON.parse(decode(entry.runConfigResourceId));
                const project = `${projectId}-${configuration.id}-copy`, root = `studio://${project}/`;
                const run = (edits = {}, id = project) => validator.runResourceCommand({ ...projectCommandOptions(bundle, entry, id, edits), argv: command.argv });
                const baseline = await run();
                expect(entry.entryArtifact).toBe(configuration.entryArtifact);
                expect(baseline.outputs).toHaveLength(Object.keys(configuration.expectedGraphOutputs).length);
                for (const [path, expected] of Object.entries(configuration.expectedGraphOutputs)) expect(baseline.outputs.find(item => item.uri.endsWith(`/${path}`)).bytes).toEqual(bundle.contents[expected]);
                const product = baseline.result.result.value.value.value;
                for (const artifact of product.artifacts) {
                    const path = artifact.destination.slice(root.length);
                    if (configuration.expectedUnmappedOutputs?.includes(path)) expect(artifact.outputSpans ?? []).toEqual([]);
                    else for (const source of configuration.expectedProvenance ?? []) expect(JSON.stringify(artifact.outputSpans)).toContain(`/${source}`);
                }
                if (configuration.expectedNativeAssets) {
                    const manifest = product.moduleAssetManifest;
                    expect(manifest.hashScheme).toBe('sha256'); expect(manifest.hash).toBe(configuration.expectedNativeAssetHash);
                    expect(manifest.assetCount).toBe(configuration.expectedNativeAssets.length); expect(manifest.assets).toHaveLength(manifest.assetCount);
                    let changed = false;
                    for (const expected of configuration.expectedNativeAssets) {
                        const asset = manifest.assets.find(item => item.specifier === expected.specifier);
                        const resource = id => bundle.project.resources.find(item => item.id === id);
                        const source = bundle.contents[expected.source], output = baseline.outputs.find(item => item.uri === `${root}${expected.destination}`).bytes;
                        expect(asset).toMatchObject({ target: expected.target, contentType: expected.contentType,
                            sourceUri: `${root}${resource(expected.source).path}`, sourceMapUri: `${root}${resource(expected.sourceMap).path}`, destination: `${root}${expected.destination}`,
                            sourceByteLength: source.byteLength, sourceSha256: await moduleAssetDigest(source), byteLength: output.byteLength, sha256: await moduleAssetDigest(output) });
                        expect(output).toEqual(bundle.contents[expected.expectedOutput]);
                        changed ||= asset.sourceSha256 !== asset.sha256 || asset.sourceByteLength !== asset.byteLength;
                    }
                    expect(manifest.contractVersion).toBe(changed ? 2 : 1);
                    const first = configuration.expectedNativeAssets[0], source = bundle.project.resources.find(item => item.id === first.source);
                    const scoped = projectCommandOptions(bundle, entry, project);
                    await expect(validator.runResourceCommand({ ...scoped, argv: command.argv,
                        dependencies: scoped.dependencies.filter(item => item.path !== `${root}${source.path}`),
                    })).rejects.toThrow();
                    if (configuration.id === 'build-v3') {
                        const target = decode('target-v3').replace('./assets/metadata.json', './assets/catalog.json');
                        expect(target).not.toBe(decode('target-v3'));
                        const retargeted = await run({ 'target-v3': target });
                        const text = path => new TextDecoder().decode(retargeted.outputs.find(item => item.uri === `${root}${path}`).bytes);
                        expect(text('generated/v3/assets/app.js')).toContain('./catalog.json');
                        expect(text(configuration.entryArtifact)).toContain('./assets/catalog.json');
                        expect(retargeted.outputs.some(item => item.uri === `${root}generated/v3/assets/metadata.json`)).toBe(false);
                        expect(retargeted.result.result.value.value.value.moduleAssetManifest.hash).not.toBe(manifest.hash);
                        const metadata = decode('module-metadata').replace('Café modules', 'Saved modules'); expect(metadata).not.toBe(decode('module-metadata'));
                        const edited = await run({ 'module-metadata': metadata });
                        const copied = edited.outputs.find(item => item.uri === `${root}generated/v3/assets/metadata.json`);
                        expect(copied.bytes).toEqual(new TextEncoder().encode(metadata));
                        const identity = edited.result.result.value.value.value.moduleAssetManifest.assets.find(item => item.specifier === '@demo/metadata');
                        expect(identity.sourceSha256).toBe(await moduleAssetDigest(copied.bytes)); expect(identity.sha256).toBe(identity.sourceSha256);
                    }
                }
                if (projectId === 'portable-config') {
                    const graph = decode('card-graph').replace('Dispatch board', 'Saved dispatch'); expect(graph).not.toBe(decode('card-graph'));
                    const source = decode('card-source').replace('Café Noord', 'Café Haven'); expect(source).not.toBe(decode('card-source'));
                    const changed = await run({ 'card-graph': graph, 'card-source': source });
                    const html = new TextDecoder().decode(changed.outputs.find(item => item.uri === `${root}${configuration.entryArtifact}`).bytes);
                    expect(html).toContain('Saved dispatch'); expect(html).toContain('Café Haven');
                }
                for (const refusal of configuration.expectedGraphRefusals ?? []) {
                    const denied = await run({ [refusal.source]: decode(refusal.replacement) }).catch(error => error);
                    expect(denied).toBeInstanceOf(Error); expect(denied.result).toBeUndefined(); expect(denied.message).toContain(refusal.code);
                }
                for (const [source, replacement] of configuration.expectedGraphFailures ?? []) await expect(run({ [source]: decode(replacement) })).rejects.toMatchObject({ result: { exitCode: 1 }, output: { available: false } });
                const independent = await run({}, `${project}-independent`);
                expect(independent.outputs.every(item => item.uri.startsWith(`studio://${project}-independent/`))).toBe(true);
                for (const output of baseline.outputs) expect(independent.outputs.find(item => item.uri.endsWith(`/${output.uri.slice(root.length)}`)).bytes).toEqual(output.bytes);
                if (configuration.expectedNativeAssets) expect(independent.result.result.value.value.value.moduleAssetManifest.hash).toBe(product.moduleAssetManifest.hash);
            }
        }, 60_000);
    }

    it('uses native portable control codecs and refuses invalid plans before reading missing inputs', async () => {
        const { bundle, decode } = await authoredProject('portable-config');
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const project = 'portable-native-controls', entry = bundle.project.entries.find(item => item.id === 'run-check');
        const command = JSON.parse(decode(entry.runConfigResourceId));
        const options = projectCommandOptions(bundle, entry, project);
        const before = await validator.previewResourceCommand({ ...options, operation: 'validate', argv: command.argv });
        expect(before.semantic.inputs.normalized[0].resolvedUri).toBe(`studio://${project}/data/card.xml`);
        const redirected = decode('card-check').replace('data/card.xml', 'data/alternate.xml'); expect(redirected).not.toBe(decode('card-check'));
        const modified = projectCommandOptions(bundle, entry, project, { 'card-check': redirected });
        const after = await validator.previewResourceCommand({ ...modified, operation: 'validate', argv: command.argv });
        expect(after.semantic.inputs.normalized[0].resolvedUri).toBe(`studio://${project}/data/alternate.xml`);
        expect((await validator.runResourceCommand({ ...modified, argv: command.argv })).result.exitCode).toBe(0);
        const duplicate = projectCommandOptions(bundle, entry, project, { 'card-check': decode('invalid-run') });
        const denied = await validator.runResourceCommand({ ...duplicate, argv: command.argv,
            dependencies: duplicate.dependencies.filter(item => !item.path.endsWith('/data/card.xml')),
        }).catch(error => error);
        expect(denied).toBeInstanceOf(Error); expect(denied.result).toBeUndefined();
        expect(denied).toMatchObject({ code: 'cem.run_config.input_uri_duplicate', exitCode: 2, message: 'input URI `data/card.xml` is declared more than once' });
        const envelope = JSON.parse(decode('card-command')); envelope.argv = ['not-a-command'];
        const bytes = new TextEncoder().encode(JSON.stringify(envelope));
        const source = bundle.project.resources.find(item => item.id === 'card-command');
        expect((await validator.validateResource({ bytes, uri: `studio://${project}/${source.path}`, contentType: source.contentType, schema: source.schema,
            projectId: project, projectRootUri: `studio://${project}/`, resourceScheme: 'studio',
        })).result.exitCode).toBe(0);
        expect(() => validator.decodeResourceCommand(bytes)).toThrow();
        const model = await validator.decodeProjectManifest(bundle.contents['project-cem']);
        expect(model).toEqual(JSON.parse(decode('expected-json'))); expect(model.resources).toHaveLength(7);
        for (const resource of model.resources) expect(resource.sha256).toBe(await moduleAssetDigest(bundle.contents[resource.id]));
        expect(decode('card-check')).not.toBe(redirected);
    }, 60_000);

    for (const projectId of ['cem-transform', 'xslt-transform']) {
        it(`builds ${projectId} explicit entry artifacts with native provenance and independent copies`, async () => {
            const { bundle, definition, decode } = await authoredProject(projectId);
            const validator = await createCemStudioBrowserValidator(); validators.push(validator);
            for (const configuration of definition.configurations.filter(item => item.kind === 'transformation-graph')) {
                const entry = bundle.project.entries.find(item => item.id === configuration.id), command = JSON.parse(decode(entry.runConfigResourceId));
                const run = (edits = {}, id = `${projectId}-graph-copy`) => validator.runResourceCommand({ ...projectCommandOptions(bundle, entry, id, edits), argv: command.argv });
                const baseline = await run(); expect(entry.entryArtifact).toBe(configuration.entryArtifact);
                expect(baseline.outputs).toHaveLength(Object.keys(configuration.expectedGraphOutputs).length);
                for (const [path, expected] of Object.entries(configuration.expectedGraphOutputs)) {
                    const actual = baseline.outputs.find(item => item.uri.endsWith(`/${path}`)).bytes;
                    if (path.endsWith('.json')) expect(JSON.parse(new TextDecoder().decode(actual))).toEqual(JSON.parse(decode(expected)));
                    else expect(actual).toEqual(bundle.contents[expected]);
                }
                for (const path of configuration.expectedUnmappedOutputs ?? []) {
                    const unmapped = baseline.result.result.value.value.value.artifacts.find(item => item.destination.endsWith(`/${path}`));
                    expect(unmapped.outputSpans ?? []).toEqual([]);
                }
                const artifact = baseline.result.result.value.value.value.artifacts.find(item => item.destination.endsWith(`/${configuration.entryArtifact}`));
                expect(artifact.outputSpans.length).toBeGreaterThan(0);
                for (const path of configuration.expectedProvenance ?? []) expect(JSON.stringify(artifact.outputSpans)).toContain(`/${path}`);
                const jsonPath = Object.keys(configuration.expectedGraphOutputs).find(path => path.endsWith('.json'));
                const rows = result => JSON.parse(new TextDecoder().decode(result.outputs.find(item => item.uri.endsWith(`/${jsonPath}`)).bytes));
                const html = result => new TextDecoder().decode(result.outputs.find(item => item.uri.endsWith(`/${configuration.entryArtifact}`)).bytes);
                if (jsonPath) expect(rows(baseline)).toHaveLength(2);
                const graph = decode(configuration.source), parameterEdit = graph.replace('@name=minimum @value=30', '@name=minimum @value=45');
                expect(parameterEdit).not.toBe(graph);
                const narrowed = await run({ [configuration.source]: parameterEdit });
                if (jsonPath) expect(rows(narrowed)).toHaveLength(1);
                expect(html(narrowed)).toContain('O-101'); expect(html(narrowed)).not.toContain('O-103');
                const data = bundle.project.resources.find(item => item.path === 'data/orders.xml');
                const editedData = decode(data.id).replace('Café Noord', 'Café Haven'); expect(editedData).not.toBe(decode(data.id));
                const updated = await run({ [data.id]: editedData });
                if (jsonPath) expect(rows(updated)[0].customer).toBe('Café Haven');
                expect(html(updated)).toContain('Café Haven');
                const template = bundle.project.resources.find(item => item.path === 'templates/page.cemt');
                const editedTemplate = decode(template.id).replace('@name=ul', '@name=ol'); expect(editedTemplate).not.toBe(decode(template.id));
                const ordered = await run({ [template.id]: editedTemplate });
                expect(html(ordered)).toContain('<ol>'); expect(html(ordered)).not.toContain('<ul>');
                if (jsonPath) expect(rows(ordered)).toEqual(rows(baseline));
                for (const [sourceId, invalidId] of configuration.expectedGraphFailures ?? []) await expect(run({ [sourceId]: decode(invalidId) })).rejects.toMatchObject({ result: { exitCode: 1 }, output: { available: false } });
                const independent = await run({}, `${projectId}-independent`);
                expect(independent.outputs.every(item => item.uri.startsWith(`studio://${projectId}-independent/`))).toBe(true);
                expect(independent.output.text).toBe(baseline.output.text);
            }
        }, 60_000);
    }

    it('keeps owned CEM instance validation separate from JSON Schema and RELAX NG source validation', async () => {
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const { bundle, decode } = await authoredProject('cem-schema');
        const entry = bundle.project.entries.find(item => item.id === 'validate-instance');
        const options = projectCommandOptions(bundle, entry, 'owned-note-copy');
        expect((await validator.validateResource(options)).result.exitCode).toBe(0);
        const invalid = { ...options, bytes: bundle.contents['invalid-instance'] };
        await expect(validator.validateResource(invalid)).rejects.toMatchObject({ result: { exitCode: 1, diagnostics: { items: expect.arrayContaining([
            expect.objectContaining({ code: 'example.note.id_required', severity: 'error' }),
        ]) } } });
        const schema = bundle.project.resources.find(item => item.id === 'note-schema');
        const changed = decode(schema.id).replace('A note must declare its id', 'Give this note an id'); expect(changed).not.toBe(decode(schema.id));
        const customized = { ...invalid, dependencies: invalid.dependencies.map(item => item.path.endsWith(`/${schema.path}`) ? { ...item, bytes: new TextEncoder().encode(changed) } : item) };
        await expect(validator.validateResource(customized)).rejects.toMatchObject({ result: { exitCode: 1, diagnostics: { items: expect.arrayContaining([
            expect.objectContaining({ code: 'example.note.id_required', severity: 'error', message: 'Give this note an id: element `note` failed field contract `note-id` (required-fields) with missing fields: id' }),
        ]) } } });
        for (const path of ['schemas/note.cem', 'schemas/package.cem']) await expect(validator.validateResource({ ...options,
            dependencies: options.dependencies.filter(item => !item.path.endsWith(`/${path}`)),
        })).rejects.toBeDefined();
        expect((await validator.validateResource(options)).result.exitCode).toBe(0);
        for (const projectId of ['json-schema', 'relax-ng']) {
            const source = await authoredProject(projectId), syntax = source.bundle.project.entries.find(item => item.id === 'validate-data-syntax');
            const syntaxOptions = projectCommandOptions(source.bundle, syntax, `${projectId}-syntax-copy`);
            expect(syntaxOptions.dependencies).toEqual([]);
            const domainInvalid = projectId === 'json-schema' ? '{"id":7}' : '<unrelated/>';
            expect((await validator.validateResource({ ...syntaxOptions, bytes: new TextEncoder().encode(domainInvalid) })).result.exitCode).toBe(0);
            expect(source.definition.configurations.some(item => item.id === 'validate-instance')).toBe(false);
        }
    }, 60_000);

    for (const projectId of ['cemql-orders', 'xpath-catalog']) {
        it(`runs ${projectId} native source operations, typed query results, original ranges and repair`, async () => {
            const { bundle, definition, decode } = await authoredProject(projectId);
            const validator = await createCemStudioBrowserValidator(); validators.push(validator);
            for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph')) {
                const entry = bundle.project.entries.find(item => item.id === configuration.id);
                const options = projectCommandOptions(bundle, entry, `${projectId}-commands-copy`);
                const commandResource = bundle.contents[entry.runConfigResourceId], command = JSON.parse(decode(entry.runConfigResourceId));
                const run = overrides => command.argv
                    ? validator.executeAuthoredResourceCommand({ ...options, ...overrides, commandResource })
                    : validator.validateResource({ ...options, ...overrides });
                const outcome = await run().catch(error => error);
                if (!outcome.result) throw new Error(`${projectId}/${configuration.id}: ${outcome.message}`, { cause: outcome });
                expect(outcome.result.exitCode, JSON.stringify(outcome.result.diagnostics.items)).toBe(configuration.expectedExit ?? 0);
                if ((configuration.expectedExit ?? 0) === 0) expect(outcome.result.diagnostics.items.filter(item => ['error', 'fatal'].includes(item.severity))).toEqual([]);
                if (configuration.expectedOutput) expect(outcome.output.text).toBe(decode(configuration.expectedOutput));
                for (const text of configuration.expectedText ?? []) expect(outcome.output.text).toContain(text);
                const resultFixture = configuration.expectedNativeItems ?? configuration.expectedNativeNodes ?? configuration.expectedNativeRanges ?? configuration.expectedSemanticContract;
                if (resultFixture) {
                    const report = JSON.parse(outcome.output.text), items = report.language === 'xpath' ? report.result.sequence.items : report.result.items;
                    if (configuration.expectedNativeItems) expect(items).toEqual(JSON.parse(decode(resultFixture)));
                    if (configuration.expectedSemanticContract) expect(items.map(item => item.value.lexicalValue)).toEqual(JSON.parse(decode(resultFixture)));
                    const source = bundle.project.resources.find(item => item.id === configuration.source);
                    if (configuration.expectedNativeNodes) {
                        expect(items.map(({ kind, nodeId, nodeKind, expandedName, sourceRange }) => ({ kind, nodeId, nodeKind, expandedName, sourceRange }))).toEqual(JSON.parse(decode(resultFixture)));
                        for (const item of items) expect(item.sourceUri).toBe(`studio://${projectId}-commands-copy/${source.path}`);
                    }
                    if (configuration.expectedNativeRanges) expect(items.map(item => {
                        expect(item.kind).toBe('native'); expect(item.representation).toBe('cem.ql.imported-cem-ast');
                        const frame = item.sourceMap.frames.find(frame => frame.transform.kind === 'SourceBoundary');
                        expect(frame.transform.uri).toBe(`studio://${projectId}-commands-copy/${source.path}`); return frame.span.ranges;
                    })).toEqual(JSON.parse(decode(resultFixture)));
                    const boundary = outcome.result.sourceMaps.items.flatMap(reference => reference.sourceMap?.value?.frames ?? []).find(frame => frame.transform.kind === 'SourceBoundary');
                    const querySource = bundle.project.resources.find(item => configuration.dependencies?.includes(item.id) && item.role === 'query');
                    expect(boundary?.transform.uri).toBe(`studio://${projectId}-commands-copy/${querySource.path}`);
                    expect(boundary.span.ranges).toEqual({ start: 0, len: bundle.contents[querySource.id].byteLength });
                }
                const expected = configuration.expectedValidation;
                if (expected) {
                    expect(outcome.result.diagnostics.items.filter(item => item.severity === 'error')).toHaveLength(expected.errorCount);
                    expect(outcome.result.diagnostics.items.filter(item => item.severity === 'warning')).toHaveLength(expected.warningCount);
                    if (configuration.repair) {
                        const source = bundle.project.resources.find(item => item.id === configuration.source);
                        const diagnostic = outcome.result.diagnostics.items.find(item => item.code === expected.errorCode);
                        expect(diagnostic).toMatchObject({ severity: 'error', uri: `studio://${projectId}-commands-copy/${source.path}` });
                        for (const [key, value] of Object.entries(expected.location)) expect(diagnostic[key] ?? null).toBe(value);
                        const original = decode(configuration.source), repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                        expect(repaired).not.toBe(original); const fixed = await run({ bytes: new TextEncoder().encode(repaired) });
                        expect(fixed.result.exitCode).toBe(0); expect(fixed.result.diagnostics.items).toEqual([]); expect(decode(configuration.source)).toBe(original);
                    }
                }
            }
        }, 60_000);
    }

    it('builds CEM-QL parameter maps as ordinary JSON tables with independent graph/data revisions', async () => {
        const projectId = 'cemql-orders', { bundle, definition, decode } = await authoredProject(projectId);
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        for (const configuration of definition.configurations.filter(item => item.kind === 'transformation-graph')) {
            const entry = bundle.project.entries.find(item => item.id === configuration.id), command = JSON.parse(decode(entry.runConfigResourceId));
            const run = (edits = {}, id = `${projectId}-build-copy`) => validator.runResourceCommand({ ...projectCommandOptions(bundle, entry, id, edits), argv: command.argv });
            const baseline = await run(); expect(entry.entryArtifact).toBe(configuration.entryArtifact);
            expect(baseline.outputs).toHaveLength(Object.keys(configuration.expectedGraphOutputs).length);
            for (const [path, expected] of Object.entries(configuration.expectedGraphOutputs)) {
                const bytes = baseline.outputs.find(item => item.uri.endsWith(`/${path}`)).bytes;
                // Generic-data JSON exports retain an abstract JSON primary;
                // CLI and worker hosts currently choose different whitespace.
                expect(JSON.parse(new TextDecoder().decode(bytes))).toEqual(JSON.parse(decode(expected)));
            }
            for (const artifact of baseline.result.result.value.value.value.artifacts) expect(artifact.outputSpans ?? []).toEqual([]);
            const original = decode(configuration.source);
            if (configuration.id === 'report-ready') {
                const edited = original.replace('@name=minimum @value=30', '@name=minimum @value=45'); expect(edited).not.toBe(original);
                const customized = await run({ [configuration.source]: edited }); expect(JSON.parse(customized.output.text)).toHaveLength(1);
            }
            for (const [sourceId, invalidId] of configuration.expectedGraphFailures ?? []) await expect(run({ [sourceId]: decode(invalidId) })).rejects.toMatchObject({ result: { exitCode: 1 }, output: { available: false } });
            const independent = await run({}, `${projectId}-independent`);
            expect(independent.outputs.every(item => item.uri.startsWith(`studio://${projectId}-independent/`))).toBe(true);
            expect(independent.output.text).toBe(baseline.output.text); expect(decode(configuration.source)).toBe(original);
        }
    }, 60_000);

    for (const projectId of ['css-stylesheet', 'css-selector', 'scss-stylesheet']) {
        it(`runs the ${projectId} native configurations with exact outputs and source repair`, async () => {
            const { bundle, definition, decode } = await authoredProject(projectId);
            const validator = await createCemStudioBrowserValidator(); validators.push(validator);
            for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph')) {
                const entry = bundle.project.entries.find(item => item.id === configuration.id);
                const options = projectCommandOptions(bundle, entry, `${projectId}-commands-copy`);
                const commandResource = bundle.contents[entry.runConfigResourceId];
                const command = JSON.parse(decode(entry.runConfigResourceId));
                const run = overrides => command.argv
                    ? validator.executeAuthoredResourceCommand({ ...options, ...overrides, commandResource })
                    : validator.validateResource({ ...options, ...overrides });
                const outcome = await run().catch(error => error);
                if (!outcome.result) throw new Error(`${projectId}/${configuration.id}: ${outcome.message}`, { cause: outcome });
                expect(outcome.result.exitCode, JSON.stringify(outcome.result.diagnostics.items)).toBe(configuration.expectedExit ?? 0);
                if ((configuration.expectedExit ?? 0) === 0) expect(outcome.result.diagnostics.items.filter(item => ['error', 'fatal'].includes(item.severity))).toEqual([]);
                if (configuration.expectedOutput) expect(outcome.output.text).toBe(decode(configuration.expectedOutput));
                for (const text of configuration.expectedText ?? []) expect(outcome.output.text).toContain(text);
                if (configuration.expectedSemanticContract) {
                    const report = JSON.parse(outcome.output.text); expect(report.language).toBe('css-selector');
                    expect(report.matches.map(({ localName, namespaceUri, documentOrder, nodeId }) => ({ localName, namespaceUri, documentOrder, nodeId })))
                        .toEqual(JSON.parse(decode(configuration.expectedSemanticContract)));
                    expect(report.matches.every(item => item.sourceMap.frames.length > 0)).toBe(true);
                }
                const expected = configuration.expectedValidation;
                if (expected) {
                    expect(outcome.result.diagnostics.items.filter(item => item.severity === 'error')).toHaveLength(expected.errorCount);
                    expect(outcome.result.diagnostics.items.filter(item => item.severity === 'warning')).toHaveLength(expected.warningCount);
                    if (configuration.repair) {
                        const source = bundle.project.resources.find(item => item.id === configuration.source);
                        const diagnostic = outcome.result.diagnostics.items.find(item => item.code === expected.errorCode);
                        expect(diagnostic).toMatchObject({ severity: 'error', uri: `studio://${projectId}-commands-copy/${source.path}` });
                        for (const [key, value] of Object.entries(expected.location)) expect(diagnostic[key] ?? null).toBe(value);
                        const original = decode(configuration.source), repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                        expect(repaired).not.toBe(original);
                        const fixed = await run({ bytes: new TextEncoder().encode(repaired) });
                        expect(fixed.result.exitCode).toBe(0); expect(fixed.result.diagnostics.items).toEqual([]);
                        expect(decode(configuration.source)).toBe(original);
                    }
                }
            }
        }, 60_000);
    }

    for (const [projectId, sourceId, find, replacement] of [
        ['css-stylesheet', 'style-source', 'rgb(24, 92, 128)', 'rgb(128, 48, 24)'],
        ['scss-stylesheet', 'scss-tokens', '#036', '#630'],
    ]) {
        it(`builds ${projectId} CSS artifacts with native origins and independent module/source edits`, async () => {
            const { bundle, definition, decode } = await authoredProject(projectId);
            const validator = await createCemStudioBrowserValidator(); validators.push(validator);
            const configuration = definition.configurations.find(item => item.kind === 'transformation-graph');
            const entry = bundle.project.entries.find(item => item.id === configuration.id);
            const command = JSON.parse(decode(entry.runConfigResourceId));
            const run = (edits = {}, id = `${projectId}-build-copy`) => validator.runResourceCommand({ ...projectCommandOptions(bundle, entry, id, edits), argv: command.argv });
            const baseline = await run();
            expect(entry.entryArtifact).toBe(configuration.entryArtifact);
            expect(baseline.outputs).toHaveLength(Object.keys(configuration.expectedGraphOutputs).length);
            for (const [path, expected] of Object.entries(configuration.expectedGraphOutputs)) expect(baseline.outputs.find(item => item.uri.endsWith(`/${path}`)).bytes).toEqual(bundle.contents[expected]);
            const css = baseline.result.result.value.value.value.artifacts.find(item => item.destination.endsWith('.css'));
            expect(css.outputSpans.length).toBeGreaterThan(0);
            const source = bundle.project.resources.find(item => item.id === sourceId);
            expect(JSON.stringify(css.outputSpans)).toContain(`/${source.path}`);
            const original = decode(sourceId), edited = original.replace(find, replacement); expect(edited).not.toBe(original);
            const customized = await run({ [sourceId]: edited });
            const cssPath = Object.keys(configuration.expectedGraphOutputs).find(path => path.endsWith('.css'));
            expect(customized.outputs.find(item => item.uri.endsWith(`/${cssPath}`)).bytes).not.toEqual(baseline.outputs.find(item => item.uri.endsWith(`/${cssPath}`)).bytes);
            for (const [sourceId, invalidId] of configuration.expectedGraphFailures ?? []) await expect(run({ [sourceId]: decode(invalidId) })).rejects.toMatchObject({ result: { exitCode: 1 }, output: { available: false } });
            const independent = await run({}, `${projectId}-independent`);
            for (const [path, expected] of Object.entries(configuration.expectedGraphOutputs)) expect(independent.outputs.find(item => item.uri.endsWith(`/${path}`)).bytes).toEqual(bundle.contents[expected]);
            expect(independent.outputs.every(item => item.uri.startsWith(`studio://${projectId}-independent/`))).toBe(true);
            expect(decode(sourceId)).toBe(original);
        }, 60_000);
    }

    for (const projectId of ['svg-diagram', 'mathml-formula']) {
        it(`runs the ${projectId} native family configurations, exact formats and namespace repair`, async () => {
            const { bundle, definition, decode } = await authoredProject(projectId);
            const validator = await createCemStudioBrowserValidator(); validators.push(validator);
            expect(definition.configurations).toHaveLength(projectId === 'svg-diagram' ? 6 : 7);
            for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph')) {
                const entry = bundle.project.entries.find(item => item.id === configuration.id);
                const options = projectCommandOptions(bundle, entry, `${projectId}-commands-copy`);
                const command = JSON.parse(decode(entry.runConfigResourceId));
                const run = overrides => command.argv
                    ? validator.executeAuthoredResourceCommand({ ...options, ...overrides, commandResource: bundle.contents[entry.runConfigResourceId] })
                    : validator.validateResource({ ...options, ...overrides });
                const outcome = await run().catch(error => error);
                if (!outcome.result) throw new Error(`${projectId}/${configuration.id}: ${outcome.message}`, { cause: outcome });
                expect(outcome.result.exitCode, JSON.stringify(outcome.result.diagnostics.items)).toBe(configuration.expectedExit ?? 0);
                if ((configuration.expectedExit ?? 0) === 0) expect(outcome.result.diagnostics.items.filter(item => ['error', 'fatal'].includes(item.severity))).toEqual([]);
                if (configuration.expectedOutput) expect(outcome.output.text).toBe(decode(configuration.expectedOutput));
                for (const text of configuration.expectedText ?? []) expect(outcome.output.text).toContain(text);
                if (configuration.expectedSemanticContract) {
                    const report = JSON.parse(outcome.output.text);
                    expect(report.language).toBe('xpath');
                    expect(report.result.sequence.items.map(item => item.value.lexicalValue)).toEqual(JSON.parse(decode(configuration.expectedSemanticContract)));
                }
                const expected = configuration.expectedValidation;
                if (expected) {
                    const diagnostics = outcome.result.diagnostics.items;
                    expect(diagnostics.filter(item => item.severity === 'error')).toHaveLength(expected.errorCount);
                    expect(diagnostics.filter(item => item.severity === 'warning')).toHaveLength(expected.warningCount);
                    if (configuration.repair) {
                        const source = bundle.project.resources.find(item => item.id === configuration.source);
                        const diagnostic = diagnostics.find(item => item.code === expected.errorCode);
                        expect(diagnostic).toMatchObject({ severity: 'error', uri: `studio://${projectId}-commands-copy/${source.path}` });
                        for (const [key, value] of Object.entries(expected.location)) expect(diagnostic[key] ?? null).toBe(value);
                        const original = decode(configuration.source), repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                        expect(repaired).not.toBe(original);
                        const fixed = await run({ bytes: new TextEncoder().encode(repaired) });
                        expect(fixed.result.exitCode).toBe(0); expect(fixed.result.diagnostics.items).toEqual([]);
                        expect(decode(configuration.source)).toBe(original);
                    }
                }
            }
        }, 60_000);

        it(`builds the ${projectId} foreign-content entry with original source owners and independent copies`, async () => {
            const { bundle, definition, decode } = await authoredProject(projectId);
            const validator = await createCemStudioBrowserValidator(); validators.push(validator);
            const configuration = definition.configurations.find(item => item.kind === 'transformation-graph');
            const entry = bundle.project.entries.find(item => item.id === configuration.id);
            const command = JSON.parse(decode(entry.runConfigResourceId));
            const inputId = definition.configurations.find(item => item.expectedSemanticContract).source;
            const source = bundle.project.resources.find(item => item.id === inputId);
            const template = bundle.project.resources.find(item => configuration.dependencies.includes(item.id) && item.role === 'template');
            const run = (edits = {}, id = `${projectId}-build-copy`) => validator.runResourceCommand({ ...projectCommandOptions(bundle, entry, id, edits), argv: command.argv });
            const baseline = await run();
            expect(baseline.outputs).toHaveLength(Object.keys(configuration.expectedGraphOutputs).length);
            expect(entry.entryArtifact).toBe(configuration.entryArtifact);
            for (const [path, expected] of Object.entries(configuration.expectedGraphOutputs)) expect(baseline.outputs.find(item => item.uri.endsWith(`/${path}`)).bytes).toEqual(bundle.contents[expected]);
            const frames = baseline.result.result.value.value.value.artifacts.flatMap(item => item.outputSpans ?? []).flatMap(span => span.origin.frames);
            for (const resource of [source, template]) expect(frames).toEqual(expect.arrayContaining([expect.objectContaining({ transform: expect.objectContaining({ uri: `studio://${projectId}-build-copy/${resource.path}` }) })]));
            const original = decode(inputId);
            const edited = projectId === 'svg-diagram' ? original.replace('>Source<', '>Café source<').replace('width="80"', 'width="72"')
                : original.replace('>x<', '>y<').replace('x squared', 'y squared');
            expect(edited).not.toBe(original); expect((await run({ [inputId]: edited })).output.text).not.toBe(baseline.output.text);
            const invalid = definition.configurations.find(item => item.repair).source;
            await expect(run({ [inputId]: decode(invalid) })).rejects.toMatchObject({ result: { exitCode: 1 }, output: { available: false } });
            expect((await run()).output.text).toBe(baseline.output.text);
            const independent = await run({}, `${projectId}-independent`);
            expect(independent.output.text).toBe(baseline.output.text);
            expect(independent.outputs.every(item => item.uri.startsWith(`studio://${projectId}-independent/`))).toBe(true);
            expect(decode(inputId)).toBe(original);
        }, 60_000);
    }

    it('keeps the submitted resource revision while asynchronous command preparation runs', async () => {
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const bytes = new TextEncoder().encode('{"title":"before"}');
        const argv = ['convert', 'studio://snapshot/input.json', '--content-type', 'application/json',
            '--schema', 'https://cem.dev/ns/data/json/1', '--to-content-type', 'application/json',
            '--to-schema', 'https://cem.dev/ns/data/json/1'];
        const options = { bytes, argv, uri: 'input.json', projectId: 'snapshot', projectRevision: 7, resourceRevision: 3,
            contentType: 'application/json', schema: 'https://cem.dev/ns/data/json/1' };
        const pending = validator.runResourceCommand(options);
        bytes.set(new TextEncoder().encode('{"title":"after!"}'));
        argv.splice(0, argv.length, 'invalid');
        options.schema = 'https://cem.dev/ns/data/csv/1';
        const outcome = await pending;
        expect(JSON.parse(outcome.output.text)).toEqual({ title: 'before' });
        expect(outcome.admission.project.revision).toBe(7);
    }, 60_000);

    it('runs the packaged JSON/YAML source, formatting and query configurations with native syntax repair', async () => {
        const { bundle, definition, decode } = await jsonYamlProject();
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        for (const configuration of definition.configurations.filter(configuration => configuration.kind !== 'transformation-graph')) {
            const entry = bundle.project.entries.find(entry => entry.id === configuration.id);
            const options = projectCommandOptions(bundle, entry, 'data-source-copy');
            const original = decode(configuration.source);
            if (configuration.kind === 'validation') {
                if (configuration.expectedExit === 1) {
                    await expect(validator.validateResource(options)).rejects.toMatchObject({ result: { exitCode: 1, diagnostics: { items: expect.arrayContaining([
                        expect.objectContaining({ code: configuration.expectedValidation.errorCode, severity: 'error' }),
                    ]) } } });
                    const repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                    expect(repaired).not.toBe(original);
                    expect((await validator.validateResource({ ...options, bytes: new TextEncoder().encode(repaired) })).result.exitCode).toBe(0);
                    expect(decode(configuration.source)).toBe(original);
                } else expect((await validator.validateResource(options)).result.exitCode).toBe(0);
                continue;
            }
            const command = JSON.parse(decode(entry.runConfigResourceId));
            const outcome = await validator.runResourceCommand({ ...options, argv: command.argv });
            expect(outcome.result.exitCode).toBe(0);
            expect(outcome.output.text.length).toBeGreaterThan(0);
            expect(outcome.output.text).not.toContain('\u001b[');
            if (configuration.expectedSemanticContract) {
                expect(JSON.parse(outcome.output.text).result.items.map(item => item.value)).toEqual(JSON.parse(decode(configuration.expectedSemanticContract)));
            }
            for (const text of configuration.expectedText ?? []) expect(outcome.output.text).toContain(text);
            if (configuration.expectedOutput) expect(outcome.output.text.trim()).toBe(decode(configuration.expectedOutput).trim());
            if (configuration.kind === 'conversion') {
                expect((await validator.validateResource({ ...options, bytes: outcome.output.bytes })).result.exitCode).toBe(0);
                if (options.contentType === 'application/json') expect(JSON.parse(outcome.output.text)).toEqual(JSON.parse(original));
            }
        }
    }, 60_000);

    it('builds the packaged JSON/YAML report from independent saved inputs with edits, provenance and failure/repair', async () => {
        const { bundle, decode } = await jsonYamlProject();
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const entry = bundle.project.entries.find(entry => entry.id === 'build-report');
        expect(entry.entryArtifact).toBe('generated/report.html');
        const command = JSON.parse(decode(entry.runConfigResourceId));
        const run = (edits = {}, projectId = 'data-report-copy') => validator.runResourceCommand({
            ...projectCommandOptions(bundle, entry, projectId, edits), argv: command.argv,
        });
        const artifacts = outcome => outcome.result.result.value.value.value.artifacts;
        const report = outcome => artifacts(outcome).find(artifact => artifact.destination.endsWith('/report.json')).primary;
        const baseline = await run();
        expect(artifacts(baseline)).toHaveLength(3);
        expect(report(baseline)).toEqual(JSON.parse(decode('expected-json')));
        expect(baseline.output.text.trim()).toBe(decode('expected-html').trim());
        expect(artifacts(baseline).find(artifact => artifact.destination.endsWith('/report.yaml')).primary.trim()).toBe(decode('expected-yaml').trim());
        expect(artifacts(baseline).flatMap(artifact => artifact.outputSpans ?? []).flatMap(span => span.origin.frames))
            .toEqual(expect.arrayContaining([expect.objectContaining({ transform: expect.objectContaining({ uri: 'studio://data-report-copy/templates/view.cemt' }) })]));
        const allActive = await run({ 'api-payload': decode('api-payload').replace('"status": "paused"', '"status": "active"') });
        expect(JSON.stringify(report(allActive))).toContain('API-102');
        const paused = await run({ 'report-settings': decode('report-settings').replace('status: active', 'status: paused') });
        expect(JSON.stringify(report(paused))).toContain('API-102');
        expect(JSON.stringify(report(paused))).not.toContain('API-101');
        for (const [target, invalid, code] of [['api-payload', 'invalid-payload', 'cem.json.parse_error'], ['report-settings', 'invalid-settings', 'cem.yaml.parse_error']]) {
            const failure = await run({ [target]: decode(invalid) }).catch(error => error);
            expect(failure).toMatchObject({ result: { exitCode: 1 }, output: { available: false, byteLength: 0, text: '' } });
            expect(failure.outputs).toEqual([]);
            const source = bundle.project.resources.find(resource => resource.id === target);
            const diagnostic = failure.result.diagnostics.items.find(item => item.code === code);
            expect(diagnostic).toMatchObject({ code, severity: 'error', uri: `studio://data-report-copy/${source.path}` });
            expect(diagnostic.line).toBeGreaterThan(0); expect(diagnostic.column).toBeGreaterThan(0);
            expect(diagnostic.byteOffset).toBeGreaterThanOrEqual(0);
            expect(diagnostic.byteOffset).toBeLessThanOrEqual(bundle.contents[invalid].byteLength);
            expect(report(await run())).toEqual(report(baseline));
        }
        const independent = await run({}, 'data-report-independent');
        expect(report(independent)).toEqual(report(baseline));
        expect(artifacts(independent).every(artifact => artifact.destination.startsWith('studio://data-report-independent/'))).toBe(true);
        expect(decode('api-payload')).toContain('"status": "paused"');
        expect(decode('report-settings')).toContain('status: active');
    }, 60_000);

    it('runs the packaged CSV configurations with native quoting, header policy, row-width warnings and exact repairs', async () => {
        const { bundle, definition, decode } = await csvInventoryProject();
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        for (const configuration of definition.configurations.filter(configuration => configuration.kind !== 'transformation-graph')) {
            const entry = bundle.project.entries.find(entry => entry.id === configuration.id);
            const options = projectCommandOptions(bundle, entry, 'inventory-commands-copy');
            const command = JSON.parse(decode(entry.runConfigResourceId));
            const run = overrides => command.argv
                ? validator.executeAuthoredResourceCommand({ ...options, ...overrides, commandResource: bundle.contents[entry.runConfigResourceId] })
                : validator.validateResource({ ...options, ...overrides });
            const outcome = await run().catch(error => error);
            if (!outcome.result) throw new Error(`CSV configuration ${configuration.id}: ${outcome.message}`, { cause: outcome });
            expect(outcome.result.exitCode).toBe(configuration.expectedExit ?? 0);
            const expected = configuration.expectedValidation;
            if (expected) {
                const diagnostics = outcome.result.diagnostics.items;
                expect(diagnostics.filter(item => item.severity === 'error')).toHaveLength(expected.errorCount);
                expect(diagnostics.filter(item => item.severity === 'warning')).toHaveLength(expected.warningCount);
                if (configuration.repair) {
                    const code = expected.errorCode ?? expected.warningCode;
                    const source = bundle.project.resources.find(resource => resource.id === (expected.source ?? configuration.source));
                    const diagnostic = diagnostics.find(item => item.code === code);
                    expect(diagnostic).toMatchObject({ code, severity: expected.errorCode ? 'error' : 'warning',
                        uri: `studio://inventory-commands-copy/${source.path}` });
                    for (const [key, value] of Object.entries(expected.location)) expect(diagnostic[key] ?? null).toBe(value);
                    const original = decode(configuration.source);
                    const repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                    expect(repaired).not.toBe(original);
                    const repair = await run({ bytes: new TextEncoder().encode(repaired) });
                    expect(repair.result.exitCode).toBe(0);
                    expect(repair.result.diagnostics.items).toEqual([]);
                    expect(decode(configuration.source)).toBe(original);
                }
            }
            if (configuration.expectedSemanticContract) {
                expect(JSON.parse(outcome.output.text).result.items.map(item => item.value)).toEqual(JSON.parse(decode(configuration.expectedSemanticContract)));
            }
            for (const text of configuration.expectedText ?? []) expect(outcome.output.text).toContain(text);
            if (configuration.expectedOutput) expect(outcome.output.text).toBe(decode(configuration.expectedOutput));
            if (configuration.kind === 'conversion') {
                if (configuration.id === 'format-csv') expect((await validator.validateResource({ ...options, bytes: outcome.output.bytes })).result.exitCode).toBe(0);
                expect(outcome.output.text).not.toContain('\u001b[');
            }
            if (configuration.id === 'repair-header-policy') {
                expect(command.argv[0]).toBe('validate'); expect(command.argv).toContain('--config');
                const preview = await validator.previewResourceCommand({ ...options, commandResource: bundle.contents[entry.runConfigResourceId] });
                const input = preview.semantic.inputs.normalized.find(input => input.resolvedUri === 'studio://inventory-commands-copy/data/inventory.csv');
                expect(input.identity.contentType).toContain('header=maybe');
                await expect(run({ dependencies: [] })).rejects.toMatchObject({ code: 'cem.browser_command.resolve', exitCode: 6 });
            }
        }
    }, 60_000);

    it('builds the packaged CSV inventory report with selected records, native provenance, independent copies and repair', async () => {
        const { bundle, definition, decode } = await csvInventoryProject();
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const entry = bundle.project.entries.find(entry => entry.id === 'build-inventory');
        expect(entry.entryArtifact).toBe('generated/low-stock.html');
        const command = JSON.parse(decode(entry.runConfigResourceId));
        const run = (edits = {}, projectId = 'inventory-report-copy') => validator.runResourceCommand({
            ...projectCommandOptions(bundle, entry, projectId, edits), argv: command.argv,
        });
        const artifacts = outcome => outcome.result.result.value.value.value.artifacts;
        const report = outcome => artifacts(outcome).find(artifact => artifact.destination.endsWith('/low-stock.json')).primary;
        const baseline = await run();
        expect(artifacts(baseline)).toHaveLength(2);
        expect(report(baseline)).toEqual(JSON.parse(decode('expected-json')));
        expect(baseline.output.text.trim()).toBe(decode('expected-html').trim());
        expect(artifacts(baseline).flatMap(artifact => artifact.outputSpans ?? []).flatMap(span => span.origin.frames))
            .toEqual(expect.arrayContaining([expect.objectContaining({ transform: expect.objectContaining({ uri: 'studio://inventory-report-copy/templates/view.cemt' }) })]));
        expect(report(await run())).toEqual(report(baseline));
        const edited = await run({ 'inventory-source': decode('inventory-source').replace('SKU-C,Glass jars,0,2', 'SKU-C,Glass jars,4,2') });
        expect(JSON.stringify(report(edited))).not.toContain('SKU-C');
        expect(JSON.stringify(report(edited))).toContain('SKU-A');
        const quoted = await run({ 'inventory-source': decode('inventory-source').replace('SKU-B,"Tea ""travel"" pot",8,4', 'SKU-B,"Tea ""travel"" pot",2,4') });
        expect(report(quoted)).toEqual(expect.arrayContaining([expect.objectContaining({ sku: 'SKU-B', name: 'Tea "travel" pot', quantity: 2 })]));
        const queryEntry = bundle.project.entries.find(entry => entry.id === 'query-low-stock');
        const queryCommand = JSON.parse(decode(queryEntry.runConfigResourceId));
        for (const quantity of ['many', '03', '-1', '3.5']) {
            const csv = decode('inventory-source').replace('SKU-A,"Café cups, set",3,5', `SKU-A,"Café cups, set",${quantity},5`);
            expect(csv).not.toBe(decode('inventory-source'));
            const invalidCount = await run({ 'inventory-source': csv });
            expect(report(invalidCount)).toEqual(expect.arrayContaining([expect.objectContaining({
                sku: 'SKU-A', quantity, reorder_level: '5', status: 'invalid-count',
            })]));
            const selected = await validator.runResourceCommand({ ...projectCommandOptions(bundle, queryEntry, 'inventory-report-copy', { 'inventory-source': csv }), argv: queryCommand.argv });
            expect(JSON.parse(selected.output.text).result.items.map(item => item.value)).toEqual(['SKU-C']);
        }
        for (const [invalid, code] of [['unclosed-quote', 'cem.csv.unclosed_quote'], ['invalid-escape', 'cem.csv.invalid_quote_escape']]) {
            const failure = await run({ 'inventory-source': decode(invalid) }).catch(error => error);
            expect(failure).toMatchObject({ result: { exitCode: 1 }, output: { available: false, byteLength: 0, text: '' } });
            expect(failure.outputs).toEqual([]);
            expect(failure.result.diagnostics.items).toEqual(expect.arrayContaining([expect.objectContaining({
                code, severity: 'error', uri: 'studio://inventory-report-copy/data/inventory.csv',
            })]));
            const diagnostic = failure.result.diagnostics.items.find(item => item.code === code);
            const expected = definition.configurations.find(configuration => configuration.source === invalid).expectedValidation;
            for (const [key, value] of Object.entries(expected.location)) expect(diagnostic[key] ?? null).toBe(value);
            expect(report(await run())).toEqual(report(baseline));
        }
        const independent = await run({}, 'inventory-report-independent');
        expect(report(independent)).toEqual(report(baseline));
        expect(artifacts(independent).every(artifact => artifact.destination.startsWith('studio://inventory-report-independent/'))).toBe(true);
        expect(decode('inventory-source')).toContain('SKU-C,Glass jars,0,2');
    }, 60_000);

    it('runs the packaged HTML and CEM card configurations with equivalent semantics and native policy repair', async () => {
        const { bundle, definition, decode } = await authoredProject('html-cem-card');
        expect(definition.configurations.map(item => item.id).sort()).toEqual([
            'build-html', 'inspect-cem', 'inspect-component', 'inspect-html', 'live-view', 'repair-html-policy', 'semantics-cem', 'semantics-html',
        ]);
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph')) {
            const entry = bundle.project.entries.find(item => item.id === configuration.id);
            const options = projectCommandOptions(bundle, entry, 'html-card-commands-copy');
            const command = JSON.parse(decode(entry.runConfigResourceId));
            const run = overrides => command.argv
                ? validator.executeAuthoredResourceCommand({ ...options, ...overrides, commandResource: bundle.contents[entry.runConfigResourceId] })
                : validator.validateResource({ ...options, ...overrides });
            const outcome = await run().catch(error => error);
            if (!outcome.result) throw new Error(`HTML/CEM ${configuration.id}: ${outcome.message}`, { cause: outcome });
            expect(outcome.result.exitCode, JSON.stringify(outcome.result.diagnostics.items)).toBe(configuration.expectedExit ?? 0);
            for (const text of configuration.expectedText ?? []) expect(outcome.output.text).toContain(text);
            if (configuration.expectedSemanticContract) {
                const report = JSON.parse(outcome.output.text);
                const values = report.language === 'xpath' ? report.result.sequence.items.map(item => item.value.lexicalValue) : report.result.items.map(item => item.value);
                expect(values).toEqual(JSON.parse(decode(configuration.expectedSemanticContract)));
            }
            const expected = configuration.expectedValidation;
            if (expected) {
                const diagnostics = outcome.result.diagnostics.items;
                expect(diagnostics.filter(item => item.severity === 'error')).toHaveLength(expected.errorCount);
                expect(diagnostics.filter(item => item.severity === 'warning')).toHaveLength(expected.warningCount);
                if (configuration.repair) {
                    const source = bundle.project.resources.find(item => item.id === configuration.source);
                    const code = expected.errorCode ?? expected.warningCode;
                    const diagnostic = diagnostics.find(item => item.code === code);
                    expect(diagnostic).toMatchObject({ code, severity: expected.errorCode ? 'error' : 'warning', uri: `studio://html-card-commands-copy/${source.path}` });
                    for (const [key, value] of Object.entries(expected.location ?? {})) expect(diagnostic[key] ?? null).toBe(value);
                    const original = decode(configuration.source), repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                    expect(repaired).not.toBe(original);
                    const result = await run({ bytes: new TextEncoder().encode(repaired) });
                    expect(result.result.exitCode).toBe(0); expect(result.result.diagnostics.items).toEqual([]);
                    expect(decode(configuration.source)).toBe(original);
                }
            }
            if (configuration.id === 'live-view') {
                expect(outcome.result.diagnostics.items.filter(item => ['error', 'fatal'].includes(item.severity))).toEqual([]);
                expect(bundle.project.resources.find(item => item.id === configuration.source).role).toBe('template');
                expect(entry.tags).toEqual(expect.arrayContaining(['live-preview', 'preview-component:cem-card', 'preview-component:cem-action', 'preview-component:cem-alert']));
                expect(entry.resourceIds).toContain(definition.configurations.find(item => item.id === 'inspect-html').source);
            }
        }
    }, 60_000);

    it('builds the packaged HTML and CEM card with exact static output, native origins and independent saved data', async () => {
        const { bundle, definition, decode } = await authoredProject('html-cem-card');
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const configuration = definition.configurations.find(item => item.id === 'build-html');
        const entry = bundle.project.entries.find(item => item.id === configuration.id);
        expect(entry.entryArtifact).toBe('generated/card.html');
        const command = JSON.parse(decode(entry.runConfigResourceId));
        const run = (edits = {}, projectId = 'html-card-build-copy') => validator.runResourceCommand({
            ...projectCommandOptions(bundle, entry, projectId, edits), argv: command.argv,
        });
        const templateId = configuration.dependencies.find(id => bundle.project.resources.find(item => item.id === id)?.role === 'template');
        expect(templateId).toBeDefined();
        const template = bundle.project.resources.find(item => item.id === templateId);
        const baseline = await run(); expect(baseline.outputs).toHaveLength(1);
        expect(baseline.outputs[0].bytes).toEqual(bundle.contents[configuration.expectedGraphOutputs[entry.entryArtifact]]);
        const artifacts = outcome => outcome.result.result.value.value.value.artifacts;
        expect(artifacts(baseline).flatMap(item => item.outputSpans ?? []).flatMap(span => span.origin.frames))
            .toEqual(expect.arrayContaining([expect.objectContaining({ transform: expect.objectContaining({ uri: `studio://html-card-build-copy/${template.path}` }) })]));
        const sourceId = definition.configurations.find(item => item.id === 'inspect-html').source;
        const original = decode(sourceId), edited = original.replaceAll('Harbor field notes', 'Canal field notes');
        expect(edited).not.toBe(original);
        expect((await run({ [sourceId]: edited })).output.text).toContain('Canal field notes');
        await expect(run({ [templateId]: '@doc cem-ml 1\n{template @name=main |' })).rejects.toMatchObject({ result: { exitCode: 1 }, output: { available: false } });
        expect((await run()).output.text).toBe(baseline.output.text);
        const independent = await run({}, 'html-card-independent');
        expect(independent.output.text).toBe(baseline.output.text);
        expect(independent.outputs.every(item => item.uri.startsWith('studio://html-card-independent/'))).toBe(true);
        expect(decode(sourceId)).toBe(original);
    }, 60_000);

    it('runs packaged Markdown article inspection, preview, outline and metadata repair with original source coordinates', async () => {
        const { bundle, definition, decode } = await authoredProject('markdown-site');
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph')) {
            const entry = bundle.project.entries.find(item => item.id === configuration.id);
            const options = projectCommandOptions(bundle, entry, 'markdown-article-copy');
            const command = JSON.parse(decode(entry.runConfigResourceId));
            const run = overrides => command.argv
                ? validator.executeAuthoredResourceCommand({ ...options, ...overrides, commandResource: bundle.contents[entry.runConfigResourceId] })
                : validator.validateResource({ ...options, ...overrides });
            const outcome = await run().catch(error => error);
            if (!outcome.result) throw new Error(`Markdown ${configuration.id}: ${outcome.message}`, { cause: outcome });
            expect(outcome.result.exitCode, JSON.stringify(outcome.result.diagnostics.items)).toBe(configuration.expectedExit ?? 0);
            const expected = configuration.expectedValidation;
            if (expected) {
                const diagnostics = outcome.result.diagnostics.items;
                expect(diagnostics.filter(item => item.severity === 'error')).toHaveLength(expected.errorCount);
                expect(diagnostics.filter(item => item.severity === 'warning')).toHaveLength(expected.warningCount);
                if (configuration.repair) {
                    const source = bundle.project.resources.find(item => item.id === configuration.source);
                    const diagnostic = diagnostics.find(item => item.code === expected.errorCode);
                    expect(diagnostic).toMatchObject({ code: expected.errorCode, severity: 'error', uri: `studio://markdown-article-copy/${source.path}` });
                    for (const [key, value] of Object.entries(expected.location ?? {})) expect(diagnostic[key] ?? null).toBe(value);
                    const original = decode(configuration.source), repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                    expect(repaired).not.toBe(original);
                    const result = await run({ bytes: new TextEncoder().encode(repaired) });
                    expect(result.result.exitCode).toBe(0); expect(result.result.diagnostics.items).toEqual([]);
                    expect(decode(configuration.source)).toBe(original);
                }
            }
            if (configuration.expectedOutput) expect(outcome.output.text).toBe(decode(configuration.expectedOutput));
            for (const text of configuration.expectedText ?? []) expect(outcome.output.text).toContain(text);
            if (configuration.expectedSemanticContract) {
                const report = JSON.parse(outcome.output.text);
                const values = report.language === 'xpath' ? report.result.sequence.items.map(item => item.value.lexicalValue) : report.result.items.map(item => item.value);
                expect(values).toEqual(JSON.parse(decode(configuration.expectedSemanticContract)));
            }
            if (configuration.id === 'preview-article') {
                expect(outcome.output.text).not.toContain('title: Build a small site');
                expect(outcome.output.text).toContain('>Build a small site</h1>');
                const original = decode(configuration.source);
                const edited = original.replace('The article stays an ordinary editable resource.', 'This saved article stays an ordinary editable resource.');
                expect(edited).not.toBe(original);
                const customized = await run({ bytes: new TextEncoder().encode(edited) });
                expect(customized.output.text).toBe(decode(configuration.expectedOutput).replace('The article stays an ordinary editable resource.', 'This saved article stays an ordinary editable resource.'));
                const independent = await validator.runResourceCommand({ ...projectCommandOptions(bundle, entry, 'markdown-article-independent'), argv: command.argv });
                expect(independent.output.text).toBe(outcome.output.text);
                expect(decode(configuration.source)).toBe(original);
            }
        }
    }, 60_000);

    it('builds the packaged Markdown folder site with exact assets, source origins and independent edits', async () => {
        const { bundle, definition, decode } = await authoredProject('markdown-site');
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const entry = bundle.project.entries.find(item => item.id === 'build-site');
        const configuration = definition.configurations.find(item => item.id === entry.id);
        const command = JSON.parse(decode(entry.runConfigResourceId));
        const run = (edits = {}, projectId = 'markdown-site-copy') => validator.runResourceCommand({
            ...projectCommandOptions(bundle, entry, projectId, edits), argv: command.argv,
        });
        const baseline = await run(); expect(baseline.outputs).toHaveLength(7);
        for (const [path, expected] of Object.entries(configuration.expectedGraphOutputs)) {
            expect(baseline.outputs.find(item => item.uri === `studio://markdown-site-copy/${path}`).bytes, path).toEqual(bundle.contents[expected]);
        }
        const artifacts = outcome => outcome.result.result.value.value.value.artifacts;
        const article = artifacts(baseline).find(item => item.destination.endsWith('/articles/first-build.html'));
        const frames = article.outputSpans.flatMap(span => span.origin.frames);
        expect(frames).toEqual(expect.arrayContaining([
            expect.objectContaining({ transform: expect.objectContaining({ uri: 'studio://markdown-site-copy/content/articles/first-build.md' }) }),
            expect.objectContaining({ transform: expect.objectContaining({ uri: 'studio://markdown-site-copy/layouts/partials/content.cemt' }) }),
            expect.objectContaining({ transform: expect.objectContaining({ uri: 'studio://markdown-site-copy/layouts/partials/chrome.cemt' }) }),
        ]));
        const original = decode('content-articles-first-build-md');
        const edited = await run({ 'content-articles-first-build-md': original.replaceAll('Build a small site', 'Only the saved site') });
        const editedArticle = edited.outputs.find(item => item.uri.endsWith('/articles/first-build.html'));
        expect(editedArticle.text).toContain('<title>Only the saved site</title>');
        for (const output of baseline.outputs.filter(item => !item.uri.endsWith('/articles/first-build.html'))) {
            expect(edited.outputs.find(item => item.uri === output.uri).bytes, output.uri).toEqual(output.bytes);
        }
        const failure = await run({ 'content-articles-first-build-md': decode('invalid-article-metadata') }).catch(error => error);
        expect(failure).toMatchObject({ result: { exitCode: 1 }, output: { available: false, byteLength: 0, text: '' } });
        expect(failure.outputs).toEqual([]);
        expect(failure.result.diagnostics.items).toEqual(expect.arrayContaining([expect.objectContaining({
            code: 'cem.markdown.front_matter_invalid', severity: 'error', uri: 'studio://markdown-site-copy/content/articles/first-build.md',
        })]));
        const repaired = await run();
        expect(repaired.outputs.map(item => item.bytes)).toEqual(baseline.outputs.map(item => item.bytes));
        const independent = await run({}, 'markdown-site-independent');
        expect(independent.outputs.map(item => item.bytes)).toEqual(baseline.outputs.map(item => item.bytes));
        expect(independent.outputs.every(item => item.uri.startsWith('studio://markdown-site-independent/'))).toBe(true);
        expect(decode('content-articles-first-build-md')).toBe(original);
    }, 60_000);

    it('runs ordinary XML and XHTML source configurations with namespace-aware queries and native repair', async () => {
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        for (const id of ['supplier-pipeline', 'xhtml-document']) {
            const { bundle, definition, decode } = await authoredProject(id);
            const projectId = `${id}-commands-copy`;
            for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph')) {
                const entry = bundle.project.entries.find(entry => entry.id === configuration.id);
                const options = projectCommandOptions(bundle, entry, projectId);
                const command = JSON.parse(decode(entry.runConfigResourceId));
                const run = overrides => command.argv
                    ? validator.executeAuthoredResourceCommand({ ...options, ...overrides, commandResource: bundle.contents[entry.runConfigResourceId] })
                    : validator.validateResource({ ...options, ...overrides });
                const outcome = await run().catch(error => error);
                if (!outcome.result) throw new Error(`${id} configuration ${configuration.id}: ${outcome.message}`, { cause: outcome });
                expect(outcome.result.exitCode, `${id}/${configuration.id}: ${JSON.stringify(outcome.result.diagnostics.items)}`).toBe(configuration.expectedExit ?? 0);
                const expected = configuration.expectedValidation;
                if (expected) {
                    const diagnostics = outcome.result.diagnostics.items;
                    expect(diagnostics.filter(item => item.severity === 'error')).toHaveLength(expected.errorCount);
                    expect(diagnostics.filter(item => item.severity === 'warning')).toHaveLength(expected.warningCount);
                    if (configuration.repair) {
                        const source = bundle.project.resources.find(resource => resource.id === (expected.source ?? configuration.source));
                        const diagnostic = diagnostics.find(item => item.code === expected.errorCode);
                        expect(diagnostic).toMatchObject({ code: expected.errorCode, severity: 'error', uri: `studio://${projectId}/${source.path}` });
                        for (const [key, value] of Object.entries(expected.location ?? {})) expect(diagnostic[key] ?? null).toBe(value);
                        const original = decode(configuration.source);
                        const repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                        expect(repaired).not.toBe(original);
                        const result = await run({ bytes: new TextEncoder().encode(repaired) });
                        expect(result.result.exitCode).toBe(0);
                        expect(result.result.diagnostics.items).toEqual([]);
                        expect(decode(configuration.source)).toBe(original);
                    }
                }
                if (configuration.expectedSemanticContract) {
                    const report = JSON.parse(outcome.output.text);
                    const values = report.language === 'xpath' ? report.result.sequence.items.map(item => item.value.lexicalValue) : report.result.items.map(item => item.value);
                    expect(values).toEqual(JSON.parse(decode(configuration.expectedSemanticContract)));
                }
                for (const text of configuration.expectedText ?? []) expect(outcome.output.text).toContain(text);
                if (configuration.id === 'query-products') {
                    const querySource = bundle.project.resources.find(resource => resource.id === 'supplier-products-query');
                    const staleMetadata = {
                        ...options,
                        dependencies: options.dependencies.map(dependency => dependency.path === `../${querySource.path}`
                            ? { ...dependency, contentType: 'application/xml', schema: 'https://cem.dev/ns/data/xml/1' } : dependency),
                    };
                    const explicit = await validator.executeAuthoredResourceCommand({ ...staleMetadata, commandResource: bundle.contents[entry.runConfigResourceId] });
                    expect(explicit.result.exitCode).toBe(0);
                    expect(JSON.parse(explicit.output.text).result.sequence.items.map(item => item.value.lexicalValue))
                        .toEqual(JSON.parse(decode(configuration.expectedSemanticContract)));
                    const foreign = decode(configuration.source).replace('urn:demo:supplier', 'urn:demo:foreign');
                    expect(foreign).not.toBe(decode(configuration.source));
                    const ignored = await run({ bytes: new TextEncoder().encode(foreign) });
                    const report = JSON.parse(ignored.output.text);
                    expect(report.language).toBe('xpath');
                    expect(report.result.sequence.items).toEqual([]);
                }
            }
        }
    }, 60_000);

    it('builds the packaged XHTML document into HTML with saved-source identities, provenance and independent copies', async () => {
        const { bundle, decode } = await authoredProject('xhtml-document');
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const entry = bundle.project.entries.find(entry => entry.id === 'build-html');
        expect(entry.entryArtifact).toBe('generated/article.html');
        const command = JSON.parse(decode(entry.runConfigResourceId));
        const run = (edits = {}, projectId = 'xhtml-report-copy') => validator.runResourceCommand({
            ...projectCommandOptions(bundle, entry, projectId, edits), argv: command.argv,
        });
        const artifacts = outcome => outcome.result.result.value.value.value.artifacts;
        const baseline = await run();
        expect(artifacts(baseline)).toHaveLength(1);
        expect(baseline.output.text.trim()).toBe(decode('expected-html').trim());
        expect(artifacts(baseline).flatMap(artifact => artifact.outputSpans ?? []).flatMap(span => span.origin.frames))
            .toEqual(expect.arrayContaining([expect.objectContaining({ transform: expect.objectContaining({ uri: 'studio://xhtml-report-copy/templates/view.cemt' }) })]));
        const source = decode('article-source');
        const edited = source.replace('An XHTML field guide', 'Saved XHTML field guide');
        expect(edited).not.toBe(source);
        const customized = await run({ 'article-source': edited });
        expect(customized.output.text).toContain('Saved XHTML field guide');
        expect(customized.output.text).not.toContain('An XHTML field guide');
        expect(customized.output.text).toContain('Namespaced documents stay structured.');
        const malformed = await run({ 'article-source': edited.replace('</html>', '</html-broken>') }).catch(error => error);
        expect(malformed).toMatchObject({ result: { exitCode: 1 }, output: { available: false, byteLength: 0, text: '' } });
        expect(malformed.outputs).toEqual([]);
        expect((await run()).output.text).toBe(baseline.output.text);
        const independent = await run({}, 'xhtml-report-independent');
        expect(independent.output.text).toBe(baseline.output.text);
        expect(artifacts(independent).every(artifact => artifact.destination.startsWith('studio://xhtml-report-independent/'))).toBe(true);
        expect(decode('article-source')).toBe(source);
    }, 60_000);

    it('runs the packaged supplier pipeline with native multi-format output, edits, failure/repair and isolated copies', async () => {
        const response = await fetch(new URL('../generated/feature-tour/examples/supplier-pipeline.cem-studio.json', import.meta.url));
        expect(response.ok).toBe(true);
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const decode = id => new TextDecoder().decode(bundle.contents[id]);
        const configuration = bundle.project.entries.find(entry => entry.id === 'availability');
        expect(configuration.kind).toBe('transformation-graph');
        const source = bundle.project.resources.find(resource => resource.id === 'supplier-graph');
        const original = decode('supplier-stock');
        const command = JSON.parse(decode(configuration.runConfigResourceId));
        const run = (stock = original, projectId = 'supplier-copy') => validator.runResourceCommand({
            bytes: bundle.contents[source.id], uri: `studio://${projectId}/${source.path}`,
            contentType: source.contentType, schema: source.schema, projectId, projectRootUri: `studio://${projectId}/`, argv: command.argv,
            dependencies: configuration.resourceIds.slice(2).map(id => {
                const resource = bundle.project.resources.find(resource => resource.id === id);
                return { path: `../${resource.path}`, bytes: id === 'supplier-stock' ? new TextEncoder().encode(stock) : bundle.contents[id],
                    contentType: resource.contentType, schema: resource.schema };
            }),
        });
        const artifacts = result => result.result.result.value.value.value.artifacts;
        const json = result => artifacts(result).find(artifact => artifact.destination.endsWith('/availability.json')).primary;
        const baseline = await run();
        expect(baseline.result.exitCode).toBe(0);
        expect(artifacts(baseline)).toHaveLength(3);
        expect(json(baseline)).toEqual(JSON.parse(decode('supplier-expected-json')));
        expect(baseline.output.text.trim()).toBe(decode('supplier-expected-html').trim());
        expect(artifacts(baseline).find(artifact => artifact.destination.endsWith('.yaml')).primary.trim()).toBe(decode('supplier-expected-yaml').trim());
        expect(artifacts(baseline).flatMap(artifact => artifact.outputSpans ?? []).flatMap(span => span.origin.frames))
            .toEqual(expect.arrayContaining([expect.objectContaining({ transform: expect.objectContaining({ uri: 'studio://supplier-copy/templates/view.cemt' }) })]));
        expect(json(await run(original.replace('SKU-A,7', 'SKU-A,0')))[0]).toMatchObject({ quantity: 0, available: false, status: 'out-of-stock' });
        await expect(run(original.replace('SKU-A,7', 'SKU-A,many'))).rejects.toMatchObject({ result: { exitCode: 1 }, output: { available: false, byteLength: 0, text: '' } });
        expect(json(await run())).toEqual(json(baseline));
        const independent = await run(original, 'supplier-independent');
        expect(json(independent)).toEqual(json(baseline));
        expect(artifacts(independent).every(artifact => artifact.destination.startsWith('studio://supplier-independent/'))).toBe(true);
        expect(decode('supplier-stock')).toBe(original);
    }, 60_000);

    it('evaluates the packaged standalone Login expression on both native sources and project-local edits', async () => {
        const response = await fetch(new URL('../generated/feature-tour/examples/login.cem-studio.json', import.meta.url));
        expect(response.ok).toBe(true);
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const decode = id => new TextDecoder().decode(bundle.contents[id]);
        const expression = decode('login-expression');
        const expected = JSON.parse(decode('login-expression-expected'));
        for (const id of ['expression-html', 'expression-cem']) {
            const entry = bundle.project.entries.find(entry => entry.id === id);
            const source = bundle.project.resources.find(resource => resource.id === entry.resourceIds[0]);
            const command = JSON.parse(decode(entry.runConfigResourceId));
            expect(command.argv).toContain('application/vnd.cem.query-expression+cem-ql');
            expect(command.argv).toContain('https://cem.dev/ns/query/cem-ql/1#expression');
            const run = async (query, sourceText = decode(source.id), projectId = 'expression-copy') => {
                const result = await validator.runResourceCommand({ bytes: new TextEncoder().encode(sourceText),
                    uri: `studio://${projectId}/${source.path}`, contentType: source.contentType, schema: source.schema,
                    projectId, projectRootUri: `studio://${projectId}/`, argv: command.argv,
                    dependencies: entry.resourceIds.slice(2).map(id => {
                        const resource = bundle.project.resources.find(resource => resource.id === id);
                        return { path: `../${resource.path}`, bytes: id === 'login-expression' ? new TextEncoder().encode(query) : bundle.contents[id],
                            contentType: resource.contentType, schema: resource.schema };
                    }) });
                expect(result.result.exitCode).toBe(0);
                return JSON.parse(result.output.text).result.items.map(item => item.value);
            };
            expect(await run(expression)).toEqual(expected);
            expect(await run(expression.replace('title=', 'titre 🍒='))).toEqual(['titre 🍒=Sign in', 'required=2']);
            expect(await run(expression, decode(source.id).replace('Sign in', 'Bienvenue 🍒'))).toEqual(['title=Bienvenue 🍒', 'required=2']);
            expect(await run(expression, decode(source.id), 'independent-copy')).toEqual(expected);
        }
        expect(decode('login-expression')).toBe(expression);
    }, 60_000);

    it('formats and inspects the packaged Login expression without evaluating or wrapping it', async () => {
        const response = await fetch(new URL('../generated/feature-tour/examples/login.cem-studio.json', import.meta.url));
        expect(response.ok).toBe(true);
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const source = bundle.project.resources.find(resource => resource.id === 'login-expression');
        const original = new TextDecoder().decode(bundle.contents[source.id]);
        const options = text => ({ bytes: new TextEncoder().encode(text), uri: `studio://expression-format/${source.path}`,
            contentType: source.contentType, schema: source.schema, resourceScheme: 'studio', projectId: 'expression-format', projectRootUri: 'studio://expression-format/' });
        for (const profile of ['pretty', 'tabular']) {
            const entry = bundle.project.entries.find(entry => entry.id === `expression-format-${profile}`);
            const command = JSON.parse(new TextDecoder().decode(bundle.contents[entry.runConfigResourceId]));
            const format = text => validator.runResourceCommand({ ...options(text), argv: command.argv });
            const baseline = await format(original);
            expect(baseline.output.contentType).toBe(source.contentType);
            expect(command.argv).toContain(source.schema);
            expect(baseline.output.text).toContain('// Login résumé');
            expect(baseline.output.text).not.toContain('module ');
            expect(baseline.output.text).not.toContain('\u001b');
            expect((await format(baseline.output.text)).output.text).toBe(baseline.output.text);
            expect((await format(original.replace('title=', 'titre 🍒='))).output.text).toContain('titre 🍒=');
            await expect(format(`${original})`)).rejects.toMatchObject({ result: { exitCode: 1 } });
            expect((await format(original)).output.text).toBe(baseline.output.text);
        }
        const parsed = await validator.parseResource(options(original));
        expect(parsed.output.text).toContain('@kind=call');
        expect(parsed.output.text).toContain(source.schema);
        expect(parsed.output.text).toContain('byte-offset');
        const inspected = await validator.inspectResource({ ...options(original), view: 'tree' });
        expect(inspected.output.text).toContain('@kind=call');
        expect(new TextDecoder().decode(bundle.contents[source.id])).toBe(original);
    }, 60_000);

    it('runs both packaged Login formatting configurations on independent saved sources', async () => {
        const response = await fetch(new URL('../generated/feature-tour/examples/login.cem-studio.json', import.meta.url));
        expect(response.ok).toBe(true);
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const source = bundle.project.resources.find(resource => resource.id === 'login-cem');
        const original = new TextDecoder().decode(bundle.contents[source.id]);
        const layouts = [];
        for (const profile of ['pretty', 'tabular']) {
            const entry = bundle.project.entries.find(entry => entry.id === `format-${profile}`);
            expect(entry.resourceIds).toContain(source.id);
            const command = JSON.parse(new TextDecoder().decode(bundle.contents[entry.runConfigResourceId]));
            expect(command.argv).toContain(profile);
            const format = async (text, projectId) => {
                const result = await validator.runResourceCommand({ bytes: new TextEncoder().encode(text),
                    uri: `studio://${projectId}/${source.path}`, contentType: source.contentType, schema: source.schema,
                    projectId, projectRootUri: `studio://${projectId}/`, argv: command.argv });
                expect(result.result.exitCode).toBe(0);
                expect(result.output.contentType).toBe('application/cem');
                expect(result.output.text).not.toContain('\u001b');
                return result.output.text;
            };
            const baseline = await format(original, 'format-copy');
            expect(baseline).toContain('@method=post');
            expect(baseline).toContain('@required');
            expect(await format(original.replace('Sign in', 'Bienvenue 🍒'), 'format-copy'))
                .toBe(baseline.replace('Sign in', 'Bienvenue 🍒'));
            expect(await format(original, 'independent-copy')).toBe(baseline);
            expect(new TextDecoder().decode(bundle.contents[source.id])).toBe(original);
            layouts.push(baseline);
        }
        expect(layouts[0]).not.toBe(layouts[1]);
    }, 60_000);

    it('exports packaged Login source colors as inert HTML with exact tabular text', async () => {
        const response = await fetch(new URL('../generated/feature-tour/examples/login.cem-studio.json', import.meta.url));
        expect(response.ok).toBe(true);
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const source = bundle.project.resources.find(resource => resource.id === 'login-cem');
        const original = new TextDecoder().decode(bundle.contents[source.id]);
        const run = async (id, text, projectId = 'source-colors') => {
            const entry = bundle.project.entries.find(entry => entry.id === id);
            const command = JSON.parse(new TextDecoder().decode(bundle.contents[entry.runConfigResourceId]));
            const result = await validator.runResourceCommand({ bytes: new TextEncoder().encode(text),
                uri: `studio://${projectId}/${source.path}`, contentType: source.contentType, schema: source.schema,
                projectId, projectRootUri: `studio://${projectId}/`, argv: command.argv });
            expect(result.result.exitCode).toBe(0);
            return result.output;
        };
        for (const text of [original, original.replace('Sign in', 'Bienvenue 🍒 <script>alert(1)</script> & café')]) {
            const plain = await run('format-tabular', text);
            const output = await run('source-colors', text);
            expect(output.contentType).toBe('text/html');
            const document = new DOMParser().parseFromString(output.text, 'text/html');
            expect(document.querySelector('pre').textContent).toBe(plain.text);
            expect(document.querySelectorAll('form, input, script')).toHaveLength(0);
            expect(document.querySelectorAll('span[data-role][style]').length).toBeGreaterThan(0);
            expect(output.text).toContain('--cem-color-');
            expect(output.text).not.toContain('\u001b');
            const host = globalThis.document.createElement('div');
            host.style.setProperty('--cem-color-syntax-attribute', 'rgb(7, 38, 83)');
            host.append(document.querySelector('pre'));
            globalThis.document.body.append(host);
            try {
                expect(getComputedStyle(host.querySelector('[data-role="syntax.attribute"]')).color).toBe('rgb(7, 38, 83)');
            } finally { host.remove(); }
        }
        expect((await run('source-colors', original, 'fresh-copy')).text)
            .toBe((await run('source-colors', original)).text);
        expect(new TextDecoder().decode(bundle.contents[source.id])).toBe(original);
    }, 60_000);

    it('edits and runs the Login graph with named generated-output provenance in the browser worker', async () => {
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const bytes = text => new TextEncoder().encode(text);
        const template = viewTemplate.replace('@doc cem-ml 1', '@doc cem-ml 1\n// café Amsterdam');
        const uri = 'studio://graph-copy/graphs/view.cem';
        const render = source => validator.runResourceCommand({ bytes: bytes(source), uri,
            contentType: 'application/cem', schema: 'https://cem.dev/ns/cli/transform-config/1', projectId: 'graph-copy', projectRootUri: 'studio://graph-copy/',
            argv: ['transform', '--config', uri, '--config-content-type', 'application/cem', '--config-schema', 'https://cem.dev/ns/cli/transform-config/1', '--scope-budget', 'queueSize=1024'],
            dependencies: [
                { path: '../data/login.cem', bytes: bytes(loginCem), contentType: 'application/cem', schema: 'https://cem.dev/ns/cem-ml/1' },
                { path: '../templates/view.cemt', bytes: bytes(template), contentType: 'application/vnd.cem.template+cem', schema: 'https://cem.dev/ns/template/cem-native/1' },
            ] });
        const baseline = await render(viewGraph);
        expect(baseline.result.exitCode).toBe(0);
        expect(baseline.output.text.trim()).toBe(viewExpected.trim());
        const artifacts = baseline.result.result.value.value.value.artifacts;
        const html = artifacts.find(artifact => artifact.destination.endsWith('/generated/login.html'));
        expect(html.outputSpans.length).toBeGreaterThan(0);
        const instruction = 'Use your email and password.';
        const offset = bytes(viewExpected.slice(0, viewExpected.indexOf(instruction))).length;
        const span = html.outputSpans.find(span => span.outputRange.start <= offset && offset < span.outputRange.start + span.outputRange.len);
        const origin = span.origin.frames.find(frame => frame.transform.kind === 'SourceBoundary');
        expect(origin.transform.uri).toBe('studio://graph-copy/templates/view.cemt');
        const range = origin.span.ranges;
        const authored = '```' + instruction + '```';
        expect(new TextDecoder().decode(bytes(template).slice(range.start, range.start + range.len))).toBe(authored);
        expect(range.start).toBe(bytes(template.slice(0, template.indexOf(authored))).length);
        const changed = validator.authorGraph({ source: viewGraph, edit: { node: 'page', attribute: 'out', value: '../generated/presentation.html' } });
        const rerun = await render(changed.source);
        expect(rerun.result.exitCode).toBe(0);
        expect(rerun.output.text.trim()).toBe(viewExpected.trim());
        expect(rerun.result.result.value.value.value.artifacts[0].destination).toBe('studio://graph-copy/generated/presentation.html');
        expect(() => validator.authorGraph({ source: viewGraph, edit: { node: 'page', attribute: 'id', value: 'view' } })).toThrow();
        await expect(render(viewGraph.replace('../templates/view.cemt', '../templates/missing.cemt'))).rejects.toMatchObject({ code: 'cem.browser_command.resolve', exitCode: 6 });
    }, 60_000);

    it('renders the Login view from both sources and an editable native template in the browser worker', async () => {
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const bytes = text => new TextEncoder().encode(text);
        const templateUri = 'studio://view-copy/templates/view.cemt';
        const templateType = 'application/vnd.cem.template+cem';
        const templateSchema = 'https://cem.dev/ns/template/cem-native/1';
        const validate = template => validator.validateResource({ bytes: bytes(template), uri: templateUri, contentType: templateType,
            schema: templateSchema, resourceScheme: 'studio', projectId: 'view-copy', projectRootUri: 'studio://view-copy/' });
        expect((await validate(viewTemplate)).result.exitCode).toBe(0);
        for (const [source, media, schema, path] of [
            [loginHtml, 'text/html', 'https://cem.dev/ns/data/html/1', 'login.html'],
            [loginCem, 'application/cem', 'https://cem.dev/ns/cem-ml/1', 'login.cem'],
        ]) {
            const uri = `studio://view-copy/data/${path}`;
            const render = async (source, template = viewTemplate) => {
                const outcome = await validator.runResourceCommand({ bytes: bytes(source), uri, contentType: media, schema,
                    projectId: 'view-copy', projectRootUri: 'studio://view-copy/',
                    argv: ['transform', uri, '--data-content-type', media, '--data-schema', schema, '--template', templateUri,
                        '--template-content-type', templateType, '--template-schema', templateSchema, '--template-entrypoint', 'main',
                        '--to-content-type', 'text/html', '--to-schema', 'https://cem.dev/ns/data/html/1', '--output-color-type', 'none',
                        '--scope-budget', 'queueSize=1024'],
                    dependencies: [{ path: '../templates/view.cemt', bytes: bytes(template), contentType: templateType, schema: templateSchema }] });
                expect(outcome.result.exitCode).toBe(0);
                expect(outcome.output.contentType).toBe('text/html');
                return outcome.output.text.trim();
            };
            const baseline = await render(source);
            expect(baseline).toBe(viewExpected.trim());
            expect(baseline).not.toContain('</input>');
            // Inspect the explicitly exported HTML artifact, independently of its serialized layout.
            const document = new DOMParser().parseFromString(baseline, 'text/html');
            expect(document.querySelector('main').getAttribute('aria-labelledby')).toBe('login-title');
            expect(document.querySelector('h1').id).toBe('login-title');
            expect(document.querySelector('h1').textContent.trim()).toBe('Sign in');
            expect(document.querySelector('form').getAttribute('action')).toBe('#session');
            expect(document.querySelector('form').getAttribute('method')).toBe('post');
            expect(document.querySelector('form').getAttribute('aria-describedby')).toBe('login-help');
            expect(document.querySelector('#login-help').textContent).toBe('Use your email and password.');
            expect([...document.querySelectorAll('input')].map(input => [input.id, input.name, input.type, input.autocomplete, input.required]))
                .toEqual([['email', 'email', 'email', 'email', true], ['password', 'password', 'password', 'current-password', true]]);
            expect([...document.querySelectorAll('label')].map(label => [label.htmlFor, label.textContent.trim()]))
                .toEqual([['email', 'Email'], ['password', 'Password']]);
            expect(document.querySelector('button').type).toBe('submit');
            expect(document.querySelector('button').textContent.trim()).toBe('Sign in');
            const edited = source.replace('Sign in', 'Welcome').replace('Email', 'Email address')
                .replace(media === 'text/html' ? 'method="post"' : '@method=post', media === 'text/html' ? 'method="get"' : '@method=get');
            expect(await render(edited)).toBe(viewExpected.trim().replace('Sign in', 'Welcome').replace('Email', 'Email address').replace('method="post"', 'method="get"'));
            const template = viewTemplate.replace('Use your email and password.', 'Use your Amsterdam account.');
            expect((await validate(template)).result.exitCode).toBe(0);
            expect(await render(source, template)).toBe(viewExpected.trim().replace('Use your email and password.', 'Use your Amsterdam account.'));
            expect(await render(source)).toBe(viewExpected.trim());
        }
    }, 60_000);

    it('formats native standalone expressions with their own identity and original failure coordinates', async () => {
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const uri = 'studio://format-copy/queries/expression.cemql';
        const contentType = 'application/vnd.cem.query-expression+cem-ql';
        const schema = 'https://cem.dev/ns/query/cem-ql/1#expression';
        const format = (source, withSchema = true) => validator.runResourceCommand({
            bytes: new TextEncoder().encode(source), uri, contentType, schema: withSchema ? schema : undefined,
            projectId: 'format-copy', projectRootUri: 'studio://format-copy/',
            argv: ['convert', uri, '--content-type', contentType, ...(withSchema ? ['--schema', schema] : []),
                '--to-content-type', contentType, '--to-schema', schema,
                '--cemt-formatter-profile', 'tabular', '--cemt-formatter-option', 'lineEnding=lf',
                '--output-color-type', 'none'],
        });
        const source = '// café\r\nunknownFunction("🍒")\r\n';
        const formatted = await format(source);
        expect(formatted.result.exitCode).toBe(0);
        expect(formatted.output.contentType).toBe(contentType);
        expect(formatted.output.text).toBe(source.replaceAll('\r\n', '\n'));
        expect((await format(source, false)).output.text).toBe(formatted.output.text);
        expect((await format(formatted.output.text)).output.text).toBe(formatted.output.text);
        await expect(format('"🍒" +')).rejects.toMatchObject({ result: {
            diagnostics: { items: expect.arrayContaining([
                expect.objectContaining({ code: 'cem.ql.parse_error', uri, byteOffset: 8, line: 1, column: 7 }),
            ]) },
        } });
        expect((await format('"🍒"')).result.exitCode).toBe(0);
        await expect(format('module "urn:wrong" 1')).rejects.toMatchObject({ result: { exitCode: 1 } });
    }, 60_000);

    it('parses and inspects native Login query sources without evaluating or wrapping expressions', async () => {
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const uri = 'studio://inspection-copy/queries/semantics.cemql';
        const options = { bytes: new TextEncoder().encode(semanticsCemql), uri,
            contentType: 'application/vnd.cem.query+cem-ql', schema: 'https://cem.dev/ns/query/cem-ql/1',
            resourceScheme: 'studio', projectId: 'inspection-copy', projectRootUri: 'studio://inspection-copy/' };
        for (const projection of ['ast', 'events']) {
            const outcome = await validator.parseResource({ ...options, projection });
            expect(outcome.result.exitCode).toBe(0);
            expect(outcome.output.contentType).toBe('application/cem');
            expect(outcome.output.text).toContain(uri);
            expect(outcome.output.text).toContain(options.schema);
            expect(outcome.output.text).toContain(projection === 'ast' ? '@kind=declare-function' : 'StringLit');
            expect(outcome.output.text).not.toContain('\u001b');
        }
        for (const view of ['summary', 'ast', 'events', 'diagnostics', 'source-offsets', 'tree']) {
            const outcome = await validator.inspectResource({ ...options, view });
            expect(outcome.result.exitCode).toBe(0);
            expect(outcome.output.text).toContain(uri);
            expect(outcome.output.text).toContain(options.schema);
        }
        const expression = { ...options, uri: 'studio://inspection-copy/queries/expression.cemql',
            contentType: 'application/vnd.cem.query-expression+cem-ql', schema: 'https://cem.dev/ns/query/cem-ql/1#expression',
            bytes: new TextEncoder().encode('unknownFunction("🍒")') };
        const parsed = await validator.parseResource(expression);
        expect(parsed.result.exitCode).toBe(0); // Parsing does not bind or execute unknownFunction.
        expect(parsed.output.text).toContain('@kind=call');
        expect(parsed.output.text).toContain('🍒');
        const broken = { ...expression, bytes: new TextEncoder().encode('"🍒" +') };
        const recovered = await validator.parseResource(broken);
        expect(recovered.result.exitCode).toBe(0); // Default parse policy fails only fatal diagnostics.
        expect(recovered.result.diagnostics.items).toEqual(expect.arrayContaining([
            expect.objectContaining({ code: 'cem.ql.parse_error', uri: broken.uri, byteOffset: 8, line: 1, column: 7 }),
        ]));
        const strictParse = options => validator.parseResource({ ...options, argv: [
            'parse', options.uri, '--content-type', options.contentType, '--schema', options.schema,
            '--fail-level', 'validate', '--no-color',
        ] });
        await expect(strictParse(broken)).rejects.toMatchObject({ result: { exitCode: 1 } });
        const diagnostics = await validator.inspectResource({ ...broken, view: 'diagnostics' });
        expect(diagnostics.output.text).toContain('cem.ql.parse_error');
        expect(diagnostics.output.text).toContain('byte-offset');
        expect(diagnostics.output.text).not.toContain('module_uri_missing');
        const mismatch = { ...expression, schema: options.schema };
        await expect(strictParse(mismatch)).rejects.toMatchObject({ result: { exitCode: 1,
            diagnostics: { items: expect.arrayContaining([expect.objectContaining({ code: 'cem.ql.query_identity_unsupported' })]) } } });
    }, 60_000);

    it('queries the same Login semantic contract with XPath and CEM-QL in the native browser worker', async () => {
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const bytes = text => new TextEncoder().encode(text);
        const expected = JSON.parse(semanticsContract);
        const maps = new Map();
        const moduleValidation = await validator.validateResource({ bytes: bytes(semanticsCemql), uri: 'studio://semantic-copy/queries/semantics.cemql',
            contentType: 'application/vnd.cem.query+cem-ql', schema: 'https://cem.dev/ns/query/cem-ql/1', resourceScheme: 'studio', projectId: 'semantic-copy', projectRootUri: 'studio://semantic-copy/' });
        expect(moduleValidation.result.exitCode).toBe(0);
        for (const [query, file, queryType, querySchema, flags, probe, probeType, probeSchema] of [
            [semanticsQuery, 'semantics.xpath', 'application/vnd.cem.xpath', 'https://cem.dev/ns/query/xpath/1', [],
                '//Q{http://www.w3.org/1999/xhtml}input', 'application/vnd.cem.xpath', 'https://cem.dev/ns/query/xpath/1'],
            [semanticsCemql, 'semantics.cemql', 'application/vnd.cem.query+cem-ql', 'https://cem.dev/ns/query/cem-ql/1', ['--ready-names', '--scope-budget', 'queueSize=1024'],
                'seq:where(dom:descendants(input), fn(node) => node.kind == "element" && node.namespace == "http://www.w3.org/1999/xhtml" && node.name == "input")',
                'application/vnd.cem.query-expression+cem-ql', 'https://cem.dev/ns/query/cem-ql/1#expression'],
        ]) for (const [source, contentType, schema, path] of [
            [loginHtml, 'text/html', 'https://cem.dev/ns/data/html/1', 'login.html'],
            [loginCem, 'application/cem', 'https://cem.dev/ns/cem-ml/1', 'login.cem'],
        ]) {
            const uri = `studio://semantic-copy/data/${path}`;
            const argv = ['query', uri, '--content-type', contentType, '--schema', schema, '--query-file', `studio://semantic-copy/queries/${file}`,
                '--query-content-type', queryType, '--query-schema', querySchema, ...flags, '--output', 'json'];
            const options = { uri, contentType, schema, projectId: 'semantic-copy', projectRootUri: 'studio://semantic-copy/', argv,
                dependencies: [{ path: `../queries/${file}`, bytes: bytes(query), contentType: queryType, schema: querySchema }] };
            const run = async source => {
                const outcome = await validator.runResourceCommand({ ...options, bytes: bytes(source) });
                expect(outcome.result.exitCode).toBe(0);
                const report = JSON.parse(outcome.output.text);
                return report.language === 'cem-ql' ? report.result.items.map(item => item.value) : report.result.sequence.items.map(item => item.value.lexicalValue);
            };
            expect(await run(source)).toEqual(expected);
            const edited = source.replace('Sign in', 'Welcome').replace(contentType === 'text/html' ? 'method="post"' : '@method=post', contentType === 'text/html' ? 'method="get"' : '@method=get');
            expect(await run(edited)).toEqual(expected.map(value => value === 'title=Sign in' ? 'title=Welcome' : value === 'method=post' ? 'method=get' : value));
            const selected = await validator.runResourceCommand({ ...options, argv: ['query', uri, '--content-type', contentType, '--schema', schema,
                '--query', probe, '--query-content-type', probeType, '--query-schema', probeSchema, ...flags, '--output', 'json'], bytes: bytes(source) });
            const report = JSON.parse(selected.output.text);
            const items = report.language === 'cem-ql' ? report.result.items : report.result.sequence.items;
            expect(items).toHaveLength(2);
            expect(items.every(item => item.sourceMap.frames.length > 0)).toBe(true);
            const sources = items.map(item => item.sourceMap);
            if (maps.has(uri)) expect(sources).toEqual(maps.get(uri));
            else { maps.set(uri, sources); expect(items[0].sourceUri).toBe(uri); }
        }
    }, 60_000);

    it('initializes native graph authoring through the public CLI browser facade', async () => {
        const { createBrowserTransformConfigAuthor } = await import('@epa-wg/cem-ml-cli/browser-commands');
        const author = await createBrowserTransformConfigAuthor();
        const source = '{run | {import @id=source @src=data.cem | {export @id=result @out=result.cem}}}\n';
        const described = JSON.parse(author(JSON.stringify({ source })));
        expect(described.error).toBeUndefined();
        expect(described.nodes.find(node => node.id === 'source')).toMatchObject({ id: 'source', kind: 'import' });
        expect(JSON.parse(author('{"source":false}')).error).toBeTypeOf('string');
    });

    it('loads owned teaching schema packages for validation, parse and inspection', async () => {
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const bytes = value => new TextEncoder().encode(value);
        const dependencies = [
            { path: '../schemas/login-schema-package.cem', bytes: bytes(loginPackage), contentType: 'application/vnd.cem.schema-package+cem', schema: 'https://cem.dev/ns/schema-package/1' },
            { path: '../schemas/login-schema.cem', bytes: bytes(loginSchema), contentType: 'application/cem', schema: 'https://cem.dev/ns/schema/1' },
        ];
        const options = { bytes: bytes(invalidLogin), uri: 'studio://amsterdam-copy/data/login-invalid.cem', resourceScheme: 'studio',
            projectId: 'amsterdam-copy', contentType: 'application/cem', schema: 'urn:demo:login', dependencies };
        const preview = await validator.previewResourceCommand({ ...options, operation: 'validate' });
        expect(preview.semantic.inputs.resources.find(resource => resource.uri.endsWith('/login-schema-package.cem')).identity)
            .toEqual(preview.semantic.identity.schemas[0].identity);
        let failure;
        try { await validator.validateResource(options); } catch (error) { if (!error.result) throw error; failure = error.result; }
        expect(failure?.exitCode).toBe(1);
        expect(failure.diagnostics.items).toEqual(expect.arrayContaining([expect.objectContaining({ code: 'demo.login.method_required', severity: 'error' })]));
        const repaired = { ...options, bytes: bytes(invalidLogin.replace('@cem:form="sign-in"', '@cem:form="sign-in" @method=post')) };
        const success = await validator.validateResource(repaired);
        expect(success.result.exitCode).toBe(0);
        expect(success.result.diagnostics.items).toHaveLength(3);
        expect(success.result.diagnostics.items.every(item => item.code === 'cem.ast.unresolved_reference')).toBe(true);
        for (const method of ['parseResource', 'inspectResource']) {
            const result = await validator[method]({ ...repaired, view: 'ast' });
            expect(result.result.exitCode).toBe(0);
            expect(result.output.text).toContain('form');
        }
        await expect(validator.validateResource({ ...options, dependencies: dependencies.slice(0, 1) })).rejects.toMatchObject({
            code: 'cem.studio.validate_failed', result: { exitCode: 1, diagnostics: { items: expect.arrayContaining([
                expect.objectContaining({ code: 'cem.schema_package.schema_source_unreadable', severity: 'error' }),
            ]) } },
        });
    }, 60_000);

    it('uses the real CEM-ML converter for canonical project.cem provider round trips', async () => {
        const validator = await createCemStudioBrowserValidator();
        validators.push(validator);
        const source = new TextEncoder().encode(`@doc cem-ml 1
@ns studio = "https://cem.dev/ns/studio/project/1"
@default studio

{project
    @schema-version=1
    @id="file-project"
    @name="File project"
    @root-uri="studio://file-project/"
    @revision=1
    @created-at="2026-08-22T00:00:00Z"
    @updated-at="2026-08-22T00:00:00Z"
}
`);

        const project = await validator.decodeProjectManifest(source);
        expect(project).toMatchObject({
            $schema: 'https://cem.dev/ns/studio/project/1',
            id: 'file-project',
            rootUri: 'studio://file-project/',
            entries: [],
            resources: [],
        });
        const encoded = await validator.encodeProjectManifest(project);
        expect(new TextDecoder().decode(encoded)).toContain('@id=file-project');
        await expect(validator.decodeProjectManifest(encoded)).resolves.toEqual(project);
    });

    it('preserves an edited or trashed copy across seed upgrades and resets to a separate identity', async () => {
        const repository = createRepository();
        const first = await installCemStudioFeatureTour(repository, await seed('1.0.0', 'original'), {
            now: () => '2026-08-21T00:00:00Z',
        });
        expect(first).toMatchObject({ status: 'installed', projectId: 'feature-tour' });

        await repository.execute(request('save-resource', {
            projectId: 'feature-tour',
            resourceId: 'source',
            expectedProjectRevision: 1,
            expectedResourceRevision: 1,
            content: 'user edit',
        }));
        const preserved = await installCemStudioFeatureTour(repository, await seed('2.0.0', 'new seed'));
        expect(preserved).toMatchObject({ status: 'preserved', projectId: 'feature-tour', seedVersion: '2.0.0' });
        expect(await exportedText(repository, 'feature-tour')).toBe('user edit');

        await repository.execute(request('trash-project', {
            projectId: 'feature-tour',
            expectedRevision: 2,
        }));
        await expect(installCemStudioFeatureTour(repository, await seed('3.0.0', 'newer seed')))
            .resolves.toMatchObject({ status: 'preserved', projectId: 'feature-tour' });

        const reset = await installCemStudioFeatureTour(repository, await seed('3.0.0', 'newer seed'), {
            reset: true,
            now: () => '2026-08-22T00:00:00Z',
        });
        expect(reset).toMatchObject({ status: 'reset', projectId: 'feature-tour-2' });
        expect(await exportedText(repository, 'feature-tour-2')).toBe('newer seed');
    });
});

async function jsonYamlProject() {
    const [archive, definition] = await Promise.all([
        fetch(new URL('../generated/feature-tour/examples/json-yaml-report.cem-studio.json', import.meta.url)),
        fetch(new URL('../../../examples/projects/json-yaml-report/example.json', import.meta.url)),
    ]);
    expect(archive.ok && definition.ok).toBe(true);
    const bundle = parseCemStudioProjectBundle(new Uint8Array(await archive.arrayBuffer()));
    return { bundle, definition: await definition.json(), decode: id => new TextDecoder().decode(bundle.contents[id]) };
}

async function csvInventoryProject() {
    const [archive, definition] = await Promise.all([
        fetch(new URL('../generated/feature-tour/examples/csv-inventory.cem-studio.json', import.meta.url)),
        fetch(new URL('../../../examples/projects/csv-inventory/example.json', import.meta.url)),
    ]);
    expect(archive.ok && definition.ok).toBe(true);
    const bundle = parseCemStudioProjectBundle(new Uint8Array(await archive.arrayBuffer()));
    return { bundle, definition: await definition.json(), decode: id => new TextDecoder().decode(bundle.contents[id]) };
}

async function authoredProject(id) {
    const [archive, definition] = await Promise.all([
        fetch(new URL(`../generated/feature-tour/examples/${id}.cem-studio.json`, import.meta.url)),
        fetch(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url)),
    ]);
    expect(archive.ok && definition.ok).toBe(true);
    const bundle = parseCemStudioProjectBundle(new Uint8Array(await archive.arrayBuffer()));
    return { bundle, definition: await definition.json(), decode: id => new TextDecoder().decode(bundle.contents[id]) };
}

function projectCommandOptions(bundle, entry, projectId, edits = {}) {
    const source = bundle.project.resources.find(resource => resource.id === entry.resourceIds[0]);
    const bytes = id => Object.hasOwn(edits, id) ? new TextEncoder().encode(edits[id]) : bundle.contents[id];
    return { bytes: bytes(source.id), uri: `studio://${projectId}/${source.path}`, resourceScheme: 'studio', contentType: source.contentType, schema: source.schema,
        projectId, projectRootUri: `studio://${projectId}/`, dependencies: entry.resourceIds.slice(2).map(id => {
            const resource = bundle.project.resources.find(resource => resource.id === id);
            return { path: `studio://${projectId}/${resource.path}`, bytes: bytes(id), contentType: resource.contentType, schema: resource.schema };
        }) };
}

async function moduleAssetDigest(bytes) {
    const value = await crypto.subtle.digest('SHA-256', bytes);
    return [...new Uint8Array(value)].map(byte => byte.toString(16).padStart(2, '0')).join('');
}

function createRepository() {
    const repository = createCemStudioProjectRepository({
        databaseName: `feature-tour-${crypto.randomUUID()}`,
        validateProject: async (bundle) => bundle,
        now: () => '2026-08-21T00:00:00Z',
    });
    repositories.push(repository);
    return repository;
}

async function seed(version, content) {
    const bytes = new TextEncoder().encode(content);
    const hash = await crypto.subtle.digest('SHA-256', bytes);
    const sha256 = [...new Uint8Array(hash)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
    return {
        catalog: {
            seed: { id: CEM_STUDIO_FEATURE_TOUR_SEED_ID, version },
        },
        bundle: {
            project: {
                $schema: 'https://cem.dev/ns/studio/project/1',
                schemaVersion: 1,
                id: CEM_STUDIO_FEATURE_TOUR_SEED_ID,
                name: 'CEM-ML Feature Tour',
                rootUri: `studio://${CEM_STUDIO_FEATURE_TOUR_SEED_ID}/`,
                revision: 1,
                createdAt: '2026-08-21T00:00:00Z',
                updatedAt: '2026-08-21T00:00:00Z',
                entries: [],
                resources: [{
                    id: 'source',
                    role: 'data',
                    sourceKind: 'project-file',
                    path: 'source.cem',
                    contentType: 'application/cem',
                    schema: 'https://cem.dev/ns/cem-ml/1',
                    revision: 1,
                    sha256,
                }],
            },
            contents: { source: bytes },
        },
    };
}

async function exportedText(repository, projectId) {
    const result = await repository.query(request('export-project', { projectId }));
    return new TextDecoder().decode(result.value.contents.source);
}

function request(operation, parameters) {
    return {
        protocolVersion: 1,
        repository: CEM_STUDIO_REPOSITORY_ID,
        operation,
        requestRevision: 1,
        parameters,
    };
}
