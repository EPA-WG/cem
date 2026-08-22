import { CEM_STUDIO_REPOSITORY_ID } from './repository.js';

export const CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID = 'file-system-access';
export const CEM_STUDIO_PROJECT_MANIFEST = 'project.cem';
export const CEM_STUDIO_PROJECT_BUNDLE_CONTENT_TYPE = 'application/vnd.cem.studio-project-bundle+json';

/**
 * @typedef {object} CemStudioFileSystemProviderOptions
 * @property {{query: Function, execute: Function}} repository
 * @property {(bytes: Uint8Array, options?: {signal?: AbortSignal}) => Promise<Record<string, any>>} [decodeProjectManifest]
 * @property {(project: Record<string, any>, options?: {signal?: AbortSignal}) => Promise<Uint8Array | ArrayBuffer | ArrayBufferView | string>} [encodeProjectManifest]
 * @property {(options?: Record<string, unknown>) => Promise<FileSystemFileHandle[]>} [showOpenFilePicker]
 * @property {(options?: Record<string, unknown>) => Promise<FileSystemDirectoryHandle>} [showDirectoryPicker]
 * @property {Crypto} [crypto]
 */

export class CemStudioFileSystemError extends Error {
    /** @param {string} code @param {string} message @param {Record<string, unknown>} [details] @param {unknown} [cause] */
    constructor(code, message, details = {}, cause) {
        super(message, cause === undefined ? undefined : { cause });
        this.name = 'CemStudioFileSystemError';
        this.code = code;
        this.details = Object.freeze(structuredClone(details));
    }
}

