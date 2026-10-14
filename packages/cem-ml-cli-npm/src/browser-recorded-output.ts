import { BrowserImageDelivery } from './browser-image-delivery.js';
import { BrowserImageObservation, type ImagePredicate, type NativeImagePreparation } from './browser-images.js';
import { BrowserDomCorrespondence, claimDocument, documentOwned, releaseDocument } from './browser-dom-correspondence.js';
import { BrowserStylesheetObservation, type StylesheetPredicate } from './browser-stylesheet-observation.js';
import type { BrowserHtmlBasePolicy } from './browser-html-base.js';
type NativeDomPredicate = BrowserDomCorrespondence['matchSlot'];

/** Actual realm-local QL WASM session; result messages cannot replace this owner. */
export interface NativeRecordedOutputSession extends NativeImagePreparation {
    enableImageReplay(nonce: string): void;
    imageReplayRoutes(route: (alias: string, source: string) => void): void;
    url(): string;
    html(): string;
    prepareBase(matches: NativeDomPredicate, decide: (candidate: string, fallback: string, stamp: string) => boolean): number;
    compose(base: string, matches: NativeDomPredicate): string;
    prepareStylesheets(base: string, matches: NativeDomPredicate, title: string | undefined, command: (owner: number, link: boolean, selected: boolean) => void): void;
    capture(base: string, matches: NativeDomPredicate, revision: () => string, observe: StylesheetPredicate, images: ImagePredicate): string;
    read(base: string, matches: NativeDomPredicate, revision: () => string, observe: StylesheetPredicate, images: ImagePredicate): string;
    close(): void;
    free(): void;
}
export interface BrowserRecordedOutputOptions {
    readonly ownership: 'exclusive';
    readonly policy: BrowserHtmlBasePolicy;
    readonly signal?: AbortSignal;
    /** Async capture only: exact external title, or empty to disable all titles. */
    readonly stylesheetTitle?: string;
    /** Opt-in pre-delivery capture through an already controlling shared worker. */
    readonly imageDelivery?: 'service-worker' | 'service-worker-replay';
    /** Async capture's whole load deadline; 1..30,000 ms. */
    readonly timeoutMs?: number;
}
export interface BrowserRecordedOutputSummary { readonly blockers: readonly string[]; readonly imageAcquisitions: number; readonly imageDeliveries: number; }
/** The installer delivers native HTML into this document synchronously. It must
 * preserve the Document/URL and must not start unowned context or CSS mutations. */
export type RecordedOutputInstaller = (html: string, stage: 'original' | 'composed') => void;

