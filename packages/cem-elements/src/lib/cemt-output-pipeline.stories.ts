import type { Meta, StoryObj } from '@storybook/web-components-vite';
import cemtPipelineSource from '../../../cem_ml/schema-packages/cem-transform/v1/examples/formatter-coloring-pipeline.cemt?raw';
import {
    createCemtPipelineShowcase,
    writeColoredTreeToHtml,
    writerReady,
} from './cemt-output-pipeline-fixture';

const meta: Meta = {
    title: 'CEM Elements/CEMT Output Pipeline',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

const pipelineShowcase = createCemtPipelineShowcase(cemtPipelineSource);
const pipelineFixture = pipelineShowcase.fixture;
const formattedTree = pipelineShowcase.formattedTree;
const coloredTree = pipelineShowcase.coloredTree;

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
        stagePanel('Source AST', 'source', pipelineShowcase.sourceAstCem),
        stagePanel('Formatted CEM Tree', 'formatted', pipelineShowcase.formattedTreeCem),
        stagePanel('Colored CEM Tree', 'colored', pipelineShowcase.coloredTreeCem)
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
