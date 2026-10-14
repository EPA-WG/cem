import { cssomText, type CssomBudget } from './browser-cssom.js';
import type { BrowserStylesheetOptions, BrowserStylesheetSummary } from './browser-stylesheets.js';

/** Realm-local opaque CEM-QL WASM session. CSS strings/worker result DTOs cannot
 * substitute for its native consumer leases. The browser host owns its disposal. */
export interface NativeCompiledStylesheetSession {
    len(): number;
    capture(revision: () => string, observe: (index: number, css: string) => [number, boolean, boolean]): string;
    read(revision: () => string, observe: (index: number, css: string) => [number, boolean, boolean]): string;
    invalidate(): void;
    free(): void;
}
export class BrowserCompiledStylesheetHost {
    readonly #document: Document;
    readonly #view: Window;
    readonly #native: NativeCompiledStylesheetSession;
    readonly #nodes: readonly HTMLStyleElement[];
    readonly #parents: readonly (Element | null)[];
    readonly #sheets: readonly CSSStyleSheet[];
    readonly #parser: Document;
    readonly #expected = new Map<number, CSSStyleSheet>();
    readonly #observer: MutationObserver;
    readonly #signal?: AbortSignal;
    #epoch = 1;
    #captured = false;
    #busy = false;
    #disposed = false;
    #budget: CssomBudget = { rules: 0, bytes: 0 };
    readonly #invalidate = () => { this.#captured = false; this.#epoch += 1; if (!this.#busy && !this.#disposed) this.#native.invalidate(); };
    readonly #abort = () => this.dispose();
    constructor(document: Document, native: NativeCompiledStylesheetSession, nodes: readonly HTMLStyleElement[], options: BrowserStylesheetOptions) {
        const view = document.defaultView;
        if (!view || view.document !== document || options.ownership !== 'exclusive' || options.signal?.aborted
            || nodes.length !== native.len() || nodes.length > 256 || new Set(nodes).size !== nodes.length) throw new Error('Complete exclusive compiled stylesheet installations required');
        this.#document = document; this.#view = view; this.#native = native; this.#nodes = [...nodes]; this.#signal = options.signal;
        this.#parents = nodes.map(node => node.parentElement);
        this.#parser = document.implementation.createHTMLDocument('');
        this.#sheets = nodes.map(node => {
            if (node.ownerDocument !== document || !node.isConnected || !node.sheet || node.getRootNode() !== document
                || node.localName !== 'style' || node.namespaceURI !== 'http://www.w3.org/1999/xhtml'
                || node.hasAttribute('title') || node.media) throw new Error('Compiled output requires connected untitled HTML style nodes without outer media');
            return node.sheet;
        });
        if (new Set(this.#sheets).size !== nodes.length) throw new Error('Compiled installations require distinct CSSOM identities');
        this.#observer = new MutationObserver(records => { if (records.length) this.#invalidate(); });
        this.#observer.observe(document, { subtree: true, attributes: true, childList: true, characterData: true });
        view.addEventListener('pagehide', this.#abort);
        view.addEventListener('resize', this.#invalidate);
        document.addEventListener('load', this.#invalidate, true);
        document.addEventListener('error', this.#invalidate, true);
        this.#signal?.addEventListener('abort', this.#abort, { once: true });
    }
    capture(): BrowserStylesheetSummary { return this.#execute(true); }
    read(): BrowserStylesheetSummary { return this.#execute(false); }
    change(change: () => void): void {
        this.#check(); this.#invalidate(); this.#busy = true;
        try {
            const result: unknown = change();
            if (result !== undefined || this.#disposed) throw new Error('Synchronous live compiled stylesheet change required');
            this.#observer.takeRecords();
        } catch (error) { this.dispose(); throw error; }
        finally { this.#busy = false; if (this.#disposed) this.#native.free(); }
    }
    dispose(): void {
        if (this.#disposed) return;
        this.#invalidate(); this.#disposed = true; this.#observer.disconnect();
        this.#view.removeEventListener('pagehide', this.#abort); this.#view.removeEventListener('resize', this.#invalidate);
        this.#document.removeEventListener('load', this.#invalidate, true); this.#document.removeEventListener('error', this.#invalidate, true);
        this.#signal?.removeEventListener('abort', this.#abort);
        this.#expected.clear(); this.#parser.head.replaceChildren();
        if (!this.#busy) this.#native.free();
    }
    #check(): void {
        if (this.#busy) { this.dispose(); throw new Error('Compiled stylesheet host reentry'); }
        if (this.#disposed || this.#signal?.aborted) throw new Error('Compiled stylesheet lifetime ended');
        if (this.#observer.takeRecords().length) this.#invalidate();
    }
    #execute(capture: boolean): BrowserStylesheetSummary {
        this.#check();
        if (!capture && !this.#captured) throw new Error('Fresh compiled stylesheet capture required');
        this.#busy = true;
        try {
            const result = JSON.parse(this.#native[capture ? 'capture' : 'read'](this.#revision, this.#observe)) as BrowserStylesheetSummary;
            this.#revision();
            this.#captured = true;
            return Object.freeze({ occurrences: result.occurrences, blockers: Object.freeze([...result.blockers]) });
        } catch (error) { this.dispose(); throw error; }
        finally { this.#busy = false; if (this.#disposed) this.#native.free(); }
    }
    readonly #revision = (): string => {
        if (this.#disposed || this.#signal?.aborted || this.#document.defaultView?.document !== this.#document) throw new Error('Compiled stylesheet document lifetime changed');
        if (this.#observer.takeRecords().length) throw new Error('DOM changed during compiled stylesheet observation');
        return `${this.#epoch}:${this.#view.innerWidth}:${this.#view.innerHeight}:${this.#view.devicePixelRatio}`;
    };
    readonly #observe = (index: number, css: string): [number, boolean, boolean] => {
        if (index === 0) this.#budget = { rules: 0, bytes: 0 };
        const node = this.#nodes[index]; const sheet = this.#sheets[index];
        if (!node || !sheet || !node.isConnected || node.ownerDocument !== this.#document || node.getRootNode() !== this.#document
            || node.parentElement !== this.#parents[index] || node.sheet !== sheet || sheet.ownerNode !== node || sheet.href !== null || node.hasAttribute('title') || node.media || sheet.media.mediaText) throw new Error('Compiled installation identity or metadata changed');
        let expected = this.#expected.get(index);
        if (!expected) {
            const style = this.#parser.createElement('style'); style.textContent = css; this.#parser.head.append(style);
            if (!style.sheet || style.sheet.cssRules.length !== 1 || style.sheet.cssRules[0].constructor.name !== 'CSSScopeRule') throw new Error('Browser cannot represent the native compiled scope');
            expected = style.sheet; this.#expected.set(index, expected);
        }
        return [index + 1, !sheet.disabled, cssomText(sheet, this.#budget) === cssomText(expected, this.#budget)];
    };
}
export function captureBrowserCompiledStylesheets(document: Document, native: NativeCompiledStylesheetSession,
    nodes: readonly HTMLStyleElement[], options: BrowserStylesheetOptions): BrowserCompiledStylesheetHost {
    let host: BrowserCompiledStylesheetHost | undefined;
    try { host = new BrowserCompiledStylesheetHost(document, native, nodes, options); host.capture(); return host; }
    catch (error) { if (host) host.dispose(); else native.free(); throw error; }
}
