import { BrowserStylesheetObservation } from './browser-stylesheet-observation.js';
import * as runtime from '@epa-wg/cem-ml/wasm';

/** Immutable acquisition input. The adapter independently compares actual CSSOM;
 * supplying a response alone never proves a browser loaded it. UTF-8 only. */
export interface BrowserStylesheetResponse {
    readonly requestedUrl: string;
    readonly finalUrl: string;
    readonly revision: string;
    readonly contentType: string;
    readonly text: string;
}
export interface BrowserStylesheetOptions {
    /** Every CSSOM, selection and environment write must use change(). Unknown
     * external writers and synchronous unobserved A-B-A are outside this profile. */
    readonly ownership: 'exclusive';
    readonly signal?: AbortSignal;
    /** This profile can establish inactive titled/alternate sets. Chromium's
     * disabled=false alone cannot prove activation of a titled set. */
    readonly titledSets?: 'disabled';
}
export interface BrowserStylesheetSelectionOptions extends Omit<BrowserStylesheetOptions, 'titledSets'> {
    /** Whole transition deadline, including every selected link's load. */
    readonly timeoutMs?: number;
}
export interface BrowserStylesheetSummary {
    readonly occurrences: number;
    readonly blockers: readonly string[];
}
export interface StylesheetDomBoundary {
    readonly document: Document;
    readonly run: <T>(work: () => T) => T;
    readonly native: runtime.HtmlBaseExecutionHost;
    readonly matches: (...args: [number, number | null, number, number, string, string, string, number, number]) => number;
    readonly node: (id: number) => Node | undefined;
}
const MAX_BYTES = 4 * 1024 * 1024;

/** Main-thread native evidence remains local to its live DOM. Worker lifecycle
 * signals revoke it; neither summaries nor worker messages carry an edit grant. */
export class BrowserStylesheetHost {
    readonly #view: Window;
    readonly #dom: StylesheetDomBoundary;
    readonly #observation: BrowserStylesheetObservation;
    readonly #native: runtime.HtmlStylesheetSession;
    readonly #signal?: AbortSignal;
    #epoch = 1;
    #busy = false;
    #disposed = false;
    #captured = false;
    readonly #invalidate = () => {
        this.#captured = false;
        this.#observation.activeTitled = undefined;
        this.#epoch += 1;
        if (!this.#busy && !this.#disposed) this.#native.invalidate();
    };
    readonly #abort = () => this.dispose();