/** @param {CemStudioFileSystemProviderOptions} options */
export function createCemStudioFileSystemProvider(options) {
    if (!options?.repository || typeof options.repository.query !== 'function' || typeof options.repository.execute !== 'function') {
        throw new TypeError('CEM Studio File System Access provider requires a repository port');
    }
    const repository = options.repository;
    const crypto = options.crypto ?? globalThis.crypto;
    const showOpenFilePicker = options.showOpenFilePicker === undefined
        ? globalThis.showOpenFilePicker?.bind(globalThis)
        : options.showOpenFilePicker;
    const showDirectoryPicker = options.showDirectoryPicker === undefined
        ? globalThis.showDirectoryPicker?.bind(globalThis)
        : options.showDirectoryPicker;
    let requestRevision = 0;

    const request = (operation, parameters = {}) => ({
        protocolVersion: 1,
        repository: CEM_STUDIO_REPOSITORY_ID,
        operation,
        requestRevision: ++requestRevision,
        parameters,
    });
    const query = async (operation, parameters, signal) =>
        (await repository.query(request(operation, parameters), signal)).value;
    const execute = async (operation, parameters, signal) =>
        (await repository.execute(request(operation, parameters), signal)).value;

    const capabilities = () => Object.freeze({
        provider: CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID,
        available: typeof showOpenFilePicker === 'function' || typeof showDirectoryPicker === 'function',
        openFile: typeof showOpenFilePicker === 'function',
        directory: typeof showDirectoryPicker === 'function',
        retainedHandles: true,
        explicitPermission: true,
        indexedDbFallback: true,
        importExportFallback: true,
    });

    /** @param {{projectId: string, scope?: 'project' | 'resource', resourceId?: string, signal?: AbortSignal}} input */
    async function status(input) {
        const scope = input.scope ?? 'project';
        const binding = await query('get-provider-binding', {
            projectId: input.projectId,
            scope,
            ...(input.resourceId ? { resourceId: input.resourceId } : {}),
        }, input.signal);
        if (!binding) {
            return providerStatus(capabilities().available ? 'unbound' : 'unsupported', { scope });
        }
        const permission = await queryPermission(binding.handle, scope === 'project' ? 'readwrite' : 'readwrite');
        return providerStatus(permission === 'granted' ? 'ready' : `${permission}-permission`, {
            scope,
            permission,
            bindingRevision: binding.revision,
            name: binding.name,
        });
    }

    /**
     * Reconnect a retained handle. Permission is requested only when the caller
     * names requestPermission=true from an explicit user action.
     * @param {{projectId: string, scope?: 'project' | 'resource', resourceId?: string, mode?: 'read' | 'readwrite', requestPermission?: boolean, signal?: AbortSignal}} input
     */
    async function reconnect(input) {
        const scope = input.scope ?? 'project';
        const binding = await requiredBinding(query, input.projectId, scope, input.resourceId, input.signal);
        const mode = input.mode ?? 'readwrite';
        const permission = await ensurePermission(binding.handle, mode, input.requestPermission === true);
        if (permission !== binding.permission) {
            const project = await requiredProject(query, input.projectId, input.signal);
            await putBinding(execute, {
                ...binding,
                permission,
            }, project.revision, binding.revision, input.signal);
        }
        if (permission !== 'granted') {
            return providerStatus(`${permission}-permission`, {
                scope,
                permission,
                bindingRevision: binding.revision,
                name: binding.name,
            });
        }
        return providerStatus('ready', {
            scope,
            permission,
            bindingRevision: binding.revision + (permission === binding.permission ? 0 : 1),
            name: binding.name,
        });
    }

    /** @param {{projectId: string, resourceId: string, handle?: FileSystemFileHandle, signal?: AbortSignal}} input */
    async function openResource(input) {
        assertNotAborted(input.signal);
        const handle = input.handle ?? await pickFile(showOpenFilePicker);
        assertHandle(handle, 'file');
        const permission = await ensurePermission(handle, 'read', true);
        requirePermission(permission, 'read');
        const external = await readFileSnapshot(handle, crypto, input.signal);
        const project = await requiredProject(query, input.projectId, input.signal);
        const resource = requiredResource(project, input.resourceId);
        let projectRevision = project.revision;
        let resourceRevision = resource.revision;
        let sha256 = resource.sha256;
        if (external.sha256 !== resource.sha256) {
            const saved = await execute('save-resource', {
                projectId: input.projectId,
                resourceId: input.resourceId,
                expectedProjectRevision: project.revision,
                expectedResourceRevision: resource.revision,
                content: external.bytes,
            }, input.signal);
            projectRevision = saved.projectRevision;
            resourceRevision = saved.resourceRevision;
            sha256 = saved.sha256;
        }
        const existingBinding = await query('get-provider-binding', {
            projectId: input.projectId,
            scope: 'resource',
            resourceId: input.resourceId,
        }, input.signal);
        const binding = await putBinding(execute, {
            provider: CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID,
            scope: 'resource',
            projectId: input.projectId,
            resourceId: input.resourceId,
            handle,
            name: handle.name,
            permission,
            base: resourceBase(projectRevision, resourceRevision, sha256, external),
        }, projectRevision, existingBinding?.revision, input.signal);
        return Object.freeze({
            status: 'opened',
            projectId: input.projectId,
            resourceId: input.resourceId,
            projectRevision,
            resourceRevision,
            sha256,
            bindingRevision: binding.revision,
            bytes: exactBytes(external.bytes),
        });
    }

    /** @param {{projectId: string, resourceId: string, requestPermission?: boolean, signal?: AbortSignal}} input */
    async function pullResource(input) {
        const binding = await requiredBinding(query, input.projectId, 'resource', input.resourceId, input.signal);
        const permission = await ensurePermission(binding.handle, 'read', input.requestPermission === true);
        requirePermission(permission, 'read');
        const [bundle, external] = await Promise.all([
            requiredBundle(query, input.projectId, input.signal),
            readFileSnapshot(binding.handle, crypto, input.signal),
        ]);
        const resource = requiredResource(bundle.project, input.resourceId);
        const externalChanged = !sameExternalSnapshot(external, binding.base.external);
        const localChanged = resource.revision !== binding.base.resourceRevision
            || resource.sha256 !== binding.base.localSha256;
        if (!externalChanged) {
            return Object.freeze({ status: localChanged ? 'local-ahead' : 'unchanged', bindingRevision: binding.revision });
        }
        if (localChanged) {
            throw externalConflict(binding, external, resource);
        }
        const saved = await execute('save-resource', {
            projectId: input.projectId,
            resourceId: input.resourceId,
            expectedProjectRevision: bundle.project.revision,
            expectedResourceRevision: resource.revision,
            content: external.bytes,
        }, input.signal);
        const next = await putBinding(execute, {
            ...binding,
            permission,
            base: resourceBase(saved.projectRevision, saved.resourceRevision, saved.sha256, external),
        }, saved.projectRevision, binding.revision, input.signal);
        return Object.freeze({
            status: 'pulled',
            projectRevision: saved.projectRevision,
            resourceRevision: saved.resourceRevision,
            sha256: saved.sha256,
            bindingRevision: next.revision,
        });
    }

    /** @param {{projectId: string, resourceId: string, requestPermission?: boolean, signal?: AbortSignal}} input */
    async function writeResource(input) {
        const binding = await requiredBinding(query, input.projectId, 'resource', input.resourceId, input.signal);
        const permission = await ensurePermission(binding.handle, 'readwrite', input.requestPermission === true);
        requirePermission(permission, 'readwrite');
        const [bundle, external] = await Promise.all([
            requiredBundle(query, input.projectId, input.signal),
            readFileSnapshot(binding.handle, crypto, input.signal),
        ]);
        const resource = requiredResource(bundle.project, input.resourceId);
        if (!sameExternalSnapshot(external, binding.base.external)) {
            throw externalConflict(binding, external, resource);
        }
        const bytes = exactBytes(bundle.contents[input.resourceId]);
        await writeFileSafely(binding.handle, bytes, input.signal);
        const written = await readFileSnapshot(binding.handle, crypto, input.signal);
        if (written.sha256 !== resource.sha256) {
            throw new CemStudioFileSystemError(
                'cem.studio.file_system.write_verification_failed',
                `file \`${binding.name}\` did not retain the exact repository bytes`,
                { expectedSha256: resource.sha256, currentSha256: written.sha256 },
            );
        }
        const next = await putBinding(execute, {
            ...binding,
            permission,
            base: resourceBase(bundle.project.revision, resource.revision, resource.sha256, written),
        }, bundle.project.revision, binding.revision, input.signal);
        return Object.freeze({
            status: 'written',
            projectRevision: bundle.project.revision,
            resourceRevision: resource.revision,
            sha256: resource.sha256,
            bindingRevision: next.revision,
        });
    }

    /** @param {{projectId: string, handle?: FileSystemDirectoryHandle, signal?: AbortSignal}} input */
    async function bindProjectDirectory(input) {
        assertNotAborted(input.signal);
        const handle = input.handle ?? await pickDirectory(showDirectoryPicker);
        assertHandle(handle, 'directory');
        const permission = await ensurePermission(handle, 'read', true);
        requirePermission(permission, 'read');
        const bundle = await requiredBundle(query, input.projectId, input.signal);
        const external = await scanProjectDirectory(handle, bundle.project, crypto, input.signal);
        assertDirectoryCompatible(bundle.project, external);
        const existingBinding = await query('get-provider-binding', {
            projectId: input.projectId,
            scope: 'project',
        }, input.signal);
        const binding = await putBinding(execute, {
            provider: CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID,
            scope: 'project',
            projectId: input.projectId,
            handle,
            name: handle.name,
            permission,
            base: projectBase(bundle.project, external),
        }, bundle.project.revision, existingBinding?.revision, input.signal);
        return Object.freeze({
            status: 'bound',
            projectId: input.projectId,
            projectRevision: bundle.project.revision,
            bindingRevision: binding.revision,
            missingFiles: external.resources.filter(({ external: item }) => item.state === 'missing').length,
        });
    }

    /** @param {{handle?: FileSystemDirectoryHandle, mode?: 'create' | 'replace', expectedRevision?: number, signal?: AbortSignal}} input */
    async function openProjectDirectory(input = {}) {
        assertNotAborted(input.signal);
        if (typeof options.decodeProjectManifest !== 'function') throw codecUnavailable('decode');
        const handle = input.handle ?? await pickDirectory(showDirectoryPicker);
        assertHandle(handle, 'directory');
        const permission = await ensurePermission(handle, 'read', true);
        requirePermission(permission, 'read');
        const manifestHandle = await fileHandleAtPath(handle, CEM_STUDIO_PROJECT_MANIFEST, false);
        const manifest = await readFileSnapshot(manifestHandle, crypto, input.signal);
        let project;
        try {
            project = await options.decodeProjectManifest(exactBytes(manifest.bytes), { signal: input.signal });
        } catch (error) {
            throw new CemStudioFileSystemError(
                'cem.studio.file_system.manifest_invalid',
                'CEM-ML rejected the selected project.cem manifest',
                {},
                error,
            );
        }
        assertPortableProject(project);
        const contents = {};
        const resources = [];
        for (const resource of project.resources) {
            const resourceHandle = await fileHandleAtPath(handle, resource.path, false);
            const snapshot = await readFileSnapshot(resourceHandle, crypto, input.signal);
            if (snapshot.sha256 !== resource.sha256) {
                throw new CemStudioFileSystemError(
                    'cem.studio.file_system.import_hash_mismatch',
                    `resource \`${resource.id}\` does not match project.cem`,
                    { resourceId: resource.id, expectedSha256: resource.sha256, currentSha256: snapshot.sha256 },
                );
            }
            contents[resource.id] = exactBytes(snapshot.bytes);
            resources.push({ resourceId: resource.id, path: resource.path, external: externalSnapshot(snapshot) });
        }
        const existingBinding = await query('get-provider-binding', {
            projectId: project.id,
            scope: 'project',
        }, input.signal);
        const imported = await execute('import-project', {
            bundle: { project, contents },
            mode: input.mode ?? 'create',
            ...(input.expectedRevision === undefined ? {} : { expectedRevision: input.expectedRevision }),
        }, input.signal);
        const binding = await putBinding(execute, {
            provider: CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID,
            scope: 'project',
            projectId: project.id,
            handle,
            name: handle.name,
            permission,
            base: projectBase(project, {
                manifest: externalSnapshot(manifest),
                resources,
            }),
        }, imported.revision, existingBinding?.revision, input.signal);
        return Object.freeze({
            status: 'imported',
            projectId: project.id,
            projectRevision: imported.revision,
            bindingRevision: binding.revision,
        });
    }

    /** @param {{projectId: string, requestPermission?: boolean, signal?: AbortSignal}} input */
    async function writeProjectDirectory(input) {
        if (typeof options.encodeProjectManifest !== 'function') throw codecUnavailable('encode');
        const binding = await requiredBinding(query, input.projectId, 'project', undefined, input.signal);
        const permission = await ensurePermission(binding.handle, 'readwrite', input.requestPermission === true);
        requirePermission(permission, 'readwrite');
        const bundle = await requiredBundle(query, input.projectId, input.signal);
        const external = await scanProjectDirectory(binding.handle, bundle.project, crypto, input.signal);
        assertExternalBase(binding, external, bundle.project);
        const manifestBytes = exactBytes(await options.encodeProjectManifest(bundle.project, { signal: input.signal }));
        const writes = [{ path: CEM_STUDIO_PROJECT_MANIFEST, bytes: manifestBytes }];
        for (const resource of bundle.project.resources) {
            writes.push({ path: resource.path, bytes: exactBytes(bundle.contents[resource.id]) });
        }
        await writeDirectoryFiles(binding.handle, writes, input.signal);
        const written = await scanProjectDirectory(binding.handle, bundle.project, crypto, input.signal);
        const expectedManifestSha256 = await sha256Hex(crypto, manifestBytes);
        if (written.manifest.sha256 !== expectedManifestSha256) {
            throw new CemStudioFileSystemError(
                'cem.studio.file_system.write_verification_failed',
                'project.cem did not retain the exact CEM-ML serialized bytes',
                { expectedSha256: expectedManifestSha256, currentSha256: written.manifest.sha256 },
            );
        }
        for (const resource of bundle.project.resources) {
            const item = written.resources.find(({ resourceId }) => resourceId === resource.id);
            if (item?.external.sha256 !== resource.sha256) {
                throw new CemStudioFileSystemError(
                    'cem.studio.file_system.write_verification_failed',
                    `resource \`${resource.id}\` did not retain the exact repository bytes`,
                    { resourceId: resource.id, expectedSha256: resource.sha256, currentSha256: item?.external.sha256 },
                );
            }
        }
        const next = await putBinding(execute, {
            ...binding,
            permission,
            base: projectBase(bundle.project, written),
        }, bundle.project.revision, binding.revision, input.signal);
        return Object.freeze({
            status: 'written',
            projectId: input.projectId,
            projectRevision: bundle.project.revision,
            fileCount: writes.length,
            bindingRevision: next.revision,
        });
    }

    /** Validated IndexedDB export remains available without File System Access. */
    async function exportFallback(input) {
        const bundle = await requiredBundle(query, input.projectId, input.signal);
        return Object.freeze({
            status: 'exported',
            bundle,
            archive: Object.freeze({
                filename: `${bundle.project.id}.cem-studio.json`,
                contentType: CEM_STUDIO_PROJECT_BUNDLE_CONTENT_TYPE,
                bytes: serializeCemStudioProjectBundle(bundle),
            }),
        });
    }

    /** Validated upload/import remains available without File System Access. */
    async function importFallback(input) {
        const bundle = input.bundle ?? parseCemStudioProjectBundle(input.archive);
        const project = await execute('import-project', {
            bundle,
            mode: input.mode ?? 'create',
            ...(input.expectedRevision === undefined ? {} : { expectedRevision: input.expectedRevision }),
        }, input.signal);
        return Object.freeze({ status: 'imported', project });
    }

    return Object.freeze({
        capabilities,
        status,
        reconnect,
        openResource,
        pullResource,
        writeResource,
        bindProjectDirectory,
        openProjectDirectory,
        writeProjectDirectory,
        exportFallback,
        importFallback,
    });
}

