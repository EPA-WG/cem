import * as runtime from '@epa-wg/cem-ml/wasm';

const HTML = 'http://www.w3.org/1999/xhtml';
const owners = new WeakMap<Document, BrowserHtmlBaseHost>();

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
    readonly #document: Document;
    readonly #native: runtime.HtmlBaseExecutionHost;
    readonly #observer: MutationObserver;
    readonly #maxNodes: number;
    readonly #identities = new WeakMap<Node, number>();
    readonly #slots = new Map<number, Node>();
    readonly #handles = new Map<number, Node>();
    #childLists = new WeakMap<Node, Node[]>();
    #attributeLists = new WeakMap<Node, Attr[]>();
    #nextIdentity = 1;
    #url: string;
    #busy = false;
    #revoked = false;
    #disposed = false;
    #captured = false;
    readonly #onPageHide = () => this.#revoke();

    /** Use createBrowserHtmlBaseHost to initialize WASM first. */
    constructor(document: Document, options: BrowserHtmlBaseOptions) {
        this.#document = document;
        this.#maxNodes = options.maxNodes ?? 10_000;
        const bytes = options.maxUrlBytes ?? 16_384;
        if (options.contextOwnership !== 'exclusive' || owners.has(document)
            || !Number.isSafeInteger(this.#maxNodes) || this.#maxNodes < 1 || this.#maxNodes > 1_000_000
            || !Number.isSafeInteger(bytes) || bytes < 1 || bytes > 16_777_216
            || document.contentType !== 'text/html' || document.characterSet !== 'UTF-8'
            || !/^https?:/u.test(document.URL) || !document.defaultView || document.defaultView.document !== document) {
            throw new TypeError('HTML base host requires an unowned, exclusive UTF-8 HTTP(S) document and bounded limits');
        }
        this.#native = new runtime.HtmlBaseExecutionHost(options.profile, options.browser.engine, options.browser.version, this.#maxNodes, bytes);
        this.#url = document.URL;
        this.#observer = new MutationObserver((records) => { if (records.length) this.#revoke(); });
        this.#observer.observe(document, { subtree: true, attributes: true, childList: true, characterData: true });
        document.defaultView.addEventListener('pagehide', this.#onPageHide);
        owners.set(document, this);
    }

    /** A synchronous exclusive write boundary. Invalidates before running code,
     * even if its final URL, policy or DOM values equal their original values.
     * Supply the resulting native execution snapshot at the next capture. */
    changeContext(change: () => void): void {
        this.#run(() => {
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
            this.#captured = false;
            this.#resetSlots();
            const stamp = policy.stamp;
            const selected = this.#native.prepare(nativeDocument, this.#document.URL, stamp, this.#matchSlot, policy.decide);
            // A policy callback must not mutate or reenter the host.
            this.#checkQuiet();
            if (policy.stamp !== stamp) throw new Error('HTML base policy changed during preparation');
            if (selected) {
                const attribute = this.#handles.get(selected) as Attr | undefined;
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
            this.#resetSlots();
            this.#native.validate(this.#matchSlot, this.#document.baseURI);
            return result;
        });
    }

    /** Consume only a current, synchronously revalidated read set. */
    read(): string {
        if (this.#busy) { this.#revoke(); throw new Error('HTML base host reentry rejected'); }
        this.#checkQuiet();
        if (!this.#captured) throw new Error('HTML base host needs a fresh capture after a context change');
        return this.#run(() => {
            this.#resetSlots();
            return this.#native.validate(this.#matchSlot, this.#document.baseURI);
        });
    }

    dispose(): void {
        if (this.#disposed) return;
        this.#revoke();
        if (this.#busy) throw new Error('HTML base host cannot be disposed during a callback');
        this.#disposed = true;
        this.#native.free();
        this.#document.defaultView?.removeEventListener('pagehide', this.#onPageHide);
        if (owners.get(this.#document) === this) owners.delete(this.#document);
        this.#resetSlots();
    }

    #run<T>(work: () => T): T {
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
        this.#revoked = true;
        this.#captured = false;
        this.#observer.disconnect();
        if (!this.#busy && !this.#disposed) this.#native.revoke();
    }
    #resetSlots(): void {
        this.#slots.clear(); this.#handles.clear();
        this.#childLists = new WeakMap(); this.#attributeLists = new WeakMap();
    }

    // Native asks predicates against its retained nodes. JS keeps only DOM
    // identities; it never stores native names/values or a projected AST.
    readonly #matchSlot = (id: number, parent: number | null, index: number, kind: number,
        namespace: string, name: string, value: string, children: number, attributes: number): number => {
        const owner = parent === null ? null : this.#slots.get(parent);
        const node = parent === null ? this.#document : kind === 2
            ? this.#attributes(owner)[index] : this.#children(owner)[index];
        if (!node || this.#slots.has(id) || this.#slots.size >= this.#maxNodes || node.nodeType !== kind
            || (node !== this.#document && node.ownerDocument !== this.#document)) throw new Error('Native/DOM slot topology mismatch');
        if (kind === 1 || kind === 2) {
            const named = node as Element | Attr;
            if ((named.namespaceURI ?? '') !== namespace || named.localName !== name) throw new Error('Native/DOM expanded name mismatch');
        }
        if ((kind === 2 || kind === 3 || kind === 8) && node.nodeValue !== value) throw new Error('Native/DOM decoded value mismatch');
        if (this.#children(node).length !== children || this.#attributes(node).length !== attributes) throw new Error('Native/DOM slot count mismatch');
        let handle = this.#identities.get(node);
        if (!handle) {
            if (this.#nextIdentity > 0xffff_ffff) throw new Error('HTML base DOM identities exhausted');
            handle = this.#nextIdentity++;
            this.#identities.set(node, handle);
        }
        this.#slots.set(id, node);
        this.#handles.set(handle, node);
        return handle;
    };
    #children(node: Node | null | undefined): Node[] {
        if (!node || node.childNodes.length > this.#maxNodes) throw new Error('HTML base DOM child bounds exceeded');
        const cached = this.#childLists.get(node);
        if (cached) return cached;
        const children = Array.from(node.childNodes).filter((child) => {
            if (child.nodeType !== 10) return true;
            const type = child as DocumentType;
            if (node !== this.#document || type.name !== 'html' || type.publicId || type.systemId) throw new Error('Unsupported HTML doctype');
            return false; // Native semantic import omits the standard doctype.
        });
        this.#childLists.set(node, children);
        return children;
    }
    #attributes(node: Node | null | undefined): Attr[] {
        if (!node) throw new Error('HTML base DOM parent missing');
        if (node.nodeType !== 1) return [];
        const element = node as Element;
        if (element.attributes.length > this.#maxNodes) throw new Error('HTML base DOM attribute bounds exceeded');
        const cached = this.#attributeLists.get(node);
        if (cached) return cached;
        const attributes = Array.from(element.attributes).filter((attribute) => {
            // XML import consumes the root's namespace declaration. In text/html
            // it is an inert ordinary attribute; admit only this exact equivalent.
            return !(element === this.#document.documentElement && attribute.name === 'xmlns'
                && attribute.value === HTML && !attribute.namespaceURI && element.namespaceURI === HTML);
        });
        this.#attributeLists.set(node, attributes);
        return attributes;
    }
}
