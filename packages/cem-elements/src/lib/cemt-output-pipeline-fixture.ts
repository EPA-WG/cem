export type CemNode = {
    kind: 'element' | 'text';
    name?: string;
    value?: string;
    colorRole?: string;
    writerAttributeNodes?: WriterAttributeNode[];
    children?: CemNode[];
};

export type WriterAttributeNode = {
    kind: 'writer-attribute';
    name: string;
    value: string;
    colorizerOwned: true;
    colorizerRole: string;
    colorProfile: string;
};

export type CemTree = {
    kind: 'cem-tree';
    contentType: string;
    schema: string;
    category: string;
    mode: 'fragment';
    canonical: boolean;
    formatterProfile: string;
    formatNodes: Array<Record<string, unknown>>;
    colored?: true;
    colorProfile?: string;
    colorNodes?: Array<Record<string, unknown>>;
    nodes: CemNode[];
};

export type CemtPipelineFixture = {
    cemtSource: string;
    sourceAst: CemNode;
    formatterName: string;
    colorizerName: string;
    colorProfile: string;
    contentType: string;
    schema: string;
    category: string;
    formattedDecision: string;
    coloredDecision: string;
    elementClass: string;
    textClass: string;
};

export type CemtPipelineShowcase = {
    fixture: CemtPipelineFixture;
    sourceAst: CemNode;
    formattedTree: CemTree;
    coloredTree: CemTree;
    sourceAstCem: string;
    formattedTreeCem: string;
    coloredTreeCem: string;
};

export function createCemtPipelineShowcase(cemtSource: string): CemtPipelineShowcase {
    const fixture = createPipelineFixture(cemtSource);
    const sourceAst = fixture.sourceAst;
    const formattedTree = formatCemTree(fixture);
    const coloredTree = colorCemTree(fixture, formattedTree);
    return {
        fixture,
        sourceAst,
        formattedTree,
        coloredTree,
        sourceAstCem: writeCemNodeSource(sourceAst),
        formattedTreeCem: writeCemTreeSource(formattedTree),
        coloredTreeCem: writeCemTreeSource(coloredTree),
    };
}

export function writerReady(tree: CemTree): boolean {
    return (
        tree.colored === true &&
        tree.colorProfile === 'classes' &&
        Array.isArray(tree.colorNodes) &&
        tree.nodes.every(nodeWriterReady)
    );
}

export function writeColoredTreeToHtml(tree: CemTree): string {
    if (!writerReady(tree)) {
        throw new Error('colored CEM tree is required before writer output');
    }
    return tree.nodes.map(writeNodeToHtml).join('');
}

export function writeCemTreeSource(tree: CemTree): string {
    const attributes = [
        cemAttribute('content-type', tree.contentType),
        cemAttribute('schema', tree.schema),
        cemAttribute('category', tree.category),
        cemAttribute('mode', tree.mode),
        cemAttribute('canonical', tree.canonical),
        cemAttribute('formatter-profile', tree.formatterProfile),
        tree.colored ? cemAttribute('colored', true) : '',
        tree.colorProfile ? cemAttribute('color-profile', tree.colorProfile) : '',
    ].filter(Boolean);
    const children = [
        writeMetadataListSource('format-nodes', tree.formatNodes, 1),
        tree.colorNodes ? writeMetadataListSource('color-nodes', tree.colorNodes, 1) : '',
        writeNodeListSource('nodes', tree.nodes, 1),
    ].filter(Boolean);

    return `{cem-tree ${attributes.join(' ')} |\n${children.join('\n')}\n}`;
}

export function writeCemNodeSource(node: CemNode, depth = 0): string {
    if (node.kind === 'text') {
        return `${indent(depth)}{text | ${escapeCemText(node.value ?? '')}}`;
    }

    const name = node.name ?? 'element';
    const attributes = [node.colorRole ? cemAttribute('color-role', node.colorRole) : ''].filter(Boolean);
    const children = [
        ...(node.writerAttributeNodes ?? []).map((attribute) => writeWriterAttributeSource(attribute, depth + 1)),
        ...(node.children ?? []).map((child) => writeCemNodeSource(child, depth + 1)),
    ];

    if (children.length === 0) {
        return `${indent(depth)}{${name}${attributes.length ? ` ${attributes.join(' ')}` : ''}}`;
    }
    return `${indent(depth)}{${name}${attributes.length ? ` ${attributes.join(' ')}` : ''} |\n${children.join('\n')}\n${indent(depth)}}`;
}

