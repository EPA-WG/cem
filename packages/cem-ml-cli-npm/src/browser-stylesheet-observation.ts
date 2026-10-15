import { cssomText, type CssomBudget } from './browser-cssom.js';
type State = { sheet: CSSStyleSheet | null; selected: boolean; conditions: boolean };
export type StylesheetPredicate = BrowserStylesheetObservation['observe'];
/** Shared scalar CSSOM comparisons. Only native callers associate these results
 * with acquired source keys; this class owns no preparation or execution seal. */
export class BrowserStylesheetObservation {
    readonly #parser: Document;
    readonly #view: Window;
    readonly #identities = new WeakMap<object, number>();
    readonly #expected = new Map<string, CSSStyleSheet | CSSStyleDeclaration>();
    readonly #states = new Map<string, State>();
    readonly #media = new Map<string, MediaQueryList>();
    readonly #seen = new Set<CSSStyleSheet>();
    #nextIdentity = 1;
    titledDisabled = false;
    activeTitled?: Map<Node, CSSStyleSheet>;
    #budget: CssomBudget = { rules: 0, bytes: 0 };

    constructor(readonly document: Document, readonly node: (id: number) => Node | undefined, readonly invalidate: () => void) {
        const view = document.defaultView;
        if (!view) throw new Error('Live stylesheet observation document required');
        this.#view = view;
        this.#parser = document.implementation.createHTMLDocument('');
    }
    readonly #node = (id: number): Node | undefined => this.node(id);
    readonly #invalidate = (): void => this.invalidate();
    validateUniverse(): void {
        const doc = this.document;
        if (doc.adoptedStyleSheets.length || doc.querySelectorAll('*').length > 65_536) throw new Error('Unsupported stylesheet installation universe');
        for (const element of doc.querySelectorAll('*')) if (element.shadowRoot) throw new Error('Shadow stylesheet installations require their own execution host');
        if (this.#states.size) this.validateSeen();
    }
    validateSeen(): void {
        for (const sheet of this.document.styleSheets) if (!this.#seen.has(sheet)) throw new Error('Unrecorded stylesheet installation');
    }
    dispose(): void {
        for (const query of this.#media.values()) query.removeEventListener('change', this.#invalidate);
        this.#media.clear(); this.#expected.clear(); this.#states.clear(); this.#seen.clear();
        this.#parser.head.replaceChildren(); this.#parser.body.replaceChildren();
    }
    #matches(query: string): boolean {
        if (!query) return true;
        let media = this.#media.get(query);
        if (!media) {
            if (this.#media.size >= 256) throw new Error('Stylesheet media query bound exceeded');
            media = this.#view.matchMedia(query);
            media.addEventListener('change', this.#invalidate);
            this.#media.set(query, media);
        }
        return media.matches;
    }
    readonly observe = (root: number, index: number, owner: number, attribute: boolean,
        parent: number | null, importIndex: number | null, url: string, text: string, media: string, title: string, disabled: boolean): [boolean, boolean, boolean, number | null] => {
        if (root === 0 && index === 0) { this.#states.clear(); this.#seen.clear(); this.#budget = { rules: 0, bytes: 0 }; }
        const key = `${root}:${index}`;
        const element = this.#node(owner) as HTMLElement | SVGElement | undefined;
        if (!element || element.nodeType !== 1) throw new Error('Native stylesheet owner has no DOM installation');
        let expected = this.#expected.get(key);
        if (!expected) {
            if (attribute) {
                const style = this.#parser.createElement('span').style;
                style.cssText = text;
                expected = style;
            } else {
                // SVG style elements default to media="all" in Chromium;
                // preserve both the owner namespace and absent versus empty media.
                const svg = parent === null && element.namespaceURI === 'http://www.w3.org/2000/svg'
                    ? this.#parser.createElementNS('http://www.w3.org/2000/svg', 'svg') : undefined;
                const style = svg ? this.#parser.createElementNS(svg.namespaceURI, 'style') as SVGStyleElement
                    : this.#parser.createElement('style');
                style.textContent = text;
                if (parent === null && (media || element.hasAttribute('media'))) style.setAttribute('media', media);
                if (svg) { svg.append(style); this.#parser.body.append(svg); }
                else this.#parser.head.append(style);
                if (!style.sheet) throw new Error('Browser source comparison parser unavailable');
                expected = style.sheet;
            }
            this.#expected.set(key, expected);
        }
        if (attribute) {
            if (element.style.cssText !== (expected as CSSStyleDeclaration).cssText) throw new Error('Inline CSSOM differs from native source');
            return [true, true, true, this.#identity(element.style)];
        }
        let selected: boolean;
        let conditions: boolean;
        let sheet: CSSStyleSheet | null;
        if (parent === null) {
            sheet = (element as HTMLStyleElement | HTMLLinkElement).sheet;
            if (!sheet || sheet.ownerNode !== element) throw new Error('Stylesheet root is not loaded');
            if (element.localName === 'link' && sheet.href !== url) throw new Error('Loaded stylesheet final URL mismatch');
            if (element.localName !== 'link' && sheet.href !== null) throw new Error('Inline stylesheet base mismatch');
            if (sheet.media.mediaText !== (expected as CSSStyleSheet).media.mediaText) throw new Error('Stylesheet media changed outside its native document metadata');
            if (title) {
                const active = this.activeTitled?.get(element);
                if (sheet.title !== title || (active ? active !== sheet || sheet.disabled
                    : (!this.titledDisabled && !this.activeTitled) || !sheet.disabled)) throw new Error('Controlled titled stylesheet selection required');
            }
            if (element.localName === 'link' && Array.from((element as HTMLLinkElement).relList).some(token => token.toLowerCase() === 'alternate') && !title) throw new Error('Untitled alternate stylesheet selection is unsupported');
            if (disabled && !sheet.disabled) throw new Error('Authored disabled stylesheet was enabled');
            selected = !sheet.disabled;
            conditions = this.#matches(media);
        } else {
            const previous = this.#states.get(`${root}:${parent}`);
            if (!previous || importIndex === null) throw new Error('Stylesheet import parent missing');
            const expectedParent = this.#expected.get(`${root}:${parent}`) as CSSStyleSheet;
            const expectedRule = (Array.from(expectedParent.cssRules).filter(r => r.type === 3) as CSSImportRule[])[importIndex];
            if (!expectedRule) throw new Error('Native import occurrence absent from browser grammar');
            const rule = previous.sheet ? (Array.from(previous.sheet.cssRules).filter(r => r.type === 3) as CSSImportRule[])[importIndex] : null;
            if (previous.sheet && (!rule || rule.cssText !== expectedRule.cssText)) throw new Error('Native import occurrence mismatch');
            const conditionRule = rule ?? expectedRule;
            const supports = (conditionRule as CSSImportRule & { supportsText?: string | null }).supportsText;
            conditions = this.#matches(conditionRule.media.mediaText) && (!supports || CSS.supports(supports) || CSS.supports(`(${supports})`));
            selected = previous.selected && previous.conditions;
            sheet = rule?.styleSheet ?? null;
            if (!sheet) {
                if (selected && conditions) throw new Error('Active imported stylesheet is unloaded');
                this.#states.set(key, { sheet: null, selected, conditions });
                return [selected, conditions, false, null];
            }
            if (sheet.href !== url) throw new Error('Imported stylesheet final URL differs');
            selected = selected && !sheet.disabled;
        }
        if (this.#rulesText(sheet) !== this.#rulesText(expected as CSSStyleSheet)) throw new Error('Loaded CSSOM differs from native acquired source');
        this.#states.set(key, { sheet, selected, conditions });
        this.#seen.add(sheet);
        return [selected, conditions, true, this.#identity(sheet)];
    };
    #rulesText(sheet: CSSStyleSheet): string { return cssomText(sheet, this.#budget); }
    #identity(object: object): number {
        let id = this.#identities.get(object);
        if (!id) {
            if (this.#nextIdentity > 0xffff_ffff) throw new Error('Stylesheet identities exhausted');
            id = this.#nextIdentity++;
            this.#identities.set(object, id);
        }
        return id;
    }
}
