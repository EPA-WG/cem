import type { Meta, StoryObj } from '@storybook/web-components-vite';
import packagePipelineColorizerSource from '../../../cem_ml/schema-packages/cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt?raw';
import packagePipelineStageFixture from '../../../cem_ml/schema-packages/cem-ml/v1/examples/formatter-coloring-pipeline.package-artifacts.fixture.cem?raw';
import packagePipelineFormatterSource from '../../../cem_ml/schema-packages/cem-ml/v1/formatters/formatter-coloring-pipeline.cemt?raw';
import cemtPipelineStageFixture from '../../../cem_ml/schema-packages/cem-transform/v1/examples/formatter-coloring-pipeline.fixture.cem?raw';
import cemtPipelineSource from '../../../cem_ml/schema-packages/cem-transform/v1/examples/formatter-coloring-pipeline.cemt?raw';
import {
    type CemtPipelineShowcase,
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

const pipelineShowcase = createCemtPipelineShowcase(cemtPipelineSource, cemtPipelineStageFixture);
const packageFormatterPath = 'schema-packages/cem-ml/v1/formatters/formatter-coloring-pipeline.cemt';
const packageColorizerPath = 'schema-packages/cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt';
const schemaPackagePipelineShowcase = createCemtPipelineShowcase(
    [
        {
            path: packageFormatterPath,
            source: packagePipelineFormatterSource,
        },
        {
            path: packageColorizerPath,
            source: packagePipelineColorizerSource,
        },
    ],
    packagePipelineStageFixture
);

export const FormatterColoringWriterStages: Story = {
    render: () => renderShowcase(pipelineShowcase),
    play: ({ canvasElement }) => {
        assertPipelineShowcase(canvasElement, pipelineShowcase);
    },
};

export const SchemaPackageFormatterColorizerAssets: Story = {
    render: () => renderShowcase(schemaPackagePipelineShowcase),
    play: ({ canvasElement }) => {
        assertPipelineShowcase(canvasElement, schemaPackagePipelineShowcase, [
            packageFormatterPath,
            packageColorizerPath,
        ]);
    },
};

function renderShowcase(showcase: CemtPipelineShowcase): HTMLElement {
    const root = document.createElement('section');
    root.className = 'cemt-pipeline-showcase';
    root.append(styles(), stageGrid(showcase), renderedOutput(showcase));
    return root;
}

function assertPipelineShowcase(
    canvasElement: HTMLElement,
    showcase: CemtPipelineShowcase,
    expectedSourcePaths: string[] = []
): void {
    const pipelineFixture = showcase.fixture;
    const formattedTree = showcase.formattedTree;
    const coloredTree = showcase.coloredTree;

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
    assertEqual(
        requiredElement(output, 'strong').className,
        pipelineFixture.keywordClass,
        'writer receives materialized keyword element class'
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
    assert(
        !formattedPanel.textContent?.includes('writer consumes colored CEM tree'),
        'formatted stage does not include writer-boundary metadata'
    );
    assert(
        !formattedPanel.textContent?.includes('queued edit replay before writer'),
        'formatted stage does not include queued color edit metadata'
    );
    assert(!formattedPanel.textContent?.includes('"formatterProfile"'), 'formatted stage is not JSONified');

    const coloredPanel = requiredElement(canvasElement, '[data-stage="colored"]');
    assert(
        coloredPanel.textContent?.includes('colored tree before writer'),
        'colored stage shows colorizer metadata'
    );
    assert(
        coloredPanel.textContent?.includes('writer consumes colored CEM tree'),
        'colored stage shows writer-boundary metadata before writer output'
    );
    assert(
        coloredPanel.textContent?.includes('@stage="after-color"'),
        'colored stage records the post-color writer boundary'
    );
    assert(
        coloredPanel.textContent?.includes('queued edit replay before writer'),
        'colored stage shows queued color edit metadata'
    );
    assert(
        coloredPanel.textContent?.includes('colorizer.queued-edit'),
        'colored stage records the queued color edit role'
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
    for (const expectedSourcePath of expectedSourcePaths) {
        assert(
            templatePanel.textContent?.includes(expectedSourcePath),
            `Storybook showcase displays schema-package CEMT source path ${expectedSourcePath}`
        );
    }
    if (expectedSourcePaths.length > 0) {
        assert(
            templatePanel.textContent?.includes('cemml.cem-tree.format-tree-base'),
            'schema-package formatter exposes reusable CEM tree formatter helper'
        );
        assert(
            templatePanel.textContent?.includes('cemml.cem-tree.color-tree-base'),
            'schema-package colorizer exposes reusable CEM tree colorizer helper'
        );
    }
    assert(
        templatePanel.textContent?.includes('appendFormatNode('),
        'formatter CEMT source shows metadata accumulation'
    );
    assert(
        templatePanel.textContent?.includes('appendWriterBoundary('),
        'coloring CEMT source records writer-boundary metadata before writer output'
    );
    assert(
        templatePanel.textContent?.includes('applyEdits('),
        'coloring CEMT source replays queued edits before writer output'
    );
    assert(
        templatePanel.textContent?.includes('drainQueue('),
        'coloring CEMT source drains deferred edits before writer output'
    );
    assert(
        templatePanel.textContent?.includes('defer([],'),
        'coloring CEMT source queues color mutations before replay'
    );
    assert(
        templatePanel.textContent?.includes('appendEdit('),
        'coloring CEMT source uses a typed queued append edit'
    );
    assert(
        templatePanel.textContent?.includes('map($subject.nodes'),
        'coloring CEMT source maps over the formatted CEM tree before writer output'
    );
    assert(
        !templatePanel.textContent?.includes('"kind":'),
        'pipeline showcase keeps CEM-native source text instead of JSON'
    );
}

function stageGrid(showcase: CemtPipelineShowcase): HTMLElement {
    const grid = document.createElement('div');
    grid.className = 'cemt-pipeline-grid';
    grid.append(
        stagePanel('Checked CEMT Source', 'cemt-source', showcase.fixture.cemtSource),
        stagePanel('Source AST', 'source', showcase.sourceAstCem),
        stagePanel('Formatted CEM Tree', 'formatted', showcase.formattedTreeCem),
        stagePanel('Colored CEM Tree', 'colored', showcase.coloredTreeCem)
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

function renderedOutput(showcase: CemtPipelineShowcase): HTMLElement {
    const panel = document.createElement('article');
    panel.className = 'cemt-pipeline-panel cemt-pipeline-writer';
    panel.dataset.stage = 'writer';

    const heading = document.createElement('h2');
    heading.textContent = 'Writer Output';

    const output = document.createElement('div');
    output.className = 'cemt-writer-surface';
    output.innerHTML = writeColoredTreeToHtml(showcase.coloredTree);

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

        .cem-color-syntax-keyword {
            color: #7c3aed;
            font-weight: 700;
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
