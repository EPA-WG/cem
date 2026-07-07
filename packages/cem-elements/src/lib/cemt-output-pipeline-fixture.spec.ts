import { describe, expect, it } from 'vitest';

import cemtPipelineStageFixture from '../../../cem_ml/schema-packages/cem-transform/v1/examples/formatter-coloring-pipeline.fixture.cem?raw';
import cemtPipelineSource from '../../../cem_ml/schema-packages/cem-transform/v1/examples/formatter-coloring-pipeline.cemt?raw';
import {
    createCemtPipelineShowcase,
    writeColoredTreeToHtml,
    writerReady,
} from './cemt-output-pipeline-fixture.js';

describe('CEMT output pipeline fixture', () => {
    it('renders intermediate stages as CEM-native source instead of JSON', () => {
        const showcase = createCemtPipelineShowcase(cemtPipelineSource, cemtPipelineStageFixture);

        expect(showcase.sourceAstCem).toBe('{article |\n    {text | Ready}\n}');
        expect(showcase.formattedTreeCem).toContain('{cem-tree @content-type="application/cem"');
        expect(showcase.formattedTreeCem).toContain('@schema="https://cem.dev/ns/cem-ml/1"');
        expect(showcase.formattedTreeCem).toContain('@formatter-profile="acme.showcase.format-tree"');
        expect(showcase.formattedTreeCem).toContain('@formatter-role="formatter.showcase"');
        expect(showcase.formattedTreeCem).toContain('@value="formatted tree before writer"');
        expect(showcase.formattedTreeCem).not.toContain('"formatterProfile"');
        expect(showcase.formattedTreeCem).not.toContain('"kind":');
    });

    it('keeps coloring as a CEM tree mutation before writer output', () => {
        const showcase = createCemtPipelineShowcase(cemtPipelineSource, cemtPipelineStageFixture);

        expect(writerReady(showcase.formattedTree)).toBe(false);
        expect(writerReady(showcase.coloredTree)).toBe(true);
        expect(showcase.coloredTreeCem).toContain('@colored=true');
        expect(showcase.coloredTreeCem).toContain('@color-profile="classes"');
        expect(showcase.coloredTreeCem).toContain('@colorizer-role="colorizer.showcase"');
        expect(showcase.coloredTreeCem).toContain('@value="colored tree before writer"');
        expect(showcase.coloredTreeCem).toContain('@color-role="syntax.string"');
    });

    it('lets the writer emit target-native HTML only after coloring', () => {
        const showcase = createCemtPipelineShowcase(cemtPipelineSource, cemtPipelineStageFixture);

        expect(() => writeColoredTreeToHtml(showcase.formattedTree)).toThrow(
            'colored CEM tree is required before writer output'
        );
        expect(writeColoredTreeToHtml(showcase.coloredTree)).toBe(
            '<article class="cem-color cem-color-syntax-name"><span class="cem-color cem-color-syntax-string">Ready</span></article>'
        );
    });

    it('rejects a CEM-native stage fixture that drifts from the checked CEMT source', () => {
        const mismatchedFixture = cemtPipelineStageFixture.replace(
            '@formatter="acme.showcase.format-tree"',
            '@formatter="acme.other.format-tree"'
        );

        expect(() => createCemtPipelineShowcase(cemtPipelineSource, mismatchedFixture)).toThrow(
            'CEMT stage fixture formatter: expected acme.showcase.format-tree, got acme.other.format-tree'
        );
    });
});
