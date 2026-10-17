import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { serializeCemStudioProjectBundle } from '../src/file-system-provider.js';

/** Build an authored project; command grammar and execution stay in the shared CLI. */
export async function createPresentationSample(workspaceRoot, commonVersion) {
    const definition = JSON.parse(await readFile(resolve(workspaceRoot, 'packages/cem-studio/fixtures/amsterdam-login/sample.json'), 'utf8'));
    const contents = {};
    const resources = [];
    const entries = [{ id: 'login-lifecycle', kind: 'subproject', name: 'Login lifecycle', description: definition.description }];
    const digest = bytes => createHash('sha256').update(bytes).digest('hex');
    const json = value => Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
    const resource = (id, path, bytes, contentType, schema, role) => {
        contents[id] = bytes;
        resources.push({ id, path, referencePath: path, role, sourceKind: 'project-file', contentType, schema, revision: 1, sha256: digest(bytes) });
    };
    for (const source of definition.sources) {
        resource(source.id, source.path, await readFile(resolve(workspaceRoot, source.source)), source.contentType, source.schema, source.role ?? 'data');
    }
    for (const page of definition.pages) {
        const source = definition.sources.find(source => source.id === page.source);
        if (!source) throw new Error(`Presentation page ${page.id} has no source`);
        const id = `command-${page.id}`;
        const dependencies = (page.dependencies ?? []).map(id => {
            const resource = definition.sources.find(source => source.id === id);
            if (!resource) throw new Error(`Presentation page ${page.id} has no dependency ${id}`);
            return resource;
        });
        const validation = page.argv[0] === 'validate';
        // These are versioned authored argv documents, never lowered run plans.
        // Native fixtures execute the same argv; browser acceptance checks the codec.
        const bytes = json(validation ? {
            inputs: [{ uri: source.path, rootScope: { defaultContentType: source.contentType, schema: source.schema } }],
            outputs: [], schemaPackages: dependencies.filter(resource => resource.schema === 'https://cem.dev/ns/schema-package/1').map(resource => ({ uri: resource.path })), resolvers: [], scheduler: {},
        } : {
            $schema: 'https://cem.dev/ns/cli/command/1', schemaVersion: 1,
            commandSchemaVersion: 1, commonVersion, binaryName: 'cem-ml',
            argv: page.argv.map(argument => argument === '$source' ? source.path : dependencies.find(resource => argument === `$${resource.id}`)?.path ?? argument),
        });
        resource(id, `config/${page.id}.json`, bytes,
            validation ? 'application/json' : 'application/vnd.cem.cli-command+json',
            `https://cem.dev/ns/cli/${validation ? 'run-config' : 'command'}/1`, 'run-config');
        entries.push({ id: page.id, parentId: 'login-lifecycle', kind: page.kind, name: page.name, ...(page.description ? { description: page.description } : {}),
            resourceIds: [source.id, id, ...dependencies.map(resource => resource.id)], runConfigResourceId: id, tags: ['amsterdam', 'login', page.argv[0]] });
    }
    const timestamp = '2026-10-17T00:00:00Z';
    const project = { $schema: 'https://cem.dev/ns/studio/project/1', schemaVersion: 1,
        id: `${definition.id}-seed`, name: definition.name, description: definition.description,
        rootUri: `studio://${definition.id}-seed/`, revision: 1, createdAt: timestamp, updatedAt: timestamp, entries, resources };
    const archive = serializeCemStudioProjectBundle({ project, contents });
    return { archive, sample: { id: definition.id, name: definition.name, version: '1.1.0',
        archive: './amsterdam/login.cem-studio.json', sha256: digest(archive) } };
}
