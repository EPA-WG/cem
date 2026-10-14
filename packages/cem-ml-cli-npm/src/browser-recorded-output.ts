import { BrowserDomCorrespondence, claimDocument, documentOwned, releaseDocument } from './browser-dom-correspondence.js';
import { cssomText, type CssomBudget } from './browser-cssom.js';
import type { BrowserHtmlBasePolicy } from './browser-html-base.js';
type NativeDomPredicate = BrowserDomCorrespondence['matchSlot'];

/** Actual realm-local QL WASM session; result messages cannot replace this owner. */
export interface NativeRecordedOutputSession {
    url(): string;
    html(): string;
    prepareBase(matches: NativeDomPredicate, decide: (candidate: string, fallback: string, stamp: string) => boolean): number;
    compose(base: string, matches: NativeDomPredicate): string;
    capture(base: string, matches: NativeDomPredicate, revision: () => string, observe: (owner: number, css: string) => [number, boolean, boolean]): string;
    read(base: string, matches: NativeDomPredicate, revision: () => string, observe: (owner: number, css: string) => [number, boolean, boolean]): string;
    close(): void;
    free(): void;
}
export interface BrowserRecordedOutputOptions {
    readonly ownership: 'exclusive';
    readonly policy: BrowserHtmlBasePolicy;
    readonly signal?: AbortSignal;
}
export interface BrowserRecordedOutputSummary { readonly blockers: readonly string[]; }
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
    readonly #parser: Document;
    readonly #expected = new Map<number, CSSStyleSheet>();
    readonly #sheets = new WeakMap<CSSStyleSheet, number>();
    readonly #initialLoads = new Set<HTMLStyleElement>();
    #nextSheet = 1;
    #busy = false;
    #disposed = false;
    #freed = false;
    #captured = false;
    #budget: CssomBudget = { rules: 0, bytes: 0 };
    readonly #revoke = () => this.dispose();
    readonly #load = (event: Event) => {
        const node = event.target as HTMLStyleElement;
        if (!event.isTrusted || this.#busy || !this.#captured || !this.#initialLoads.delete(node)) { this.dispose(); return; }
        // Inline sheets are synchronously installed, but their initial load event
        // may arrive later. Preserve the record only after complete native reread.
        try { this.read(); } catch { this.dispose(); }
    };
    constructor(document: Document, native: NativeRecordedOutputSession, options: BrowserRecordedOutputOptions) {
        const view = document.defaultView;
        if (!view || view.document !== document || documentOwned(document) || options.ownership !== 'exclusive'
            || options.signal?.aborted || document.contentType !== 'text/html' || document.characterSet !== 'UTF-8'
            || !/^https?:/u.test(document.URL) || document.URL !== native.url()) throw new Error('Recorded output requires an exclusive live UTF-8 HTTP(S) document matching its native URL');
        this.#document = document; this.#view = view; this.#native = native; this.#options = options;
        this.#url = document.URL; this.#stamp = options.policy.stamp;
        this.#dom = new BrowserDomCorrespondence(document, 10_000);
        this.#parser = document.implementation.createHTMLDocument('');
        this.#observer = new MutationObserver(records => { if (records.length) this.dispose(); });
        this.#observer.observe(document, { subtree: true, attributes: true, characterData: true, childList: true });
        view.addEventListener('pagehide', this.#revoke); view.addEventListener('resize', this.#revoke);
        document.addEventListener('load', this.#load, true); document.addEventListener('error', this.#revoke, true);
        options.signal?.addEventListener('abort', this.#revoke, { once: true });
        claimDocument(document, this);
    }
    capture(install: RecordedOutputInstaller): BrowserRecordedOutputSummary {
        return this.#run(() => {
            if (this.#captured) throw new Error('Recorded output already captured');
            this.#deliver(this.#native.html(), 'original', install);
            this.#freeze();
            const composed = this.#native.compose(this.#document.baseURI, this.#match);
            this.#quiet();
            this.#deliver(composed, 'composed', install);
            this.#freeze();
            const summary = this.#summary(this.#native.capture(this.#document.baseURI, this.#match, this.#revision, this.#observe));
            this.#captured = true;
            return summary;
        });
    }
    read(): BrowserRecordedOutputSummary {
        return this.#run(() => {
            if (!this.#captured) throw new Error('Recorded output capture required');
            return this.#summary(this.#native.read(this.#document.baseURI, this.#match, this.#revision, this.#observe));
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
        this.#view.removeEventListener('pagehide', this.#revoke); this.#view.removeEventListener('resize', this.#revoke);
        this.#document.removeEventListener('load', this.#load, true); this.#document.removeEventListener('error', this.#revoke, true);
        this.#options.signal?.removeEventListener('abort', this.#revoke);
        releaseDocument(this.#document, this); this.#expected.clear(); this.#initialLoads.clear(); this.#parser.head.replaceChildren();
        if (!this.#busy) this.#free();
    }
    #free(): void { if (!this.#freed) { this.#freed = true; try { this.#native.close(); } finally { this.#native.free(); } } }
    #run<T>(work: () => T): T {
        if (this.#busy) { this.dispose(); throw new Error('Recorded output reentry'); }
        this.#busy = true;
        this.#budget = { rules: 0, bytes: 0 };
        try { this.#quiet(); const result = work(); this.#quiet(); return result; }
        catch (error) { this.dispose(); throw error; }
        finally { this.#busy = false; if (this.#disposed) this.#free(); }
    }
    #quiet(): void {
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
        if (this.#document.adoptedStyleSheets.length || [...this.#document.querySelectorAll('*')].some(node => node.shadowRoot)
            || this.#document.querySelector('link[rel~="stylesheet"]')) throw new Error('Recorded observation excludes external, adopted and shadow sheets');
        return `${this.#view.innerWidth}:${this.#view.innerHeight}:${this.#view.devicePixelRatio}`;
    };
    readonly #observe = (owner: number, css: string): [number, boolean, boolean] => {
        this.#quiet();
        const node = this.#dom.node(owner) as HTMLStyleElement | undefined;
        const sheet = node?.sheet;
        if (!node || node.localName !== 'style' || node.namespaceURI !== 'http://www.w3.org/1999/xhtml'
            || !node.isConnected || node.ownerDocument !== this.#document || node.getRootNode() !== this.#document
            || !sheet || sheet.ownerNode !== node || sheet.href !== null || node.media || node.hasAttribute('title') || sheet.media.mediaText) throw new Error('Recorded inline stylesheet changed');
        let expected = this.#expected.get(owner);
        if (!expected) {
            if (this.#expected.size >= 256) throw new Error('Recorded stylesheet count limit');
            const style = this.#parser.createElement('style'); style.textContent = css; this.#parser.head.append(style);
            if (!style.sheet || Array.from(style.sheet.cssRules).some(rule => rule.type === 3)) throw new Error('Recorded styles must have flattened imports');
            expected = style.sheet; this.#expected.set(owner, expected);
        }
        let identity = this.#sheets.get(sheet);
        if (!identity) { if (this.#nextSheet > 0xffff_ffff) throw new Error('Recorded stylesheet identity exhausted'); identity = this.#nextSheet++; this.#sheets.set(sheet, identity); }
        const result: [number, boolean, boolean] = [identity, !sheet.disabled, cssomText(sheet, this.#budget) === cssomText(expected, this.#budget)];
        if (!this.#captured) this.#initialLoads.add(node);
        this.#quiet(); return result;
    };
    #summary(json: string): BrowserRecordedOutputSummary {
        const value = JSON.parse(json) as BrowserRecordedOutputSummary;
        if (!Array.isArray(value.blockers) || !value.blockers.every(b => typeof b === 'string')) throw new Error('Invalid native preparation diagnostic summary');
        return Object.freeze({ blockers: Object.freeze([...value.blockers]) });
    }
}
/** Takes ownership of the native session, including failed construction/capture. */
export function captureBrowserRecordedOutput(document: Document, native: NativeRecordedOutputSession, install: RecordedOutputInstaller, options: BrowserRecordedOutputOptions): BrowserRecordedOutputHost {
    let host: BrowserRecordedOutputHost | undefined;
    try { host = new BrowserRecordedOutputHost(document, native, options); host.capture(install); return host; }
    catch (error) { if (host) host.dispose(); else { try { native.close(); } finally { native.free(); } } throw error; }
}
