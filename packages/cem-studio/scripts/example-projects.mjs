import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { serializeCemStudioProjectBundle } from '../src/file-system-provider.js';

/** Build an authored project; command grammar and execution stay in the shared CLI. */
export async function createExampleProjects(workspaceRoot, commonVersion) {
    const root = resolve(workspaceRoot, 'examples/projects');
    const projects = [];
    for (const directory of (await readdir(root, { withFileTypes: true })).filter(item => item.isDirectory()).sort((a, b) => a.name.localeCompare(b.name))) {
        const definition = JSON.parse(await readFile(resolve(root, directory.name, 'example.json'), 'utf8'));
        projects.push(await createExampleProject(workspaceRoot, commonVersion, definition));
    }
    return projects;
}

async function createExampleProject(workspaceRoot, commonVersion, definition) {
    const contents = {};
    const resources = [];
    const entries = [];
    const digest = bytes => createHash('sha256').update(bytes).digest('hex');
    const json = value => Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
    const resource = (id, path, bytes, contentType, schema, role) => {
        contents[id] = bytes;
        resources.push({ id, path, referencePath: path, role, sourceKind: 'project-file', contentType, schema, revision: 1, sha256: digest(bytes) });
    };
    for (const source of definition.sources) {
        resource(source.id, source.path, await readFile(resolve(workspaceRoot, source.source)), source.contentType, source.schema, source.role ?? 'data');
    }
    for (const configuration of definition.configurations) {
        const source = definition.sources.find(source => source.id === configuration.source);
        if (!source) throw new Error(`Example configuration ${configuration.id} has no source`);
        const id = `command-${configuration.id}`;
        const dependencies = (configuration.dependencies ?? []).map(id => {
            const resource = definition.sources.find(source => source.id === id);
            if (!resource) throw new Error(`Example configuration ${configuration.id} has no dependency ${id}`);
            return resource;
        });
        const validation = configuration.argv[0] === 'validate';
        // These are versioned authored argv documents, never lowered run plans.
        // Native fixtures execute the same argv; browser acceptance checks the codec.
        const bytes = json(validation ? {
            inputs: [{ uri: source.path, rootScope: { defaultContentType: source.contentType, schema: source.schema } }],
            outputs: [], schemaPackages: dependencies.filter(resource => resource.schema === 'https://cem.dev/ns/schema-package/1').map(resource => ({ uri: resource.path })), resolvers: [], scheduler: {},
        } : {
            $schema: 'https://cem.dev/ns/cli/command/1', schemaVersion: 1,
            commandSchemaVersion: 1, commonVersion, binaryName: 'cem-ml',
            argv: configuration.argv.map(argument => argument === '$source' ? source.path : dependencies.find(resource => argument === `$${resource.id}`)?.path ?? argument),
        });
        resource(id, `config/${configuration.id}.json`, bytes,
            validation ? 'application/json' : 'application/vnd.cem.cli-command+json',
            `https://cem.dev/ns/cli/${validation ? 'run-config' : 'command'}/1`, 'run-config');
        entries.push({ id: configuration.id, kind: configuration.kind, name: configuration.name, ...(configuration.description ? { description: configuration.description } : {}),
            resourceIds: [source.id, id, ...dependencies.map(resource => resource.id)], runConfigResourceId: id, tags: [...new Set([configuration.argv[0], ...(configuration.tags ?? [])])] });
    }
    const timestamp = '2026-10-17T00:00:00Z';
    const project = { $schema: 'https://cem.dev/ns/studio/project/1', schemaVersion: 1,
        id: `${definition.id}-seed`, name: definition.name, description: definition.description,
        rootUri: `studio://${definition.id}-seed/`, revision: 1, createdAt: timestamp, updatedAt: timestamp, entries, resources };
    const archive = serializeCemStudioProjectBundle({ project, contents });
    return { archive, record: { id: definition.id, name: definition.name, version: definition.version, collections: definition.collections ?? [],
        archive: `./examples/${definition.id}.cem-studio.json`, sha256: digest(archive) } };
}
