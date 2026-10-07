import { resolveAriaReferenceProfile } from './aria-reference-profile.js';
import { loadEdgeStylesheets, type EdgeStylesheetLoadOptions, type EdgeStylesheetOutput } from './edge-ssr-stylesheets.js';
import { serializeDeclarationStylesheets } from './declaration-style-markup.js';
import type {
    DataIslandSnapshot,
    SerializedPayloadElement,
} from './cem-elements.js';
import {
    advanceEdgeRenderState,
    edgeContentAddress,
    projectTemplate,
    payloadStylesheetSources,
    readEdgeRenderStateContents,
    renderPlanIdentity,
    scopeRenderPlan,
    validateRenderPlanGeneratedIds,
    type EdgeContentAddress,
    type EdgeRenderStateRecord,
    type EdgeRenderStateAdvanceResult,
    type EdgeRenderStateStore,
    type RenderPlan,
    type RenderPlanIdentity,
    type RenderPlanNode,
    type RenderRevision,
    type TemplateSourceNode,
    type TemplateValue,
} from './projection.js';
import {
    assertCemEdgeSsrHostEnvelope,
    createCemEdgeSsrHostFailureEnvelope,
    createCemEdgeSsrHostProgressEnvelope,
    createCemEdgeSsrHostSuccessEnvelope,
    type CemEdgeSsrHostFailureEnvelope,
    type CemEdgeSsrHostProgressEnvelope,
    type CemEdgeSsrHostRequestEnvelope,
    type CemEdgeSsrHostSuccessEnvelope,
    type CemEdgeSsrPreviousRenderPlan,
    type CemEdgeSsrRenderInput,
    type CemEdgeStylesheetBatch,
    type CemEdgeStylesheetState,
    type CemEdgeSsrInitialRenderResult,
    type CemEdgeSsrTemplateInput,
} from './edge-ssr-host.js';

const RENDER_NODE_ID_ATTR = 'data-cem-render-node-id';
const TEMPLATE_ARTIFACT_ID_ATTR = 'data-cem-template-artifact-id';
const DATA_REVISION_ATTR = 'data-cem-data-revision';
const SOURCE_FIDELITY_ATTR = 'data-cem-source-fidelity';
const SOURCE_FRAME_ATTR = 'data-cem-source-frame';
const HTML_NAME = /^[A-Za-z][A-Za-z0-9._:-]*$/;
const HTML_ATTRIBUTE_NAME_FORBIDDEN = new Set(['"', "'", '<', '>', '/', '=']);
const HTML_VOID_ELEMENTS = new Set([
    'area',
    'base',
    'br',
    'col',
    'embed',
    'hr',
    'img',
    'input',
    'link',
    'meta',
    'param',
    'source',
    'track',
    'wbr',
]);
const UNSUPPORTED_RAW_TEXT_ELEMENTS = new Set([
    'iframe',
    'noembed',
    'noframes',
    'noscript',
    'plaintext',
    'script',
    'xmp',
]);

export type NonBrowserSsrInitialRenderFixtureResult =
    | CemEdgeSsrHostSuccessEnvelope<'render-initial'>
    | CemEdgeSsrHostFailureEnvelope<'render-initial'>;

/**
 * Node-only Phase 3.5 evidence host. It intentionally supports serialized
 * template source only; artifact resolution belongs to later deployment adapters.
 */
export function executeNonBrowserSsrInitialRenderFixture(
    request: CemEdgeSsrHostRequestEnvelope<'render-initial'>,
    store: EdgeRenderStateStore
): NonBrowserSsrInitialRenderFixtureResult {
    return renderInitialFixture(request, store);
}

export interface NativeSsrStylesheetOptions extends Omit<EdgeStylesheetLoadOptions, 'owner' | 'sources'> {
    /** Caller-selected declaration/context batches, including any shared source declarations. */
    declarations?: Array<Pick<EdgeStylesheetLoadOptions, 'sources' | 'context' | 'baseUrl'> & {
        owner: Extract<EdgeStylesheetLoadOptions['owner'], { kind: 'declaration' }>;
    }>;
}

export type NativeSsrStylesheetContext = Pick<NativeSsrStylesheetOptions, 'baseUrl' | 'context' | 'declarations'>;

function stylesheetInputKey(input: CemEdgeSsrRenderInput, source: readonly TemplateSourceNode[], context: NativeSsrStylesheetContext): string {
    return edgeContentAddress('stylesheets', {
        scopeUid: input.scopeUid, instanceId: input.snapshot.instanceId, producedTag: input.snapshot.producedTag,
        templateArtifactId: input.snapshot.templateArtifactId, scopePolicyStamp: input.snapshot.scopePolicyStamp,
        templateContent: edgeContentAddress('template-artifact', source).key,
        payloadSources: payloadStylesheetSources(input.snapshot.payload), ...context,
    }).key;
}

