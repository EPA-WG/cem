export const CEM_STUDIO_FEATURE_TOUR_SEED_ID = 'cem-ml-feature-tour-seed';
export const CEM_STUDIO_FEATURE_TOUR_COPY_ID = 'feature-tour';
export const CEM_STUDIO_PROJECT_CONTENT_TYPE = 'application/vnd.cem.studio-project+json';
export const CEM_STUDIO_PROJECT_SCHEMA = 'https://cem.dev/ns/studio/project/1';

const CEM_STDOUT_URI = 'cem-stdio://stdout';
const ARTIFACT_READ_BYTES = 1024 * 1024;

/** Create one reusable real CEM-ML browser command validator. */
export async function createCemStudioBrowserValidator() {
    const {
        buildBrowserCommandInvocation,
        createBrowserCommandServiceClient,
        parseCemMlCommand,
        projectBrowserCommandPresentation,
    } = await import('@epa-wg/cem-ml-cli/browser');
    const ledgers = new Map();
    const pendingWrites = new Map();
    const committedWrites = new Map();
    let nextWrite = 1;
    const client = await createBrowserCommandServiceClient({
        host: {
            currentRevision: async ({ requestId }) => {
                const ledger = ledgers.get(requestId);
                if (!ledger) throw new Error(`missing CEM Studio command ledger ${requestId}`);
                return ledger;
            },
            readResource: async ({ uri }) => {
                throw new Error(`inline CEM Studio command unexpectedly read ${uri}`);
            },
            prepareWrite: async (request, bytes) => {
                const { requestId } = request;
                const token = `${requestId}:command-output:${nextWrite++}`;
                pendingWrites.set(token, { request, bytes: new Uint8Array(bytes) });
                return { token };
            },
            commitWrite: async (token) => {
                const staged = pendingWrites.get(token);
                if (!staged) {
                    throw new Error(`CEM Studio command publication token is unknown: ${token}`);
                }
                pendingWrites.delete(token);
                const writes = committedWrites.get(staged.request.requestId) ?? [];
                writes.push(staged);
                committedWrites.set(staged.request.requestId, writes);
                return { uri: `memory:${token}` };
            },
            rollbackWrite: async (token) => {
                pendingWrites.delete(token);
            },
        },
    });
    let nextRequest = 1;

    const executeResourceCommand = async ({
        operation,
        argv,
        bytes,
        uri = 'input.cem',
        dependencies = [],
        projectId = 'cem-studio-command',
        projectRevision = 1,
        resourceRevision = 1,
        signal,
        readOutput = false,
    }) => {
        const requestId = `cem-studio-${operation}-${nextRequest++}`;
        const source = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
        const inputResourceUri = /^[a-z][a-z0-9+.-]*:/i.test(uri)
            ? uri
            : new URL(uri, `cem-studio://${projectId}/`).href;
        const parsed = parseCemMlCommand(argv(inputResourceUri), { runtime: 'wasm-browser-worker' });
        const dependencyUris = new Map(dependencies.map((dependency) => [
            dependency.path,
            resolveVirtualUri(inputResourceUri, dependency.path),
        ]));
        const availableResources = new Map([
            [inputResourceUri, { uri: inputResourceUri, bytes: source }],
            ...dependencies.map((dependency) => {
                const dependencyUri = dependencyUris.get(dependency.path);
                return [dependencyUri, { uri: dependencyUri, bytes: dependency.bytes }];
            }),
        ]);
        const invocation = await buildBrowserCommandInvocation(
            parsed,
            async (requirement) => {
                if (!availableResources.has(requirement.uri)) {
                    throw new Error(`CEM-ML ${operation} requested undeclared resource ${requirement.uri}`);
                }
                return [...availableResources.values()];
            },
            {
                requestId,
                projectId,
                projectRevision,
                resourceRevision,
                cwd: `/${projectId}`,
            },
        );
        const resources = { ...invocation.request.resources };
        if (!resources[inputResourceUri]) throw new Error(`CEM-ML ${operation} omitted input ${inputResourceUri}`);
        for (const dependency of dependencies) {
            const dependencyUri = dependencyUris.get(dependency.path);
            if (!dependencyUri) throw new Error(`CEM-ML ${operation} did not resolve dependency ${dependency.path}`);
            const resource = resources[dependencyUri];
            if (!resource) throw new Error(`CEM-ML ${operation} omitted dependency ${dependency.path}`);
            resources[dependencyUri] = {
                ...resource,
                identity: {
                    contentType: dependency.contentType,
                    schema: dependency.schema,
                    baseUri: dependencyUri,
                },
            };
        }
        const request = { ...invocation.request, resources };
        ledgers.set(requestId, {
            project: request.project,
            resourceVersions: request.resourceVersions,
        });
        const handle = client.execute(request, { signal });
        try {
            const result = await handle.result();
            const presentation = projectBrowserCommandPresentation(invocation.presentation, result);
            let output;
            if (readOutput) {
                try {
                    output = await readCommandOutput(
                        handle,
                        result,
                        invocation.presentation.stdoutArtifactUri,
                        committedWrites.get(requestId),
                    );
                } catch (error) {
                    if (result.exitCode === 0) throw error;
                }
            }
            if (result.exitCode !== 0) {
                const diagnostics = result.diagnostics?.items
                    ?.map(({ code, message }) => `${code}: ${message}`)
                    .join('; ');
                const error = new Error(
                    `CEM-ML ${operation} rejected ${uri} with exit code ${result.exitCode}`
                    + (diagnostics ? ` (${diagnostics})` : ''),
                );
                error.code = `cem.studio.${operation}_failed`;
                error.result = result;
                error.presentation = presentation;
                error.output = output;
                throw error;
            }
            return Object.freeze({ result, presentation, output });
        } finally {
            ledgers.delete(requestId);
            committedWrites.delete(requestId);
            await handle.dispose().catch(() => undefined);
        }
    };

    const resourceCommandOptions = (options, operation, argv, readOutput = false) => ({
        ...options,
        operation,
        argv,
        readOutput,
    });

    const validateResource = async (options) => executeResourceCommand(resourceCommandOptions(
        options,
        'validate',
        (inputResourceUri) => [
            'validate',
            '--format',
            'json',
            '--content-type',
            options.contentType,
            '--schema',
            options.schema,
            inputResourceUri,
        ],
    ));

    const parseResource = async (options) => executeResourceCommand(resourceCommandOptions(
        options,
        'parse',
        (inputResourceUri) => [
            'parse',
            '--no-color',
            '--format',
            options.projection ?? 'ast',
            '--content-type',
            options.contentType,
            '--schema',
            options.schema,
            inputResourceUri,
        ],
        true,
    ));

    const inspectResource = async (options) => executeResourceCommand(resourceCommandOptions(
        options,
        'inspect',
        (inputResourceUri) => [
            'inspect',
            '--no-color',
            '--show',
            options.view ?? 'summary',
            '--format',
            'cem',
            '--content-type',
            options.contentType,
            '--schema',
            options.schema,
            inputResourceUri,
        ],
        true,
    ));

    return Object.freeze({
        capability: client.capability,
        commonVersion: client.worker.commonVersion,
        parseResource,
        inspectResource,
        validateResource,
        async validateProject(bundle, { signal } = {}) {
            if (!bundle?.project) throw new TypeError('CEM Studio project bundle is missing project metadata');
            await validateResource({
                bytes: new TextEncoder().encode(`${JSON.stringify(bundle.project)}\n`),
                contentType: CEM_STUDIO_PROJECT_CONTENT_TYPE,
                schema: CEM_STUDIO_PROJECT_SCHEMA,
                uri: 'project.json',
                signal,
            });
            return bundle;
        },
        assertCatalog(catalog) {
            assertFeatureTourCatalog(catalog);
            for (const key of ['runtime', 'targetIdentity', 'abiIdentity']) {
                if (catalog.capability[key] !== client.capability[key]) {
                    throw new Error(
                        `Feature Tour ${key} ${catalog.capability[key]} does not match ${client.capability[key]}`,
                    );
                }
            }
            if (catalog.commonVersion !== client.worker.commonVersion) {
                throw new Error(
                    `Feature Tour common version ${catalog.commonVersion} does not match ${client.worker.commonVersion}`,
                );
            }
        },
        close: () => client.close(),
    });
}

