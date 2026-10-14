const HTML = 'http://www.w3.org/1999/xhtml';
const owners = new WeakMap<Document, object>();
export function documentOwned(document: Document): boolean { return owners.has(document); }
export function claimDocument(document: Document, owner: object): void {
    if (owners.has(document)) throw new Error('Document already has an exclusive native observation owner');
    owners.set(document, owner);
}
export function releaseDocument(document: Document, owner: object): void {
    if (owners.get(document) === owner) owners.delete(document);
}
/** Shared scalar DOM correspondence. Stores DOM references, never native AST metadata. */
export class BrowserDomCorrespondence {
    readonly #document: Document;
    readonly #maxNodes: number;
    readonly #identities = new WeakMap<Node, number>();
    readonly #slots = new Map<number, Node>();
    readonly #handles = new Map<number, Node>();
    #childLists = new WeakMap<Node, Node[]>();
    #attributeLists = new WeakMap<Node, Attr[]>();
    #nextIdentity = 1;
    constructor(document: Document, maxNodes: number) { this.#document = document; this.#maxNodes = maxNodes; }
    node(id: number): Node | undefined { return this.#slots.get(id); }
    handle(id: number): Node | undefined { return this.#handles.get(id); }
    reset(): void {
        this.#slots.clear(); this.#handles.clear();
        this.#childLists = new WeakMap(); this.#attributeLists = new WeakMap();
    }

    // Native asks predicates against its retained nodes. JS keeps only DOM
    // identities; it never stores native names/values or a projected AST.
    readonly matchSlot = (id: number, parent: number | null, index: number, kind: number,
        namespace: string, name: string, value: string, children: number, attributes: number): number => {
        if (parent === null) this.reset(); // Native dependency validation makes repeated complete passes.
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