/** Native CSS path. The host adapter owns source discovery and sidecar placement. */
export async function executeNativeSsrInitialRenderFixture(
    request: CemEdgeSsrHostRequestEnvelope<'render-initial'>,
    store: EdgeRenderStateStore,
    stylesheets: NativeSsrStylesheetOptions,
): Promise<NonBrowserSsrInitialRenderFixtureResult> {
    const failure = initialRenderInputFailure(request);
    if (failure) return failure;
    const owned = structuredClone(request);
    const signal = stylesheets.signal;
    try {
        signal.throwIfAborted();
        if (!owned.payload.snapshot.scopePolicyStamp.split(':').includes('retained-instance-css')) {
            return renderInitialFixture(owned, store);
        }
        const { baseUrl, context, declarations } = structuredClone({
            baseUrl: stylesheets.baseUrl, context: stylesheets.context, declarations: stylesheets.declarations,
        });
        const capabilities = { native: stylesheets.native, read: stylesheets.read, signal };
        const instanceStyles = await loadEdgeStylesheets({ ...capabilities, baseUrl, context,
            owner: { kind: 'instance', identity: owned.payload.snapshot.instanceId },
            sources: payloadStylesheetSources(owned.payload.snapshot.payload),
        });
        const declarationStylesheets: CemEdgeSsrInitialRenderResult['declarationStylesheets'] = declarations === undefined ? undefined : [];
        const emittedDeclarations: CemEdgeStylesheetBatch['declarations'] = [];
        for (const declaration of declarations ?? []) {
            const loaded = await loadEdgeStylesheets({ ...capabilities, ...declaration });
            emittedDeclarations.push({ declarationIdentity: declaration.owner.identity, tag: declaration.owner.tag, styles: loaded.styles });
            const serialized = serializeDeclarationStylesheets(loaded.styles.map(style => {
                if (style.scope.kind === 'instance') throw new TypeError('expected declaration stylesheet scope');
                return { index: style.index, scope: style.scope, output: style };
            }));
            declarationStylesheets?.push({ declarationIdentity: declaration.owner.identity, tag: declaration.owner.tag, ...serialized });
            instanceStyles.diagnostics.push(...loaded.diagnostics);
        }
        signal.throwIfAborted();
        if (owned.payload.template.kind !== 'serialized-template-source-v1') throw new Error('expected serialized template source');
        const inputKey = stylesheetInputKey(owned.payload, owned.payload.template.source, { baseUrl, context, declarations });
        return renderInitialFixture(owned, store, instanceStyles, declarationStylesheets, inputKey,
            { kind: 'native-css-batch-v1', inputKey, instance: instanceStyles.styles, declarations: emittedDeclarations });
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (signal.aborted) {
            return createCemEdgeSsrHostFailureEnvelope(owned, 'cancelled', 'cancelled',
                [fixtureDiagnostic('cem.edge_ssr.stylesheet_cancelled', message)]);
        }
        return fixtureFailure(owned, 'render-failed', 'cem.edge_ssr.stylesheet_failed', message);
    }
}

function initialRenderInputFailure(
    request: CemEdgeSsrHostRequestEnvelope<'render-initial'>,
): CemEdgeSsrHostFailureEnvelope<'render-initial'> | undefined {
    assertCemEdgeSsrHostEnvelope(request);

    const identityFailure = renderInputIdentityFailure(request.payload);
    if (identityFailure) {
        return fixtureFailure(request, 'invalid-request', 'cem.edge_ssr.initial_identity_invalid', identityFailure);
    }
    if (!isCompleteRenderSnapshot(request.payload.snapshot)) {
        return fixtureFailure(
            request,
            'privacy-policy-rejected',
            'cem.edge_ssr.snapshot_fields_unavailable',
            'the sanitized snapshot omits fields required to produce hydration-safe initial HTML'
        );
    }
    if (request.payload.template.kind !== 'serialized-template-source-v1') {
        return fixtureFailure(
            request,
            'content-unavailable',
            'cem.edge_ssr.template_source_unavailable',
            'the non-browser SSR fixture accepts serialized template source only'
        );
    }

    return undefined;
}