/** Fetch and integrity-check the graph-emitted read-only seed bundle. */
export async function loadCemStudioFeatureTour(options = {}) {
    const fetchResource = options.fetch ?? globalThis.fetch;
    if (typeof fetchResource !== 'function') throw new Error('Feature Tour loading requires fetch');
    const baseUrl = new URL(options.baseUrl ?? './samples/feature-tour/', globalThis.document?.baseURI);
    const catalog = await fetchJson(new URL('catalog.json', baseUrl), fetchResource);
    assertFeatureTourCatalog(catalog);
    options.validator?.assertCatalog(catalog);
    const projectBytes = await fetchBytes(new URL(catalog.seed.project, baseUrl), fetchResource);
    if (await sha256(projectBytes) !== catalog.seed.projectSha256) {
        throw new Error('Feature Tour project metadata failed its SHA-256 integrity check');
    }
    const project = JSON.parse(new TextDecoder().decode(projectBytes));
    if (project.id !== catalog.seed.id || project.id !== CEM_STUDIO_FEATURE_TOUR_SEED_ID) {
        throw new Error('Feature Tour seed identity does not match its catalog');
    }

    const contents = {};
    await Promise.all(catalog.examples.map(async (example) => {
        const [asset, runConfig, dependencies] = await Promise.all([
            fetchBytes(new URL(example.asset, baseUrl), fetchResource),
            fetchBytes(new URL(example.runConfig, baseUrl), fetchResource),
            Promise.all(example.dependencies.map(async (dependency) => {
                const bytes = await fetchBytes(new URL(dependency.asset, baseUrl), fetchResource);
                if (await sha256(bytes) !== dependency.sha256) {
                    throw new Error(`Feature Tour dependency ${dependency.resourceId} failed its SHA-256 integrity check`);
                }
                return { dependency, bytes };
            })),
        ]);
        if (await sha256(asset) !== example.sha256) {
            throw new Error(`Feature Tour example ${example.id} failed its SHA-256 integrity check`);
        }
        contents[example.resourceId] = asset;
        contents[example.runConfigResourceId] = runConfig;
        for (const dependency of dependencies) contents[dependency.dependency.resourceId] = dependency.bytes;
    }));
    if (project.resources.some(({ id }) => !(id in contents))) {
        throw new Error('Feature Tour bundle is missing a declared project resource');
    }
    return Object.freeze({ catalog, bundle: { project, contents } });
}

