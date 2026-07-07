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

export function createCemtPipelineShowcase(
    cemtSource: string,
    stageFixtureSource: string
): CemtPipelineShowcase {
    const fixture = createPipelineFixture(cemtSource);
    const stages = parseStageFixture(stageFixtureSource, fixture);
    const sourceAst = fixture.sourceAst;
    const formattedTree = formatCemTree(fixture);
    const coloredTree = colorCemTree(fixture, formattedTree);
    return {
        fixture,
        sourceAst,
        formattedTree,
        coloredTree,
        sourceAstCem: stages.sourceAstCem,
        formattedTreeCem: stages.formattedTreeCem,
        coloredTreeCem: stages.coloredTreeCem,
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

type ParsedStageFixture = {
    sourceAstCem: string;
    formattedTreeCem: string;
    coloredTreeCem: string;
};

function parseStageFixture(source: string, fixture: CemtPipelineFixture): ParsedStageFixture {
    assertStageFixtureMetadata(source, fixture);
    const stages = new Map<string, string>();
    const pattern = /\{stage\b([^|]*?)\|\s*```cem\r?\n([\s\S]*?)\r?\n```\s*\}/g;
    for (const match of source.matchAll(pattern)) {
        const name = match[1].match(/@name="([^"]+)"/)?.[1];
        invariant(name, 'CEMT pipeline stage fixture is missing @name');
        stages.set(name, normalizeStageSource(match[2]));
    }

    return {
        sourceAstCem: requiredStage(stages, 'source-ast'),
        formattedTreeCem: requiredStage(stages, 'formatted-cem-tree'),
        coloredTreeCem: requiredStage(stages, 'colored-cem-tree'),
    };
}

function assertStageFixtureMetadata(source: string, fixture: CemtPipelineFixture): void {
    const fixtureSource = requiredStageFixtureAttribute(source, 'source');
    invariant(
        fixtureSource.endsWith('formatter-coloring-pipeline.cemt'),
        `unexpected CEMT pipeline fixture source ${fixtureSource}`
    );
    assertEqual(
        requiredStageFixtureAttribute(source, 'formatter'),
        fixture.formatterName,
        'CEMT stage fixture formatter'
    );
    assertEqual(
        requiredStageFixtureAttribute(source, 'colorizer'),
        fixture.colorizerName,
        'CEMT stage fixture colorizer'
    );
    assertEqual(
        requiredStageFixtureAttribute(source, 'color-profile'),
        fixture.colorProfile,
        'CEMT stage fixture color profile'
    );
}

function requiredStageFixtureAttribute(source: string, name: string): string {
    const value = source.match(new RegExp(`@${name}="([^"]+)"`))?.[1];
    invariant(value, `CEMT pipeline stage fixture is missing @${name}`);
    return value;
}

function requiredStage(stages: Map<string, string>, name: string): string {
    const stage = stages.get(name);
    invariant(stage, `CEMT pipeline stage fixture is missing ${name}`);
    return stage;
}

function normalizeStageSource(source: string): string {
    return source.replace(/\r\n/g, '\n').trim();
}

function assertEqual(actual: string, expected: string, label: string): void {
    invariant(actual === expected, `${label}: expected ${expected}, got ${actual}`);
}