function renderInitialFixture(
    request: CemEdgeSsrHostRequestEnvelope<'render-initial'>,
    store: EdgeRenderStateStore,
    instanceStyles?: EdgeStylesheetOutput,
    declarationStylesheets?: CemEdgeSsrInitialRenderResult['declarationStylesheets'],
    inputKey?: string,
    batch?: CemEdgeStylesheetBatch,
): NonBrowserSsrInitialRenderFixtureResult {
    const failure = initialRenderInputFailure(request);
    if (failure) return failure;
    // The validation above admits only this source kind.
    if (request.payload.template.kind !== 'serialized-template-source-v1'
        || !isCompleteRenderSnapshot(request.payload.snapshot)) throw new Error('invalid render input');
    const cssFailure = retainedCssCapabilityFailure(request.payload);
    if (cssFailure && !instanceStyles) {
        return fixtureFailure(request, 'content-unavailable', 'cem.edge_ssr.retained_css_unavailable', cssFailure);
    }

    try {
        const snapshot = request.payload.snapshot;
        const projected = projectTemplate(request.payload.template.source, {
            snapshot,
            values: templateValues(snapshot, request.payload.template.source),
        });
        const sourceMapped = request.payload.sourceMapMode === 'dev'
            ? projected
            : stripRenderPlanSourceMaps(projected);
        const scoped = scopeRenderPlan(sourceMapped, request.payload.scopeUid, {
            payload: snapshot.payload,
            payloadStylesInstalled: instanceStyles !== undefined,
        });
        const plan = scoped.renderPlan;
        const identity = renderPlanIdentity(plan);
        if (!sameRenderPlanIdentity(identity, {
            producedTag: snapshot.producedTag,
            ...request.payload.revision,
        })) {
            return fixtureFailure(
                request,
                'render-failed',
                'cem.edge_ssr.render_plan_identity_invalid',
                'the projected render-plan identity did not match the validated initial-render request'
            );
        }

        const renderedHtml = serializeRenderPlanToHtmlFixture(plan);
        const instanceStylesheetHtml = instanceStyles?.styles.map(style =>
            `<style data-cem-instance-style="${style.index}">${serializeStyleChildren([{ kind: 'text', text: style.css }])}</style>`
        ).join('');
        const stateInput = {
            renderPlan: plan,
            templateArtifact: request.payload.template.source,
            sanitizedSnapshot: snapshot,
            renderedHtml,
            privacyPolicyStamp: snapshot.privacyPolicyStamp,
            ...(inputKey === undefined ? {} : { stylesheetState: {
                kind: 'native-ssr-stylesheets-v1', inputKey, instanceStylesheetHtml, declarationStylesheets, ...(batch ? { batch } : {}),
            } }),
        };
        const write = store.writeRenderState(stateInput, { ifAbsent: true });
        if (!write.ok) {
            return createCemEdgeSsrHostFailureEnvelope(
                request,
                'failure',
                'render-state-conflict',
                [fixtureDiagnostic(
                    'cem.edge_ssr.initial_state_exists',
                    'initial render refused to replace an existing render-state pointer'
                )],
                write.current
            );
        }
        const retained = readEdgeRenderStateContents(store, write.record);
        if (!retained.ok) {
            return createCemEdgeSsrHostFailureEnvelope(
                request,
                'failure',
                'content-unavailable',
                [fixtureDiagnostic(
                    'cem.edge_ssr.retained_render_state_unavailable',
                    `the committed render state could not be verified: ${retained.reason}`
                )],
                write.record
            );
        }
        if (!sameRenderPlanIdentity(renderPlanIdentity(retained.contents.renderPlan), identity)) {
            return createCemEdgeSsrHostFailureEnvelope(
                request,
                'failure',
                'render-failed',
                [fixtureDiagnostic(
                    'cem.edge_ssr.retained_render_plan_identity_invalid',
                    'the retained render plan did not preserve the initial-render identity'
                )],
                write.record
            );
        }

        return createCemEdgeSsrHostSuccessEnvelope(request, {
            kind: 'initial-render',
            renderedHtml,
            ...(instanceStylesheetHtml === undefined ? {} : { instanceStylesheetHtml }),
            ...(declarationStylesheets === undefined ? {} : { declarationStylesheets }),
            hydrationData: {
                kind: 'cem-ssr-hydration-v1',
                snapshot,
                revision: request.payload.revision,
                renderPlanIdentity: identity,
                sourceMapMode: request.payload.sourceMapMode,
            },
            renderState: write.record,
            diagnostics: [
                ...(instanceStyles?.diagnostics ?? []),
                ...scoped.diagnostics.map((diagnostic) => ({
                    code: diagnostic.code,
                    severity: diagnostic.severity,
                    message: diagnostic.message,
                })),
                ...validateRenderPlanGeneratedIds(plan).map((diagnostic) => ({
                    code: diagnostic.code,
                    severity: diagnostic.severity,
                    message: diagnostic.message,
                })),
            ],
        });
    } catch (error) {
        return fixtureFailure(
            request,
            'render-failed',
            'cem.edge_ssr.initial_render_failed',
            error instanceof Error ? error.message : String(error)
        );
    }
}

export type NonBrowserEdgeRenderUpdateFixtureResponse =
    | CemEdgeSsrHostProgressEnvelope
    | CemEdgeSsrHostSuccessEnvelope<'render-update'>
    | CemEdgeSsrHostFailureEnvelope<'render-update'>;

/**
 * DOM-free Phase 3.5 edge-update evidence host. State is committed and verified
 * before the commit frame becomes observable on the returned response stream.
 */
export async function* executeNonBrowserEdgeRenderUpdateFixture(
    request: CemEdgeSsrHostRequestEnvelope<'render-update'>,
    store: EdgeRenderStateStore
): AsyncGenerator<NonBrowserEdgeRenderUpdateFixtureResponse, void, void> {
    yield* renderUpdateFixture(request, store);
}

/** Emit a complete native replacement before committing state or exposing patch frames. */
export function executeNativeEdgeRenderUpdateFixture(
    request: CemEdgeSsrHostRequestEnvelope<'render-update'>,
    store: EdgeRenderStateStore,
    stylesheets: NativeSsrStylesheetContext & Partial<Pick<NativeSsrStylesheetOptions, 'native' | 'read'>>,
    signal: AbortSignal,
): AsyncGenerator<NonBrowserEdgeRenderUpdateFixtureResponse, void, void> {
    const { baseUrl, context, declarations } = stylesheets;
    return renderUpdateFixture(structuredClone(request), store, {
        context: structuredClone({ baseUrl, context, declarations }), signal,
        native: stylesheets.native, read: stylesheets.read,
    });
}

