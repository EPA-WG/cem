export type CemDiagnosticSeverity = 'error' | 'warning' | 'info';

export interface CemSourceLocation {
    offset: number;
    line: number;
    column: number;
}

export interface CemDiagnostic {
    code: string;
    severity: CemDiagnosticSeverity;
    message: string;
    location?: CemSourceLocation;
    node?: string;
}

export interface CemDomAttribute {
    name: string;
    value: string;
    location: CemSourceLocation;
}

export interface CemDomTextNode {
    type: 'text';
    value: string;
    location: CemSourceLocation;
}

export interface CemDomElementNode {
    type: 'element';
    tagName: string;
    attributes: CemDomAttribute[];
    children: CemDomNode[];
    location: CemSourceLocation;
}

export type CemDomNode = CemDomElementNode | CemDomTextNode;

export interface CemDomDocument {
    sourceName?: string;
    rootNodes: CemDomNode[];
    elements: CemDomElementNode[];
    diagnostics: CemDiagnostic[];
}

export interface ParseCemDomOptions {
    sourceName?: string;
}

export interface ValidateCemDomOptions extends ParseCemDomOptions {
    requireSemanticRoot?: boolean;
}

const voidElements = new Set([
    'area',
    'base',
    'br',
    'col',
    'embed',
    'hr',
    'img',
    'input',
    'link',
    'meta',
    'param',
    'source',
    'track',
    'wbr',
]);

const refAttributes = new Set([
    'aria-activedescendant',
    'aria-controls',
    'aria-describedby',
    'aria-details',
    'aria-errormessage',
    'aria-flowto',
    'aria-labelledby',
    'aria-owns',
    'for',
    'form',
    'headers',
    'list',
]);

const accessibleNameRoles = new Set(['data-cem-screen', 'data-cem-form', 'data-cem-action']);

export function parseCemDom(source: string, options: ParseCemDomOptions = {}): CemDomDocument {
    const diagnostics: CemDiagnostic[] = [];
    const elements: CemDomElementNode[] = [];
    const rootNodes: CemDomNode[] = [];
    const stack: CemDomElementNode[] = [];
    const tokenPattern = /<!--[\s\S]*?-->|<!doctype[^>]*>|<\/?[a-zA-Z][^>]*>|[^<]+|</gi;

    for (const tokenMatch of source.matchAll(tokenPattern)) {
        const token = tokenMatch[0];
        const offset = tokenMatch.index ?? 0;
        const location = getLocation(source, offset);

        if (token.startsWith('<!--') || /^<!doctype/i.test(token)) {
            continue;
        }

        if (token === '<') {
            diagnostics.push({
                code: 'parse.invalid-lt',
                severity: 'error',
                message: 'Unexpected "<" in text content.',
                location,
            });
            continue;
        }

        if (!token.startsWith('<')) {
            if (token.trim().length === 0) {
                continue;
            }

            appendNode({ type: 'text', value: decodeHtmlText(token), location }, stack, rootNodes);
            continue;
        }

        if (token.startsWith('</')) {
            closeElement(token, stack, diagnostics, location);
            continue;
        }

        const parsed = parseStartTag(token, source, offset, location);
        if (!parsed) {
            diagnostics.push({
                code: 'parse.invalid-tag',
                severity: 'error',
                message: `Could not parse tag token: ${token}`,
                location,
            });
            continue;
        }

        elements.push(parsed.node);
        appendNode(parsed.node, stack, rootNodes);

        if (!parsed.selfClosing && !voidElements.has(parsed.node.tagName)) {
            stack.push(parsed.node);
        }
    }

    while (stack.length > 0) {
        const node = stack.pop();
        if (!node) {
            continue;
        }
        diagnostics.push({
            code: 'parse.unclosed-element',
            severity: 'error',
            message: `Unclosed <${node.tagName}> element.`,
            location: node.location,
            node: describeNode(node),
        });
    }

    return {
        sourceName: options.sourceName,
        rootNodes,
        elements,
        diagnostics,
    };
}

