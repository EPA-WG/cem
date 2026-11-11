import { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';
import { bindCemGraphqlActions, readCemGraphqlQuery, type CemGraphqlActionRoute, type CemGraphqlGuard, type CemGraphqlQueryOptions } from './graphql-actions.js';
import { validateLivePreviewDom } from './live-preview-policy.js';
import { readByteStream, streamCemHttpResponseBody } from './http-resource-transport.js';
import type { CemLivePreviewResource } from './live-preview.js';
import { CemGraphqlPreviewResponseOwner, type CemGraphqlPreviewGuard, type CemGraphqlPreviewRecovery } from './graphql-preview-recovery.js';
import type { CemCheckpointPublicationCoordinator as Publication } from './checkpoint-publication.js';
import type { CemNativeRecoveryScope } from './internal/runtime-support/native-generation.js';

export interface CemGraphqlPreviewGrant {
    readonly mutationIdempotency: 'server-keyed';
    readonly id: string;
    readonly endpoint: string;
    readonly factoryUrl: string;
    readonly wasmUrl: string;
    readonly wasmSha256: string;
    readonly sourceSha256: string;
    readonly resources: readonly { path: string; sha256: string }[];
    readonly initialRead: { operation: string; variables: Readonly<Record<string, unknown>> };
    readonly actions: Readonly<Record<string, CemGraphqlActionRoute>>;
    readonly bindings: Readonly<Record<string, string>>;
}

export interface CemPreviewApplicationRequest {
    readonly id: string;
    readonly source: string;
    readonly sourceUri: string;
    readonly resources: readonly CemLivePreviewResource[];
}

export interface CemPreviewApplication {
    readonly baseUrl: string;
    readonly grant: CemGraphqlPreviewGrant;
    readonly guard: CemGraphqlPreviewGuard;
    connectRecovery(scope: CemNativeRecoveryScope, journal: Publication, rendering: Publication, current: () => boolean): CemGraphqlPreviewRecovery;
    dispose(): void;
}
export type CemPreviewApplicationHost = (request: CemPreviewApplicationRequest, signal: AbortSignal) => Promise<CemPreviewApplication>;
const encoder = new TextEncoder();
const digest = async (bytes: Uint8Array<ArrayBuffer>) => [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))]
    .map(value => value.toString(16).padStart(2, '0')).join('');

/** Deployment-owned application grants. Authored resources can select a grant
 * but cannot supply code URLs, service URLs, hashes or action authority. */
export function createCemGraphqlPreviewHost(baseUrl: string, grants: readonly CemGraphqlPreviewGrant[]): CemPreviewApplicationHost {
    const origin = new URL(baseUrl).origin;
    const registered = new Map<string, CemGraphqlPreviewGrant>();
    for (const input of grants) {
        const grant = structuredClone(input);
        if (!grant.id || registered.has(grant.id)) throw Error('cem.preview.application.identity');
        if (grant.mutationIdempotency !== 'server-keyed') throw Error('cem.preview.application.idempotency_required');
        for (const path of [grant.endpoint, grant.factoryUrl, grant.wasmUrl]) {
            const url = new URL(path, baseUrl);
            if (!['http:', 'https:'].includes(url.protocol) || url.origin !== origin || url.username || url.password || url.hash)
                throw Error('cem.preview.application.origin');
        }
        registered.set(grant.id, grant);
    }
    return async (request, signal) => {
        signal.throwIfAborted();
        const grant = registered.get(request.id);
        if (!grant) throw Error('cem.preview.application.unauthorized');
        if (await digest(encoder.encode(request.source)) !== grant.sourceSha256) throw Error('cem.preview.application.source_changed');
        for (const pin of grant.resources) {
            const uri = new URL(pin.path, request.sourceUri).href;
            const resource = request.resources.find(resource => resource.uri === uri);
            if (!resource || await digest(encoder.encode(resource.text)) !== pin.sha256) throw Error('cem.preview.application.resource_changed');
        }
        const response = await fetch(new URL(grant.wasmUrl, baseUrl), { signal, redirect: 'error', credentials: 'same-origin' });
        if (!response.ok) throw Error('cem.preview.application.runtime_unavailable');
        const bytes = await readByteStream(streamCemHttpResponseBody(response), 16 * 1024 * 1024, signal);
        if (await digest(new Uint8Array(bytes)) !== grant.wasmSha256)
            throw Error('cem.preview.application.runtime_changed');
        signal.throwIfAborted();
        const factoryUrl = new URL(grant.factoryUrl, baseUrl).href;
        const factory = await import(/* @vite-ignore */ factoryUrl);
        const module = await WebAssembly.compile(new Uint8Array(bytes));
        const create = async () => {
            const guard: CemGraphqlPreviewGuard & { initialize(module: WebAssembly.Module): Promise<void> } = factory.createCemComponentWasmBindings();
            try { await guard.initialize(module); return guard; }
            catch (error) { try { guard.release(); } catch { /* An initializer can already be retired. */ } throw error; }
        };
        const guard = await create();
        try {
            if (!['applicationIdentity', 'retainResponse', 'claimMutation', 'captureResponse', 'responseRevision', 'sealCheckpoint',
                'confirmResponseRetirement', 'restoreResponse', 'disposeResponse', 'memory', 'inventory']
                .every(method => typeof guard[method as keyof CemGraphqlPreviewGuard] === 'function'))
                throw Error('cem.preview.application.runtime_contract');
            signal.throwIfAborted();
            const owner = new CemGraphqlPreviewResponseOwner(guard, create);
            return { baseUrl, grant: structuredClone(grant), guard: owner.guard,
                connectRecovery: (...args) => owner.connect(...args), dispose: () => owner.dispose() };
        } catch (error) {
            try { guard.release(); } catch { /* Initialization can already retire a failed heap. */ }
            throw error;
        }
    };
}