async function* renderUpdateFixture(
    request: CemEdgeSsrHostRequestEnvelope<'render-update'>,
    store: EdgeRenderStateStore,
    nativeStyles?: { context: NativeSsrStylesheetContext; signal: AbortSignal } & Partial<Pick<NativeSsrStylesheetOptions, 'native' | 'read'>>,
): AsyncGenerator<NonBrowserEdgeRenderUpdateFixtureResponse, void, void> {
    assertCemEdgeSsrHostEnvelope(request);
    if (nativeStyles?.signal.aborted) {
        yield createCemEdgeSsrHostFailureEnvelope(request, 'cancelled', 'cancelled',
            [fixtureDiagnostic('cem.edge_ssr.stylesheet_cancelled', 'native style-preserving update cancelled before commit')]);
        return;
    }

    const identityFailure = renderInputIdentityFailure(request.payload);
    if (identityFailure) {
        yield updateFixtureFailure(
            request,
            'invalid-request',
            'cem.edge_ssr.update_identity_invalid',
            identityFailure
        );
        return;
    }
    const previousFailure = previousRenderPlanIdentityFailure(request.payload.previousRenderPlan);
    if (previousFailure) {
        yield updateFixtureFailure(
            request,
            'invalid-request',
            'cem.edge_ssr.previous_render_plan_invalid',
            previousFailure
        );
        return;
    }
    if (!isCompleteRenderSnapshot(request.payload.snapshot)) {
        yield updateFixtureFailure(
            request,
            'privacy-policy-rejected',
            'cem.edge_ssr.snapshot_fields_unavailable',
            'the sanitized snapshot omits fields required to produce an edge update'
        );
        return;
    }

    const cssFailure = retainedCssCapabilityFailure(request.payload);
    if (cssFailure && !nativeStyles) {
        yield updateFixtureFailure(request, 'content-unavailable', 'cem.edge_ssr.retained_css_unavailable', cssFailure);
        return;
    }

    const previous = request.payload.previousRenderPlan;
    const current = store.readRecord(previous.stateKey);
    if (!current) {
        yield updateFixtureFailure(
            request,
            'render-state-not-found',
            'cem.edge_ssr.render_state_not_found',
            `render state ${previous.stateKey} was not found`
        );
        return;
    }
    if (current.etag !== previous.expectedEtag) {
        yield updateFixtureFailure(
            request,
            'render-state-conflict',
            'cem.edge_ssr.render_state_conflict',
            'the expected render-state ETag is stale',
            current
        );
        return;
    }
    if (!sameContentAddress(current.currentRenderPlan, previous.address)) {
        yield updateFixtureFailure(
            request,
            'invalid-request',
            'cem.edge_ssr.previous_render_plan_address_invalid',
            'the supplied previous render-plan address does not match the current pointer',
            current
        );
        return;
    }
    if (!sameRenderPlanIdentity({
        producedTag: current.producedTag,
        ...current.renderRevision,
    }, previous.identity)) {
        yield updateFixtureFailure(
            request,
            'invalid-request',
            'cem.edge_ssr.previous_render_plan_identity_invalid',
            'the supplied previous render-plan identity does not match the current pointer',
            current
        );
        return;
    }

    const retainedPrevious = readEdgeRenderStateContents(store, current);
    if (!retainedPrevious.ok) {
        yield updateFixtureFailure(
            request,
            'content-unavailable',
            'cem.edge_ssr.previous_render_state_unavailable',
            `the previous render state could not be verified: ${retainedPrevious.reason}`,
            current
        );
        return;
    }
    if (!sameRenderPlanIdentity(
        renderPlanIdentity(retainedPrevious.contents.renderPlan),
        previous.identity
    )) {
        yield updateFixtureFailure(
            request,
            'invalid-request',
            'cem.edge_ssr.retained_previous_identity_invalid',
            'the retained previous plan does not match the supplied previous identity',
            current
        );
        return;
    }

    const templateSource = resolveTemplateSource(request.payload.template, store);
    if (!templateSource.ok) {
        yield updateFixtureFailure(
            request,
            'content-unavailable',
            'cem.edge_ssr.template_source_unavailable',
            templateSource.message,
            current
        );
        return;
    }

    try {
        let stylesheetState = retainedPrevious.contents.stylesheetState;
        let replacement: CemEdgeStylesheetBatch | undefined;
        let replacementState: CemEdgeStylesheetState | undefined;
        const stylesheetDiagnostics: CemEdgeSsrInitialRenderResult['diagnostics'] = [];
        if (nativeStyles) {
            if (!isPlainRecord(stylesheetState) || stylesheetState.kind !== 'native-ssr-stylesheets-v1'
                || !request.payload.snapshot.scopePolicyStamp.split(':').includes('retained-instance-css')) {
                yield updateFixtureFailure(request, 'content-unavailable', 'cem.edge_ssr.stylesheet_state_unavailable',
                    'native updates require retained stylesheet state and the retained-instance-css policy', current);
                return;
            }
            if (isPlainRecord(stylesheetState.batch) && stylesheetState.batch.kind === 'native-css-batch-v1') {
                replacementState = stylesheetState as unknown as CemEdgeStylesheetState;
            }
            const inputKey = stylesheetInputKey(request.payload, templateSource.source, nativeStyles.context);
            if (stylesheetState.inputKey !== inputKey) {
                if (!nativeStyles.native || !nativeStyles.read) {
                    yield updateFixtureFailure(request, 'content-unavailable', 'cem.edge_ssr.stylesheet_transport_unavailable',
                        'changed native styles require native bindings and a byte reader', current);
                    return;
                }
                const capabilities = { native: nativeStyles.native, read: nativeStyles.read, signal: nativeStyles.signal };
                const instance = await loadEdgeStylesheets({ ...capabilities, ...nativeStyles.context,
                    owner: { kind: 'instance', identity: request.payload.snapshot.instanceId },
                    sources: payloadStylesheetSources(request.payload.snapshot.payload) });
                stylesheetDiagnostics.push(...instance.diagnostics);
                replacement = { kind: 'native-css-batch-v1', inputKey, instance: instance.styles, declarations: [] };
                const declarations: NonNullable<CemEdgeSsrInitialRenderResult['declarationStylesheets']> = [];
                for (const declaration of nativeStyles.context.declarations ?? []) {
                    const loaded = await loadEdgeStylesheets({ ...capabilities, ...declaration });
                    stylesheetDiagnostics.push(...loaded.diagnostics);
                    replacement.declarations.push({ declarationIdentity: declaration.owner.identity, tag: declaration.owner.tag, styles: loaded.styles });
                    const serialized = serializeDeclarationStylesheets(loaded.styles.map(style => {
                        if (style.scope.kind === 'instance') throw new Error('invalid declaration scope');
                        return { index: style.index, scope: style.scope, output: style };
                    }));
                    declarations.push({ declarationIdentity: declaration.owner.identity, tag: declaration.owner.tag, ...serialized });
                }
                if (stylesheetDiagnostics.some(d => d.severity === 'error' || d.severity === 'fatal')) {
                    yield createCemEdgeSsrHostFailureEnvelope(request, 'failure', 'render-failed', stylesheetDiagnostics, current);
                    return;
                }
                replacementState = { kind: 'native-ssr-stylesheets-v1', inputKey, batch: replacement,
                    instanceStylesheetHtml: replacement.instance.map(style =>
                        `<style data-cem-instance-style="${style.index}">${serializeStyleChildren([{ kind: 'text', text: style.css }])}</style>`).join(''),
                    declarationStylesheets: declarations };
                stylesheetState = replacementState;
            }
        }
        const snapshot = request.payload.snapshot;
        const projected = projectTemplate(templateSource.source, {
            snapshot,
            values: templateValues(snapshot, templateSource.source),
        });
        const sourceMapped = request.payload.sourceMapMode === 'dev'
            ? projected
            : stripRenderPlanSourceMaps(projected);
        const scoped = scopeRenderPlan(sourceMapped, request.payload.scopeUid, {
            payload: snapshot.payload,
            payloadStylesInstalled: nativeStyles !== undefined,
        });
        const plan = scoped.renderPlan;
        const identity = renderPlanIdentity(plan);
        if (!sameRenderPlanIdentity(identity, {
            producedTag: snapshot.producedTag,
            ...request.payload.revision,
        })) {
            yield updateFixtureFailure(
                request,
                'render-failed',
                'cem.edge_ssr.render_plan_identity_invalid',
                'the projected render-plan identity did not match the validated update request',
                current
            );
            return;
        }

        const renderedHtml = serializeRenderPlanToHtmlFixture(plan);
        if (nativeStyles?.signal.aborted) {
            yield createCemEdgeSsrHostFailureEnvelope(request, 'cancelled', 'cancelled',
                [fixtureDiagnostic('cem.edge_ssr.stylesheet_cancelled', 'native style-preserving update cancelled before commit')]);
            return;
        }
        const advanced = advanceEdgeRenderState(
            store,
            {
                renderPlan: plan,
                templateArtifact: templateSource.source,
                sanitizedSnapshot: snapshot,
                renderedHtml,
                privacyPolicyStamp: snapshot.privacyPolicyStamp,
                stateKey: previous.stateKey,
                ...(nativeStyles ? { stylesheetState } : {}),
            },
            { expectedEtag: previous.expectedEtag, patchOptions: { textNodeIdsAvailable: false } }
        );
        if (!advanced.ok) {
            yield advanceFailureEnvelope(request, advanced);
            return;
        }

        const retainedNext = readEdgeRenderStateContents(store, advanced.record);
        if (!retainedNext.ok) {
            yield updateFixtureFailure(
                request,
                'content-unavailable',
                'cem.edge_ssr.retained_render_state_unavailable',
                `the committed render state could not be verified: ${retainedNext.reason}`,
                advanced.record
            );
            return;
        }
        if (!sameRenderPlanIdentity(renderPlanIdentity(retainedNext.contents.renderPlan), identity)) {
            yield updateFixtureFailure(
                request,
                'render-failed',
                'cem.edge_ssr.retained_render_plan_identity_invalid',
                'the retained render plan did not preserve the edge-update identity',
                advanced.record
            );
            return;
        }

        for (const frame of advanced.frames) {
            yield createCemEdgeSsrHostProgressEnvelope(request, frame);
        }
        yield createCemEdgeSsrHostSuccessEnvelope(request, {
            kind: 'render-update-complete',
            ...(replacementState ? { stylesheets: replacementState } : {}),
            renderPlanIdentity: identity,
            renderState: advanced.record,
            diagnostics: [...stylesheetDiagnostics, ...renderPlanDiagnostics(scoped.diagnostics, plan)],
        });
    } catch (error) {
        if (nativeStyles?.signal.aborted) {
            yield createCemEdgeSsrHostFailureEnvelope(request, 'cancelled', 'cancelled',
                [fixtureDiagnostic('cem.edge_ssr.stylesheet_cancelled', 'native stylesheet update cancelled before commit')]);
            return;
        }
        yield updateFixtureFailure(
            request,
            'render-failed',
            'cem.edge_ssr.render_update_failed',
            error instanceof Error ? error.message : String(error),
            current
        );
    }
}

