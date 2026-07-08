export type CemNode = {
    kind: 'element' | 'text';
    name?: string;
    value?: string;
    colorRole?: string;
    formatLayout?: Record<string, unknown>;
    style?: Record<string, unknown>;
    writerAttributeNodes?: WriterAttributeNode[];
    colorWrapperNodes?: Array<Record<string, unknown>>;
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
    writerBoundaries?: Array<Record<string, unknown>>;
    nodes: CemNode[];
};

export type CemtPipelineFixture = {
    cemtSource: string;
    sourcePaths: string[];
    sourceAst: CemNode;
    formatterName: string;
    colorizerName: string;
    colorProfile: string;
    contentType: string;
    schema: string;
    category: string;
    formattedDecision: string;
    coloredDecision: string;
    queuedEditDecision: string;
    writerBoundaryStage: string;
    writerBoundaryDecision: string;
    elementClass: string;
    textClass: string;
    keywordClass: string;
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

export type CemtPipelineSourceInput =
    | string
    | readonly {
          path: string;
          source: string;
      }[];

export function createCemtPipelineShowcase(
    cemtSourceInput: CemtPipelineSourceInput,
    stageFixtureSource: string
): CemtPipelineShowcase {
    const source = normalizeCemtSourceInput(cemtSourceInput);
    const fixture = createPipelineFixture(source.analysisSource, source.displaySource, source.sourcePaths);
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
        Array.isArray(tree.writerBoundaries) &&
        tree.writerBoundaries.some((boundary) => boundary['stage'] === 'after-color') &&
        tree.nodes.every(nodeWriterReady)
    );
}

export function writeColoredTreeToHtml(tree: CemTree): string {
    if (!writerReady(tree)) {
        throw new Error('colored CEM tree is required before writer output');
    }
    return tree.nodes.map(writeNodeToHtml).join('');
}

function createPipelineFixture(
    cemtSource: string,
    displaySource: string,
    sourcePaths: string[]
): CemtPipelineFixture {
    return {
        cemtSource: displaySource,
        sourcePaths,
        sourceAst: {
            kind: 'element',
            name: 'article',
            children: [
                { kind: 'text', value: 'Ready ' },
                {
                    kind: 'element',
                    name: 'strong',
                    children: [{ kind: 'text', value: 'now' }],
                },
                { kind: 'text', value: '.' },
            ],
        },
        formatterName: requiredFunctionAttribute(cemtSource, 'format-function', 'name'),
        colorizerName: requiredFunctionAttribute(cemtSource, 'color-function', 'name'),
        colorProfile: requiredFunctionAttribute(cemtSource, 'color-function', 'profile'),
        contentType: requiredFunctionAttribute(cemtSource, 'format-function', 'content-type'),
        schema: requiredFunctionAttribute(cemtSource, 'format-function', 'schema'),
        category: requiredFunctionAttribute(cemtSource, 'format-function', 'category'),
        formattedDecision: requiredValueAfter(cemtSource, 'formatterRole: "formatter.showcase"'),
        coloredDecision: requiredValueAfter(cemtSource, 'colorizerRole: "colorizer.showcase"'),
        queuedEditDecision: requiredValueAfter(cemtSource, 'colorizerRole: "colorizer.queued-edit"'),
        writerBoundaryStage: requiredStringFieldAfter(cemtSource, 'kind: "writer-boundary"', 'stage'),
        writerBoundaryDecision: requiredStringFieldAfter(cemtSource, 'kind: "writer-boundary"', 'value'),
        elementClass: requiredColorClass(cemtSource, 'syntax-name'),
        textClass: requiredColorClass(cemtSource, 'syntax-string'),
        keywordClass: requiredColorClass(cemtSource, 'syntax-keyword'),
    };
}

function normalizeCemtSourceInput(input: CemtPipelineSourceInput): {
    analysisSource: string;
    displaySource: string;
    sourcePaths: string[];
} {
    if (typeof input === 'string') {
        return {
            analysisSource: input,
            displaySource: input,
            sourcePaths: [],
        };
    }

    const sourcePaths = input.map((file) => file.path);
    return {
        analysisSource: input.map((file) => file.source).join('\n\n'),
        displaySource: input
            .map((file) => `// ${file.path}\n${file.source.trimEnd()}`)
            .join('\n\n'),
        sourcePaths,
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
        nodes: [formatNode(fixture.sourceAst)],
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
        writerBoundaries: [
            {
                kind: 'writer-boundary',
                stage: fixture.writerBoundaryStage,
                value: fixture.writerBoundaryDecision,
            },
        ],
        nodes: formatted.nodes.map((node) => colorNode(fixture, node)),
    };
}

