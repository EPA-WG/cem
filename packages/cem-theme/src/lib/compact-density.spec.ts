import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import postcss, { type Rule } from 'postcss';

const root = resolve(import.meta.dirname, '../../dist/lib/css');
const preciseInput = '(hover: hover) and (pointer: fine) and (not (any-pointer: coarse))';
const css = (name: string) => postcss.parse(readFileSync(resolve(root, `${name}.css`), 'utf8'));

function profile(name: string, density: string): Rule {
  let match: Rule | undefined;
  css(name).walkRules((rule) => {
    if (rule.selector.includes(`[data-cem-density="${density}"]`)) {
      if (rule.parent?.type !== 'atrule') return; // Input-neutral glyph geometry is independent of target size.
      expect(rule.parent?.type).toBe('atrule');
      expect((rule.parent as postcss.AtRule).name).toBe('media');
      expect((rule.parent as postcss.AtRule).params).toBe(preciseInput);
      match = rule;
    }
  });
  expect(match, `${name} must emit the canonical ${density} profile`).toBeDefined();
  if (!match) throw new Error(`Missing ${density} profile in ${name}`);
  return match;
}

function value(rule: Rule, name: string) {
  return rule.nodes.find((node) => node.type === 'decl' && node.prop === name);
}

function property(rule: Rule, name: string) {
  const node = value(rule, name);
  expect(node, `${name} must be declared`).toBeDefined();
  return node?.type === 'decl' ? node.value : undefined;
}

describe('shared desktop density policy', () => {
  it('keeps the modality-neutral defaults and existing coupling profiles', () => {
    const tree = css('cem-coupling');
    const baseline = tree.nodes.find((node) => node.type === 'rule' && node.selector === ':root') as Rule;
    expect(property(baseline, '--cem-coupling-zone-min')).toBe('3rem');
    expect(property(baseline, '--cem-coupling-guard-min')).toBe('0.5rem');
    tree.walkRules((rule) => {
      if (rule.selector.includes('data-cem-coupling')) {
        expect(value(rule, '--cem-coupling-zone-min')).toBeUndefined();
        expect(value(rule, '--cem-coupling-guard-min')).toBeUndefined();
      }
    });
  });

  it('gates every desktop density override against coarse and hybrid inputs', () => {
    for (const name of ['cem-dimension', 'cem-coupling', 'cem-controls', 'cem-stroke']) {
      for (const density of ['compact', 'super-compact']) profile(name, density);
    }
    expect(property(profile('cem-coupling', 'compact'), '--cem-coupling-zone-min')).toBe('1.5rem');
    expect(property(profile('cem-coupling', 'super-compact'), '--cem-coupling-zone-min')).toBe('1lh');
  });

  it('keeps compact glyphs at one text em even when touch targets stay large', () => {
    let glyphs: Rule | undefined;
    css('cem-controls').walkRules((rule) => {
      if (rule.selector.includes('[data-cem-density="super-compact"]') && rule.parent?.type === 'root') glyphs = rule;
    });
    if (!glyphs) throw new Error('Missing input-neutral density glyph mapping');
    expect(property(glyphs, '--cem-icon-size')).toBe('1em');
    expect(property(glyphs, '--cem-icon-button-icon-size')).toBe('1em');
    expect(property(glyphs, '--cem-data-icon-size')).toBe('1em');
    expect(value(glyphs, '--cem-icon-button-size')).toBeUndefined();
    expect(value(glyphs, '--cem-control-height')).toBeUndefined();
    expect(value(glyphs, '--cem-coupling-zone-min')).toBeUndefined();
  });

  it('makes structured rows adjacent text lines without padding, gaps or borders', () => {
    const dimension = profile('cem-dimension', 'super-compact');
    for (const name of ['--cem-data-row-padding-block', '--cem-data-row-padding-inline', '--cem-data-row-gap']) {
      expect(property(dimension, name)).toBe('0px');
    }
    const controls = profile('cem-controls', 'super-compact');
    expect(property(controls, '--cem-data-row-height')).toBe('1lh');
    expect(property(controls, '--cem-table-row-height')).toBe('1lh');
    expect(property(controls, '--cem-data-icon-size')).toBe('1em');
    expect(property(controls, '--cem-icon-size')).toBe('1em');
    const stroke = profile('cem-stroke', 'super-compact');
    expect(property(stroke, '--cem-data-row-border-width')).toBe('0px');
    expect(property(stroke, '--cem-data-row-focus-offset')).toBe('calc(-1 * var(--cem-stroke-focus))');
  });

  it('offers inherited profiles without shrinking typography or selecting components', () => {
    for (const name of ['cem-dimension', 'cem-coupling', 'cem-controls', 'cem-stroke']) {
      for (const density of ['compact', 'super-compact']) {
        const rule = profile(name, density);
        expect(rule.selector).toContain(`:where([data-cem-density="${density}"])`);
        rule.walkDecls((decl) => {
          expect(decl.prop.startsWith('--cem-')).toBe(true);
          expect(decl.prop).not.toContain('font-size');
          expect(decl.prop).not.toContain('line-height');
        });
      }
    }
  });

  it('declares kind colors in each explicit theme and forced-color boundary', () => {
    const tree = css('cem-colors');
    for (const name of ['--cem-resource-project-color', '--cem-resource-file-color', '--cem-resource-output-color']) {
      let declarations = 0;
      let forced = false;
      tree.walkDecls(name, () => {
        declarations++;
      });
      tree.walkAtRules('media', (media) => {
        if (media.params === '(forced-colors: active)') {
          media.walkDecls(name, (decl) => {
            expect(decl.value).toBe('CanvasText');
            forced = true;
          });
        }
      });
      expect(declarations).toBeGreaterThanOrEqual(4);
      expect(forced).toBe(true);
    }
  });
});