/** Serialize the owned render range without constructing or reading browser DOM. */
export function serializeRenderPlanToHtmlFixture(plan: RenderPlan): string {
    return plan.nodes.map((node) => serializeRenderNode(node, plan)).join('');
}

/** This evidence host has no native stylesheet owner, resolver or byte reader. */
function retainedCssCapabilityFailure(input: CemEdgeSsrRenderInput): string | undefined {
    const policies = input.snapshot.scopePolicyStamp.split(':');
    if (policies.includes('retained-declaration-css') || policies.includes('retained-instance-css')) {
        return 'the non-browser evidence host cannot render retained CSS; a native stylesheet load capability is required';
    }
    return undefined;
}

function renderInputIdentityFailure(input: CemEdgeSsrRenderInput): string | undefined {
    const { revision, snapshot, sourceMapMode, template, scopeUid } = input;
    if (
        !isPlainRecord(revision)
        || !isPlainRecord(snapshot)
        || !isPlainRecord(template)
        || typeof template.templateArtifactId !== 'string'
        || typeof snapshot.templateArtifactId !== 'string'
        || typeof snapshot.instanceId !== 'string'
        || typeof snapshot.dataRevision !== 'string'
        || typeof snapshot.scopePolicyStamp !== 'string'
        || typeof snapshot.producedTag !== 'string'
        || typeof snapshot.privacyPolicyStamp !== 'string'
        || typeof revision.templateArtifactId !== 'string'
        || typeof revision.instanceId !== 'string'
        || typeof revision.dataRevision !== 'string'
        || typeof revision.scopePolicyStamp !== 'string'
        || revision.outputTarget !== 'light-dom'
        || snapshot.outputTarget !== 'light-dom'
        || (sourceMapMode !== 'dev' && sourceMapMode !== 'prod')
        || typeof scopeUid !== 'string'
    ) {
        return 'the render request is missing required identity fields';
    }
    if (
        template.kind !== 'serialized-template-source-v1'
        && template.kind !== 'compiled-template-artifact-v1'
        && template.kind !== 'content-addressed-template-artifact-v1'
    ) {
        return 'the render request names an unsupported template input kind';
    }
    if (template.templateArtifactId !== snapshot.templateArtifactId) {
        return 'template input and snapshot template artifact identities differ';
    }
    if (revision.templateArtifactId !== snapshot.templateArtifactId) {
        return 'render revision and snapshot template artifact identities differ';
    }
    if (revision.instanceId !== snapshot.instanceId) {
        return 'render revision and snapshot instance identities differ';
    }
    if (revision.dataRevision !== snapshot.dataRevision) {
        return 'render revision and snapshot data revisions differ';
    }
    if (revision.scopePolicyStamp !== snapshot.scopePolicyStamp) {
        return 'render revision and snapshot scope-policy stamps differ';
    }
    try {
        if (resolveAriaReferenceProfile(revision.ariaReferenceProfile) !== resolveAriaReferenceProfile(snapshot.ariaReferenceProfile)) {
            return 'render revision and snapshot ARIA reference profiles differ';
        }
    } catch {
        return 'the render request names an unknown ARIA reference profile';
    }
    if (revision.outputTarget !== snapshot.outputTarget) {
        return 'render revision and snapshot output targets differ';
    }
    if ((revision.renderAttempt ?? undefined) !== (snapshot.renderAttempt ?? undefined)) {
        return 'render revision and snapshot render attempts differ';
    }
    if (snapshot.sourceMapMode !== sourceMapMode) {
        return 'request and snapshot source-map modes differ or are incomplete';
    }
    if (
        snapshot.producedTag.length === 0
        || snapshot.privacyPolicyStamp.length === 0
        || scopeUid.length === 0
    ) {
        return 'produced-tag, privacy-policy, and scope identities must be non-empty';
    }
    const snapshotScopeUid = snapshot.hostAttributes?.['data-cem-render-scope'];
    if (typeof snapshotScopeUid === 'string' && snapshotScopeUid !== scopeUid) {
        return 'snapshot and request scope identities differ';
    }
    return undefined;
}