/** Install once, preserve every existing user copy, or create a new reset copy. */
export async function installCemStudioFeatureTour(repository, seed, options = {}) {
    const listed = await repository.query(repositoryRequest('list-projects', { includeTrash: true }));
    const projects = Array.isArray(listed.value) ? listed.value : [];
    const copies = projects.filter(({ id }) => copyOrdinal(id) !== undefined);
    if (!options.reset && copies.length > 0) {
        copies.sort((left, right) => copyOrdinal(left.id) - copyOrdinal(right.id));
        return Object.freeze({ status: 'preserved', projectId: copies[0].id, seedVersion: seed.catalog.seed.version });
    }
    const projectId = nextCopyId(projects.map(({ id }) => id));
    const bundle = createCemStudioFeatureTourCopy(seed, {
        projectId,
        now: options.now?.() ?? new Date().toISOString(),
    });
    const imported = await repository.execute(repositoryRequest('import-project', {
        bundle,
        mode: 'create',
    }));
    return Object.freeze({
        status: options.reset ? 'reset' : 'installed',
        projectId,
        seedVersion: seed.catalog.seed.version,
        repositoryRevision: imported.repositoryRevision,
    });
}

export function createCemStudioFeatureTourCopy(seed, { projectId, now }) {
    const bundle = structuredClone(seed.bundle);
    const source = bundle.project;
    bundle.project = {
        ...source,
        id: projectId,
        name: source.name,
        description: `Editable copy of ${seed.catalog.seed.id}@${seed.catalog.seed.version}.`,
        rootUri: `studio://${projectId}/`,
        revision: 1,
        createdAt: now,
        updatedAt: now,
    };
    return bundle;
}

function assertFeatureTourCatalog(catalog) {
    if (
        catalog?.schemaVersion !== 1
        || catalog.seed?.id !== CEM_STUDIO_FEATURE_TOUR_SEED_ID
        || catalog.capability?.operation !== 'validate'
        || catalog.capability.availability !== 'available'
        || !Array.isArray(catalog.examples)
        || catalog.examples.length !== catalog.exampleCount
        || catalog.exampleCount !== catalog.packageCount
    ) {
        throw new Error('Feature Tour catalog is incompatible');
    }
}