/** Encode a deterministic lossless archive envelope; project semantics remain in the embedded portable manifest. */
export function serializeCemStudioProjectBundle(bundle) {
    if (!bundle?.project || !bundle?.contents || typeof bundle.contents !== 'object') {
        throw new TypeError('CEM Studio bundle export requires project metadata and contents');
    }
    const contents = Object.keys(bundle.contents)
        .sort((left, right) => left.localeCompare(right))
        .map((resourceId) => ({ resourceId, base64: bytesToBase64(exactBytes(bundle.contents[resourceId])) }));
    return new TextEncoder().encode(`${JSON.stringify({
        archiveSchemaVersion: 1,
        project: bundle.project,
        contents,
    }, null, 2)}\n`);
}

/** Decode the archive envelope. Repository import still performs native CEM-ML validation and hash checks. */
export function parseCemStudioProjectBundle(value) {
    const bytes = exactBytes(value);
    let archive;
    try {
        archive = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    } catch (error) {
        throw new CemStudioFileSystemError(
            'cem.studio.file_system.bundle_invalid',
            'the selected CEM Studio project archive is invalid',
            {},
            error,
        );
    }
    if (archive?.archiveSchemaVersion !== 1 || !archive.project || !Array.isArray(archive.contents)) {
        throw new CemStudioFileSystemError(
            'cem.studio.file_system.bundle_invalid',
            'the selected CEM Studio project archive has an unsupported envelope',
        );
    }
    const contents = {};
    for (const entry of archive.contents) {
        if (!entry || typeof entry.resourceId !== 'string' || typeof entry.base64 !== 'string' || entry.resourceId in contents) {
            throw new CemStudioFileSystemError(
                'cem.studio.file_system.bundle_invalid',
                'the selected CEM Studio project archive has invalid resource contents',
            );
        }
        contents[entry.resourceId] = base64ToBytes(entry.base64);
    }
    return { project: archive.project, contents };
}