function previousRenderPlanIdentityFailure(previous: CemEdgeSsrPreviousRenderPlan): string | undefined {
    if (
        !isPlainRecord(previous)
        || !isPlainRecord(previous.identity)
        || !isPlainRecord(previous.address)
        || typeof previous.stateKey !== 'string'
        || previous.stateKey.length === 0
        || typeof previous.expectedEtag !== 'string'
        || previous.expectedEtag.length === 0
        || previous.address.kind !== 'render-plan'
        || previous.address.algorithm !== 'stable-json-fnv1a64-v1'
        || typeof previous.address.digest !== 'string'
        || typeof previous.address.key !== 'string'
        || !isRenderPlanIdentity(previous.identity)
    ) {
        return 'the update request is missing a complete previous state, ETag, address, or identity';
    }
    return undefined;
}

function isRenderPlanIdentity(value: RenderPlanIdentity): boolean {
    return (
        typeof value.producedTag === 'string'
        && value.producedTag.length > 0
        && typeof value.instanceId === 'string'
        && typeof value.dataRevision === 'string'
        && typeof value.templateArtifactId === 'string'
        && typeof value.scopePolicyStamp === 'string'
        && value.outputTarget === 'light-dom'
        && (value.renderAttempt === undefined || typeof value.renderAttempt === 'number')
    );
}

function resolveTemplateSource(
    template: CemEdgeSsrTemplateInput,
    store: EdgeRenderStateStore
): { ok: true; source: TemplateSourceNode[] } | { ok: false; message: string } {
    if (template.kind === 'serialized-template-source-v1') {
        return Array.isArray(template.source)
            ? { ok: true, source: template.source }
            : { ok: false, message: 'the serialized template source is not an array' };
    }
    if (template.kind === 'compiled-template-artifact-v1') {
        return {
            ok: false,
            message: 'the DOM-parity edge fixture does not execute compiled template artifacts',
        };
    }
    const value = store.getContent<unknown>(template.address);
    if (value === undefined) {
        return { ok: false, message: `template artifact ${template.address.key} was not found` };
    }
    const actual = contentAddressForTemplate(value);
    if (!sameContentAddress(actual, template.address)) {
        return { ok: false, message: `template artifact ${template.address.key} failed address verification` };
    }
    return Array.isArray(value)
        ? { ok: true, source: value as TemplateSourceNode[] }
        : { ok: false, message: 'the addressed template artifact is not serialized template source' };
}

function isCompleteRenderSnapshot(value: unknown): value is DataIslandSnapshot {
    if (!isPlainRecord(value)) {
        return false;
    }
    const payload = value.payload;
    return (
        isPlainRecord(value.hostAttributes)
        && isPlainRecord(value.dataset)
        && isPlainRecord(payload)
        && typeof payload.text === 'string'
        && typeof payload.childCount === 'number'
        && Array.isArray(payload.nodes)
        && isPlainRecord(payload.slots)
        && isPlainRecord(payload.elementsByAttribute)
        && Array.isArray(payload.data)
        && Array.isArray(payload.options)
        && isPlainRecord(payload.dataByValue)
        && isPlainRecord(payload.optionsByValue)
        && isPlainRecord(value.slices)
        && (value.formData === undefined || isPlainRecord(value.formData))
        && isPlainRecord(value.validationState)
        && isPlainRecord(value.eventPayloads)
    );
}

