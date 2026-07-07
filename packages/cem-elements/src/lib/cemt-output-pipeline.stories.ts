import type { Meta, StoryObj } from '@storybook/web-components-vite';

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
    formatterProfile: string;
    formatNodes: Array<Record<string, unknown>>;
    colored?: true;
    colorProfile?: string;
    colorNodes?: Array<Record<string, unknown>>;
    nodes: CemNode[];
};

const sourceAst = {
    kind: 'element',
    name: 'article',
    children: [{ kind: 'text', value: 'Ready' }],
};

const formattedTree: CemTree = {
    kind: 'cem-tree',
    formatterProfile: 'acme.showcase.format-tree',
    formatNodes: [
        {
            kind: 'format-marker',
            name: 'cem.format-tree',
            formatterRole: 'formatter.boundary',
            formatterProfile: 'acme.showcase.format-tree',
        },
        {
            kind: 'format-decision',
            name: 'showcase',
            formatterRole: 'formatter.showcase',
            value: 'formatted tree before writer',
        },
    ],
    nodes: [
        {
            kind: 'element',
            name: 'article',
            children: [{ kind: 'text', value: 'Ready' }],
        },
    ],
};

const coloredTree: CemTree = {
    ...formattedTree,
    colored: true,
    colorProfile: 'classes',
    colorNodes: [
        {
            kind: 'color-marker',
            name: 'cem.color-tree',
            colorizerRole: 'colorizer.boundary',
            colorProfile: 'classes',
        },
        {
            kind: 'color-decision',
            name: 'showcase',
            colorizerRole: 'colorizer.showcase',
            value: 'colored tree before writer',
        },
    ],
    nodes: [
        {
            kind: 'element',
            name: 'article',
            colorRole: 'syntax.name',
            writerAttributeNodes: [
                {
                    kind: 'writer-attribute',
                    name: 'class',
                    value: 'cem-color cem-color-syntax-name',
                    colorizerOwned: true,
                    colorizerRole: 'colorizer.writer-attribute',
                    colorProfile: 'classes',
                },
            ],
            children: [
                {
                    kind: 'element',
                    name: 'span',
                    colorRole: 'syntax.string',
                    writerAttributeNodes: [
                        {
                            kind: 'writer-attribute',
                            name: 'class',
                            value: 'cem-color cem-color-syntax-string',
                            colorizerOwned: true,
                            colorizerRole: 'colorizer.writer-attribute',
                            colorProfile: 'classes',
                        },
                    ],
                    children: [{ kind: 'text', value: 'Ready' }],
                },
            ],
        },
    ],
};

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
        assertEqual(output.className, 'cem-color cem-color-syntax-name', 'writer receives materialized element class');
        assertEqual(
            requiredElement(output, 'span').className,
            'cem-color cem-color-syntax-string',
            'writer receives materialized text wrapper class'
        );

        const formattedPanel = requiredElement(canvasElement, '[data-stage="formatted"]');
        assert(
            formattedPanel.textContent?.includes('formatted tree before writer'),
            'formatted stage shows formatter metadata'
        );
        assert(
            !formattedPanel.textContent?.includes('colored tree before writer'),
            'formatted stage does not include color metadata'
        );

        const coloredPanel = requiredElement(canvasElement, '[data-stage="colored"]');
        assert(
            coloredPanel.textContent?.includes('colored tree before writer'),
            'colored stage shows colorizer metadata'
        );
    },
};

function stageGrid(): HTMLElement {
    const grid = document.createElement('div');
    grid.className = 'cemt-pipeline-grid';
    grid.append(
        stagePanel('Source AST', 'source', sourceAst),
        stagePanel('Formatted CEM Tree', 'formatted', formattedTree),
        stagePanel('Colored CEM Tree', 'colored', coloredTree)
    );
    return grid;
}

function stagePanel(title: string, stage: string, value: unknown): HTMLElement {
    const panel = document.createElement('article');
    panel.className = 'cemt-pipeline-panel';
    panel.dataset.stage = stage;

    const heading = document.createElement('h2');
    heading.textContent = title;

    const pre = document.createElement('pre');
    pre.textContent = JSON.stringify(value, null, 2);

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
            grid-template-columns: repeat(3, minmax(0, 1fr));
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
