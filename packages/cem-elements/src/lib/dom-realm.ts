type Constructors = Pick<typeof globalThis, 'Document' | 'ShadowRoot' | 'Element' | 'HTMLElement' | 'HTMLInputElement' | 'HTMLTextAreaElement' | 'HTMLSelectElement' | 'HTMLButtonElement' | 'HTMLFormElement'>;
/** Native elements are checked against their owning realm, including script-free previews. */
export function isDomInstance<K extends keyof Constructors>(value: unknown, name: K): value is InstanceType<Constructors[K]> {
    const node = value as Node | null;
    const document = node?.nodeType === 9 ? node as Document : node?.ownerDocument;
    const constructor = document?.defaultView?.[name];
    return !!constructor && value instanceof constructor;
}
