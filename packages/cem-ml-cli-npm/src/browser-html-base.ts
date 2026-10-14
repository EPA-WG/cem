import * as runtime from '@epa-wg/cem-ml/wasm';

import { BrowserStylesheetHost, type BrowserStylesheetResponse, type BrowserStylesheetOptions, type BrowserStylesheetSelectionOptions } from './browser-stylesheets.js';

import { BrowserDomCorrespondence, claimDocument, documentOwned, releaseDocument } from './browser-dom-correspondence.js';

export interface BrowserHtmlBaseOptions {
    readonly profile: 'standard-agreement' | 'chromium-148';
    readonly browser: { readonly engine: string; readonly version: string };
    /** All history, policy and document writes must run through changeContext.
     * MutationObserver cannot reconstruct unobserved synchronous history A-B-A. */
    readonly contextOwnership: 'exclusive';
    readonly maxNodes?: number;
    readonly maxUrlBytes?: number;
}
export interface BrowserHtmlBasePolicy {
    readonly stamp: string;
    /** Execution host's current CSP/base policy, not an authored markup callback. */
    readonly decide: (candidate: string, fallback: string, stamp: string) => boolean;
}

/** Native import must already own a DOM-equivalent, materialized execution tree.
 * This host compares live DOM slots; it never imports HTML or rebuilds an AST. */
export async function createBrowserHtmlBaseHost(document: Document, options: BrowserHtmlBaseOptions): Promise<BrowserHtmlBaseHost> {
    await (runtime as unknown as { default: () => Promise<unknown> }).default();
    return new BrowserHtmlBaseHost(document, options);
}

/** Bounded exclusive host for ordinary UTF-8 HTTP(S) documents. Creation does not
 * certify an existing freeze. capture explicitly reassigns the first href before
 * confirming native evidence. No resource-load closure or edit grant is implied. */
export class BrowserHtmlBaseHost {
    readonly #stylesheets = new Set<BrowserStylesheetHost>();
    readonly #document: Document;
    readonly #native: runtime.HtmlBaseExecutionHost;
    readonly #observer: MutationObserver;
    readonly #maxNodes: number;
    readonly #dom: BrowserDomCorrespondence;
    #url: string;
    #busy = false;
    #pending?: AbortController;
    #revoked = false;
    #disposed = false;
    #captured = false;
    readonly #onPageHide = () => this.#revoke();

    /** Use createBrowserHtmlBaseHost to initialize WASM first. */
    constructor(document: Document, options: BrowserHtmlBaseOptions) {
        this.#document = document;
        this.#maxNodes = options.maxNodes ?? 10_000;
        const bytes = options.maxUrlBytes ?? 16_384;
        if (options.contextOwnership !== 'exclusive' || documentOwned(document)
            || !Number.isSafeInteger(this.#maxNodes) || this.#maxNodes < 1 || this.#maxNodes > 1_000_000
            || !Number.isSafeInteger(bytes) || bytes < 1 || bytes > 16_777_216
            || document.contentType !== 'text/html' || document.characterSet !== 'UTF-8'
            || !/^https?:/u.test(document.URL) || !document.defaultView || document.defaultView.document !== document) {
            throw new TypeError('HTML base host requires an unowned, exclusive UTF-8 HTTP(S) document and bounded limits');
        }
        this.#dom = new BrowserDomCorrespondence(document, this.#maxNodes);
        this.#native = new runtime.HtmlBaseExecutionHost(options.profile, options.browser.engine, options.browser.version, this.#maxNodes, bytes);
        this.#url = document.URL;
        this.#observer = new MutationObserver((records) => { if (records.length) this.#revoke(); });
        this.#observer.observe(document, { subtree: true, attributes: true, childList: true, characterData: true });
        document.defaultView.addEventListener('pagehide', this.#onPageHide);
        claimDocument(document, this);
    }

    /** A synchronous exclusive write boundary. Invalidates before running code,
     * even if its final URL, policy or DOM values equal their original values.
     * Supply the resulting native execution snapshot at the next capture. */
    changeContext(change: () => void): void {
        this.#run(() => {
            this.#disposeStylesheets();
            this.#native.invalidate();
            this.#captured = false;
            const result: unknown = change();
            if (result !== undefined) throw new TypeError('HTML base context changes must be synchronous void callbacks');
            this.#observer.takeRecords();
            this.#url = this.#document.URL;
            this.#checkDocument();
        });
    }

