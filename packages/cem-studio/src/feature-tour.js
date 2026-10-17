import { commandSchema, parseCemMlCommandText, serializeCemMlCommandText } from '@epa-wg/cem-ml-cli/browser-commands';
import {
    CEM_STUDIO_LIMITS,
    assertCemStudioResourceUri,
    assertCemStudioResultSize,
    assertCemStudioSourceSet,
    redactCemStudioSecrets,
} from './preview.js';

export const CEM_STUDIO_FEATURE_TOUR_SEED_ID = 'cem-ml-feature-tour-seed';
export const CEM_STUDIO_FEATURE_TOUR_COPY_ID = 'feature-tour';
export const CEM_STUDIO_PROJECT_CEM_CONTENT_TYPE = 'application/vnd.cem.studio-project+cem';
export const CEM_STUDIO_PROJECT_CONTENT_TYPE = 'application/vnd.cem.studio-project+json';
export const CEM_STUDIO_PROJECT_SCHEMA = 'https://cem.dev/ns/studio/project/1';

const CEM_STDOUT_URI = 'cem-stdio://stdout';
const ARTIFACT_READ_BYTES = 1024 * 1024;
const PORTABLE_STUDIO_OPERATIONS = new Set(['parse', 'inspect', 'convert', 'query', 'transform', 'trace']);

