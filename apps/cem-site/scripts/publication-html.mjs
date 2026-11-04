// Publication sanity check, not an HTML sanitizer or security boundary.
// An escaped document outside a code example indicates a broken native bridge.
export function hasEscapedPageBridge(html) {
    return html.replace(/<cem-demo-element\b(?:[^"'<>]|"[^"]*"|'[^']*')*>/gi, '')
        .replace(/<(pre|code)\b[^>]*>[\s\S]*?<\/\1\s*>/gi, '').includes('&lt;h1');
}
