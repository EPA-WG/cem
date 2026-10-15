import { BrowserSvgComputedObservation, type SvgComputedPredicate } from './browser-svg-computed.js';
import { BrowserImageDelivery } from './browser-image-delivery.js';
import { BrowserImageObservation, type ImagePredicate, type NativeImagePreparation } from './browser-images.js';
import { BrowserDomCorrespondence, claimDocument, documentOwned, releaseDocument } from './browser-dom-correspondence.js';
import { BrowserStylesheetObservation, type StylesheetPredicate } from './browser-stylesheet-observation.js';
import type { BrowserHtmlBasePolicy } from './browser-html-base.js';
type NativeDomPredicate = BrowserDomCorrespondence['matchSlot'];

/** Actual realm-local QL WASM session; result messages cannot replace this owner. */
export interface NativeRecordedOutputSession extends NativeImagePreparation {
    beginSvgDelivery(): void;
    acceptSvgDelivery(url: string, type: string, bytes: Uint8Array): void;
    beginSvgSnapshots(): void;
    addSvgSnapshot(requested: string, finalUrl: string, contentType: string, revision: string, bytes: Uint8Array): void;
    enableImageReplay(nonce: string): void;
    imageReplayRoutes(route: (alias: string, source: string) => void): void;
    url(): string;
    html(): string;
    prepareBase(matches: NativeDomPredicate, decide: (candidate: string, fallback: string, stamp: string) => boolean): number;
    compose(base: string, matches: NativeDomPredicate): string;
    prepareStylesheets(base: string, matches: NativeDomPredicate, title: string | undefined, command: (owner: number, link: boolean, selected: boolean) => void): void;
    enableBoundedPaintColors(): void;
    enableSvgReplayPlan(nonce: string): void;
    confirmCloneProfile(name: string, observe: (index: number) => readonly number[]): void;
    capture(base: string, matches: NativeDomPredicate, revision: () => string, observe: StylesheetPredicate, images: ImagePredicate, svg: SvgComputedPredicate): string;
    read(base: string, matches: NativeDomPredicate, revision: () => string, observe: StylesheetPredicate, images: ImagePredicate, svg: SvgComputedPredicate): string;
    close(): void;
    free(): void;
}
/** Trusted acquired entity bytes, never a browser response receipt. */
export interface BrowserSvgSnapshot {
    readonly requestedUrl: string;
    readonly finalUrl: string;
    readonly contentType: string;
    readonly revision: string;
    readonly bytes: Uint8Array;
}
export interface BrowserRecordedOutputOptions {
    /** Preparation-only closed presentation-author proof; never installs replay aliases. */
    readonly svgReplayPlan?: { readonly nonce: string };
    /** Async SVG image response capture; requires snapshots and a controlling worker.
     * Does not attest to fill/stroke paint consumption. Exclusive with imageDelivery. */
    readonly svgDelivery?: 'service-worker';
    /** Explicit immutable snapshot ingress. Abort when the acquisition lifetime ends. */
    readonly svgSnapshots?: { readonly responses: readonly BrowserSvgSnapshot[]; readonly signal: AbortSignal };