    /** Perform an actual fresh freeze using a retained native document handle.
     * Only the selected href is set, including a same-value assignment. Empty
     * templates represent inert content outside this document topology. */
    capture(nativeDocument: number, policy: BrowserHtmlBasePolicy): string {
        return this.#run(() => {
            this.#disposeStylesheets();
            this.#captured = false;
            this.#dom.reset();
            const stamp = policy.stamp;
            const selected = this.#native.prepare(nativeDocument, this.#document.URL, stamp, this.#dom.matchSlot, policy.decide);
            // A policy callback must not mutate or reenter the host.
            this.#checkQuiet();
            if (policy.stamp !== stamp) throw new Error('HTML base policy changed during preparation');
            if (selected) {
                const attribute = this.#dom.handle(selected) as Attr | undefined;
                const element = attribute?.ownerElement;
                if (!attribute || attribute.nodeType !== 2 || !element) throw new Error('Selected native href has no live DOM attribute');
                element.setAttributeNS(null, 'href', attribute.value);
                const records = this.#observer.takeRecords();
                if (records.length !== 1 || records[0]?.type !== 'attributes'
                    || records[0]?.target !== element || records[0]?.attributeName !== 'href'
                    || records[0]?.attributeNamespace !== null) throw new Error('Unexpected DOM change during HTML base freeze');
            }
            const result = this.#native.confirm(this.#document.baseURI);
            this.#captured = true;
            // Native correspondence and identities are rechecked after the write.
            this.#dom.reset();
            this.#native.validate(this.#dom.matchSlot, this.#document.baseURI);
            return result;
        });
    }

    /** Consume only a current, synchronously revalidated read set. */
    read(): string {
        if (this.#pending) throw new Error('HTML stylesheet transition pending');
        if (this.#busy) { this.#revoke(); throw new Error('HTML base host reentry rejected'); }
        this.#checkQuiet();
        if (!this.#captured) throw new Error('HTML base host needs a fresh capture after a context change');
        return this.#run(() => {
            this.#dom.reset();
            return this.#native.validate(this.#dom.matchSlot, this.#document.baseURI);
        });
    }

    /** Capture the bounded native HTML CSS forest against its live installation. */
    captureStylesheets(responses: readonly BrowserStylesheetResponse[], options: BrowserStylesheetOptions): BrowserStylesheetHost {
        this.read();
        // Dispose superseded sessions before publishing a replacement.
        this.#disposeStylesheets();
        const host = this.#run(() => new BrowserStylesheetHost({
            document: this.#document, native: this.#native, matches: this.#dom.matchSlot,
            node: (id) => this.#dom.node(id), run: (work) => this.#run(work),
        }, responses, options));
        try {
            host.capture();
            this.#stylesheets.add(host);
            return host;
        } catch (error) { host.dispose(); throw error; }
    }
    /** Explicit external titled-set activation. Empty title disables all titled
     * sheets. No read/capture/write is admitted while links are loading. */
    async selectStylesheets(nativeDocument: number, policy: BrowserHtmlBasePolicy, responses: readonly BrowserStylesheetResponse[], title: string, options: BrowserStylesheetSelectionOptions): Promise<BrowserStylesheetHost> {
        this.read();
        this.#disposeStylesheets();
        const lifetime = new AbortController();
        const signal = options.signal ? AbortSignal.any([options.signal, lifetime.signal]) : lifetime.signal;
        let host: BrowserStylesheetHost | undefined;
        try {
            host = this.#run(() => new BrowserStylesheetHost({
                document: this.#document, native: this.#native, matches: this.#dom.matchSlot,
                node: (id) => this.#dom.node(id), run: (work) => this.#run(work),
            }, responses, { ownership: options.ownership, signal }));
            const preparedHost = host;
            const selection = this.#run(() => preparedHost.prepareSelection(title, options.timeoutMs ?? 10_000));
            let loaded: Promise<void> | undefined;
            this.changeContext(() => { loaded = selection.start(); });
            this.#pending = lifetime;
            await loaded;
            this.#checkQuiet();
            if (signal.aborted) throw new Error('Stylesheet selection lifecycle ended');
            this.#pending = undefined;
            this.capture(nativeDocument, policy);
            this.#run(() => selection.finish());
            host.capture();
            this.#stylesheets.add(host);
            return host;
        } catch (error) {
            host?.dispose();
            this.#revoke();
            throw error;
        } finally {
            this.#pending = undefined;
        }
    }
    #disposeStylesheets(): void {
        for (const host of this.#stylesheets) host.dispose();
        this.#stylesheets.clear();
    }

    dispose(): void {
        if (this.#disposed) return;
        this.#revoke();
        if (this.#busy) throw new Error('HTML base host cannot be disposed during a callback');
        this.#disposed = true;
        this.#native.free();
        this.#document.defaultView?.removeEventListener('pagehide', this.#onPageHide);
        releaseDocument(this.#document, this);
        this.#dom.reset();
    }

    #run<T>(work: () => T): T {
        if (this.#pending) throw new Error('HTML stylesheet transition pending');
        if (this.#busy) { this.#revoke(); throw new Error('HTML base host reentry rejected'); }
        this.#checkQuiet();
        this.#busy = true;
        try {
            const result = work();
            this.#checkQuiet();
            return result;
        } catch (error) {
            this.#revoke();
            throw error;
        } finally {
            this.#busy = false;
            if (this.#revoked) this.#native.revoke();
        }
    }
    #checkDocument(): void {
        const doc = this.#document;
        if (doc.defaultView?.document !== doc || doc.contentType !== 'text/html'
            || doc.characterSet !== 'UTF-8' || !/^https?:/u.test(doc.URL) || doc.URL !== this.#url) {
            throw new Error('HTML base execution document or unowned context changed');
        }
    }
    #checkQuiet(): void {
        if (this.#revoked || this.#disposed) throw new Error('HTML base host session revoked');
        try {
            this.#checkDocument();
            if (this.#observer.takeRecords().length) throw new Error('Unowned HTML execution DOM mutation');
        } catch (error) { this.#revoke(); throw error; }
    }
    #revoke(): void {
        this.#pending?.abort();
        this.#disposeStylesheets();
        this.#revoked = true;
        this.#captured = false;
        this.#observer.disconnect();
        if (!this.#busy && !this.#disposed) this.#native.revoke();
    }
}
