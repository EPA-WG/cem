/** Scalar CSSOM observations only. Source/property ownership stays native. */
export type SvgComputedPredicate = (owner: number, property: string) => string;
const events = ['focusin', 'focusout', 'pointerover', 'pointerout', 'pointerdown', 'pointerup', 'keydown', 'keyup', 'input', 'change', 'animationstart', 'transitionrun'];
const media = ['(prefers-color-scheme: dark)', '(forced-colors: active)', '(prefers-contrast: more)', '(prefers-reduced-motion: reduce)'];
export class BrowserSvgComputedObservation {
    readonly #document: Document;
    readonly #node: (id: number) => Node | undefined;
    readonly #quiet: () => void;
    readonly #revoke: () => void;
    readonly #media: MediaQueryList[];
    #active = false;
    readonly #changed = () => { if (this.#active) this.#revoke(); };
    constructor(document: Document, node: (id: number) => Node | undefined, quiet: () => void, revoke: () => void) {
        const view = document.defaultView;
        if (!view) throw new Error('Live SVG observation document required');
        this.#document = document; this.#node = node; this.#quiet = quiet; this.#revoke = revoke;
        for (const event of events) document.addEventListener(event, this.#changed, true);
        this.#media = media.map(query => view.matchMedia(query));
        for (const query of this.#media) query.addEventListener('change', this.#changed);
    }
    readonly observe: SvgComputedPredicate = (owner, property) => {
        this.#quiet(); this.#active = true;
        const node = this.#node(owner) as Element | undefined;
        const view = this.#document.defaultView;
        if (!view || !node || node.nodeType !== 1 || !node.isConnected || node.ownerDocument !== this.#document
            || node.namespaceURI !== 'http://www.w3.org/2000/svg'
            || this.#document.getAnimations().length
            || this.#document.querySelector('svg animate, svg animateMotion, svg animateTransform, svg set, svg discard')) {
            throw new Error('Static live SVG computed owner required');
        }
        const value = view.getComputedStyle(node).getPropertyValue(property);
        this.#quiet(); return value;
    };
    dispose(): void {
        this.#active = false;
        for (const event of events) this.#document.removeEventListener(event, this.#changed, true);
        for (const query of this.#media) query.removeEventListener('change', this.#changed);
    }
}