    /** Explicit compatibility report; not an execution seal. Probe order:
     * own/inherited/descriptive gradient, then own/inherited/descriptive pattern.
     * Each synchronous callback returns source RGBA, host RGBA, pixel RGBA. */
    readonly cloneProfile?: {
        readonly name: 'chromium-148-solid-currentcolor-v1';
        readonly observe: (index: number) => readonly number[];
    };
    /** Bounded native author-color derivation with installed-sheet checks; requires cloneProfile. */
    readonly boundedPaintColors?: boolean;
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
export interface BrowserRecordedOutputSummary { readonly blockers: readonly string[]; readonly imageAcquisitions: number; readonly imageDeliveries: number; readonly svgComputedSlots: number; readonly svgReplayCandidates: number; readonly svgResourceBindings: number; readonly svgDeliveredBindings: number; readonly svgPaintRoots: number; readonly svgPaintLinks: number; readonly svgPaintServers: number; readonly svgPaintContentNodes: number; readonly svgPaintStyleHosts: number; readonly svgPaintStyleInstances: number; readonly svgCloneProfileConfirmed: boolean; readonly svgPaintColors: number; readonly svgPaintSuppressedHosts: number; }
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
    readonly #svgSignal?: AbortSignal;
    readonly #svgUrls: readonly string[];
    readonly #svgLoads = new Set<Element>();
    readonly #svgLoadOwners = new Set<Element>();
    readonly #loadEvents = new WeakSet<Event>();
    #svgLoaded?: () => void;
    readonly #url: string;
    readonly #stamp: string;
    readonly #observation: BrowserStylesheetObservation;
    readonly #images: BrowserImageObservation;
    readonly #svg: BrowserSvgComputedObservation;
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
        if (this.#loadEvents.has(event)) return;
        this.#loadEvents.add(event);
        const node = event.target as Element;
        if (event.isTrusted && this.#pending && this.#svgLoads.delete(node)) {
            if (!this.#svgLoads.size) this.#svgLoaded?.();
        }
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
        if ((options.svgDelivery !== undefined && (options.svgDelivery !== 'service-worker' || !options.svgSnapshots || options.imageDelivery !== undefined))
            || (options.imageDelivery !== undefined && options.imageDelivery !== 'service-worker' && options.imageDelivery !== 'service-worker-replay')
            || !view || view.document !== document || documentOwned(document) || options.ownership !== 'exclusive'
            || options.signal?.aborted || options.svgSnapshots?.signal.aborted || document.contentType !== 'text/html' || document.characterSet !== 'UTF-8'
            || !/^https?:/u.test(document.URL) || document.URL !== native.url()) throw new Error('Recorded output requires an exclusive live UTF-8 HTTP(S) document matching its native URL');
        const snapshots = options.svgSnapshots;
        this.#svgSignal = snapshots?.signal;
        this.#svgUrls = Object.freeze(snapshots?.responses.map(response => response.requestedUrl) ?? []);
        if (snapshots) {
            if (!snapshots.signal || !Array.isArray(snapshots.responses) || snapshots.responses.length > 256) throw new Error('Bounded SVG snapshots and acquisition signal required');
            native.beginSvgSnapshots();
            let size = 0;
            const encoder = new TextEncoder();
            for (const response of snapshots.responses) {
                const metadata = [response.requestedUrl, response.finalUrl, response.contentType, response.revision];
                if (!metadata.every(v => typeof v === 'string') || !ArrayBuffer.isView(response.bytes)
                    || Object.prototype.toString.call(response.bytes) !== '[object Uint8Array]') throw new Error('SVG snapshot requires strings and entity bytes');
                size += response.bytes.byteLength + metadata.reduce((n, v) => n + encoder.encode(v).length, 0);
                if (size > 4 * 1024 * 1024 || snapshots.signal.aborted) throw new Error('SVG snapshot bounds or lifetime exceeded');
                native.addSvgSnapshot(response.requestedUrl, response.finalUrl, response.contentType, response.revision, response.bytes);
            }
        }
        this.#document = document; this.#view = view; this.#native = native; this.#options = options;
        this.#url = document.URL; this.#stamp = options.policy.stamp;
        this.#dom = new BrowserDomCorrespondence(document, 10_000);
        this.#svg = new BrowserSvgComputedObservation(document, id => this.#dom.node(id), () => this.#quiet(), this.#revoke);
        this.#images = new BrowserImageObservation(document, id => this.#dom.node(id), () => this.#quiet(), this.#revoke, work => this.#run(work));
        this.#observation = new BrowserStylesheetObservation(document, id => this.#dom.node(id), this.#revoke);
        this.#observer = new MutationObserver(records => { if (records.length) this.dispose(); });
        this.#observer.observe(document, { subtree: true, attributes: true, characterData: true, childList: true });
        view.addEventListener('pagehide', this.#revoke); view.addEventListener('resize', this.#revoke); view.addEventListener('orientationchange', this.#revoke);
        document.addEventListener('load', this.#load, true); document.addEventListener('error', this.#revoke, true);
        options.signal?.addEventListener('abort', this.#revoke, { once: true });
        this.#svgSignal?.addEventListener('abort', this.#revoke, { once: true });
        claimDocument(document, this);
    }
    capture(install: RecordedOutputInstaller): BrowserRecordedOutputSummary {
        if (this.#options.imageDelivery || this.#options.svgDelivery) throw new Error('Resource delivery capture requires async preparation');
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
            this.#confirmCloneProfile();
            const summary = this.#summary(this.#native.capture(this.#document.baseURI, this.#match, this.#revision, this.#observe, this.#images.observe, this.#svg.observe));
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
                if (this.#options.svgDelivery) this.#native.beginSvgDelivery();
            });
            if (this.#options.imageDelivery) {
                this.#delivery = new BrowserImageDelivery(this.#document,
                    (url, type, bytes) => this.#run(() => this.#native.acceptImageDelivery(url, type, bytes)), this.#revoke);
                await this.#delivery.open(deadline, this.#options.imageDelivery === 'service-worker-replay');
                if (this.#options.imageDelivery === 'service-worker-replay') this.#run(() => this.#native.enableImageReplay(this.#delivery?.nonce ?? ''));
            }
            if (this.#options.svgDelivery) {
                this.#delivery = new BrowserImageDelivery(this.#document,
                    (url, type, bytes) => this.#run(() => this.#native.acceptSvgDelivery(url, type, bytes)), this.#revoke);
                await this.#delivery.open(deadline, false, this.#svgUrls);
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
            await this.#images.prepare(this.#native, deadline, this.#revision, this.#options.imageDelivery ? this.#delivery : undefined);
            if (this.#options.svgDelivery) {
                await this.#waitSvgLoads(deadline);
                await this.#delivery?.seal(deadline);
            }
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
                this.#confirmCloneProfile();
                const result = this.#summary(this.#native.capture(this.#document.baseURI, this.#match, this.#revision, this.#observe, this.#images.observe, this.#svg.observe));
                this.#observation.validateSeen();
                this.#captured = true; this.#pending = false;
                return result;
            });
        } catch (error) { this.dispose(); throw error; }
    }
    #waitSvgLoads(deadline: number): Promise<void> {
        this.#quiet();
        if (!this.#svgLoads.size) return Promise.resolve();
        return new Promise((resolve, reject) => {
            const finish = (error?: Error) => {
                clearTimeout(timer); this.#svgLoaded = undefined; this.#cancelWait = undefined;
                if (error) reject(error); else resolve();
            };
            const timer = setTimeout(() => finish(new Error('SVG delivery load timed out')), Math.max(0, deadline - performance.now()));
            this.#svgLoaded = () => finish();
            this.#cancelWait = () => finish(new Error('SVG delivery cancelled'));
        });
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
            return this.#summary(this.#native.read(this.#document.baseURI, this.#match, this.#revision, this.#observe, this.#images.observe, this.#svg.observe));
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
        this.#svgSignal?.removeEventListener('abort', this.#revoke);
        this.#clearSvgListeners();
        this.#delivery?.dispose(); releaseDocument(this.#document, this); this.#cancelWait?.(); this.#images.dispose(); this.#svg.dispose(); this.#observation.dispose(); this.#initialLoads.clear(); this.#svgLoads.clear();
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
        if (this.#disposed || this.#svgSignal?.aborted || this.#options.signal?.aborted || this.#document.defaultView?.document !== this.#document
            || this.#document.URL !== this.#url || this.#options.policy.stamp !== this.#stamp
            || this.#observer.takeRecords().length) throw new Error('Recorded document, policy or lifetime changed');
    }
    #clearSvgListeners(): void {
        for (const node of this.#svgLoadOwners) {
            node.removeEventListener('load', this.#load);
            node.removeEventListener('error', this.#revoke);
        }
        this.#svgLoadOwners.clear();
    }
    #deliver(html: string, stage: 'original' | 'composed', install: RecordedOutputInstaller): void {
        if (html.length > 4 * 1024 * 1024) throw new Error('Recorded HTML delivery limit');
        const result: unknown = install(html, stage);
        if (result !== undefined) throw new Error('Recorded HTML installation must be synchronous');
        this.#observer.takeRecords();
        this.#quiet(); this.#dom.reset();
        this.#initialLoads.clear();
        for (const node of this.#document.querySelectorAll('style, img')) this.#initialLoads.add(node);
        this.#clearSvgListeners();
        this.#svgLoads.clear();
        if (this.#options.svgDelivery) {
            for (const node of this.#document.querySelectorAll('image')) {
                if (node.namespaceURI !== 'http://www.w3.org/2000/svg') continue;
                this.#initialLoads.add(node);
                node.addEventListener('load', this.#load);
                node.addEventListener('error', this.#revoke);
                this.#svgLoadOwners.add(node);
                if (stage === 'composed') this.#svgLoads.add(node);
            }
            if (this.#svgLoads.size > 256) throw new Error('SVG image load bound exceeded');
        }
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
    #confirmCloneProfile(): void {
        if (this.#options.svgReplayPlan) this.#native.enableSvgReplayPlan(this.#options.svgReplayPlan.nonce);
        this.#quiet();
        const profile = this.#options.cloneProfile;
        if (profile) this.#native.confirmCloneProfile(profile.name, index => {
            this.#quiet();
            const result = profile.observe(index);
            this.#quiet();
            return result;
        });
        this.#quiet();
        if (this.#options.boundedPaintColors) this.#native.enableBoundedPaintColors();
        this.#quiet();
    }
    #summary(json: string): BrowserRecordedOutputSummary {
        const value = JSON.parse(json) as BrowserRecordedOutputSummary;
        if (!Number.isInteger(value.svgDeliveredBindings) || value.svgDeliveredBindings < 0 || value.svgDeliveredBindings > value.svgResourceBindings
            || !Number.isInteger(value.svgReplayCandidates) || value.svgReplayCandidates < 0 || value.svgReplayCandidates > value.svgComputedSlots
            || !Number.isInteger(value.svgResourceBindings) || value.svgResourceBindings < 0 || value.svgResourceBindings > value.svgComputedSlots
            || typeof value.svgCloneProfileConfirmed !== 'boolean'
            || !Number.isInteger(value.svgPaintColors) || value.svgPaintColors < 0
            || !Number.isInteger(value.svgPaintSuppressedHosts) || value.svgPaintSuppressedHosts < 0
            || value.svgPaintColors + value.svgPaintSuppressedHosts > value.svgPaintStyleHosts
            || (!value.svgCloneProfileConfirmed && (value.svgPaintColors !== 0 || value.svgPaintSuppressedHosts !== 0))
            || !Number.isInteger(value.svgPaintRoots) || value.svgPaintRoots < 0 || value.svgPaintRoots > 256
            || !Number.isInteger(value.svgPaintLinks) || value.svgPaintLinks < 0 || value.svgPaintLinks > value.svgPaintRoots * 64
            || !Number.isInteger(value.svgPaintServers) || value.svgPaintServers < 0 || value.svgPaintServers > 256
            || !Number.isInteger(value.svgPaintContentNodes) || value.svgPaintContentNodes < value.svgPaintServers || value.svgPaintContentNodes > 4096
            || (value.svgPaintServers === 0 && value.svgPaintContentNodes !== 0)
            || !Number.isInteger(value.svgPaintStyleHosts) || value.svgPaintStyleHosts !== value.svgPaintServers
            || !Number.isInteger(value.svgPaintStyleInstances) || value.svgPaintStyleInstances < 0 || value.svgPaintStyleInstances > 4096
            || (value.svgPaintStyleHosts === 0 && value.svgPaintStyleInstances !== 0)
            || !Number.isInteger(value.svgComputedSlots) || value.svgComputedSlots < 0 || value.svgComputedSlots > 256
            || !Array.isArray(value.blockers) || !value.blockers.every(b => typeof b === 'string')
            || !Number.isInteger(value.imageDeliveries) || value.imageDeliveries < 0 || value.imageDeliveries > value.imageAcquisitions
            || !Number.isInteger(value.imageAcquisitions) || value.imageAcquisitions < 0 || value.imageAcquisitions > 256) throw new Error('Invalid native preparation diagnostic summary');
        return Object.freeze({ blockers: Object.freeze([...value.blockers]), imageAcquisitions: value.imageAcquisitions, imageDeliveries: value.imageDeliveries, svgComputedSlots: value.svgComputedSlots, svgReplayCandidates: value.svgReplayCandidates, svgResourceBindings: value.svgResourceBindings, svgDeliveredBindings: value.svgDeliveredBindings, svgPaintRoots: value.svgPaintRoots, svgPaintLinks: value.svgPaintLinks, svgPaintServers: value.svgPaintServers, svgPaintContentNodes: value.svgPaintContentNodes, svgPaintStyleHosts: value.svgPaintStyleHosts, svgPaintStyleInstances: value.svgPaintStyleInstances, svgCloneProfileConfirmed: value.svgCloneProfileConfirmed, svgPaintColors: value.svgPaintColors, svgPaintSuppressedHosts: value.svgPaintSuppressedHosts });
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
