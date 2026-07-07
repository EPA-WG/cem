import type { Meta, StoryObj } from '@storybook/web-components-vite';
import cemtPipelineSource from '../../../cem_ml/schema-packages/cem-transform/v1/examples/formatter-coloring-pipeline.cemt?raw';

const meta: Meta = {
    title: 'CEM Elements/CEMT Output Pipeline',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

type CemNode = {
    kind: 'element' | 'text';
    name?: string;
    value?: string;
    colorRole?: string;
    writerAttributeNodes?: WriterAttributeNode[];
    children?: CemNode[];
};

type WriterAttributeNode = {
    kind: 'writer-attribute';
    name: string;
    value: string;
    colorizerOwned: true;
    colorizerRole: string;
    colorProfile: string;
};

type CemTree = {
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

type CemtPipelineFixture = {
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

const pipelineFixture = createPipelineFixture(cemtPipelineSource);
const sourceAst = pipelineFixture.sourceAst;
const formattedTree = formatCemTree(pipelineFixture);
const coloredTree = colorCemTree(pipelineFixture, formattedTree);
const sourceAstCem = writeCemNodeSource(sourceAst);
const formattedTreeCem = writeCemTreeSource(formattedTree);
const coloredTreeCem = writeCemTreeSource(coloredTree);

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
    assert(value, `missing @${attributeName} on ${elementName}`);
    return value;
}

function requiredValueAfter(source: string, marker: string): string {
    const start = source.indexOf(marker);
    assert(start >= 0, `missing CEMT marker ${marker}`);
    const value = source.slice(start).match(/value:\s*"([^"]+)"/)?.[1];
    assert(value, `missing value after ${marker}`);
    return value;
}

function requiredMatches(source: string, pattern: RegExp, count: number, label: string): string[] {
    const values = Array.from(source.matchAll(pattern), (match) => match[1]);
    assert(values.length >= count, `expected at least ${count} ${label}`);
    return values;
}

function writeCemTreeSource(tree: CemTree): string {
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

function writeCemNodeSource(node: CemNode, depth = 0): string {
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
    assert(typeof value === 'string' && value.length > 0, `missing ${label}`);
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

export const FormatterColoringWriterStages: Story = {
    render: () => {
        const root = document.createElement('section');
        root.className = 'cemt-pipeline-showcase';
        root.append(styles(), stageGrid(), renderedOutput());
        return root;
    },
    play: ({ canvasElement }) => {
        assertEqual(
            writerReady(formattedTree),
            false,
            'formatted CEM tree must not be writer-ready before coloring'
        );
        assertEqual(writerReady(coloredTree), true, 'colored CEM tree is writer-ready');

        const output = requiredElement(canvasElement, '[data-stage="writer"] article');
        assertEqual(output.className, pipelineFixture.elementClass, 'writer receives materialized element class');
        assertEqual(
            requiredElement(output, 'span').className,
            pipelineFixture.textClass,
            'writer receives materialized text wrapper class'
        );

        const formattedPanel = requiredElement(canvasElement, '[data-stage="formatted"]');
        assert(
            formattedPanel.textContent?.includes('formatted tree before writer'),
            'formatted stage shows formatter metadata'
        );
        assert(
            formattedPanel.textContent?.includes(`@content-type="${pipelineFixture.contentType}"`),
            'formatted stage shows destination identity'
        );
        assert(
            !formattedPanel.textContent?.includes('colored tree before writer'),
            'formatted stage does not include color metadata'
        );
        assert(!formattedPanel.textContent?.includes('"formatterProfile"'), 'formatted stage is not JSONified');

        const coloredPanel = requiredElement(canvasElement, '[data-stage="colored"]');
        assert(
            coloredPanel.textContent?.includes('colored tree before writer'),
            'colored stage shows colorizer metadata'
        );

        const templatePanel = requiredElement(canvasElement, '[data-stage="cemt-source"]');
        assert(
            templatePanel.textContent?.includes(`@name="${pipelineFixture.formatterName}"`),
            'Storybook showcase displays the checked formatter CEMT source'
        );
        assert(
            templatePanel.textContent?.includes(`@name="${pipelineFixture.colorizerName}"`),
            'Storybook showcase displays the checked colorizer CEMT source'
        );
        assert(
            templatePanel.textContent?.includes('appendFormatNode('),
            'formatter CEMT source shows metadata accumulation'
        );
        assert(
            templatePanel.textContent?.includes('applyEdits('),
            'coloring CEMT source shows tree patching before writer output'
        );
        assert(
            !templatePanel.textContent?.includes('"kind":'),
            'pipeline showcase keeps CEM-native source text instead of JSON'
        );
    },
};

function stageGrid(): HTMLElement {
    const grid = document.createElement('div');
    grid.className = 'cemt-pipeline-grid';
    grid.append(
        stagePanel('Checked CEMT Source', 'cemt-source', pipelineFixture.cemtSource),
        stagePanel('Source AST', 'source', sourceAstCem),
        stagePanel('Formatted CEM Tree', 'formatted', formattedTreeCem),
        stagePanel('Colored CEM Tree', 'colored', coloredTreeCem)
    );
    return grid;
}

function stagePanel(title: string, stage: string, value: string): HTMLElement {
    const panel = document.createElement('article');
    panel.className = 'cemt-pipeline-panel';
    panel.dataset.stage = stage;

    const heading = document.createElement('h2');
    heading.textContent = title;

    const pre = document.createElement('pre');
    pre.textContent = value;

    panel.append(heading, pre);
    return panel;
}

function renderedOutput(): HTMLElement {
    const panel = document.createElement('article');
    panel.className = 'cemt-pipeline-panel cemt-pipeline-writer';
    panel.dataset.stage = 'writer';

    const heading = document.createElement('h2');
    heading.textContent = 'Writer Output';

    const output = document.createElement('div');
    output.className = 'cemt-writer-surface';
    output.innerHTML = writeColoredTreeToHtml(coloredTree);

    panel.append(heading, output);
    return panel;
}

function writerReady(tree: CemTree): boolean {
    return (
        tree.colored === true &&
        tree.colorProfile === 'classes' &&
        Array.isArray(tree.colorNodes) &&
        tree.nodes.every(nodeWriterReady)
    );
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

function writeColoredTreeToHtml(tree: CemTree): string {
    if (!writerReady(tree)) {
        throw new Error('colored CEM tree is required before writer output');
    }
    return tree.nodes.map(writeNodeToHtml).join('');
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

function styles(): HTMLStyleElement {
    const style = document.createElement('style');
    style.textContent = `
        .cemt-pipeline-showcase {
            color: #172033;
            font: 14px/1.45 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
            max-width: 1180px;
            padding: 20px;
        }

        .cemt-pipeline-grid {
            display: grid;
            gap: 12px;
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }

        .cemt-pipeline-panel {
            border: 1px solid #c8d1dc;
            border-radius: 8px;
            background: #f7f9fb;
            min-width: 0;
            overflow: hidden;
        }

        .cemt-pipeline-panel h2 {
            margin: 0;
            padding: 10px 12px;
            border-bottom: 1px solid #c8d1dc;
            background: #e9eef4;
            font-size: 14px;
            font-weight: 650;
        }

        .cemt-pipeline-panel pre {
            box-sizing: border-box;
            min-height: 340px;
            max-height: 520px;
            margin: 0;
            overflow: auto;
            padding: 12px;
            font-size: 12px;
            line-height: 1.45;
            white-space: pre-wrap;
        }

        .cemt-pipeline-writer {
            margin-top: 12px;
        }

        .cemt-writer-surface {
            padding: 16px;
            background: #ffffff;
        }

        .cemt-writer-surface article {
            display: inline-flex;
            align-items: center;
            border: 1px solid #9ab5c0;
            border-radius: 6px;
            padding: 10px 14px;
            background: #eef9fb;
        }

        .cem-color-syntax-name {
            color: #087990;
        }

        .cem-color-syntax-string {
            color: #067647;
            font-weight: 650;
        }

        @media (max-width: 860px) {
            .cemt-pipeline-grid {
                grid-template-columns: 1fr;
            }
        }
    `;
    return style;
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector(selector);
    assert(element instanceof HTMLElement, `missing element ${selector}`);
    return element;
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) {
        throw new Error(message);
    }
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${String(expected)}, got ${String(actual)}`);
    }
}
