import type { CemDeclarationScope } from './declaration-scope.js';
import { DeclarationStyleOwnership, type DeclarationStylesheetCommit } from './declaration-style-ownership.js';
import type { CemEdgeSsrRenderUpdateResult } from './edge-ssr-host.js';
import { edgeContentAddress, preparePatchFramesForRange, renderRevisionKey,
    type EdgeRenderStateRecord, type PatchFrame, type RenderRevision } from './projection.js';
import { CemCssDomPublicationQueue } from './internal/runtime-support/css-dom-publication-queue.js';
import { prepareEmittedInstanceStylesheets } from './internal/runtime-support/instance-stylesheet-installation.js';

export interface CemEdgeCssPublicationOptions {
    element: HTMLElement;
    bounds: { start: Comment; end: Comment };
    scope: CemDeclarationScope;
    /** Include every declaration owner currently installed for this consumer, including removed sources. */
    declarations: ReadonlyMap<string, DeclarationStyleOwnership>;
    frames: readonly PatchFrame[];
    result: CemEdgeSsrRenderUpdateResult;
    previousEtag: string;
    currentState(): EdgeRenderStateRecord;
    /** Adopt the committed state before retiring the previous stylesheet generation. */
    adopt(state: EdgeRenderStateRecord): void;
    /** Abort on consumer teardown; native server output has no browser-side native handles. */
    signal: AbortSignal;
    /** Supply a full authoritative patch transaction after publication has required recovery. */
    recovery?: boolean;
}

/** Publish a completed Edge response. Never apply progress frames before its terminal success. */
export function publishEdgeCssDomUpdate(options: CemEdgeCssPublicationOptions) {
    const { frames, result, previousEtag } = structuredClone({ frames: options.frames,
        result: options.result, previousEtag: options.previousEtag });
    const queue = CemCssDomPublicationQueue.forElement(options.element);
    const prepare = async () => {
        const state = result.stylesheets;
        const address = result.renderState.currentStylesheets;
        const actualAddress = state ? edgeContentAddress('stylesheets', state) : undefined;
        if (!state || !address || state.kind !== 'native-ssr-stylesheets-v1' || state.batch.kind !== 'native-css-batch-v1'
            || state.batch.inputKey !== state.inputKey
            || actualAddress?.key !== address.key || actualAddress?.digest !== address.digest
            || actualAddress?.algorithm !== address.algorithm || address.kind !== 'stylesheets') {
            throw new Error('Edge stylesheet state does not match its committed content address');
        }
        const revision: RenderRevision = result.renderPlanIdentity;
        let adopted = false;
        const currentRevision = () => {
            if (options.signal.aborted || options.scope.disposed || options.currentState().etag !== (adopted ? result.renderState.etag : previousEtag)) {
                throw new Error('stale Edge CSS/DOM transaction');
            }
            return revision;
        };
        currentRevision();
        if (renderRevisionKey(revision) !== renderRevisionKey(result.renderState.renderRevision)
            || result.renderPlanIdentity.producedTag !== options.element.localName
            || result.renderState.stateKey !== options.currentState().stateKey) {
            throw new Error('Edge render state does not match the consumer transaction');
        }
        const batches = new Map(state.batch.declarations.map(batch => [batch.declarationIdentity, batch]));
        if (batches.size !== state.batch.declarations.length
            || [...batches.keys()].some(identity => !options.declarations.has(identity))) {
            throw new Error('Edge stylesheet declaration ownership is incomplete');
        }
        const ready = (style: typeof state.batch.instance[number]) => ({ status: 'ready' as const,
            loadId: `edge:${style.identity.cacheKey}`, css: style.css, identity: style.identity, diagnostics: [] });
        if (state.batch.instance.some(style => style.scope.kind !== 'instance')) throw new Error('invalid instance stylesheet scope');
        const outputs = state.batch.instance.map(style => ({ index: style.index, scope: { kind: 'instance' as const }, output: ready(style) }));
        const entries: DeclarationStylesheetCommit[] = [];
        const styles = prepareEmittedInstanceStylesheets({ element: options.element, signal: options.signal, outputs });
        try {
            for (const [identity, owner] of options.declarations) {
                const batch = batches.get(identity);
                const emitted = (batch?.styles ?? []).map(style => {
                    if (style.scope.kind === 'instance' || (style.scope.kind === 'private' && style.scope.tag !== options.element.localName)
                        || (style.scope.kind === 'shared' && style.scope.name !== options.element.getAttribute('scope'))) {
                        throw new Error('Edge stylesheet scope does not match its consumer');
                    }
                    return { index: style.index, scope: style.scope, output: ready(style) };
                });
                const lease = owner.stageConsumer(options.element, options.scope);
                const cancel = () => lease.release();
                options.signal.addEventListener('abort', cancel, { once: true });
                entries.push({ lease, outputs: emitted, isCurrent: () => !options.signal.aborted,
                    release: () => options.signal.removeEventListener('abort', cancel) });
            }
            await styles.ready;
            return { entries, instanceStyles: styles, signal: options.signal,
                patch: preparePatchFramesForRange(options.bounds, frames, revision, options.element.ownerDocument),
                currentRevision,
                onPublished: () => { adopted = true; options.adopt(result.renderState); } };
        } catch (error) {
            for (const entry of entries) { entry.lease.release(); entry.release(); }
            styles.cancelPreparation();
            throw error;
        }
    };
    return options.recovery ? queue.recoverPublication(prepare) : queue.publish(prepare);
}