export class BrowserRecordedOutputHost {
    readonly #document: Document;
    readonly #view: Window;
    readonly #native: NativeRecordedOutputSession;
    readonly #dom: BrowserDomCorrespondence;
    readonly #observer: MutationObserver;
    readonly #options: BrowserRecordedOutputOptions;
    readonly #url: string;
    readonly #stamp: string;
    readonly #observation: BrowserStylesheetObservation;
    readonly #images: BrowserImageObservation;
    #delivery?: BrowserImageDelivery;
    readonly #initialLoads = new Set<Element>();
    readonly #waitingLinks = new Set<Element>();
    #cancelWait?: () => void;
    #pending = false;
    #started = false;
    #busy = false;
    #disposed = false;
    #freed = false;
    #captured = false;
    readonly #revoke = () => this.dispose();
    readonly #load = (event: Event) => {
        const node = event.target as Element;
        if (event.isTrusted && this.#pending && (this.#waitingLinks.has(node) || this.#initialLoads.delete(node))) {
            try { this.#quiet(); } catch { this.dispose(); }
            return;
        }
        if (!event.isTrusted || this.#busy || !this.#captured || !this.#initialLoads.delete(node)) { this.dispose(); return; }
        // Inline sheets are synchronously installed, but their initial load event
        // may arrive later. Preserve the record only after complete native reread.
        try { this.read(); } catch { this.dispose(); }
    };
    constructor(document: Document, native: NativeRecordedOutputSession, options: BrowserRecordedOutputOptions) {
        const view = document.defaultView;
        if ((options.imageDelivery !== undefined && options.imageDelivery !== 'service-worker' && options.imageDelivery !== 'service-worker-replay')
            || !view || view.document !== document || documentOwned(document) || options.ownership !== 'exclusive'
            || options.signal?.aborted || document.contentType !== 'text/html' || document.characterSet !== 'UTF-8'
            || !/^https?:/u.test(document.URL) || document.URL !== native.url()) throw new Error('Recorded output requires an exclusive live UTF-8 HTTP(S) document matching its native URL');
        this.#document = document; this.#view = view; this.#native = native; this.#options = options;
        this.#url = document.URL; this.#stamp = options.policy.stamp;
        this.#dom = new BrowserDomCorrespondence(document, 10_000);
        this.#images = new BrowserImageObservation(document, id => this.#dom.node(id), () => this.#quiet(), this.#revoke, work => this.#run(work));
        this.#observation = new BrowserStylesheetObservation(document, id => this.#dom.node(id), this.#revoke);
        this.#observer = new MutationObserver(records => { if (records.length) this.dispose(); });
        this.#observer.observe(document, { subtree: true, attributes: true, characterData: true, childList: true });
        view.addEventListener('pagehide', this.#revoke); view.addEventListener('resize', this.#revoke); view.addEventListener('orientationchange', this.#revoke);
        document.addEventListener('load', this.#load, true); document.addEventListener('error', this.#revoke, true);
        options.signal?.addEventListener('abort', this.#revoke, { once: true });
        claimDocument(document, this);
    }
    capture(install: RecordedOutputInstaller): BrowserRecordedOutputSummary {
        if (this.#options.imageDelivery) throw new Error('Image delivery capture requires async preparation');
        return this.#run(() => {
            if (this.#started) throw new Error('Recorded output already started');
            this.#started = true;
            if (this.#options.stylesheetTitle !== undefined) throw new Error('Title selection requires asynchronous capture');
            this.#deliver(this.#native.html(), 'original', install);
            this.#freeze();
            const composed = this.#native.compose(this.#document.baseURI, this.#match);
            this.#quiet();
            this.#deliver(composed, 'composed', install);
            this.#freeze();
            this.#native.prepareStylesheets(this.#document.baseURI, this.#match, undefined, () => { throw new Error('Unexpected selection command'); });
            const summary = this.#summary(this.#native.capture(this.#document.baseURI, this.#match, this.#revision, this.#observe, this.#images.observe));
            this.#observation.validateSeen();
            this.#captured = true;
            return summary;
        });
    }
    /** Own the document through initial loading and explicit native title commands.
     * No evidence is published until all asynchronous work has settled. */
    async captureAsync(install: RecordedOutputInstaller): Promise<BrowserRecordedOutputSummary> {
        const timeout = this.#options.timeoutMs ?? 5000;
        const deadline = performance.now() + timeout;
        const title = this.#options.stylesheetTitle;
        try {
            this.#run(() => {
                if (this.#started || !Number.isSafeInteger(timeout) || timeout < 1 || timeout > 30_000) throw new Error('Fresh bounded asynchronous capture required');
                this.#started = true; this.#pending = true;
                if (this.#options.imageDelivery) this.#native.beginImageDelivery();
            });
            if (this.#options.imageDelivery) {
                this.#delivery = new BrowserImageDelivery(this.#document,
                    (url, type, bytes) => this.#run(() => this.#native.acceptImageDelivery(url, type, bytes)), this.#revoke);
                await this.#delivery.open(deadline, this.#options.imageDelivery === 'service-worker-replay');
                if (this.#options.imageDelivery === 'service-worker-replay') this.#run(() => this.#native.enableImageReplay(this.#delivery?.nonce ?? ''));
            }
            const composed = this.#run(() => {
                this.#deliver(this.#native.html(), 'original', install);
                this.#freeze();
                return this.#native.compose(this.#document.baseURI, this.#match);
            });
            if (this.#options.imageDelivery === 'service-worker-replay') {
                const routes: [string, string][] = [];
                this.#run(() => this.#native.imageReplayRoutes((alias, source) => { routes.push([alias, source]); }));
                await this.#delivery?.configure(routes, deadline);
            }
            this.#run(() => {
                this.#deliver(composed, 'composed', install);
                this.#freeze();
            });
            const links = [...this.#document.querySelectorAll('link')].filter(node => [...node.relList].some(t => t.toLowerCase() === 'stylesheet'));
            if (links.length > 256) throw new Error('Recorded stylesheet root bound');
            const loaded = await this.#waitLinks(links, deadline);
            for (const [node, sheet] of loaded) if (node.sheet !== sheet) throw new Error('Initial stylesheet installation changed');
            const actions: { node: HTMLStyleElement | HTMLLinkElement; selected: boolean; sheet: CSSStyleSheet }[] = [];
            this.#run(() => {
                this.#native.prepareStylesheets(this.#document.baseURI, this.#match, title,
                    (owner, link, selected) => {
                        this.#quiet();
                        const node = this.#dom.node(owner) as HTMLStyleElement | HTMLLinkElement;
                        if (!node || node.localName !== (link ? 'link' : 'style') || !node.sheet || node.sheet.ownerNode !== node) throw new Error('Loaded native selection owner required');
                        actions.push({ node, selected, sheet: node.sheet });
                    });
            });
            const selected = actions.filter(a => a.selected);
            const activated = await this.#waitLinks(selected.map(a => a.node as HTMLLinkElement), deadline, () => {
                for (const action of actions) {
                    if (!action.selected) action.sheet.disabled = true;
                    else {
                        const link = action.node as HTMLLinkElement;
                        link.disabled = true; link.disabled = false;
                    }
                }
                // Only the native command's disabled toggles may be consumed.
                const mutations = this.#observer.takeRecords();
                if (mutations.some(r => r.type !== 'attributes' || r.attributeName !== 'disabled' || r.attributeNamespace !== null
                    || !selected.some(a => a.node === r.target))) throw new Error('Unexpected selection mutation');
            });
            await this.#images.prepare(this.#native, deadline, this.#revision, this.#delivery);
            return this.#run(() => {
                const active = new Map<Node, CSSStyleSheet>();
                for (const action of actions) {
                    const sheet = action.node.sheet;
                    if (!sheet || sheet.disabled !== !action.selected || sheet.ownerNode !== action.node
                        || (action.selected ? sheet === action.sheet || sheet !== activated.get(action.node as HTMLLinkElement) : sheet !== action.sheet)) throw new Error('Native selection did not install the expected stylesheet');
                    if (action.selected) active.set(action.node, sheet);
                }
                if (title !== undefined) {
                    this.#observation.activeTitled = active;
                    this.#observation.titledDisabled = title === '';
                }
                const result = this.#summary(this.#native.capture(this.#document.baseURI, this.#match, this.#revision, this.#observe, this.#images.observe));
                this.#observation.validateSeen();
                this.#captured = true; this.#pending = false;
                return result;
            });
        } catch (error) { this.dispose(); throw error; }
    }
    #waitLinks(nodes: HTMLLinkElement[], deadline: number, write?: () => void): Promise<Map<HTMLLinkElement, CSSStyleSheet>> {
        this.#quiet();
        return new Promise((resolve, reject) => {
            const cleanup: (() => void)[] = [];
            const loadedSheets = new Map<HTMLLinkElement, CSSStyleSheet>();
            let remaining = nodes.length, settled = false;
            const finish = (error?: Error) => {
                if (settled) return;
                settled = true; cleanup.forEach(remove => remove());
                this.#waitingLinks.clear(); this.#cancelWait = undefined;
                if (error) reject(error); else resolve(loadedSheets);
            };
            this.#cancelWait = () => finish(new Error('Recorded stylesheet loading cancelled'));
            const timer = setTimeout(() => finish(new Error('Recorded stylesheet load timed out')), Math.max(0, deadline - performance.now()));
            cleanup.push(() => clearTimeout(timer));
            try {
                if (performance.now() >= deadline) throw new Error('Recorded stylesheet deadline elapsed');
                for (const node of nodes) {
                    this.#waitingLinks.add(node);
                    const loaded = (event: Event) => {
                        if (!event.isTrusted || event.target !== node) { this.dispose(); return; }
                        try {
                            this.#quiet();
                            if (!node.sheet || node.sheet.ownerNode !== node) throw new Error('Recorded stylesheet did not load');
                            loadedSheets.set(node, node.sheet);
                            this.#waitingLinks.delete(node);
                            node.removeEventListener('load', loaded);
                            if (--remaining === 0) finish();
                        } catch (error) { finish(error instanceof Error ? error : new Error(String(error))); }
                    };
                    node.addEventListener('load', loaded);
                    cleanup.push(() => node.removeEventListener('load', loaded));
                }
                write?.();
                this.#quiet();
                if (!remaining) finish();
            } catch (error) { finish(error instanceof Error ? error : new Error(String(error))); }
        });
    }
    read(): BrowserRecordedOutputSummary {
        return this.#run(() => {
            if (!this.#captured) throw new Error('Recorded output capture required');
            return this.#summary(this.#native.read(this.#document.baseURI, this.#match, this.#revision, this.#observe, this.#images.observe));
        });
    }
    /** Revoke before all controlled writes, including synchronous A-B-A changes. */
    change(change: () => void): void {
        try { this.#quiet(); } catch (error) { this.dispose(); throw error; }
        if (this.#busy) { this.dispose(); throw new Error('Recorded output reentry'); }
        this.dispose();
        const result: unknown = change();
        if (result !== undefined) throw new Error('Recorded output changes must be synchronous');
    }
    dispose(): void {
        if (this.#disposed) return;
        this.#disposed = true; this.#captured = false; this.#observer.disconnect();
        this.#view.removeEventListener('pagehide', this.#revoke); this.#view.removeEventListener('resize', this.#revoke); this.#view.removeEventListener('orientationchange', this.#revoke);
        this.#document.removeEventListener('load', this.#load, true); this.#document.removeEventListener('error', this.#revoke, true);
        this.#options.signal?.removeEventListener('abort', this.#revoke);
        this.#delivery?.dispose(); releaseDocument(this.#document, this); this.#cancelWait?.(); this.#images.dispose(); this.#observation.dispose(); this.#initialLoads.clear();
        if (!this.#busy) this.#free();
    }
    #free(): void { if (!this.#freed) { this.#freed = true; try { this.#native.close(); } finally { this.#native.free(); } } }
    #run<T>(work: () => T): T {
        if (this.#busy) { this.dispose(); throw new Error('Recorded output reentry'); }
        this.#busy = true;
        try { this.#quiet(); const result = work(); this.#quiet(); return result; }
        catch (error) { this.dispose(); throw error; }
        finally { this.#busy = false; if (this.#disposed) this.#free(); }
    }
    #quiet(): void {
        this.#delivery?.validate();
        if (this.#disposed || this.#options.signal?.aborted || this.#document.defaultView?.document !== this.#document
            || this.#document.URL !== this.#url || this.#options.policy.stamp !== this.#stamp
            || this.#observer.takeRecords().length) throw new Error('Recorded document, policy or lifetime changed');
    }
    #deliver(html: string, stage: 'original' | 'composed', install: RecordedOutputInstaller): void {
        if (html.length > 4 * 1024 * 1024) throw new Error('Recorded HTML delivery limit');
        const result: unknown = install(html, stage);
        if (result !== undefined) throw new Error('Recorded HTML installation must be synchronous');
        this.#observer.takeRecords();
        this.#quiet(); this.#dom.reset();
        this.#initialLoads.clear();
        for (const node of this.#document.querySelectorAll('style, img')) this.#initialLoads.add(node);
    }
    #freeze(): void {
        const selected = this.#native.prepareBase(this.#match, (candidate, fallback, stamp) => {
            this.#quiet();
            if (stamp !== this.#stamp) throw new Error('Native base policy stamp mismatch');
            const result = this.#options.policy.decide(candidate, fallback, stamp);
            this.#quiet(); return result;
        });
        this.#quiet();
        if (selected) {
            const attr = this.#dom.handle(selected) as Attr | undefined;
            const element = attr?.ownerElement;
            if (!attr || attr.nodeType !== 2 || !element) throw new Error('Native selected base attribute missing');
            element.setAttributeNS(null, 'href', attr.value);
            const records = this.#observer.takeRecords();
            if (records.length !== 1 || records[0]?.target !== element || records[0]?.attributeName !== 'href'
                || records[0]?.attributeNamespace !== null || records[0]?.type !== 'attributes') throw new Error('Unexpected mutation during recorded base freeze');
        }
        this.#quiet();
    }
    readonly #match = (...args: Parameters<BrowserDomCorrespondence['matchSlot']>): number => {
        this.#quiet(); const result = this.#dom.matchSlot(...args); this.#quiet(); return result;
    };
    readonly #revision = (): string => {
        this.#quiet();
        this.#observation.validateUniverse();
        return `${this.#view.innerWidth}:${this.#view.innerHeight}:${this.#view.devicePixelRatio}`;
    };
    readonly #observe: StylesheetPredicate = (...args) => {
        this.#quiet();
        const result = this.#observation.observe(...args);
        this.#quiet(); return result;
    };
    #summary(json: string): BrowserRecordedOutputSummary {
        const value = JSON.parse(json) as BrowserRecordedOutputSummary;
        if (!Array.isArray(value.blockers) || !value.blockers.every(b => typeof b === 'string')
            || !Number.isInteger(value.imageDeliveries) || value.imageDeliveries < 0 || value.imageDeliveries > value.imageAcquisitions
            || !Number.isInteger(value.imageAcquisitions) || value.imageAcquisitions < 0 || value.imageAcquisitions > 256) throw new Error('Invalid native preparation diagnostic summary');
        return Object.freeze({ blockers: Object.freeze([...value.blockers]), imageAcquisitions: value.imageAcquisitions, imageDeliveries: value.imageDeliveries });
    }
}
/** Takes ownership of the native session, including failed construction/capture. */
export function captureBrowserRecordedOutput(document: Document, native: NativeRecordedOutputSession, install: RecordedOutputInstaller, options: BrowserRecordedOutputOptions): BrowserRecordedOutputHost {
    let host: BrowserRecordedOutputHost | undefined;
    try { host = new BrowserRecordedOutputHost(document, native, options); host.capture(install); return host; }
    catch (error) { if (host) host.dispose(); else { try { native.close(); } finally { native.free(); } } throw error; }
}

/** Takes ownership immediately, including cancellation during pending loads. */
export async function captureBrowserRecordedOutputAsync(document: Document, native: NativeRecordedOutputSession, install: RecordedOutputInstaller, options: BrowserRecordedOutputOptions): Promise<BrowserRecordedOutputHost> {
    let host: BrowserRecordedOutputHost | undefined;
    try { host = new BrowserRecordedOutputHost(document, native, options); await host.captureAsync(install); return host; }
    catch (error) { if (host) host.dispose(); else { try { native.close(); } finally { native.free(); } } throw error; }
}
