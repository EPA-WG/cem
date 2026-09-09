const TOKEN_PATTERN = /<!--[\s\S]*?-->|\/\*[\s\S]*?\*\/|\/\/[^\n]*|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|\b(?:const|let|var|function|return|if|else|for|of|class|extends|new|import|export|from|async|await|true|false|null|undefined)\b|\b\d+(?:\.\d+)?\b|[@$]?[A-Za-z_][\w:-]*(?=\s*=)|[{}[\]():;,.|=@$]+/g;

export function highlightedSource(source: string, type: string): string {
    // HTML and CEM-ML are colored by the shared Rust/WASM formatter. If that
    // runtime is unavailable, preserve safe visible source instead of keeping
    // a second markup tokenizer here.
    if (type === 'html' || type === 'cem-ml') return escapeHtml(source);
    if (type === 'text' || type === 'plain') return escapeHtml(source);
    return tokenize(source, TOKEN_PATTERN, generalTag);
}

function tokenize(
    source: string,
    pattern: RegExp,
    tagForToken: (token: string) => string | undefined
): string {
    let result = '';
    let offset = 0;
    pattern.lastIndex = 0;
    for (const match of source.matchAll(pattern)) {
        const index = match.index;
        result += escapeHtml(source.slice(offset, index));
        const token = escapeHtml(match[0]);
        const tag = tagForToken(match[0]);
        result += tag ? `<${tag}>${token}</${tag}>` : token;
        offset = index + match[0].length;
    }
    return result + escapeHtml(source.slice(offset));
}

function generalTag(token: string): string | undefined {
    if (token.startsWith('//') || token.startsWith('/*')) return 'small';
    if (/^["'`]/.test(token)) return 'i';
    if (/^\d/.test(token)) return 'u';
    if (/^(?:const|let|var|function|return|if|else|for|of|class|extends|new|import|export|from|async|await|true|false|null|undefined)$/.test(token)) {
        return 'strong';
    }
    if (/^[@$]?[A-Za-z_]/.test(token)) return 'var';
    return undefined;
}

function escapeHtml(value: string): string {
    return value
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;')
        .replaceAll("'", '&#39;');
}