function providerStatus(state, details = {}) {
    return Object.freeze({ provider: CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID, state, ...details });
}

async function requiredProject(query, projectId, signal) {
    const project = await query('get-project', { projectId }, signal);
    if (!project) throw new CemStudioFileSystemError('cem.studio.file_system.project_not_found', `project \`${projectId}\` was not found`);
    return project;
}

async function requiredBundle(query, projectId, signal) {
    const bundle = await query('export-project', { projectId }, signal);
    if (!bundle) throw new CemStudioFileSystemError('cem.studio.file_system.project_not_found', `project \`${projectId}\` was not found`);
    return bundle;
}

async function requiredBinding(query, projectId, scope, resourceId, signal) {
    const binding = await query('get-provider-binding', { projectId, scope, ...(resourceId ? { resourceId } : {}) }, signal);
    if (!binding) {
        throw new CemStudioFileSystemError(
            'cem.studio.file_system.binding_not_found',
            `no retained ${scope} handle exists for project \`${projectId}\``,
            { projectId, scope, resourceId },
        );
    }
    return binding;
}

async function putBinding(execute, binding, expectedProjectRevision, expectedBindingRevision, signal) {
    return execute('put-provider-binding', {
        binding,
        expectedProjectRevision,
        ...(expectedBindingRevision === undefined ? {} : { expectedBindingRevision }),
    }, signal);
}