/** Create one reusable real CEM-ML browser command validator. */
export async function createCemStudioBrowserValidator() {
    const {
        buildBrowserCommandInvocation,
        createBrowserTransformConfigAuthor,
        commandSchema,
        createBrowserCommandServiceClient,
        parseCemMlCommand,
        parseCemMlCommandResource,
        parseCemMlCommandText,
        projectBrowserCommandPresentation,
        serializeCemMlCommand,
        serializeCemMlCommandResource,
        serializeCemMlCommandText,
    } = await import('@epa-wg/cem-ml-cli/browser-commands');
    const authorTransformConfigV1 = await createBrowserTransformConfigAuthor();
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
                throw new Error(`inline CEM Studio command unexpectedly read ${redactCemStudioSecrets(uri)}`);
            },
            prepareWrite: async (request, bytes) => {
                assertCemStudioResultSize(request.byteLength ?? bytes.byteLength);
                assertCemStudioResultSize(bytes);
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
        parsedCommand,
        bytes,
        uri = 'input.cem',
        dependencies = [],
        projectId = 'cem-studio-command',
        projectRevision = 1,
        resourceRevision = 1,
        resourceScheme = 'cem-studio',
        signal,
        readOutput = false,
    }) => {
        const requestId = `cem-studio-${operation}-${nextRequest++}`;
        const inputResourceUri = resourceUri(uri, projectId, resourceScheme);
        const parsed = parsedCommand ?? parseCemMlCommand(argv(inputResourceUri), { runtime: 'wasm-browser-worker' });
        const invocation = await prepareResourceInvocation(
            parsed,
            {
                bytes,
                uri,
                dependencies,
                projectId,
                projectRevision,
                resourceRevision,
                resourceScheme,
            },
            requestId,
            true,
        );
        const request = invocation.request;
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
                    `CEM-ML ${operation} rejected ${uri} with exit code ${result.exitCode}` +
                        (diagnostics ? ` (${diagnostics})` : ''),
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

    const prepareResourceInvocation = async (parsed, options, requestId, requireResourceSet) => {
        const {
            bytes,
            uri = 'input.cem',
            dependencies = [],
            projectId = 'cem-studio-command',
            projectRevision = 1,
            resourceRevision = 1,
            resourceScheme = 'cem-studio',
        } = options;
        const operation = parsed.commandPath[0] ?? parsed.metaAction ?? 'command';
        const source = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
        assertCemStudioSourceSet(source, dependencies);
        const inputResourceUri = resourceUri(uri, projectId, resourceScheme);
        const dependencyUris = new Map(
            dependencies.map((dependency) => [
                dependency.path,
                assertCemStudioResourceUri(resolveVirtualUri(inputResourceUri, dependency.path), resourceScheme),
            ]),
        );
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
                cwd: assertCemStudioResourceUri(options.projectRootUri ?? `${resourceScheme}://${projectId}/`, resourceScheme),
            },
        );
        const resources = { ...invocation.request.resources };
        const packageIdentities = new Map((invocation.request.runPlan?.schemaPackages ?? [])
            .map(resource => [resource.resolvedUri ?? resource.declaredUri, resource.identity]));
        if (requireResourceSet && !resources[inputResourceUri]) {
            throw new Error(`CEM-ML ${operation} omitted input ${inputResourceUri}`);
        }
        for (const dependency of dependencies) {
            const dependencyUri = dependencyUris.get(dependency.path);
            if (!dependencyUri) throw new Error(`CEM-ML ${operation} did not resolve dependency ${dependency.path}`);
            const resource = resources[dependencyUri];
            if (!resource) {
                if (requireResourceSet) throw new Error(`CEM-ML ${operation} omitted dependency ${dependency.path}`);
                continue;
            }
            resources[dependencyUri] = {
                ...resource,
                // Package identity comes from its explicit normalized scope;
                // other dependencies use their declared project identities.
                identity: packageIdentities.get(dependencyUri) ?? {
                    contentType: dependency.contentType,
                    schema: dependency.schema,
                    baseUri: dependencyUri,
                },
            };
        }
        return Object.freeze({
            ...invocation,
            request: Object.freeze({ ...invocation.request, resources: Object.freeze(resources) }),
        });
    };

    const resourceCommandOptions = (options, operation, argv, readOutput = false) => ({
        ...options,
        operation,
        argv,
        readOutput,
    });

    const validateResource = async (options) =>
        executeResourceCommand(
            resourceCommandOptions(options, 'validate', (inputResourceUri) =>
                resourceCommandArguments('validate', options, inputResourceUri),
            ),
        );

    const parseResource = async (options) =>
        executeResourceCommand(
            resourceCommandOptions(
                options,
                'parse',
                (inputResourceUri) => resourceCommandArguments('parse', options, inputResourceUri),
                true,
            ),
        );

    const inspectResource = async (options) =>
        executeResourceCommand(
            resourceCommandOptions(
                options,
                'inspect',
                (inputResourceUri) => resourceCommandArguments('inspect', options, inputResourceUri),
                true,
            ),
        );

    const runResourceCommand = async (options) => {
        if (!Array.isArray(options.argv) || options.argv.length === 0) {
            throw new TypeError('CEM Studio resource command requires literal argv');
        }
        const parsed = parseCemMlCommand(options.argv, { runtime: 'wasm-browser-worker' });
        const operation = parsed.commandPath[0];
        if (!PORTABLE_STUDIO_OPERATIONS.has(operation)) {
            throw new TypeError(`unsupported CEM Studio resource command: ${String(operation)}`);
        }
        return executeResourceCommand({
            ...options,
            operation,
            parsedCommand: parsed,
            resourceScheme: 'studio',
            readOutput: true,
        });
    };

    const convertProjectManifest = async (bytes, fromContentType, toContentType, signal) => {
        const source = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
        const outcome = await executeResourceCommand({
            operation: 'convert',
            argv: (inputResourceUri) => [
                'convert',
                inputResourceUri,
                '--content-type',
                fromContentType,
                '--schema',
                CEM_STUDIO_PROJECT_SCHEMA,
                '--to-content-type',
                toContentType,
                '--to-schema',
                CEM_STUDIO_PROJECT_SCHEMA,
            ],
            bytes: source,
            uri: fromContentType === CEM_STUDIO_PROJECT_CEM_CONTENT_TYPE ? 'project.cem' : 'project.json',
            signal,
            readOutput: true,
        });
        return new Uint8Array(outcome.output.bytes);
    };

    const previewResourceCommand = async (options) => {
        const inputResourceUri = resourceUri(
            options.uri ?? 'input.cem',
            options.projectId ?? 'cem-studio-command',
            'studio',
        );
        const parsed = options.commandResource !== undefined
            ? parseCemMlCommandResource(options.commandResource, { runtime: 'wasm-browser-worker' }).command
            : options.text === undefined
                ? parseCemMlCommand(
                      options.argv ?? resourceCommandArguments(options.operation ?? 'parse', options, inputResourceUri),
                      { runtime: 'wasm-browser-worker' },
                  )
                : parseCemMlCommandText(options.text, { runtime: 'wasm-browser-worker' });
        const requestId = `cem-studio-command-preview-${nextRequest++}`;
        const invocation = await prepareResourceInvocation(
            parsed,
            { ...options, resourceScheme: 'studio' },
            requestId,
            false,
        );
        return Object.freeze({
            projection: 'studio',
            binaryName: commandSchema.binaryName,
            commonVersion: parsed.commonVersion,
            argv: serializeCemMlCommand(parsed),
            text: serializeCemMlCommandText(parsed),
            parsed,
            semantic: projectCommandSemanticPreview(invocation),
        });
    };

    const decodeResourceCommand = (bytes) => parseCemMlCommandResource(bytes, { runtime: 'wasm-browser-worker' }).command;

    const serializeResourceCommand = (parsed) => serializeCemMlCommandResource(parsed);

    const executeAuthoredResourceCommand = async (options) => {
        const authored = parseCemMlCommandResource(
            options.commandResource instanceof ArrayBuffer
                ? new Uint8Array(options.commandResource)
                : options.commandResource,
            { runtime: 'wasm-browser-worker' },
        );
        const parsed = authored.command;
        const operation = parsed.commandPath[0];
        if (!PORTABLE_STUDIO_OPERATIONS.has(operation)) {
            throw new TypeError(`unsupported authored CEM Studio resource command: ${String(operation)}`);
        }
        let outcome;
        try {
            outcome = await executeResourceCommand({
                ...options,
                operation,
                parsedCommand: parsed,
                resourceScheme: 'studio',
                readOutput: true,
            });
        } catch (error) {
            if (error && typeof error === 'object') error.parsed = parsed;
            throw error;
        }
        return Object.freeze({ ...outcome, parsed });
    };

    return Object.freeze({
        capability: client.capability,
        commonVersion: client.worker.commonVersion,
        parseResource,
        inspectResource,
        runResourceCommand,
        executeAuthoredResourceCommand,
        previewResourceCommand,
        serializeResourceCommand,
        decodeResourceCommand,
        authorGraph(request) {
            const response = JSON.parse(authorTransformConfigV1(JSON.stringify(request)));
            if (response.error) throw new Error(response.error);
            return response;
        },
        validateResource,
        async decodeProjectManifest(bytes, { signal } = {}) {
            const converted = await convertProjectManifest(
                bytes,
                CEM_STUDIO_PROJECT_CEM_CONTENT_TYPE,
                CEM_STUDIO_PROJECT_CONTENT_TYPE,
                signal,
            );
            return JSON.parse(new TextDecoder().decode(converted));
        },
        async encodeProjectManifest(project, { signal } = {}) {
            return convertProjectManifest(
                new TextEncoder().encode(`${JSON.stringify(project)}\n`),
                CEM_STUDIO_PROJECT_CONTENT_TYPE,
                CEM_STUDIO_PROJECT_CEM_CONTENT_TYPE,
                signal,
            );
        },
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
    if ((await sha256(projectBytes)) !== catalog.seed.projectSha256) {
        throw new Error('Feature Tour project metadata failed its SHA-256 integrity check');
    }
    const project = JSON.parse(new TextDecoder().decode(projectBytes));
    if (project.id !== catalog.seed.id || project.id !== CEM_STUDIO_FEATURE_TOUR_SEED_ID) {
        throw new Error('Feature Tour seed identity does not match its catalog');
    }

    const contents = {};
    await Promise.all(
        catalog.examples.map(async (example) => {
            const [asset, runConfig, dependencies] = await Promise.all([
                fetchBytes(new URL(example.asset, baseUrl), fetchResource),
                fetchBytes(new URL(example.runConfig, baseUrl), fetchResource),
                Promise.all(
                    example.dependencies.map(async (dependency) => {
                        const bytes = await fetchBytes(new URL(dependency.asset, baseUrl), fetchResource);
                        if ((await sha256(bytes)) !== dependency.sha256) {
                            throw new Error(
                                `Feature Tour dependency ${dependency.resourceId} failed its SHA-256 integrity check`,
                            );
                        }
                        return { dependency, bytes };
                    }),
                ),
            ]);
            if ((await sha256(asset)) !== example.sha256) {
                throw new Error(`Feature Tour example ${example.id} failed its SHA-256 integrity check`);
            }
            contents[example.resourceId] = asset;
            contents[example.runConfigResourceId] = runConfig;
            for (const dependency of dependencies) contents[dependency.dependency.resourceId] = dependency.bytes;
        }),
    );
    await Promise.all(
        catalog.workbenches.map(async (workbench) => {
            const [asset, runConfig, expected, dependencies] = await Promise.all([
                fetchBytes(new URL(workbench.asset, baseUrl), fetchResource),
                fetchBytes(new URL(workbench.runConfig, baseUrl), fetchResource),
                fetchBytes(new URL(workbench.expected, baseUrl), fetchResource),
                Promise.all(
                    workbench.dependencies.map(async (dependency) => ({
                        dependency,
                        bytes: await fetchBytes(new URL(dependency.asset, baseUrl), fetchResource),
                    })),
                ),
            ]);
            if ((await sha256(asset)) !== workbench.sha256) {
                throw new Error(`Feature Tour workbench ${workbench.id} failed its SHA-256 integrity check`);
            }
            if ((await sha256(expected)) !== workbench.expectedSha256) {
                throw new Error(
                    `Feature Tour workbench ${workbench.id} expected result failed its SHA-256 integrity check`,
                );
            }
            contents[workbench.resourceId] = asset;
            contents[workbench.runConfigResourceId] = runConfig;
            contents[workbench.expectedResourceId] = expected;
            for (const { dependency, bytes } of dependencies) {
                if ((await sha256(bytes)) !== dependency.sha256) {
                    throw new Error(
                        `Feature Tour workbench dependency ${dependency.resourceId} failed its SHA-256 integrity check`,
                    );
                }
                contents[dependency.resourceId] = bytes;
            }
        }),
    );
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
    const imported = await repository.execute(
        repositoryRequest('import-project', {
            bundle,
            mode: 'create',
        }),
    );
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
        catalog?.schemaVersion !== 1 ||
        catalog.seed?.id !== CEM_STUDIO_FEATURE_TOUR_SEED_ID ||
        catalog.capability?.operation !== 'validate' ||
        catalog.capability.availability !== 'available' ||
        !Array.isArray(catalog.examples) ||
        !Array.isArray(catalog.workbenches) ||
        catalog.workbenches.length !== 5 ||
        catalog.examples.length !== catalog.exampleCount ||
        catalog.exampleCount !== catalog.packageCount
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

function resourceUri(uri, projectId, scheme = 'cem-studio') {
    const resolved = /^[a-z][a-z0-9+.-]*:/i.test(uri) ? uri : new URL(uri, `${scheme}://${projectId}/`).href;
    return assertCemStudioResourceUri(resolved, scheme);
}

function resourceCommandArguments(operation, options, inputResourceUri) {
    if (Array.isArray(options.argv)) return options.argv;
    const packages = (options.dependencies ?? [])
        .filter(dependency => dependency.schema === 'https://cem.dev/ns/schema-package/1')
        .flatMap(dependency => ['--schema-package', resolveVirtualUri(inputResourceUri, dependency.path)]);
    const identity = ['--content-type', options.contentType, '--schema', options.schema, ...packages];
    if (operation === 'validate') {
        return ['validate', '--format', 'json', ...identity, inputResourceUri];
    }
    if (operation === 'parse') {
        return ['parse', '--no-color', '--format', options.projection ?? 'ast', ...identity, inputResourceUri];
    }
    if (operation === 'inspect') {
        return [
            'inspect',
            '--no-color',
            '--show',
            options.view ?? 'summary',
            '--format',
            'cem',
            ...identity,
            inputResourceUri,
        ];
    }
    throw new TypeError(`unsupported CEM Studio resource command: ${operation}`);
}

function projectCommandSemanticPreview(invocation) {
    const { request, presentation } = invocation;
    const plan = request.runPlan;
    const resourceIdentities = Object.entries(request.resources)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([uri, resource]) => ({
            uri,
            byteLength: resource.bytes.length,
            identity: resource.identity ?? null,
        }));
    return freezeCommandValue({
        operation: request.operation,
        inputs: {
            normalized: plan?.inputs ?? [],
            resources: resourceIdentities,
        },
        identity: {
            config: plan?.configIdentity ?? null,
            schemas: plan?.schemaPackages ?? [],
            versions: request.resourceVersions,
        },
        configuration: {
            commandProfile: plan?.commandProfile ?? null,
            effective: plan?.effectiveConfig ?? null,
            authoredSources: plan?.authoredSources ?? [],
            resolvers: plan?.resolvers ?? [],
            scheduler: plan?.scheduler ?? null,
            diagnosticsMode: plan?.diagnosticsMode ?? null,
        },
        outputs: {
            normalized: plan?.outputs ?? [],
            presentation,
        },
        scope: {
            project: request.project,
            policy: request.policyStamp,
            provenance: plan?.provenance ?? [],
        },
    });
}

function freezeCommandValue(value) {
    if (Array.isArray(value)) return Object.freeze(value.map((entry) => freezeCommandValue(entry)));
    if (value && typeof value === 'object') {
        return Object.freeze(
            Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, freezeCommandValue(entry)])),
        );
    }
    return value;
}

