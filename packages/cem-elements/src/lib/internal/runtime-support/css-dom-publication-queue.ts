import { DeclarationStyleOwnership, type DeclarationStylesheetCommit,
    type DeclarationStylesheetPatchResult } from '../../declaration-style-ownership.js';
import type { PreparedPatchFrames, RenderRevision } from '../../projection.js';
import type { CemPreparedStylesheetConnection } from './stylesheet-registry.js';
import type { PreparedInstanceStylesheets } from './instance-stylesheet-installation.js';

export interface PreparedCssDomPublication {
    entries: readonly DeclarationStylesheetCommit[];
    patch: PreparedPatchFrames;
    currentRevision(): RenderRevision;
    signal?: AbortSignal;
    instanceStyles?: PreparedInstanceStylesheets;
    registryConnection?: CemPreparedStylesheetConnection;
}

const queues = new WeakMap<HTMLElement, CemCssDomPublicationQueue>();

/** Serializes preparation, publication and explicit recovery for one host. */
export class CemCssDomPublicationQueue {
    private readonly jobs: Array<() => Promise<void>> = [];
    private running = false;
    private blocked = false;

    private constructor(private readonly element: HTMLElement) {}

    static forElement(element: HTMLElement): CemCssDomPublicationQueue {
        let queue = queues.get(element);
        if (!queue) { queue = new CemCssDomPublicationQueue(element); queues.set(element, queue); }
        return queue;
    }

    get recoveryRequired(): boolean { return this.blocked; }

    /** Create leases inside prepare; it owns cleanup if it fails before returning. */
    publish(prepare: () => PreparedCssDomPublication | Promise<PreparedCssDomPublication>): Promise<DeclarationStylesheetPatchResult> {
        return this.schedule(async () => {
            if (this.blocked) return { status: 'rejected', errors: [], diagnostics: [{
                code: 'cem.css_dom.recovery_required', severity: 'error',
                message: 'authoritative recovery must complete before preparing another update',
            }] };
            let request: PreparedCssDomPublication;
            try { request = await prepare(); }
            catch (error) { return { status: 'rejected', diagnostics: [], errors: [error] }; }
            let result: DeclarationStylesheetPatchResult;
            try {
                const invalidHost = request.patch.container !== this.element;
                const rejected = new AbortController();
                if (invalidHost) rejected.abort();
                result = DeclarationStyleOwnership.commitGroupWithPatch(request.entries, request.patch,
                    request.currentRevision, invalidHost ? rejected.signal : request.signal, request.instanceStyles, request.registryConnection);
            } catch (error) {
                // An unexpected exception cannot establish that publication never began.
                result = { status: 'recovery-required', diagnostics: [], errors: [error] };
            }
            if (result.status === 'recovery-required') this.blocked = true;
            return result;
        });
    }

    /** The caller restores authoritative state; failed recovery leaves the queue blocked. */
    recover(restore: () => void | Promise<void>): Promise<void> {
        return this.schedule(async () => {
            if (!this.blocked) return;
            await restore();
            if (!this.element.isConnected) throw new Error('recovery requires a connected host');
            this.blocked = false;
        });
    }

    private schedule<T>(run: () => T | Promise<T>): Promise<T> {
        return new Promise<T>((resolve, reject) => {
            this.jobs.push(async () => { try { resolve(await run()); } catch (error) { reject(error); } });
            if (!this.running) {
                this.running = true;
                queueMicrotask(() => { void this.drain(); });
            }
        });
    }

    private async drain(): Promise<void> {
        while (this.jobs.length) await this.jobs.shift()?.();
        this.running = false;
    }
}