function requiredResource(project, resourceId) {
    const resource = project.resources.find(({ id }) => id === resourceId);
    if (!resource) throw new CemStudioFileSystemError('cem.studio.file_system.resource_not_found', `resource \`${resourceId}\` was not found`);
    if (resource.sourceKind !== 'project-file') {
        throw new CemStudioFileSystemError('cem.studio.file_system.resource_not_writable', `resource \`${resourceId}\` is not a project file`);
    }
    return resource;
}

function assertPortableProject(project) {
    if (!project || typeof project !== 'object' || typeof project.id !== 'string' || !Array.isArray(project.resources)) {
        throw new CemStudioFileSystemError('cem.studio.file_system.manifest_invalid', 'project.cem did not normalize to a Studio project');
    }
    if (project.rootUri !== `studio://${project.id}/`) {
        throw new CemStudioFileSystemError('cem.studio.file_system.manifest_invalid', 'project.cem must retain its studio:// logical root identity');
    }
}

function resourceBase(projectRevision, resourceRevision, localSha256, external) {
    return Object.freeze({
        projectRevision,
        resourceRevision,
        localSha256,
        external: externalSnapshot(external),
    });
}

function projectBase(project, external) {
    return Object.freeze({
        projectRevision: project.revision,
        localResources: project.resources
            .map(({ id, path, revision, sha256 }) => ({ resourceId: id, path, revision, sha256 })),
        manifest: structuredClone(external.manifest),
        resources: structuredClone(external.resources),
    });
}