function templateValues(
    snapshot: DataIslandSnapshot,
    source: readonly TemplateSourceNode[]
): Record<string, TemplateValue> {
    const values: Record<string, TemplateValue> = {};
    for (const child of source) {
        if (child.kind !== 'element' || child.tag !== 'attribute') {
            continue;
        }
        const name = child.attributes.find((attribute) => attribute.name === 'name')?.value.trim();
        if (!name) {
            continue;
        }
        const text = child.children
            .map((node) => (node.kind === 'text' ? node.text : ''))
            .join('')
            .trim();
        values[name] = text.length > 0 ? text : null;
    }
    for (const [name, value] of Object.entries(snapshot.hostAttributes)) {
        values[name] = value;
    }
    for (const [name, value] of Object.entries(snapshot.slices)) {
        values[name] = toTemplateValue(value);
    }
    addTemplateValuePaths(values, 'datadom', dataDocumentFromSnapshot(snapshot));
    return values;
}

function dataDocumentFromSnapshot(snapshot: DataIslandSnapshot): Record<string, unknown> {
    return {
        attributes: snapshot.hostAttributes,
        dataset: snapshot.dataset,
        elementsByAttribute: dataDocumentElementsByAttribute(snapshot),
        payload: snapshot.payload,
        slots: snapshot.payload.slots,
        data: snapshot.payload.dataByValue,
        options: snapshot.payload.optionsByValue,
        dataItems: snapshot.payload.data,
        optionItems: snapshot.payload.options,
        slices: snapshot.slices,
        formData: snapshot.formData ?? {},
        validationState: snapshot.validationState,
        eventPayloads: snapshot.eventPayloads,
    };
}

function dataDocumentElementsByAttribute(
    snapshot: DataIslandSnapshot
): Record<string, SerializedPayloadElement[]> {
    const byAttribute: Record<string, SerializedPayloadElement[]> = {};
    for (const [name, elements] of Object.entries(snapshot.payload.elementsByAttribute)) {
        byAttribute[name] = [...elements];
    }
    const hostElement: SerializedPayloadElement = {
        key: 'host',
        tag: snapshot.producedTag,
        namespace: null,
        text: '',
        attributes: Object.fromEntries(
            Object.entries(snapshot.hostAttributes)
                .filter((entry): entry is [string, string | boolean] => entry[1] !== null)
                .map(([name, value]) => [name, value === true ? '' : value === false ? 'false' : value])
        ),
        slot: '',
    };
    for (const name of Object.keys(hostElement.attributes)) {
        byAttribute[name] = [...(byAttribute[name] ?? []), hostElement];
    }
    return byAttribute;
}

function addTemplateValuePaths(
    values: Record<string, TemplateValue>,
    prefix: string,
    value: unknown
): void {
    if (
        value === null
        || typeof value === 'string'
        || typeof value === 'boolean'
        || typeof value === 'number'
        || typeof value === 'undefined'
    ) {
        values[prefix] = toTemplateValue(value);
        return;
    }
    if (Array.isArray(value)) {
        return;
    }
    if (typeof value !== 'object') {
        values[prefix] = toTemplateValue(value);
        return;
    }
    for (const [name, child] of Object.entries(value)) {
        addTemplateValuePaths(values, `${prefix}.${name}`, child);
    }
}

function toTemplateValue(value: unknown): TemplateValue {
    if (value === null || typeof value === 'string' || typeof value === 'boolean') {
        return value;
    }
    if (value === undefined) {
        return null;
    }
    return String(value);
}

function stripRenderPlanSourceMaps(plan: RenderPlan): RenderPlan {
    return {
        ...plan,
        nodes: plan.nodes.map(stripRenderNodeSourceMaps),
    };
}

function stripRenderNodeSourceMaps(node: RenderPlanNode): RenderPlanNode {
    if (node.kind === 'text' || node.kind === 'comment') {
        const { sourceMapRef: _sourceMapRef, ...plain } = node;
        return plain;
    }
    const { sourceMapRef: _sourceMapRef, ...plain } = node;
    return {
        ...plain,
        children: node.children.map(stripRenderNodeSourceMaps),
    };
}

function serializeRenderNode(node: RenderPlanNode, plan: RenderPlan): string {
    if (node.kind === 'text') {
        assertNoNullCharacter(node.text, 'rendered text');
        return escapeHtmlText(node.text);
    }
    if (node.kind === 'comment') {
        assertNoNullCharacter(node.text, 'rendered comment');
        if (node.text.includes('--') || node.text.endsWith('-')) {
            throw new TypeError('rendered comments may not contain `--` or end with `-`');
        }
        return `<!--${node.text}-->`;
    }

    if (!HTML_NAME.test(node.tag)) {
        throw new TypeError(`rendered element name is not safely serializable: ${node.tag}`);
    }
    const lowerTag = node.tag.toLowerCase();
    if (node.namespace === null && UNSUPPORTED_RAW_TEXT_ELEMENTS.has(lowerTag)) {
        throw new TypeError(`rendered ${lowerTag} elements are not supported by the SSR fixture`);
    }
    const attributes = new Map(node.attributes.map((attribute) => [attribute.name, attribute.value]));
    attributes.set(RENDER_NODE_ID_ATTR, node.renderNodeId);
    attributes.set(TEMPLATE_ARTIFACT_ID_ATTR, plan.templateArtifactId);
    attributes.set(DATA_REVISION_ATTR, plan.dataRevision);
    if (node.sourceMapRef) {
        attributes.set(SOURCE_FIDELITY_ATTR, node.sourceMapRef.fidelity);
        attributes.set(SOURCE_FRAME_ATTR, node.sourceMapRef.frame);
    }
    const serializedAttributes = Array.from(attributes, ([name, value]) => {
        if (!isSafeHtmlAttributeName(name)) {
            throw new TypeError(`rendered attribute name is not safely serializable: ${name}`);
        }
        assertNoNullCharacter(value, `rendered attribute ${name}`);
        return ` ${name}="${escapeHtmlAttribute(value)}"`;
    }).join('');
    if (node.namespace === null && HTML_VOID_ELEMENTS.has(lowerTag)) {
        if (node.children.length > 0) {
            throw new TypeError(`rendered void element ${lowerTag} may not contain children`);
        }
        return `<${node.tag}${serializedAttributes}>`;
    }
    const children = node.namespace === null && lowerTag === 'style'
        ? serializeStyleChildren(node.children)
        : node.children.map((child) => serializeRenderNode(child, plan)).join('');
    return `<${node.tag}${serializedAttributes}>${children}</${node.tag}>`;
}

