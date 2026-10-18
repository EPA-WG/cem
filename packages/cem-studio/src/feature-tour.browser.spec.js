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
