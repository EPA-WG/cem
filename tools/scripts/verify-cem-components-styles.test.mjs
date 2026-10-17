import assert from 'node:assert/strict';
import { after, before, test } from 'node:test';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readCanonicalStyles, inspectCssRules, inspectActionStyles } from '../cem-components/style-audit.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
let fixtureRoot;
before(() => { fixtureRoot = mkdtempSync(join(tmpdir(), 'cem-style-audit-')); });
after(() => { rmSync(fixtureRoot, { recursive: true, force: true }); });
function declaration(source) {
    const path = join(fixtureRoot, 'cem-fixture.xhtml');
    writeFileSync(path, source);
    return path;
}
const wrap = text => `<cem-element xmlns="http://www.w3.org/1999/xhtml" tag="cem-fixture"><template id="cem-fixture" type="text/cem-ml">${text}</template></cem-element>`;

test('native XML and CEM boundaries decode entities once and collect every static stylesheet', () => {
    const path = declaration(wrap('{style @type=text/css |``` :scope { &amp;:hover { /* &amp;amp; &#38; &#x26; &quot; &apos; &#x1F600; */ color: inherit; } } ```}' +
        '<![CDATA[{style @type=text/css |``` :scope::after { content: "&amp;"; } ```}]]>'));
    const css = readCanonicalStyles(path, 'cem-fixture');
    assert.match(css, /&:hover/);
    assert.ok(css.includes('/* &amp; & & " \' 😀 */'));
    assert.ok(css.includes('content: "&amp;";'), 'CDATA text must not be decoded again');
    assert.equal((css.match(/:scope/g) ?? []).length, 2);
});

test('native readers reject malformed XML, missing/duplicate owners, and invalid CEM templates', () => {
    for (const source of [
        wrap('{style |``` :scope {} ```}').replace('</template>', ''),
        wrap('{style |``` :scope {} ```}').replace('id="cem-fixture"', 'id="other"'),
        wrap('{style |``` :scope {} ```}').replace('type="text/cem-ml"', 'type="text/html"'),
        wrap('{style |``` :scope {} ```}').replace('</cem-element>', '<template id="cem-fixture" type="text/cem-ml">x</template></cem-element>'),
        wrap('{style |``` :scope {} ```} {div @'),
        wrap('{style |``` :scope { content: "&unknown;"; } ```}'),
        wrap('{div | No static styles}'),
    ]) assert.throws(() => readCanonicalStyles(declaration(source), 'cem-fixture'));
});

test('nested states retain functional selector lists and media ancestry', () => {
    const { rules, failures } = inspectCssRules('fixture.css', `
      :scope > :is(button, a) {
        color: var(--cem-action-primary-default-text);
        &:where(:enabled:hover, [href]:hover) { color: var(--cem-action-primary-hover-text); }
        @media (forced-colors: active) {
          color: CanvasText;
          &:focus-visible { outline-color: Highlight; }
        }
      }`);
    assert.deepEqual(failures, []);
    assert.deepEqual(rules.map(({ selector, media }) => [selector, media]), [
        [':scope > :is(button, a)', null],
        [':scope > :is(button, a):where(:enabled:hover, [href]:hover)', null],
        [':scope > :is(button, a)', '(forced-colors: active)'],
        [':scope > :is(button, a):focus-visible', '(forced-colors: active)'],
    ]);
    assert.equal(rules[2].declarations.get('color'), 'CanvasText');
});

test('nesting handles parent lists, implicit descendants, repeated parents and literal ampersands', () => {
    const { rules, failures } = inspectCssRules('fixture.css', `
      .a, #b {
        > .child { color: currentColor; }
        & + & { color: inherit; }
        &[data-label="&"], &.escaped\\& { color: inherit; }
        :is(&, .other) { color: inherit; }
      }`);
    assert.deepEqual(failures, []);
    assert.deepEqual(rules.map(rule => rule.selector), [
        ':is(.a, #b) > .child', ':is(.a, #b) + :is(.a, #b)',
        ':is(.a, #b)[data-label="&"]', ':is(.a, #b).escaped\\&', ':is(:is(.a, #b), .other)',
    ]);
});

test('nested media never satisfies a normal-state binding; unsupported syntax and duplicates fail', () => {
    const inspected = inspectCssRules('fixture.css', '@media (forced-colors: active) { .a { &:hover { color: Highlight; } } }');
    assert.deepEqual(inspected.failures, []);
    assert.equal(inspected.rules[0].media, '(forced-colors: active)');
    const combined = inspectCssRules('fixture.css', '@media (forced-colors: active) { .a { @media (prefers-reduced-motion: reduce) { &:hover { color: Highlight; } } } }');
    assert.deepEqual(combined.failures, []);
    assert.equal(combined.rules[0].media, '(forced-colors: active) and (prefers-reduced-motion: reduce)');
    for (const css of ['.a { color: inherit; color: initial; }', '.a { @supports (display: grid) { color: inherit; } }', '.a {', '@media print { .a { color: inherit; } }']) {
        assert.ok(inspectCssRules('fixture.css', css).failures.length > 0, css);
    }
});

for (const tag of ['cem-action', 'cem-icon-button']) {
    let css;
    before(() => { css = readCanonicalStyles(join(root, `packages/cem-components/src/components/${tag}/${tag}.xhtml`), tag); });
    test(`${tag}: canonical flat/nested intent and state contracts pass`, () => {
        assert.deepEqual(inspectActionStyles(tag, tag, css), []);
    });
    test(`${tag}: semantic state mutations still fail`, () => {
        const hoverPaint = /background-color: var\(--cem-action-explicit-hover-background\);\s*color: var\(--cem-action-explicit-hover-text\);/;
        for (const [description, mutant, message] of [
            ['wrong nested endpoint', css.replace('var(--cem-action-explicit-hover-text)', 'var(--cem-action-primary-hover-text)'), /explicit hover.*text/],
            ['missing state', css.replace('background-color: var(--cem-action-explicit-hover-background);', ''), /explicit hover/],
            ['state moved into forced colors', css.replace(hoverPaint, '@media (forced-colors: active) { $& }'), /explicit hover/],
            ['added state geometry', css.replace('color: var(--cem-action-explicit-hover-text);', 'color: var(--cem-action-explicit-hover-text); padding: var(--cem-inset-container);'), /explicit hover changes only/],
            ['duplicate declaration', css.replace('color: var(--cem-action-explicit-hover-text);', 'color: var(--cem-action-explicit-hover-text); color: var(--cem-action-explicit-hover-text);'), /duplicate color/],
            ['literal color', css.replace('var(--cem-action-explicit-hover-text)', '#123456'), /color literals/],
            ['literal spacing', css + '\n:scope { padding: 9px; }', /spacing literal/],
            ['broken keyframes', css.replace('background-position-x: 0px', 'background-position-x: 1px'), /pending keyframes/],
            ['invalid CSS', css + '\n:scope {', /invalid CSS/],
        ]) assert.match(inspectActionStyles(tag, tag, mutant).join('\n'), message, description);
    });
}
