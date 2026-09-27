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
  const native = spawnSync('cargo', ['test', '-p', 'cem-ml', '--test', 'css_subtree', '--test', 'css_import_closure',
    '--target-dir', 'dist/target/cem_ml', 'browser_fixture_emits_'], {
    env: { ...process.env, CEM_CSS_SUBTREE_FIXTURE_DIR: directory }, stdio: 'inherit',
  });
  assert.equal(native.status, 0, 'native CSS fixture must pass');
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  const markup = '<cem-fixture class="active"><div class="card active"><span class="label">text</span></div><div class="pseudo"></div></cem-fixture>';
  const cases = [
    ['nth-filter', '.pseudo', null, 'color', 'rgb(128, 0, 128)'],
    ['nth-filter', '.card', null, 'backgroundColor', 'rgb(255, 192, 203)'],
    ['nth-structural', '.card', null, 'color', 'rgb(255, 165, 0)'],
    ['nth-structural', '.pseudo', null, 'color', 'rgb(128, 0, 128)'],
    ['nth-structural', '.card', null, 'backgroundColor', 'rgb(255, 192, 203)'],
    ['nth-structural', '.pseudo', null, 'backgroundColor', 'rgb(255, 165, 0)'],
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
  await page.setContent(markup);
  await page.addStyleTag({ content: await readFile(join(directory, 'direction.css'), 'utf8') });
  const directions = await page.evaluate(() => {
    const host = document.querySelector('cem-fixture');
    const card = host.querySelector('.card');
    const pseudo = host.querySelector('.pseudo');
    host.setAttribute('dir', 'rtl');
    pseudo.setAttribute('dir', 'ltr');
    const inherited = { color: getComputedStyle(card).color, direction: getComputedStyle(card).direction,
      attribute: card.getAttribute('dir'), override: getComputedStyle(pseudo).color };
    host.setAttribute('dir', 'ltr');
    const changed = getComputedStyle(card).color;
    host.setAttribute('dir', 'rtl');
    card.setAttribute('dir', 'ltr');
    const explicit = getComputedStyle(card).color;
    card.setAttribute('dir', 'auto');
    card.textContent = 'עברית';
    return { inherited, changed, explicit, automatic: getComputedStyle(card).color };
  });
  assert.deepEqual(directions, {
    inherited: { color: 'rgb(128, 0, 128)', direction: 'ltr', attribute: null, override: 'rgb(255, 165, 0)' },
    changed: 'rgb(0, 128, 0)', explicit: 'rgb(0, 128, 0)', automatic: 'rgb(128, 0, 128)',
  }, ':dir must follow document directionality, including inheritance, overrides and auto');
  await page.setContent(markup);
  await page.addStyleTag({ content: await readFile(join(directory, 'language.css'), 'utf8') });
  const languages = await page.evaluate(() => {
    const host = document.querySelector('cem-fixture');
    const card = host.querySelector('.card');
    const pseudo = host.querySelector('.pseudo');
    host.setAttribute('lang', 'en-US');
    pseudo.setAttribute('lang', 'fr-CA');
    const inherited = { color: getComputedStyle(card).color, attribute: card.getAttribute('lang'),
      override: getComputedStyle(pseudo).color };
    host.setAttribute('lang', 'de-DE');
    const changed = getComputedStyle(card).color;
    card.setAttribute('lang', 'FR-ca');
    const explicit = getComputedStyle(card).color;
    card.setAttribute('lang', '');
    return { inherited, changed, explicit, untagged: getComputedStyle(card).color };
  });
  assert.deepEqual(languages, {
    inherited: { color: 'rgb(128, 0, 128)', attribute: null, override: 'rgb(255, 165, 0)' },
    changed: 'rgb(0, 128, 0)', explicit: 'rgb(128, 0, 128)', untagged: 'rgb(0, 0, 0)',
  }, ':lang must follow document language, including inheritance, ranges and overrides');
  const languageSupport = {};
  for (const [name, selector] of [
    ['language-list', ':lang(en, fr)'], ['language-string', ':lang("en")'],
    ['language-wildcard', ':lang("*")'], ['language-empty', ':lang("")'],
  ]) {
    const results = [];
    for (const suffix of ['authored.css', 'css']) {
      await page.setContent(markup);
      await page.addStyleTag({ content: await readFile(join(directory, `${name}.${suffix}`), 'utf8') });
      results.push(await page.evaluate((selector) => {
        const host = document.querySelector('cem-fixture');
        const card = host.querySelector('.card');
        const colors = ['en-US', 'fr-CA', 'de-DE', ''].map((language) => {
          host.setAttribute('lang', language);
          return getComputedStyle(card).color;
        });
        return { supported: CSS.supports(`selector(${selector})`), colors };
      }, selector));
    }
    assert.deepEqual(results[1], results[0], `${name}: emitted CSS must preserve authored browser behavior`);
    languageSupport[selector] = results[0].supported;
  }
  console.log('Browser language syntax support:', languageSupport);
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
  for (const name of ['keyframes', 'keyframes-empty', 'keyframes-string', 'keyframes-shorthand', 'keyframes-conditional']) {
    await page.setContent(markup);
    await page.addStyleTag({ content: await readFile(join(directory, `${name}.css`), 'utf8') });
    const state = await page.evaluate(() => {
      const card = document.querySelector('.card');
      const animation = card.getAnimations().find((item) => item instanceof CSSAnimation);
      if (!animation) return null;
      animation.currentTime = 500;
      return { name: animation.animationName, opacity: getComputedStyle(card).opacity,
        frameCount: animation.effect.getKeyframes().length };
    });
    assert.deepEqual(state, {
      name: name === 'keyframes-string' ? 'quoted name-fixture' : name === 'keyframes-shorthand' ? 'linear-fixture' : 'pulse-fixture',
      opacity: name === 'keyframes-empty' ? '1' : name === 'keyframes-conditional' ? '0.4' : '0.5',
      frameCount: name === 'keyframes-empty' ? 0 : 2,
    }, `${name} must preserve scoped animation identity and lifecycle`);
  }
  await page.setContent(markup);
  await page.addStyleTag({ content: await readFile(join(directory, 'import-closure.css'), 'utf8') });
  const importedAnimation = await page.evaluate(() => {
    const card = document.querySelector('.card');
    const animation = card.getAnimations().find((item) => item instanceof CSSAnimation);
    if (!animation) return null;
    animation.currentTime = 500;
    return { name: animation.animationName, opacity: getComputedStyle(card).opacity,
      color: getComputedStyle(card).color };
  });
  assert.deepEqual(importedAnimation, { name: 'pulse-cem-66697874757265', opacity: '0.5', color: 'rgb(0, 128, 0)' },
    'nested imports preserve cross-sheet animation names and cascade order');
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
  console.log(`Native nested CSS: ${cases.length} computed-style checks, direction and language inheritance/updates, two container updates, five keyframe animations, an imported animation and the starting-style transition passed.`);
} finally {
  await browser?.close();
  await rm(directory, { recursive: true, force: true });
}
