/** Keep native-owned style nodes in order without reinstalling matching sheets.
 * Instance styles form a suffix outside the render range; declaration styles
 * keep their existing positions among other declaration children when possible.
 * This places validated output only: it never parses CSS or authorizes reuse.
 */
export function placeManagedStylesheets(element: HTMLElement, styles: readonly HTMLStyleElement[],
    trailing: boolean, remount?: ReadonlySet<HTMLStyleElement>): void {
    const managed = trailing ? undefined : new Set<Node>(styles);
    let cursor: ChildNode | null = element.lastChild;
    let next: ChildNode | null = null;
    for (let index = styles.length - 1; index >= 0; index--) {
        const style = styles[index];
        while (cursor && managed && !managed.has(cursor)) cursor = cursor.previousSibling;
        const inPlace = style === cursor;
        if (inPlace) cursor = style.previousSibling;
        if (!inPlace || remount?.has(style)) element.insertBefore(style, next);
        next = style;
    }
}