function externalSnapshot(snapshot) {
    if (!snapshot || snapshot.state === 'missing') return Object.freeze({ state: 'missing' });
    return Object.freeze({
        state: 'file',
        sha256: snapshot.sha256,
        byteLength: snapshot.byteLength,
        lastModified: snapshot.lastModified,
    });
}

function sameExternalSnapshot(left, right) {
    const current = externalSnapshot(left);
    if (current.state !== right?.state) return false;
    return current.state === 'missing' || current.sha256 === right.sha256;
}

function externalConflict(binding, external, resource) {
    return new CemStudioFileSystemError(
        'cem.studio.file_system.external_conflict',
        `external file \`${binding.name}\` changed after the retained base snapshot`,
        {
            projectId: binding.projectId,
            resourceId: binding.resourceId,
            baseSha256: binding.base.external?.sha256,
            externalSha256: external.sha256,
            localSha256: resource.sha256,
            recommendedAction: 'review-and-rebind',
        },
    );
}

function assertDirectoryCompatible(project, external) {
    for (const resource of project.resources) {
        const current = external.resources.find(({ resourceId }) => resourceId === resource.id)?.external;
        if (current?.state === 'file' && current.sha256 !== resource.sha256) {
            throw new CemStudioFileSystemError(
                'cem.studio.file_system.external_conflict',
                `directory resource \`${resource.path}\` differs from the IndexedDB working copy`,
                { resourceId: resource.id, localSha256: resource.sha256, externalSha256: current.sha256, recommendedAction: 'open-project-directory' },
            );
        }
    }
}