function createPipelineFixture(cemtSource: string): CemtPipelineFixture {
    const writerClasses = requiredMatches(
        cemtSource,
        /name:\s*"class",\s*value:\s*"([^"]+)"/g,
        2,
        'writer class values'
    );
    return {
        cemtSource,
        sourceAst: {
            kind: 'element',
            name: 'article',
            children: [{ kind: 'text', value: 'Ready' }],
        },
        formatterName: requiredFunctionAttribute(cemtSource, 'format-function', 'name'),
        colorizerName: requiredFunctionAttribute(cemtSource, 'color-function', 'name'),
        colorProfile: requiredFunctionAttribute(cemtSource, 'color-function', 'profile'),
        contentType: requiredFunctionAttribute(cemtSource, 'format-function', 'content-type'),
        schema: requiredFunctionAttribute(cemtSource, 'format-function', 'schema'),
        category: requiredFunctionAttribute(cemtSource, 'format-function', 'category'),
        formattedDecision: requiredValueAfter(cemtSource, 'formatterRole: "formatter.showcase"'),
        coloredDecision: requiredValueAfter(cemtSource, 'colorizerRole: "colorizer.showcase"'),
        elementClass: writerClasses[0],
        textClass: writerClasses[1],
    };
}

function formatCemTree(fixture: CemtPipelineFixture): CemTree {
    return {
        kind: 'cem-tree',
        contentType: fixture.contentType,
        schema: fixture.schema,
        category: fixture.category,
        mode: 'fragment',
        canonical: true,
        formatterProfile: fixture.formatterName,
        formatNodes: [
            {
                kind: 'format-marker',
                name: 'cem.format-tree',
                formatterRole: 'formatter.boundary',
                formatterProfile: fixture.formatterName,
            },
            {
                kind: 'format-decision',
                name: 'showcase',
                formatterRole: 'formatter.showcase',
                value: fixture.formattedDecision,
            },
        ],
        nodes: [cloneNode(fixture.sourceAst)],
    };
}

function colorCemTree(fixture: CemtPipelineFixture, formatted: CemTree): CemTree {
    return {
        ...formatted,
        colored: true,
        colorProfile: fixture.colorProfile,
        colorNodes: [
            {
                kind: 'color-marker',
                name: 'cem.color-tree',
                colorizerRole: 'colorizer.boundary',
                colorProfile: fixture.colorProfile,
            },
            {
                kind: 'color-decision',
                name: 'showcase',
                colorizerRole: 'colorizer.showcase',
                value: fixture.coloredDecision,
            },
        ],
        nodes: formatted.nodes.map((node) => colorRootNode(fixture, node)),
    };
}

function colorRootNode(fixture: CemtPipelineFixture, node: CemNode): CemNode {
    const [firstChild, ...restChildren] = node.children ?? [];
    return {
        ...node,
        colorRole: 'syntax.name',
        writerAttributeNodes: [writerClassAttribute(fixture.elementClass, fixture.colorProfile)],
        children: firstChild
            ? [colorTextWrapper(fixture, firstChild), ...restChildren.map(cloneNode)]
            : restChildren.map(cloneNode),
    };
}

function colorTextWrapper(fixture: CemtPipelineFixture, child: CemNode): CemNode {
    return {
        kind: 'element',
        name: 'span',
        colorRole: 'syntax.string',
        writerAttributeNodes: [writerClassAttribute(fixture.textClass, fixture.colorProfile)],
        children: [cloneNode(child)],
    };
}

function writerClassAttribute(value: string, colorProfile: string): WriterAttributeNode {
    return {
        kind: 'writer-attribute',
        name: 'class',
        value,
        colorizerOwned: true,
        colorizerRole: 'colorizer.writer-attribute',
        colorProfile,
    };
}

function cloneNode(node: CemNode): CemNode {
    return {
        ...node,
        writerAttributeNodes: node.writerAttributeNodes?.map((attribute) => ({ ...attribute })),
        children: node.children?.map(cloneNode),
    };
}

