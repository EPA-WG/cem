const TOKEN_PATTERN = /<!--[\s\S]*?-->|\/\*[\s\S]*?\*\/|\/\/[^\n]*|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|\b(?:const|let|var|function|return|if|else|for|of|class|extends|new|import|export|from|async|await|true|false|null|undefined)\b|\b\d+(?:\.\d+)?\b|[@$]?[A-Za-z_][\w:-]*(?=\s*=)|[{}[\]():;,.|=@$]+/g;
const MARKUP_PATTERN = /<!--[\s\S]*?-->|<![^>]*>|<\/?[A-Za-z][^>]*>/g;

export function highlightedSource(source: string, type: string): string {
    if (isMarkup(type)) return tokenize(source, MARKUP_PATTERN, markupClass);
    if (type === 'text' || type === 'plain') return escapeHtml(source);
    return tokenize(source, TOKEN_PATTERN, generalClass);
}

function tokenize(
    source: string,
    pattern: RegExp,
    classify: (token: string) => string
): string {
    let result = '';
    let offset = 0;
    pattern.lastIndex = 0;
    for (const match of source.matchAll(pattern)) {
        const index = match.index;
        result += escapeHtml(source.slice(offset, index));
        result += `<span class="cem-demo-token ${classify(match[0])}">${escapeHtml(match[0])}</span>`;
        offset = index + match[0].length;
    }
    return result + escapeHtml(source.slice(offset));
}

function isMarkup(type: string): boolean {
    return type === 'html' || type === 'xml' || type === 'svg';
}

function markupClass(token: string): string {
    return token.startsWith('<!--') ? 'cem-demo-comment' : 'cem-demo-tag';
}

function generalClass(token: string): string {
    if (token.startsWith('//') || token.startsWith('/*')) return 'cem-demo-comment';
    if (/^["'`]/.test(token)) return 'cem-demo-string';
    if (/^\d/.test(token)) return 'cem-demo-number';
    if (/^(?:const|let|var|function|return|if|else|for|of|class|extends|new|import|export|from|async|await|true|false|null|undefined)$/.test(token)) {
        return 'cem-demo-keyword';
    }
    if (/^[@$]?[A-Za-z_]/.test(token)) return 'cem-demo-name';
    return 'cem-demo-punctuation';
}

function escapeHtml(value: string): string {
    return value
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;')
        .replaceAll("'", '&#39;');
}