function assertExternalBase(binding, external, project) {
    if (!sameExternalSnapshot(external.manifest, binding.base.manifest)) {
        throw new CemStudioFileSystemError(
            'cem.studio.file_system.external_conflict',
            'project.cem changed after the retained base snapshot',
            { projectId: project.id, recommendedAction: 'review-and-reopen' },
        );
    }
    for (const current of external.resources) {
        const base = binding.base.resources?.find(({ resourceId }) => resourceId === current.resourceId)?.external;
        if (!sameExternalSnapshot(current.external, base ?? { state: 'missing' })) {
            throw new CemStudioFileSystemError(
                'cem.studio.file_system.external_conflict',
                `directory resource \`${current.path}\` changed after the retained base snapshot`,
                { projectId: project.id, resourceId: current.resourceId, recommendedAction: 'review-and-reopen' },
            );
        }
    }
}

async function scanProjectDirectory(handle, project, crypto, signal) {
    const manifest = await readOptionalPath(handle, CEM_STUDIO_PROJECT_MANIFEST, crypto, signal);
    const resources = [];
    for (const resource of project.resources) {
        resources.push({
            resourceId: resource.id,
            path: resource.path,
            external: externalSnapshot(await readOptionalPath(handle, resource.path, crypto, signal)),
        });
    }
    return Object.freeze({ manifest: externalSnapshot(manifest), resources: Object.freeze(resources) });
}

async function readOptionalPath(directory, path, crypto, signal) {
    try {
        const handle = await fileHandleAtPath(directory, path, false);
        return readFileSnapshot(handle, crypto, signal);
    } catch (error) {
        if (error?.name === 'NotFoundError') return { state: 'missing' };
        throw normalizeFileSystemError(error, 'read');
    }
}

async function readFileSnapshot(handle, crypto, signal) {
    assertNotAborted(signal);
    let file;
    try {
        file = await handle.getFile();
    } catch (error) {
        throw normalizeFileSystemError(error, 'read');
    }
    const bytes = new Uint8Array(await file.arrayBuffer());
    assertNotAborted(signal);
    return Object.freeze({
        state: 'file',
        bytes,
        byteLength: bytes.byteLength,
        lastModified: Number.isFinite(file.lastModified) ? file.lastModified : 0,
        sha256: await sha256Hex(crypto, bytes),
    });
}

async function writeDirectoryFiles(directory, files, signal) {
    const staged = [];
    try {
        for (const file of files) {
            assertNotAborted(signal);
            const handle = await fileHandleAtPath(directory, file.path, true);
            const writable = await handle.createWritable({ keepExistingData: false });
            staged.push(writable);
            await writable.write(file.bytes);
        }
        assertNotAborted(signal);
        await Promise.all(staged.map((writable) => writable.close()));
    } catch (error) {
        await Promise.allSettled(staged.map((writable) => writable.abort?.()));
        throw normalizeFileSystemError(error, 'write');
    }
}

async function writeFileSafely(handle, bytes, signal) {
    assertNotAborted(signal);
    let writable;
    try {
        writable = await handle.createWritable({ keepExistingData: false });
        await writable.write(bytes);
        assertNotAborted(signal);
        await writable.close();
    } catch (error) {
        await writable?.abort?.().catch(() => undefined);
        throw normalizeFileSystemError(error, 'write');
    }
}

async function fileHandleAtPath(root, path, create) {
    const parts = normalizedPath(path);
    let directory = root;
    for (const part of parts.slice(0, -1)) directory = await directory.getDirectoryHandle(part, { create });
    return directory.getFileHandle(parts.at(-1), { create });
}

function normalizedPath(path) {
    if (typeof path !== 'string' || !path || path.includes('\\') || path.startsWith('/')) throw pathInvalid(path);
    const parts = path.split('/');
    if (parts.some((part) => !part || part === '.' || part === '..' || part === '.cem-studio')) throw pathInvalid(path);
    return parts;
}

function pathInvalid(path) {
    return new CemStudioFileSystemError('cem.studio.file_system.path_invalid', `provider path is not project-contained: ${String(path)}`);
}

async function queryPermission(handle, mode) {
    if (typeof handle.queryPermission !== 'function') return 'granted';
    try {
        return await handle.queryPermission({ mode });
    } catch (error) {
        throw normalizeFileSystemError(error, 'permission');
    }
}

async function ensurePermission(handle, mode, request) {
    let permission = await queryPermission(handle, mode);
    if (permission === 'prompt' && request) {
        if (typeof handle.requestPermission !== 'function') return 'denied';
        try {
            permission = await handle.requestPermission({ mode });
        } catch (error) {
            throw normalizeFileSystemError(error, 'permission');
        }
    }
    return permission;
}