function requiredFunctionAttribute(source: string, elementName: string, attributeName: string): string {
    const pattern = new RegExp(String.raw`\{${elementName}[\s\S]*?@${attributeName}="([^"]+)"`);
    const value = source.match(pattern)?.[1];
    invariant(value, `missing @${attributeName} on ${elementName}`);
    return value;
}

function requiredValueAfter(source: string, marker: string): string {
    const start = source.indexOf(marker);
    invariant(start >= 0, `missing CEMT marker ${marker}`);
    const value = source.slice(start).match(/value:\s*"([^"]+)"/)?.[1];
    invariant(value, `missing value after ${marker}`);
    return value;
}

function requiredMatches(source: string, pattern: RegExp, count: number, label: string): string[] {
    const values = Array.from(source.matchAll(pattern), (match) => match[1]);
    invariant(values.length >= count, `expected at least ${count} ${label}`);
    return values;
}

function writeMetadataListSource(name: string, values: Array<Record<string, unknown>>, depth: number): string {
    const children = values.map((value) => writeMetadataSource(value, depth + 1)).join('\n');
    return `${indent(depth)}{${name} |\n${children}\n${indent(depth)}}`;
}

function writeMetadataSource(value: Record<string, unknown>, depth: number): string {
    const kind = requiredString(value.kind, 'metadata kind');
    const attributes = Object.entries(value)
        .filter(([name]) => name !== 'kind')
        .map(([name, attributeValue]) => cemAttribute(kebabCase(name), attributeValue))
        .filter(Boolean);
    return `${indent(depth)}{${kind}${attributes.length ? ` ${attributes.join(' ')}` : ''}}`;
}

function writeNodeListSource(name: string, nodes: CemNode[], depth: number): string {
    const children = nodes.map((node) => writeCemNodeSource(node, depth + 1)).join('\n');
    return `${indent(depth)}{${name} |\n${children}\n${indent(depth)}}`;
}

function writeWriterAttributeSource(attribute: WriterAttributeNode, depth: number): string {
    const attributes = [
        cemAttribute('name', attribute.name),
        cemAttribute('value', attribute.value),
        cemAttribute('colorizer-owned', attribute.colorizerOwned),
        cemAttribute('colorizer-role', attribute.colorizerRole),
        cemAttribute('color-profile', attribute.colorProfile),
    ];
    return `${indent(depth)}{writer-attribute ${attributes.join(' ')}}`;
}

function nodeWriterReady(node: CemNode): boolean {
    if (node.kind === 'text') {
        return true;
    }
    if (node.colorRole && !writerAttribute(node, 'class')) {
        return false;
    }
    return (node.children ?? []).every(nodeWriterReady);
}

function writeNodeToHtml(node: CemNode): string {
    if (node.kind === 'text') {
        return escapeHtml(node.value ?? '');
    }
    const name = node.name ?? 'span';
    const attributes = writerAttribute(node, 'class');
    const classAttribute = attributes ? ` class="${escapeHtmlAttribute(attributes.value)}"` : '';
    const children = (node.children ?? []).map(writeNodeToHtml).join('');
    return `<${name}${classAttribute}>${children}</${name}>`;
}

function writerAttribute(node: CemNode, name: string): WriterAttributeNode | undefined {
    return node.writerAttributeNodes?.find((attribute) => attribute.name === name);
}

function cemAttribute(name: string, value: unknown): string {
    if (typeof value === 'boolean') {
        return `@${name}=${String(value)}`;
    }
    if (typeof value === 'string') {
        return `@${name}="${escapeCemAttribute(value)}"`;
    }
    return '';
}

function requiredString(value: unknown, label: string): string {
    invariant(typeof value === 'string' && value.length > 0, `missing ${label}`);
    return value;
}

function kebabCase(value: string): string {
    return value.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
}

function indent(depth: number): string {
    return '    '.repeat(depth);
}

function escapeCemAttribute(value: string): string {
    return value.replaceAll('\\', '\\\\').replaceAll('"', '\\"').replaceAll('\n', '\\n');
}

function escapeCemText(value: string): string {
    return value.replaceAll('\\', '\\\\').replaceAll('{', '\\{').replaceAll('}', '\\}');
}

function escapeHtml(value: string): string {
    return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}

function escapeHtmlAttribute(value: string): string {
    return escapeHtml(value).replaceAll('"', '&quot;');
}

function invariant(condition: unknown, message: string): asserts condition {
    if (!condition) {
        throw new Error(message);
    }
}