function formatNode(node: CemNode): CemNode {
    if (node.kind === 'text') {
        return cloneNode(node);
    }
    return {
        ...cloneNode(node),
        formatLayout: {
            kind: 'format-decision',
            formatterRole: node.name === 'strong' ? 'formatter.inline-emphasis' : 'formatter.layout',
            value: node.name === 'strong' ? 'inline-emphasis' : 'inline',
        },
        children: node.children?.map(formatNode),
    };
}

function colorNode(
    fixture: CemtPipelineFixture,
    node: CemNode,
    role = 'syntax.string',
    className = fixture.textClass
): CemNode {
    if (node.kind === 'text') {
        return colorTextWrapper(fixture, node, role, className);
    }
    const isKeyword = node.name === 'strong';
    const elementRole = isKeyword ? 'syntax.keyword' : 'syntax.name';
    const elementClass = isKeyword ? fixture.keywordClass : fixture.elementClass;
    const childRole = isKeyword ? 'syntax.keyword' : 'syntax.string';
    const childClass = isKeyword ? fixture.keywordClass : fixture.textClass;
    return {
        ...node,
        colorRole: elementRole,
        style: { colorRole: elementRole, colorProfile: fixture.colorProfile },
        writerAttributeNodes: [writerClassAttribute(elementClass, fixture.colorProfile)],
        children: node.children?.map((child) => colorNode(fixture, child, childRole, childClass)),
    };
}

function colorTextWrapper(fixture: CemtPipelineFixture, child: CemNode, role: string, className: string): CemNode {
    return {
        kind: 'element',
        name: 'span',
        colorRole: role,
        style: { colorRole: role, colorProfile: fixture.colorProfile },
        writerAttributeNodes: [writerClassAttribute(className, fixture.colorProfile)],
        colorWrapperNodes: [
            {
                kind: 'color-wrapper',
                name: 'span',
                colorizerOwned: true,
                colorizerRole: 'colorizer.text-wrapper',
                colorProfile: fixture.colorProfile,
            },
            {
                kind: 'color-decision',
                name: 'wrapped-role',
                value: role,
                colorizerOwned: true,
                colorizerRole: 'colorizer.wrapped-role',
                colorProfile: fixture.colorProfile,
            },
            ...(role === 'syntax.keyword'
                ? [
                      {
                          kind: 'color-decision',
                          name: 'queued-edit',
                          value: fixture.queuedEditDecision,
                          colorizerOwned: true,
                          colorizerRole: 'colorizer.queued-edit',
                          colorProfile: fixture.colorProfile,
                      },
                  ]
                : []),
        ],
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
        formatLayout: node.formatLayout ? { ...node.formatLayout } : undefined,
        style: node.style ? { ...node.style } : undefined,
        writerAttributeNodes: node.writerAttributeNodes?.map((attribute) => ({ ...attribute })),
        colorWrapperNodes: node.colorWrapperNodes?.map((metadata) => ({ ...metadata })),
        children: node.children?.map(cloneNode),
    };
}

function requiredFunctionAttribute(source: string, elementName: string, attributeName: string): string {
    const declarations = Array.from(
        source.matchAll(new RegExp(String.raw`^    \{${elementName}\b[\s\S]*?^    \}`, 'gm')),
        (match) => match[0]
    );
    const declaration =
        declarations.find((block) => block.includes('@visibility="public"')) ?? declarations[0] ?? '';
    const value = declaration.match(new RegExp(String.raw`@${attributeName}="([^"]+)"`))?.[1];
    invariant(value, `missing @${attributeName} on ${elementName}`);
    return value;
}

function requiredValueAfter(source: string, marker: string): string {
    return requiredStringFieldAfter(source, marker, 'value');
}

function requiredStringFieldAfter(source: string, marker: string, field: string): string {
    const start = source.indexOf(marker);
    invariant(start >= 0, `missing CEMT marker ${marker}`);
    const value = source.slice(start).match(new RegExp(String.raw`${field}:\s*"([^"]+)"`))?.[1];
    invariant(value, `missing ${field} after ${marker}`);
    return value;
}

function requiredColorClass(source: string, role: string): string {
    const value = `cem-color cem-color-${role}`;
    invariant(source.includes(`"${value}"`), `missing writer class ${value}`);
    return value;
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
        fixtureSource.endsWith('formatter-coloring-pipeline.cemt') || fixtureSource.endsWith('/package.cem'),
        `unexpected CEMT pipeline fixture source ${fixtureSource}`
    );
    if (fixtureSource.endsWith('/package.cem')) {
        invariant(
            fixture.sourcePaths.some((path) => path.includes('/formatters/formatter-coloring-pipeline.cemt')),
            'schema-package CEMT pipeline fixture is missing formatter source path'
        );
        invariant(
            fixture.sourcePaths.some((path) => path.includes('/colorizers/formatter-coloring-pipeline.cemt')),
            'schema-package CEMT pipeline fixture is missing colorizer source path'
        );
    }
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