    /** Created by BrowserHtmlBaseHost.captureStylesheets after native DOM confirmation. */
    constructor(dom: StylesheetDomBoundary, responses: readonly BrowserStylesheetResponse[], options: BrowserStylesheetOptions) {
        if (options.ownership !== 'exclusive' || options.signal?.aborted) throw new Error('Exclusive live stylesheet ownership required');
        const view = dom.document.defaultView;
        if (!view) throw new Error('Live browser document required');
        this.#view = view;
        this.#dom = dom;
        this.#signal = options.signal;
        this.#observation = new BrowserStylesheetObservation(dom.document, dom.node, this.#invalidate);
        // Bounds precede JSON serialization and native import. JS strings use
        // at most three UTF-8 bytes per code unit (surrogate pairs use four).
        let bytes = 0;
        if (responses.length > 256) throw new Error('Stylesheet response count exceeded');
        for (const r of responses) {
            for (const value of [r.text, r.requestedUrl, r.finalUrl, r.revision, r.contentType]) {
                if (typeof value !== 'string') throw new TypeError('Stylesheet response strings required');
                bytes += value.length;
                if (bytes > MAX_BYTES) throw new Error('Stylesheet response byte bound exceeded');
            }
        }
        this.#native = new runtime.HtmlStylesheetSession(dom.native, JSON.stringify(responses), dom.matches, dom.document.baseURI);
        dom.document.addEventListener('load', this.#invalidate, true);
        dom.document.addEventListener('error', this.#invalidate, true);
        this.#view.addEventListener('resize', this.#invalidate);
        this.#view.addEventListener('orientationchange', this.#invalidate);
        this.#view.addEventListener('pagehide', this.#abort);
        this.#signal?.addEventListener('abort', this.#abort, { once: true });
        try {
            if (options.titledSets !== undefined) {
                if (options.titledSets !== 'disabled') throw new Error('Active titled stylesheet selection requires selectStylesheets');
                this.#disableTitled();
            }
        }
        catch (error) { this.dispose(); throw error; }
    }
    /** Internal transition protocol used by the owning HTML host. Native commands
     * carry slot identities only; neither this plan nor its return value is evidence. */
    prepareSelection(title: string, timeoutMs: number): { start: () => Promise<void>; finish: () => void } {
        this.#guard();
        if (typeof title !== 'string' || title.length > 4096 || !Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 30_000) throw new Error('Bounded stylesheet selection required');
        const actions: { node: HTMLLinkElement | HTMLStyleElement; selected: boolean; sheet: CSSStyleSheet }[] = [];
        this.#native.beginSelection(this.#dom.native, this.#dom.matches, this.#dom.document.baseURI, title,
            (owner: number, link: boolean, selected: boolean) => {
                const node = this.#dom.node(owner) as HTMLLinkElement | HTMLStyleElement | undefined;
                if (!node || node.localName !== (link ? 'link' : 'style') || !node.sheet || node.sheet.ownerNode !== node) throw new Error('Titled stylesheet root must already be loaded');
                actions.push({ node, selected, sheet: node.sheet });
            });
        let started = false, completed = false, finished = false;
        const active = new Map<Node, CSSStyleSheet>();
        return {
            start: () => {
                if (started) throw new Error('Stylesheet transition already started');
                started = true;
                this.#invalidate();
                return new Promise<void>((resolve, reject) => {
                    const cleanup: (() => void)[] = [];
                    let settled = false, remaining = actions.filter(a => a.selected).length;
                    const end = (error?: Error) => {
                        if (settled) return;
                        settled = true;
                        for (const remove of cleanup) remove();
                        if (error) reject(error); else { completed = true; resolve(); }
                    };
                    const abort = () => end(new Error('Stylesheet transition cancelled'));
                    const timer = setTimeout(() => end(new Error('Stylesheet transition load timed out')), timeoutMs);
                    cleanup.push(() => clearTimeout(timer));
                    this.#signal?.addEventListener('abort', abort, { once: true });
                    cleanup.push(() => this.#signal?.removeEventListener('abort', abort));
                    try {
                        if (this.#signal?.aborted || this.#disposed) throw new Error('Stylesheet transition lifecycle ended');
                        for (const action of actions) {
                            if (!action.selected) { action.sheet.disabled = true; continue; }
                            const node = action.node as HTMLLinkElement;
                            const load = (event: Event) => {
                                if (!event.isTrusted || event.target !== node) return;
                                const sheet = node.sheet;
                                if (!sheet || sheet === action.sheet || sheet.disabled || sheet.ownerNode !== node) {
                                    end(new Error('Selected stylesheet did not install a fresh enabled sheet')); return;
                                }
                                active.set(node, sheet);
                                node.removeEventListener('load', load);
                                node.removeEventListener('error', error);
                                if (--remaining === 0) end();
                            };
                            const error = (event: Event) => { if (event.isTrusted && event.target === node) end(new Error('Selected stylesheet load failed')); };
                            node.addEventListener('load', load); node.addEventListener('error', error);
                            cleanup.push(() => { node.removeEventListener('load', load); node.removeEventListener('error', error); });
                            // HTMLLinkElement's explicit enable flag activates alternates.
                            // CSSStyleSheet.disabled alone does not establish that flag.
                            node.disabled = true; node.disabled = false;
                        }
                        if (remaining === 0) end();
                    } catch (error) { end(error instanceof Error ? error : new Error(String(error))); }
                });
            },
            finish: () => {
                this.#guard();
                if (!completed || finished) throw new Error('Completed, unconsumed stylesheet transition required');
                finished = true;
                for (const action of actions) {
                    if (action.node.sheet !== (action.selected ? active.get(action.node) : action.sheet)
                        || action.node.sheet?.disabled !== !action.selected) throw new Error('Stylesheet selection changed during load');
                }
                this.#native.recaptureSelection(this.#dom.native, this.#dom.matches, this.#dom.document.baseURI);
                this.#observation.activeTitled = active;
                this.#observation.titledDisabled = title === '';
            },
        };
    }
    capture(): BrowserStylesheetSummary { return this.#execute(true); }
    read(): BrowserStylesheetSummary { return this.#execute(false); }

    /** Invalidates before synchronous writes, including same-value assignments.
     * DOM/history/base writes instead use the owning HTML host's changeContext. */
    change(change: () => void): void {
        this.#guard();
        this.#dom.run(() => {
            this.#invalidate();
            this.#busy = true;
            try {
                const result: unknown = change();
                if (result !== undefined) throw new TypeError('Stylesheet changes require synchronous void callbacks');
                if (this.#disposed) throw new Error('Stylesheet lifecycle ended during change');
            } catch (error) { this.dispose(); throw error; }
            finally { this.#busy = false; if (this.#disposed) this.#native.free(); }
        });
    }
    /** Establish inactive titled sets before a fresh capture. */
    disableTitled(): void { this.change(() => { this.#disableTitled(); }); }
    #disableTitled(): void {
        const sheets = Array.from(this.#dom.document.styleSheets);
        for (const sheet of sheets) if (sheet.title) sheet.disabled = true;
        this.#observation.titledDisabled = true;
    }

    dispose(): void {
        if (this.#disposed) return;
        this.#invalidate();
        this.#disposed = true;
        const doc = this.#dom.document;
        doc.removeEventListener('load', this.#invalidate, true);
        doc.removeEventListener('error', this.#invalidate, true);
        doc.defaultView?.removeEventListener('resize', this.#invalidate);
        doc.defaultView?.removeEventListener('orientationchange', this.#invalidate);
        doc.defaultView?.removeEventListener('pagehide', this.#abort);
        this.#signal?.removeEventListener('abort', this.#abort);
        this.#observation.dispose();
        if (!this.#busy) this.#native.free();
    }
    #guard(): void {
        if (this.#busy) { this.dispose(); throw new Error('Stylesheet host reentry rejected'); }
        if (this.#disposed || this.#signal?.aborted) throw new Error('Stylesheet host lifecycle ended');
    }
    #execute(capture: boolean): BrowserStylesheetSummary {
        this.#guard();
        if (!capture && !this.#captured) throw new Error('Fresh stylesheet capture required');
        return this.#dom.run(() => {
            this.#busy = true;
            try {
                const raw = this.#native[capture ? 'capture' : 'read'](this.#dom.native, this.#dom.matches,
                    this.#dom.document.baseURI, this.#revision, this.#observation.observe);
                if (this.#disposed) throw new Error('Stylesheet lifecycle ended during observation');
                this.#observation.validateSeen();
                this.#captured = true;
                const summary = JSON.parse(raw) as BrowserStylesheetSummary;
                return Object.freeze({ occurrences: summary.occurrences, blockers: Object.freeze([...summary.blockers]) });
            } catch (error) { this.dispose(); throw error; }
            finally { this.#busy = false; if (this.#disposed) this.#native.free(); }
        });
    }
    readonly #revision = (): string => {
        if (this.#disposed || this.#signal?.aborted) throw new Error('Stylesheet lifecycle ended');
        this.#observation.validateUniverse();
        const view = this.#view;
        return `${this.#epoch}:${view.innerWidth}:${view.innerHeight}:${view.devicePixelRatio}`;
    };

}