function serializeStyleChildren(children: readonly RenderPlanNode[]): string {
    return children.map((child) => {
        if (child.kind !== 'text') {
            throw new TypeError('rendered style elements may contain text only in the SSR fixture');
        }
        assertNoNullCharacter(child.text, 'rendered style text');
        if (/<\/style/i.test(child.text)) {
            throw new TypeError('rendered style text may not contain a closing style tag');
        }
        return child.text;
    }).join('');
}

function escapeHtmlText(value: string): string {
    return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}

function isSafeHtmlAttributeName(value: string): boolean {
    return value.length > 0 && Array.from(value).every(
        (character) => character.charCodeAt(0) > 0x20 && !HTML_ATTRIBUTE_NAME_FORBIDDEN.has(character)
    );
}

function escapeHtmlAttribute(value: string): string {
    return escapeHtmlText(value).replaceAll('"', '&quot;');
}

function assertNoNullCharacter(value: string, label: string): void {
    if (value.includes('\0')) {
        throw new TypeError(`${label} may not contain a null character`);
    }
}

function sameRenderPlanIdentity(left: RenderPlanIdentity, right: RenderPlanIdentity): boolean {
    return (
        left.producedTag === right.producedTag
        && sameRenderRevision(left, right)
    );
}

function sameRenderRevision(left: RenderRevision, right: RenderRevision): boolean {
    return (
        left.instanceId === right.instanceId
        && left.dataRevision === right.dataRevision
        && left.templateArtifactId === right.templateArtifactId
        && left.scopePolicyStamp === right.scopePolicyStamp
        && left.outputTarget === right.outputTarget
        && (left.renderAttempt ?? undefined) === (right.renderAttempt ?? undefined)
    );
}

function sameContentAddress(left: EdgeContentAddress, right: EdgeContentAddress): boolean {
    return (
        left.kind === right.kind
        && left.algorithm === right.algorithm
        && left.digest === right.digest
        && left.key === right.key
    );
}

function contentAddressForTemplate(value: unknown): EdgeContentAddress {
    return edgeContentAddress('template-artifact', value);
}

function renderPlanDiagnostics(
    scopedDiagnostics: ReadonlyArray<{ code: string; severity: 'warning'; message: string }>,
    plan: RenderPlan
) {
    return [
        ...scopedDiagnostics.map((diagnostic) => ({
            code: diagnostic.code,
            severity: diagnostic.severity,
            message: diagnostic.message,
        })),
        ...validateRenderPlanGeneratedIds(plan).map((diagnostic) => ({
            code: diagnostic.code,
            severity: diagnostic.severity,
            message: diagnostic.message,
        })),
    ];
}

function advanceFailureEnvelope(
    request: CemEdgeSsrHostRequestEnvelope<'render-update'>,
    result: Exclude<EdgeRenderStateAdvanceResult, { ok: true }>
): CemEdgeSsrHostFailureEnvelope<'render-update'> {
    if (result.reason === 'etag-mismatch') {
        return updateFixtureFailure(
            request,
            'render-state-conflict',
            'cem.edge_ssr.render_state_conflict',
            'the render-state pointer changed before the edge update committed',
            result.current
        );
    }
    if (result.reason === 'missing-render-plan') {
        return updateFixtureFailure(
            request,
            'content-unavailable',
            'cem.edge_ssr.previous_render_plan_missing',
            `the previous render plan ${result.address.key} is unavailable`,
            result.current
        );
    }
    if (result.reason === 'content-address-mismatch') {
        return updateFixtureFailure(
            request,
            'content-unavailable',
            'cem.edge_ssr.previous_render_plan_corrupt',
            `the previous render plan ${result.expected.key} failed address verification`,
            result.current
        );
    }
    return updateFixtureFailure(
        request,
        'content-unavailable',
        'cem.edge_ssr.previous_render_plan_revision_invalid',
        'the retained previous render plan does not match its pointer revision',
        result.current
    );
}

function fixtureFailure(
    request: CemEdgeSsrHostRequestEnvelope<'render-initial'>,
    reason: Exclude<CemEdgeSsrHostFailureEnvelope<'render-initial'>['reason'], 'cancelled'>,
    code: string,
    message: string
): CemEdgeSsrHostFailureEnvelope<'render-initial'> {
    return createCemEdgeSsrHostFailureEnvelope(
        request,
        'failure',
        reason,
        [fixtureDiagnostic(code, message)]
    );
}

function updateFixtureFailure(
    request: CemEdgeSsrHostRequestEnvelope<'render-update'>,
    reason: Exclude<CemEdgeSsrHostFailureEnvelope<'render-update'>['reason'], 'cancelled'>,
    code: string,
    message: string,
    currentRenderState?: EdgeRenderStateRecord
): CemEdgeSsrHostFailureEnvelope<'render-update'> {
    return createCemEdgeSsrHostFailureEnvelope(
        request,
        'failure',
        reason,
        [fixtureDiagnostic(code, message)],
        currentRenderState
    );
}

function fixtureDiagnostic(code: string, message: string) {
    return {
        code,
        severity: 'error' as const,
        message,
    };
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}
