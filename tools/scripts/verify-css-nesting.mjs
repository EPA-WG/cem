import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { chromium } from 'playwright';

// Native emission is the input under test. Browser runtime installation remains
// pending; this fixture only proves CSS semantics of the emitted rule subset.
const directory = await mkdtemp(join(tmpdir(), 'cem-css-nesting-'));
let browser;
try {
  const native = spawnSync('cargo', ['test', '-p', 'cem-ml', '--test', 'css_subtree',
    '--target-dir', 'dist/target/cem_ml', 'browser_fixture_emits_scoped_native_nesting'], {
    env: { ...process.env, CEM_CSS_SUBTREE_FIXTURE_DIR: directory }, stdio: 'inherit',
  });
  assert.equal(native.status, 0, 'native CSS fixture must pass');
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  const markup = '<cem-fixture class="active"><div class="card active"><span class="label">text</span></div><div class="pseudo"></div></cem-fixture>';
  const cases = [
    ['global-alias', 'cem-fixture', null, 'color', 'rgb(128, 0, 128)'],
    ['global-alias', 'cem-fixture', null, 'backgroundColor', 'rgb(255, 165, 0)'],
    ['global-alias', '.card', null, 'backgroundColor', 'rgba(0, 0, 0, 0)'],
    ['global-instance', 'cem-fixture', null, 'backgroundColor', 'rgb(255, 165, 0)'],
    ['global-instance', '.card', null, 'backgroundColor', 'rgba(0, 0, 0, 0)'],
    ['container', '.card', null, 'color', 'rgb(255, 165, 0)'],
    ['container', '.card', null, 'backgroundColor', 'rgb(255, 192, 203)'],
    ['container-style', '.card', null, 'color', 'rgb(255, 165, 0)'],
    ['parent-list', '.label', null, 'color', 'rgb(255, 165, 0)'],
    ['declarations', '.pseudo', '::before', 'color', 'rgb(0, 128, 0)'],
    ['group-order', '.card', null, 'color', 'rgb(128, 0, 128)'],
    ['group-order', '.card', null, 'backgroundColor', 'rgb(255, 165, 0)'],
    ['rejected-parent', '.card', null, 'color', 'rgb(0, 128, 0)'],
    ['instance', '.label', null, 'color', 'rgb(255, 165, 0)'],
    ['host', 'cem-fixture', null, 'color', 'rgb(128, 0, 128)'],
    ['host', 'cem-fixture', null, 'backgroundColor', 'rgb(255, 165, 0)'],
    ['duplicates', '.card', null, 'color', 'rgb(0, 128, 0)'],
    ['zero-weight', '.label', null, 'color', 'rgb(255, 165, 0)'],
  ];
  for (const [name, selector, pseudo, property, expected] of cases) {
    await page.setContent(markup);
    const css = await readFile(join(directory, `${name}.css`), 'utf8');
    await page.evaluate(({ css, instance }) => {
      const style = document.createElement('style');
      style.textContent = css;
      (instance ? document.querySelector('cem-fixture') : document.head).append(style);
    }, { css, instance: name === 'instance' || name === 'global-instance' });
    const actual = await page.evaluate(({ selector, pseudo, property }) =>
      getComputedStyle(document.querySelector(selector), pseudo)[property], { selector, pseudo, property });
    assert.equal(actual, expected, `${name}: ${selector}${pseudo ?? ''} ${property}`);
  }
  for (const name of ['container', 'container-style']) {
    await page.setContent(markup);
    await page.addStyleTag({ content: await readFile(join(directory, `${name}.css`), 'utf8') });
    const after = await page.evaluate((name) => {
      const host = document.querySelector('cem-fixture');
      const card = host.querySelector('.card');
      const before = getComputedStyle(card).color;
      if (name === 'container') host.style.width = '200px';
      else host.style.setProperty('--theme', 'light');
      return { before, color: getComputedStyle(card).color };
    }, name);
    assert.deepEqual(after, { before: 'rgb(255, 165, 0)', color: 'rgb(0, 0, 0)' },
      `${name} must re-evaluate changed container state`);
  }
  const startingCss = await readFile(join(directory, 'starting-style.css'), 'utf8');
  await page.setContent('<cem-fixture></cem-fixture>');
  const transition = await page.evaluate((css) => {
    const style = document.createElement('style');
    style.textContent = css;
    document.head.append(style);
    const host = document.querySelector('cem-fixture');
    host.getBoundingClientRect();
    const card = document.createElement('div');
    card.className = 'card';
    card.textContent = 'appearing';
    host.append(card);
    const initial = getComputedStyle(card).opacity;
    const animation = card.getAnimations().find((item) => item instanceof CSSTransition);
    if (!animation) return null;
    animation.pause();
    animation.currentTime = 500;
    const midpoint = getComputedStyle(card).opacity;
    const keyframes = animation.effect.getKeyframes().map((frame) => frame.opacity);
    animation.finish();
    return { initial, midpoint, keyframes, final: getComputedStyle(card).opacity };
  }, startingCss);
  assert.deepEqual(transition, { initial: '0', midpoint: '0.5', keyframes: ['0', '1'], final: '1' },
    'native @starting-style must supply the initial transition value');
  console.log(`Native nested CSS: ${cases.length} computed-style checks, two container updates and the starting-style transition passed.`);
} finally {
  await browser?.close();
  await rm(directory, { recursive: true, force: true });
}