function requirePermission(permission, mode) {
    if (permission === 'granted') return;
    throw new CemStudioFileSystemError(
        'cem.studio.file_system.permission_required',
        `${mode} permission must be granted from an explicit user action`,
        { permission, mode, fallback: 'indexeddb-import-export' },
    );
}

async function pickFile(picker) {
    if (typeof picker !== 'function') throw unsupported('file picker');
    try {
        const handles = await picker({ multiple: false, id: 'cem-studio-resource' });
        if (!Array.isArray(handles) || handles.length !== 1) throw new TypeError('file picker did not return one handle');
        return handles[0];
    } catch (error) {
        throw normalizeFileSystemError(error, 'picker');
    }
}

async function pickDirectory(picker) {
    if (typeof picker !== 'function') throw unsupported('directory picker');
    try {
        return await picker({ id: 'cem-studio-project', mode: 'read' });
    } catch (error) {
        throw normalizeFileSystemError(error, 'picker');
    }
}

function assertHandle(handle, kind) {
    if (!handle || handle.kind !== kind) throw new TypeError(`CEM Studio expected a ${kind} handle`);
}

function unsupported(capability) {
    return new CemStudioFileSystemError(
        'cem.studio.file_system.unsupported',
        `File System Access ${capability} is unavailable; use IndexedDB import/export`,
        { fallback: 'indexeddb-import-export' },
    );
}

function codecUnavailable(direction) {
    return new CemStudioFileSystemError(
        'cem.studio.file_system.codec_unavailable',
        `CEM-ML project manifest ${direction} support is unavailable; use validated import/export`,
        { fallback: 'indexeddb-import-export' },
    );
}

function normalizeFileSystemError(error, operation) {
    if (error instanceof CemStudioFileSystemError) return error;
    const name = error instanceof DOMException ? error.name : error?.name;
    if (name === 'AbortError') {
        return new CemStudioFileSystemError('cem.studio.file_system.cancelled', 'the file selection was cancelled', { operation }, error);
    }
    if (name === 'NotAllowedError' || name === 'SecurityError') {
        return new CemStudioFileSystemError(
            'cem.studio.file_system.permission_denied',
            'the browser denied File System Access; the IndexedDB project remains available',
            { operation, fallback: 'indexeddb-import-export' },
            error,
        );
    }
    return new CemStudioFileSystemError(
        `cem.studio.file_system.${operation}_failed`,
        error instanceof Error ? error.message : `File System Access ${operation} failed`,
        { operation, fallback: 'indexeddb-import-export' },
        error,
    );
}

function exactBytes(value) {
    if (typeof value === 'string') return new TextEncoder().encode(value);
    if (value instanceof Uint8Array) return new Uint8Array(value);
    if (value instanceof ArrayBuffer) return new Uint8Array(value.slice(0));
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer.slice(value.byteOffset, value.byteOffset + value.byteLength));
    throw new TypeError('CEM Studio provider expected exact resource bytes');
}

function bytesToBase64(bytes) {
    let binary = '';
    const chunkSize = 0x8000;
    for (let offset = 0; offset < bytes.byteLength; offset += chunkSize) {
        binary += String.fromCharCode(...bytes.subarray(offset, offset + chunkSize));
    }
    return btoa(binary);
}

function base64ToBytes(value) {
    let binary;
    try {
        binary = atob(value);
    } catch (error) {
        throw new CemStudioFileSystemError(
            'cem.studio.file_system.bundle_invalid',
            'the selected CEM Studio project archive contains invalid base64 data',
            {},
            error,
        );
    }
    return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

async function sha256Hex(crypto, bytes) {
    if (!crypto?.subtle) throw new CemStudioFileSystemError('cem.studio.file_system.crypto_unavailable', 'SHA-256 is unavailable');
    const digest = await crypto.subtle.digest('SHA-256', bytes);
    return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function assertNotAborted(signal) {
    if (signal?.aborted) throw signal.reason ?? new DOMException('operation aborted', 'AbortError');
}
