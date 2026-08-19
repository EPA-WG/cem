export function deepEqual(a: unknown, b: unknown): boolean;
export function cloneAs(sourceNode: Element, tag: string): Element;
export function mix<T extends object, U extends object>(objTo: T, objFrom: U): T & U;
export function mergeAttr(fromEl: Element, toEl: Element): void;

export function xml2dom(xmlString: string): Document;
export function xmlString(doc: Node | Document): string;
export function obj2node(value: unknown, tag: string, doc?: Document): HTMLElement;
export function tagUid<T extends Element>(node: T): T;

export function getCustomElementRuntime(host?: Window, options?: Record<string, unknown>): unknown;
export const customElementRuntime: unknown;
export function installCustomElementRuntime(host?: Window, options?: Record<string, unknown>): unknown;
export function diagnosticsFor(target: object): readonly unknown[];
export function whenDeclarationSettled(declaration: object): Promise<void>;
export function whenRenderSettled(instance: HTMLElement): Promise<void>;
export function normalizeLegacyDeclaration(declaration: HTMLElement): HTMLElement;

/**
 * @summary Declarative Custom Element adapter over the CEM element substrate.
 * ```html
 * <custom-element tag="my-element">
 *             <template>
 *                 <attribute name="p1" >default_P1</attribute>
 *                 <style>
 *                     color:green;
 *                     b{ color: blue;}
 *                     input:checked+b{ color: darkblue; text-shadow: 0 0 4px springgreen;}
 *                 </style>
 *                 <label>
 *                     green
 *                     <input type="checkbox" value="Glowing Blue" checked/><b>blue</b>
 *                 </label>
 *                 p1:{$p1}
 *             </template>
 *         </custom-element>
 * <my-element p1="abc"></my-element>
 * ```
 *
 * @extends HTMLElement
 * @tag custom-element
 * @tag-name custom-element
 * @attr {boolean} hidden - hides DCE definition to prevent visual appearance of content. Wrap the payload into template tag to prevent applying the inline CSS in global scope.
 * @attr {string} tag - HTML tag for Custom Element. Used for window.customElements.define().
 * @attr {string} src - full, relative, or hash URL to DCE template.
 *
 */
export class CustomElement extends HTMLElement {
    static observedAttributes: string[];
}
export default CustomElement;