async function readCommandOutput(handle, result, requestedUri = CEM_STDOUT_URI, committed = []) {
    const artifacts = result?.artifacts?.items ?? [];
    const artifact =
        artifacts.find(({ uri }) => uri === requestedUri) ?? artifacts.find(({ kind }) => kind === 'output');
    let metadata;
    let bytes;
    if (artifact) {
        metadata = artifact;
        assertCemStudioResultSize(artifact.byteLength);
        bytes = new Uint8Array(artifact.byteLength);
        let offset = 0;
        while (offset < artifact.byteLength) {
            const read = await handle.readArtifact(artifact, {
                offset,
                maxBytes: Math.min(ARTIFACT_READ_BYTES, CEM_STUDIO_LIMITS.resultBytes, artifact.byteLength - offset),
            });
            if (read.metadata.offset !== offset || read.bytes.byteLength === 0) {
                throw new Error(`CEM-ML command returned an invalid artifact range at byte ${offset}`);
            }
            bytes.set(read.bytes, offset);
            offset += read.bytes.byteLength;
        }
    } else {
        const publication =
            committed.find(({ request }) => request.uri === requestedUri) ??
            committed.find(({ request }) => request.kind === 'output');
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
        assertCemStudioResultSize(metadata.byteLength);
        assertCemStudioResultSize(publication.bytes);
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
        bytes: bytes.slice(),
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


// Studio selects editable operation concerns; argument grammar stays native-owned.
const OPERATION_FORM_OPTIONS = Object.freeze({
    parse: ['format', 'from_format', 'fail_level', 'preserve_source_offsets'],
    inspect: ['show', 'format', 'from_format'],
    convert: ['from_format', 'to_format', 'to_content_type', 'to_schema', 'output_color_type', 'tabular', 'cemt_formatter', 'cemt_formatter_profile', 'cemt_colorizer', 'cemt_color_profile', 'preserve_source_offsets'],
    query: ['query', 'query_file', 'query_content_type', 'query_schema', 'output'],
    transform: ['template', 'template_expression', 'template_content_type', 'template_schema', 'template_entrypoint', 'to_content_type', 'to_schema', 'output_color_type', 'source_map_summary', 'module_asset_cache_key'],
    trace: ['format', 'from_format'],
});

// Studio selects bounded editing concerns; the native schema still owns all
// argument constraints and mutually exclusive/required source groups.
const OPERATION_FORM_GROUPS = Object.freeze({
    parse: { 'operation-options': ['format', 'fail_level', 'preserve_source_offsets'] },
    inspect: { 'operation-options': ['show', 'format', 'from_format'] },
    convert: { 'output-options': ['to_format', 'to_content_type', 'to_schema'],
        'formatter-options': ['cemt_formatter', 'cemt_formatter_profile'],
        'colorizer-options': ['cemt_colorizer', 'cemt_color_profile'],
        'output-style': ['output_color_type', 'tabular', 'preserve_source_offsets'] },
    query: { 'source-context': ['query_content_type', 'query_schema'] },
    transform: { 'source-context': ['template_content_type', 'template_schema', 'template_entrypoint'],
        'output-options': ['to_content_type', 'to_schema'] },
    trace: { 'operation-options': ['format', 'from_format'] },
});

export function describeCemStudioOperationFields(text) {
    const parsed = parseCemMlCommandText(text, { runtime: 'wasm-browser-worker' });
    return operationFields(parsed);
}

function operationFields(parsed) {
    if (parsed.commandPath.length !== 1) return [];
    const operation = parsed.commandPath[0];
    const allowed = OPERATION_FORM_OPTIONS[operation];
    if (!allowed) return [];
    const descriptor = commandSchema.commands.find(item => item.name === operation);
    return descriptor.arguments.filter(argument => allowed.includes(argument.id) && !argument.hidden &&
        (argument.action === 'set-true' || argument.action === 'set-false' ||
            (argument.action === 'set' && argument.maxValues === 1))).map(argument => ({
        id: argument.id, label: argument.long ?? argument.id,
        multiline: ['query', 'template_expression'].includes(argument.id) || String(parsed.options[argument.id] ?? '').includes('\n'),
        help: argument.help ?? '', required: argument.required,
        type: ['set-true', 'set-false'].includes(argument.action) ? 'boolean' : argument.possibleValues.length ? 'enum' : 'text',
        choices: argument.possibleValues.map(value => ({ value })),
        value: parsed.options[argument.id] === undefined ? '' : String(parsed.options[argument.id]),
        defaultValue: argument.defaultValues.join(', '),
    }));
}

export function describeCemStudioOperationGroups(text) {
    const parsed = parseCemMlCommandText(text, { runtime: 'wasm-browser-worker' });
    const fields = operationFields(parsed);
    const operation = parsed.commandPath.length === 1 ? parsed.commandPath[0] : undefined;
    const native = commandSchema.commands.find(command => command.name === operation)?.groups ?? [];
    const groups = [...native.map(group => [group.id, group.arguments]), ...Object.entries(OPERATION_FORM_GROUPS[operation] ?? {})];
    return groups.filter(([, ids]) => ids.length >= 2 && ids.length <= 3 && ids.every(id => fields.some(field => field.id === id)))
        .map(([id, ids]) => ({ id, fields: ids.map(id => fields.find(field => field.id === id)) }));
}

/** Validate the complete candidate once, allowing coupled options to change together. */
export function editCemStudioOperationFields(text, edits) {
    if (typeof text !== 'string' || !Array.isArray(edits)) throw new TypeError('An option batch requires command text and an edit list');
    if (!edits.length) return text;
    const parsed = parseCemMlCommandText(text, { runtime: 'wasm-browser-worker' });
    const fields = operationFields(parsed);
    if (edits.length > fields.length) throw new Error('Too many option edits');
    const options = { ...parsed.options };
    const seen = new Set();
    for (const edit of edits) {
        const { id, value, reset = false } = edit ?? {};
        const field = fields.find(field => field.id === id);
        if (!field) throw new Error('This option is not available in the operation form');
        if (seen.has(id)) throw new Error('An option may be edited only once per batch');
        seen.add(id);
        if (typeof value !== 'string' || typeof reset !== 'boolean') throw new TypeError('An option value must be text and reset must be boolean');
        if (reset && field.required) throw new Error('A required option cannot be reset');
        if (!reset && field.type === 'enum' && !field.choices.some(choice => choice.value === value)) throw new Error('Choose a value from the native command options');
        if (!reset && field.type === 'boolean' && !['true', 'false'].includes(value)) throw new Error('Choose true or false');
        if (reset) delete options[id];
        else options[id] = field.type === 'boolean' ? value === 'true' : value;
    }
    const serialized = serializeCemMlCommandText({ ...parsed, options });
    // Parsing applies native defaults, conflicts and value constraints before delivery.
    return serializeCemMlCommandText(parseCemMlCommandText(serialized, { runtime: 'wasm-browser-worker' }));
}

/** Edit only one scalar option, retaining positional, global and repeated values. */
export function editCemStudioOperationField(text, id, value, reset = false) {
    return editCemStudioOperationFields(text, [{ id, value, reset }]);
}
