/**
 * <cem-css-loader> - Dynamic CSS loader web component
 *
 * Dynamically loads CSS rules into the page via a <style> tag.
 * Previous rules are removed before loading new ones.
 *
 * @example
 * <cem-css-loader value=":root { --color: red; }"></cem-css-loader>
 *
 * Note on STYLE tag removal:
 * When a <style> element is removed from the DOM, the browser immediately
 * removes its associated CSS rules from the CSSOM. This is synchronous -
 * the styles stop applying as soon as the element is detached.
 */
export class CemCssLoader extends HTMLElement {
    static get observedAttributes() {
        return ['value'];
    }

    /** @type {HTMLStyleElement|null} */
    #styleElement = null;

    /** @type {string} */
    #styleId;

    constructor() {
        super();
        this.#styleId = `cem-css-loader-${crypto.randomUUID().slice(0, 8)}`;
    }

    connectedCallback() {
        this.#applyStyles(this.getAttribute('value') || '');
    }

    disconnectedCallback() {
        this.#removeStyles();
    }

    attributeChangedCallback(name, oldValue, newValue) {
        if (name === 'value' && oldValue !== newValue && this.isConnected) {
            this.#applyStyles(newValue || '');
        }
    }

    /**
     * Apply CSS rules to the document
     * @param {string} css - CSS content to apply
     */
    #applyStyles(css) {
        this.#removeStyles();

        if (!css.trim()) {
            return;
        }

        this.#styleElement = document.createElement('style');
        this.#styleElement.id = this.#styleId;
        this.#styleElement.setAttribute('data-cem-css-loader', '');
        this.#styleElement.textContent = css;
        document.head.appendChild(this.#styleElement);
    }

    /**
     * Remove previously injected styles from the document
     */
    #removeStyles() {
        if (this.#styleElement) {
            this.#styleElement.remove();
            this.#styleElement = null;
        }
    }

    /**
     * Programmatic API to update CSS
     * @param {string} css - CSS content to apply
     */
    set value(css) {
        this.setAttribute('value', css);
    }

    get value() {
        return this.getAttribute('value') || '';
    }

    /**
     * Get the injected style element (for inspection/testing)
     * @returns {HTMLStyleElement|null}
     */
    get styleElement() {
        return this.#styleElement;
    }
}

customElements.define('cem-css-loader', CemCssLoader);

export default CemCssLoader;
