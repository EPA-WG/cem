import { afterEach, beforeAll, describe, expect, it } from 'vitest';

import {
    CEM_STUDIO_FEATURE_TOUR_SEED_ID,
    createCemStudioBrowserValidator,
    installCemStudioFeatureTour,
} from './feature-tour.js';
import { CEM_STUDIO_REPOSITORY_ID, createCemStudioProjectRepository } from './repository.js';
let invalidLogin, loginSchema, loginPackage, loginHtml, loginCem, semanticsQuery, semanticsContract;
beforeAll(async () => {
    [invalidLogin, loginSchema, loginPackage, loginHtml, loginCem, semanticsQuery, semanticsContract] = await Promise.all([
        new URL('../../../examples/projects/login/login-invalid.cem', import.meta.url),
        new URL('../../../examples/projects/login/login-schema.cem', import.meta.url),
        new URL('../../../examples/projects/login/login-schema-package.cem', import.meta.url),
        new URL('../../../examples/semantic/login.html', import.meta.url),
        new URL('../../../examples/cem-ml/login.cem', import.meta.url),
        new URL('../../../examples/projects/login/queries/semantics.xpath', import.meta.url),
        new URL('../../../examples/projects/login/expected/semantics.json', import.meta.url),
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
    it('queries the same Login semantic contract from HTML and CEM in the native browser worker', async () => {
        const validator = await createCemStudioBrowserValidator(); validators.push(validator);
        const bytes = text => new TextEncoder().encode(text);
        const expected = JSON.parse(semanticsContract);
        for (const [source, contentType, schema, path] of [
            [loginHtml, 'text/html', 'https://cem.dev/ns/data/html/1', 'login.html'],
            [loginCem, 'application/cem', 'https://cem.dev/ns/cem-ml/1', 'login.cem'],
        ]) {
            const uri = `studio://semantic-copy/data/${path}`;
            const argv = ['query', uri, '--content-type', contentType, '--schema', schema, '--query-file', 'studio://semantic-copy/queries/semantics.xpath',
                '--query-content-type', 'application/vnd.cem.xpath', '--query-schema', 'https://cem.dev/ns/query/xpath/1', '--output', 'json'];
            const options = { uri, contentType, schema, projectId: 'semantic-copy', projectRootUri: 'studio://semantic-copy/', argv,
                dependencies: [{ path: '../queries/semantics.xpath', bytes: bytes(semanticsQuery), contentType: 'application/vnd.cem.xpath', schema: 'https://cem.dev/ns/query/xpath/1' }] };
            const run = async source => {
                const outcome = await validator.runResourceCommand({ ...options, bytes: bytes(source) });
                expect(outcome.result.exitCode).toBe(0);
                return JSON.parse(outcome.output.text).result.sequence.items.map(item => item.value.lexicalValue);
            };
            expect(await run(source)).toEqual(expected);
            const edited = source.replace('Sign in', 'Welcome').replace(contentType === 'text/html' ? 'method="post"' : '@method=post', contentType === 'text/html' ? 'method="get"' : '@method=get');
            const result = await run(edited);
            expect(result).toEqual(expected.map(value => value === 'title=Sign in' ? 'title=Welcome' : value === 'method=post' ? 'method=get' : value));
            const selected = await validator.runResourceCommand({ ...options, argv: ['query', uri, '--content-type', contentType, '--schema', schema, '--query', '//Q{http://www.w3.org/1999/xhtml}input', '--query-content-type', 'application/vnd.cem.xpath', '--output', 'json'], bytes: bytes(source) });
            const items = JSON.parse(selected.output.text).result.sequence.items;
            expect(items).toHaveLength(2);
            expect(items.every(item => item.sourceMap.frames.length > 0)).toBe(true);
            expect(items[0].sourceUri).toBe(uri);
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