export function validateCemDom(source: string, options: ValidateCemDomOptions = {}): CemDiagnostic[] {
    const document = parseCemDom(source, options);
    const diagnostics = [...document.diagnostics];
    const ids = new Map<string, CemDomElementNode>();

    for (const element of document.elements) {
        const id = getAttribute(element, 'id');
        if (!id) {
            continue;
        }

        if (ids.has(id.value)) {
            diagnostics.push({
                code: 'validate.duplicate-id',
                severity: 'error',
                message: `Duplicate id "${id.value}".`,
                location: id.location,
                node: describeNode(element),
            });
        } else {
            ids.set(id.value, element);
        }
    }

    for (const element of document.elements) {
        validateReferences(element, ids, diagnostics);
        validateUnsafeContent(element, diagnostics);
        validateAccessibleName(element, ids, diagnostics);
    }

    const hasSemanticElement = document.elements.some((element) =>
        element.attributes.some((attribute) => attribute.name.startsWith('data-cem-')),
    );

    if (options.requireSemanticRoot !== false && !hasSemanticElement) {
        diagnostics.push({
            code: 'validate.missing-semantic-root',
            severity: 'warning',
            message: 'No data-cem-* semantic element was found.',
            location: document.rootNodes[0]?.location,
        });
    }

    return diagnostics;
}

export function formatDiagnostics(diagnostics: readonly CemDiagnostic[]): string {
    if (diagnostics.length === 0) {
        return 'No CEM DOM diagnostics.';
    }

    return diagnostics
        .map((diagnostic) => {
            const location = diagnostic.location
                ? `${diagnostic.location.line}:${diagnostic.location.column}`
                : '-';
            const node = diagnostic.node ? ` ${diagnostic.node}` : '';
            return `${diagnostic.severity.toUpperCase()} ${diagnostic.code} ${location}${node} ${diagnostic.message}`;
        })
        .join('\n');
}

function appendNode(node: CemDomNode, stack: CemDomElementNode[], rootNodes: CemDomNode[]): void {
    const parent = stack.at(-1);
    if (parent) {
        parent.children.push(node);
    } else {
        rootNodes.push(node);
    }
}

function closeElement(
    token: string,
    stack: CemDomElementNode[],
    diagnostics: CemDiagnostic[],
    location: CemSourceLocation,
): void {
    const tagName = token.slice(2, -1).trim().toLowerCase();
    let openIndex = -1;
    for (let index = stack.length - 1; index >= 0; index -= 1) {
        if (stack[index]?.tagName === tagName) {
            openIndex = index;
            break;
        }
    }

    if (openIndex === -1) {
        diagnostics.push({
            code: 'parse.unmatched-close',
            severity: 'error',
            message: `Closing </${tagName}> has no matching open element.`,
            location,
        });
        return;
    }

    for (let index = stack.length - 1; index >= openIndex; index -= 1) {
        const node = stack.pop();
        if (node && node.tagName !== tagName) {
            diagnostics.push({
                code: 'parse.misnested-element',
                severity: 'error',
                message: `Element <${node.tagName}> was closed by </${tagName}>.`,
                location,
                node: describeNode(node),
            });
        }
    }
}

function parseStartTag(
    token: string,
    source: string,
    tokenOffset: number,
    location: CemSourceLocation,
): { node: CemDomElementNode; selfClosing: boolean } | undefined {
    const match = /^<([a-zA-Z][^\s/>]*)([\s\S]*?)\/?>$/.exec(token);
    if (!match) {
        return undefined;
    }

    const [, rawTagName, rawAttributes] = match;
    const tagName = rawTagName.toLowerCase();
    const attributes = parseAttributes(rawAttributes, source, tokenOffset + rawTagName.length + 1);

    return {
        node: {
            type: 'element',
            tagName,
            attributes,
            children: [],
            location,
        },
        selfClosing: /\/>$/.test(token),
    };
}