function nextCopyId(projectIds) {
    const used = new Set(projectIds);
    if (!used.has(CEM_STUDIO_FEATURE_TOUR_COPY_ID)) return CEM_STUDIO_FEATURE_TOUR_COPY_ID;
    for (let ordinal = 2; Number.isSafeInteger(ordinal); ordinal += 1) {
        const candidate = `${CEM_STUDIO_FEATURE_TOUR_COPY_ID}-${ordinal}`;
        if (!used.has(candidate)) return candidate;
    }
    throw new Error('Feature Tour copy identity space is exhausted');
}

function copyOrdinal(projectId) {
    if (projectId === CEM_STUDIO_FEATURE_TOUR_COPY_ID) return 1;
    const match = /^feature-tour-([2-9][0-9]*)$/.exec(projectId);
    return match ? Number(match[1]) : undefined;
}

function repositoryRequest(operation, parameters) {
    return {
        protocolVersion: 1,
        repository: 'studio-projects',
        operation,
        requestRevision: 1,
        parameters,
    };
}

function resolveVirtualUri(baseUri, relativePath) {
    if (/^[a-z][a-z0-9+.-]*:/i.test(baseUri)) return new URL(relativePath, baseUri).href;
    const absoluteBase = baseUri.startsWith('/') ? baseUri : `/${baseUri}`;
    return new URL(relativePath, `https://cem.invalid${absoluteBase}`).pathname;
}

async function readCommandOutput(handle, result, requestedUri = CEM_STDOUT_URI, committed = []) {
    const artifacts = result?.artifacts?.items ?? [];
    const artifact = artifacts.find(({ uri }) => uri === requestedUri)
        ?? artifacts.find(({ kind }) => kind === 'output');
    let metadata;
    let bytes;
    if (artifact) {
        metadata = artifact;
        bytes = new Uint8Array(artifact.byteLength);
        let offset = 0;
        while (offset < artifact.byteLength) {
            const read = await handle.readArtifact(artifact, {
                offset,
                maxBytes: Math.min(ARTIFACT_READ_BYTES, artifact.byteLength - offset),
            });
            if (read.metadata.offset !== offset || read.bytes.byteLength === 0) {
                throw new Error(`CEM-ML command returned an invalid artifact range at byte ${offset}`);
            }
            bytes.set(read.bytes, offset);
            offset += read.bytes.byteLength;
        }
    } else {
        const publication = committed.find(({ request }) => request.uri === requestedUri)
            ?? committed.find(({ request }) => request.kind === 'output');
        if (!publication) {
            throw new Error(
                `CEM-ML command omitted its ${requestedUri} output publication: ${JSON.stringify({
                    operation: result?.operation,
                    status: result?.status,
                    exitCode: result?.exitCode,
                    artifacts: artifacts.map(({ kind, uri, byteLength }) => ({ kind, uri, byteLength })),
                    committed: committed.map(({ request }) => ({
                        kind: request.kind,
                        uri: request.uri,
                        byteLength: request.byteLength,
                    })),
                })}`,
            );
        }
        metadata = publication.request;
        bytes = publication.bytes;
    }
    if (bytes.byteLength !== metadata.byteLength) {
        throw new Error(`CEM-ML command output length ${bytes.byteLength} does not match ${metadata.byteLength}`);
    }
    const digest = await sha256(bytes);
    if (digest !== metadata.sha256) {
        throw new Error(`CEM-ML command output hash ${digest} does not match ${metadata.sha256}`);
    }
    return Object.freeze({
        uri: metadata.uri ?? requestedUri,
        contentType: metadata.contentType,
        byteLength: metadata.byteLength,
        sha256: metadata.sha256,
        bytes: Object.freeze([...bytes]),
        text: new TextDecoder().decode(bytes),
    });
}

async function fetchJson(url, fetchResource) {
    return JSON.parse(new TextDecoder().decode(await fetchBytes(url, fetchResource)));
}

async function fetchBytes(url, fetchResource) {
    const response = await fetchResource(url);
    if (!response.ok) throw new Error(`Feature Tour resource failed: ${response.status} ${url.href}`);
    return new Uint8Array(await response.arrayBuffer());
}

async function sha256(bytes) {
    const digest = await crypto.subtle.digest('SHA-256', bytes);
    return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}
