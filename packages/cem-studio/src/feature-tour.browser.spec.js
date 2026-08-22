import { afterEach, describe, expect, it } from 'vitest';

import {
    CEM_STUDIO_FEATURE_TOUR_SEED_ID,
    createCemStudioBrowserValidator,
    installCemStudioFeatureTour,
} from './feature-tour.js';
import { CEM_STUDIO_REPOSITORY_ID, createCemStudioProjectRepository } from './repository.js';

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