function parseAttributes(rawAttributes: string, source: string, offsetBase: number): CemDomAttribute[] {
    const attributes: CemDomAttribute[] = [];
    const attributePattern =
        /([^\s"'=<>`/]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;

    for (const match of rawAttributes.matchAll(attributePattern)) {
        const rawName = match[1];
        if (!rawName) {
            continue;
        }

        const value = match[2] ?? match[3] ?? match[4] ?? '';
        attributes.push({
            name: rawName.toLowerCase(),
            value: decodeHtmlText(value),
            location: getLocation(source, offsetBase + (match.index ?? 0)),
        });
    }

    return attributes;
}

function validateReferences(
    element: CemDomElementNode,
    ids: ReadonlyMap<string, CemDomElementNode>,
    diagnostics: CemDiagnostic[],
): void {
    for (const attribute of element.attributes) {
        if (!refAttributes.has(attribute.name)) {
            continue;
        }

        for (const reference of attribute.value.split(/\s+/).filter(Boolean)) {
            if (!ids.has(reference)) {
                diagnostics.push({
                    code: 'validate.broken-reference',
                    severity: 'error',
                    message: `${attribute.name} references missing id "${reference}".`,
                    location: attribute.location,
                    node: describeNode(element),
                });
            }
        }
    }
}

function validateUnsafeContent(element: CemDomElementNode, diagnostics: CemDiagnostic[]): void {
    if (element.tagName === 'script') {
        diagnostics.push({
            code: 'validate.unsafe-script',
            severity: 'error',
            message: 'Inline <script> is not allowed in CEM semantic documents.',
            location: element.location,
            node: describeNode(element),
        });
    }

    for (const attribute of element.attributes) {
        if (attribute.name.startsWith('on')) {
            diagnostics.push({
                code: 'validate.unsafe-event-handler',
                severity: 'error',
                message: `Inline event handler "${attribute.name}" is not allowed.`,
                location: attribute.location,
                node: describeNode(element),
            });
        }

        if ((attribute.name === 'href' || attribute.name === 'src') && /^javascript:/i.test(attribute.value.trim())) {
            diagnostics.push({
                code: 'validate.unsafe-url',
                severity: 'error',
                message: `${attribute.name} must not use a javascript: URL.`,
                location: attribute.location,
                node: describeNode(element),
            });
        }

        if (attribute.name === 'srcdoc') {
            diagnostics.push({
                code: 'validate.unsafe-srcdoc',
                severity: 'warning',
                message: 'srcdoc content should be avoided in CEM semantic documents.',
                location: attribute.location,
                node: describeNode(element),
            });
        }
    }
}

function validateAccessibleName(
    element: CemDomElementNode,
    ids: ReadonlyMap<string, CemDomElementNode>,
    diagnostics: CemDiagnostic[],
): void {
    if (!element.attributes.some((attribute) => accessibleNameRoles.has(attribute.name))) {
        return;
    }

    if (getAccessibleName(element, ids).length > 0) {
        return;
    }

    diagnostics.push({
        code: 'validate.missing-accessible-name',
        severity: 'warning',
        message: `<${element.tagName}> with a CEM role should expose an accessible name.`,
        location: element.location,
        node: describeNode(element),
    });
}

function getAccessibleName(element: CemDomElementNode, ids: ReadonlyMap<string, CemDomElementNode>): string {
    const directName =
        getAttribute(element, 'aria-label')?.value ??
        getAttribute(element, 'data-cem-label')?.value ??
        getAttribute(element, 'title')?.value;

    if (directName && directName.trim().length > 0) {
        return directName.trim();
    }

    const labelledBy = getAttribute(element, 'aria-labelledby')?.value;
    if (labelledBy) {
        const referencedText = labelledBy
            .split(/\s+/)
            .filter(Boolean)
            .map((id) => ids.get(id))
            .filter((node): node is CemDomElementNode => node !== undefined)
            .map((node) => collectText(node))
            .join(' ')
            .trim();

        if (referencedText.length > 0) {
            return referencedText;
        }
    }

    const heading = findFirstElement(element, (node) => /^h[1-6]$/.test(node.tagName));
    return heading ? collectText(heading).trim() : '';
}

function findFirstElement(
    element: CemDomElementNode,
    predicate: (element: CemDomElementNode) => boolean,
): CemDomElementNode | undefined {
    for (const child of element.children) {
        if (child.type !== 'element') {
            continue;
        }

        if (predicate(child)) {
            return child;
        }

        const nested = findFirstElement(child, predicate);
        if (nested) {
            return nested;
        }
    }

    return undefined;
}

function collectText(element: CemDomElementNode): string {
    return element.children
        .map((child) => (child.type === 'text' ? child.value : collectText(child)))
        .join(' ')
        .replace(/\s+/g, ' ')
        .trim();
}

function getAttribute(element: CemDomElementNode, name: string): CemDomAttribute | undefined {
    return element.attributes.find((attribute) => attribute.name === name);
}

function describeNode(element: CemDomElementNode): string {
    const id = getAttribute(element, 'id')?.value;
    return id ? `<${element.tagName}#${id}>` : `<${element.tagName}>`;
}

function decodeHtmlText(value: string): string {
    return value
        .replaceAll('&lt;', '<')
        .replaceAll('&gt;', '>')
        .replaceAll('&quot;', '"')
        .replaceAll('&#39;', "'")
        .replaceAll('&apos;', "'")
        .replaceAll('&amp;', '&');
}

function getLocation(source: string, offset: number): CemSourceLocation {
    const prefix = source.slice(0, offset);
    const lines = prefix.split(/\r\n|\n|\r/);
    const currentLine = lines[lines.length - 1] ?? '';
    return {
        offset,
        line: lines.length,
        column: currentLine.length + 1,
    };
}
