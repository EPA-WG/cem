import { describe, expect, it } from 'vitest';

import { convertLegacyTemplateToCemMl } from './convert.js';
import type { TemplateSourceNode } from '../projection.js';

const XSL = 'http://www.w3.org/1999/XSL/Transform';

function el(
    tag: string,
    attributes: Array<[string, string]>,
    children: TemplateSourceNode[],
    namespace: string | null = null,
): TemplateSourceNode {
    return {
        kind: 'element',
        namespace,
        tag,
        attributes: attributes.map(([name, value]) => ({ name, value })),
        children,
    };
}

function txt(text: string): TemplateSourceNode {
    return { kind: 'text', text };
}

function convert(nodes: TemplateSourceNode[]) {
    return convertLegacyTemplateToCemMl(nodes);
}

describe('convertLegacyTemplateToCemMl', () => {
    it('maps an element with an AVT attribute and text interpolation', () => {
        const { source, diagnostics } = convert([
            el('a', [['href', '{$href}']], [txt('Go')]),
        ]);
        expect(source).toBe('{a @href="{$href}" | Go}');
        expect(diagnostics).toEqual([]);
    });

    it('maps bare <choose>/<when> with contains() to cem:choose', () => {
        const { source } = convert([
            el('choose', [], [
                el('when', [['test', "contains($icon,'/')"]], [
                    el('img', [['src', '{$icon}']], []),
                ]),
                el('when', [['test', '$icon']], [
                    el('span', [], [txt('{$icon}')]),
                ]),
            ]),
        ]);
        expect(source).toBe(
            '{cem:choose | ' +
                '{cem:when @test=\'str:contains(icon, "/")\' | {img @src="{$icon}"}}' +
                '{cem:when @test="icon" | {span | {$icon}}}}',
        );
    });

    it('maps bare <if> with not() to a cem:if using the not operator', () => {
        const { source } = convert([
            el('if', [['test', 'not($disabled)']], [el('button', [], [txt('Go')])]),
        ]);
        expect(source).toBe('{cem:if @test="not (disabled)" | {button | Go}}');
    });

    it('maps xsl:value-of (namespaced) to interpolation', () => {
        const { source } = convert([
            el('xsl:value-of', [['select', '$name']], [], XSL),
        ]);
        expect(source).toBe('{$name}');
    });

    it('maps for-each with context item, attribute step and position()', () => {
        const { source } = convert([
            el('for-each', [['select', '$rows']], [
                el('div', [['style', 'color:{@hex}']], [txt('{position()}. {.}')]),
            ]),
        ]);
        expect(source).toBe(
            '{cem:for-each @select="rows" @as="item" | ' +
                '{div @style="color:{$item.hex}" | {$position}. {$item}}}',
        );
    });

    it('maps // slice paths and comparison in a test', () => {
        const { source } = convert([
            el('if', [['test', "//show-items = 'yes'"]], [txt('shown')]),
        ]);
        expect(source).toBe('{cem:if @test=\'datadom.slices.show-items = "yes"\' | shown}');
    });

    it('wraps <style> content in rich content (literal CSS braces)', () => {
        const { source } = convert([
            el('style', [], [txt('a { color: red; }')]),
        ]);
        expect(source).toBe('{style | ```a { color: red; }```}');
    });

    it('renders a named slot with fallback', () => {
        const { source } = convert([
            el('slot', [['name', 'legend']], [txt('default')]),
        ]);
        expect(source).toBe('{slot @name="legend" | default}');
    });

    it('emits a diagnostic and drops unsupported Tier-3 constructs', () => {
        const { source, diagnostics } = convert([
            el('xsl:apply-templates', [['select', 'node()']], [], XSL),
        ]);
        expect(source).toBe('');
        expect(diagnostics).toHaveLength(1);
        expect(diagnostics[0].code).toBe('legacy_xslt.unsupported_construct');
    });

    it('unrolls a for-each over an inline node-set variable', () => {
        const { source, diagnostics } = convert([
            el('variable', [['name', 'fruits']], [
                el('item', [], [txt('Apple')]),
                el('item', [], [txt('Banana')]),
            ]),
            el('ul', [], [
                el('for-each', [['select', 'exsl:node-set($fruits)/*']], [
                    el('li', [], [txt('{.}')]),
                ]),
            ]),
        ]);
        expect(source).toBe('{ul | {li | Apple}{li | Banana}}');
        expect(diagnostics).toEqual([]);
    });

    it('unrolls with @attr and position() substituted as literals', () => {
        const { source } = convert([
            el('variable', [['name', 'colors']], [
                el('color', [['hex', '#f00']], [txt('Red')]),
                el('color', [['hex', '#0f0']], [txt('Green')]),
            ]),
            el('for-each', [['select', 'exsl:node-set($colors)/*']], [
                el('div', [['style', 'background:{@hex}']], [txt('{position()}. {.}')]),
            ]),
        ]);
        expect(source).toBe(
            '{div @style="background:#f00" | 1. Red}{div @style="background:#0f0" | 2. Green}',
        );
    });

    it('wraps unrolled items in cem:if for a scalar-variable predicate', () => {
        const { source } = convert([
            el('variable', [['name', 'show'], ['select', "//show-items = 'yes'"]], []),
            el('variable', [['name', 'items']], [
                el('item', [], [txt('First')]),
                el('item', [], [txt('Second')]),
            ]),
            el('for-each', [['select', 'exsl:node-set($items)/*[$show]']], [
                el('span', [], [txt('{.}')]),
            ]),
        ]);
        expect(source).toBe(
            '{cem:if @test=\'datadom.slices.show-items = "yes"\' | {span | First}}' +
                '{cem:if @test=\'datadom.slices.show-items = "yes"\' | {span | Second}}',
        );
    });

    it('maps concat() to a sequence join', () => {
        const { source } = convert([
            el('span', [['title', '{concat($a, "-", $b)}']], []),
        ]);
        expect(source).toBe('{span @title=\'{str:concat((a, "-", b))}\'}');
    });
});