/** Shared DOM/action adapter. Original source/operation ownership stays outside
 * the rendering heap and joins its physical group through the response route. */
export async function mountCemGraphqlPreview(root: HTMLElement, application: CemPreviewApplication, signal: AbortSignal,
    reportError: (message: string) => void, recovery?: { scope: CemNativeRecoveryScope; rendering: Publication; current: () => boolean;
        connected: (owner: CemGraphqlPreviewRecovery) => void }): Promise<() => void> {
    const { baseUrl, grant, guard } = application;
    // Original bytes and source maps remain in the selected heap. Rendering an
    // invalid response leaves that source owner untouched.
    const retainedGuard: CemGraphqlQueryOptions['guard'] & CemGraphqlGuard = {
        applicationDocument: () => guard.applicationDocument(),
        validateVariables: (operation, bytes, uri) => guard.validateVariables(operation, bytes, uri),
        validateQueryVariables: (operation, bytes, uri) => guard.validateQueryVariables(operation, bytes, uri),
        renderResponse: (operation, bytes, uri) => guard.retainResponse(operation, bytes, uri),
    };
    const seal = (candidate: string, receipt: string) => guard.sealCheckpoint(candidate, receipt);
    const journal = recovery?.rendering.fork(seal) ?? new CemCheckpointPublicationCoordinator({
        runtimeId: crypto.randomUUID(), generation: 1, policyStamp: 'preview-application',
        seal,
    });
    const owner = journal.register('application', () => `cem-application-response/1:${guard.responseRevision()}`);
    let active = true;
    const publish = (response: { html: string }) => {
        if (!active || signal.aborted || !root.isConnected) return false;
        const template = root.ownerDocument.createElement('template');
        template.innerHTML = response.html;
        validateLivePreviewDom(template.content);
        if (!active || signal.aborted || !root.isConnected) return false;
        root.replaceChildren(template.content);
        owner.published({ instanceId: 'application', declarationIdentity: guard.applicationIdentity(), dataRevision: guard.responseRevision() });
        return true;
    };
    let unbind: (() => void) | undefined;
    const dispose = () => {
        if (!active) return;
        active = false;
        unbind?.();
        owner.remove();
        journal.dispose();
    };
    signal.addEventListener('abort', dispose, { once: true });
    try {
        const response = await readCemGraphqlQuery({ baseUrl, endpoint: grant.endpoint, guard: retainedGuard, ...grant.initialRead, signal });
        if (!publish(response)) throw Error('cem.preview.disposed');
        unbind = bindCemGraphqlActions({ root, baseUrl, endpoint: grant.endpoint, guard: retainedGuard, owner,
            mutationIdempotency: grant.mutationIdempotency,
            admitMutation: operationId => { guard.claimMutation(operationId); owner.input(); },
            actions: grant.actions, bindings: grant.bindings, publish, reportError: error => reportError(String(error)) });
        if (signal.aborted) throw Error('cem.preview.disposed');
        if (recovery) recovery.connected(application.connectRecovery(recovery.scope, journal, recovery.rendering, recovery.current));
        return () => { signal.removeEventListener('abort', dispose); dispose(); };
    } catch (error) {
        signal.removeEventListener('abort', dispose); dispose(); throw error;
    }
}
