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
    }, { css, instance: name === 'instance' });
    const actual = await page.evaluate(({ selector, pseudo, property }) =>
      getComputedStyle(document.querySelector(selector), pseudo)[property], { selector, pseudo, property });
    assert.equal(actual, expected, `${name}: ${selector}${pseudo ?? ''} ${property}`);
  }
  console.log(`Native nested CSS: ${cases.length} computed-style checks passed.`);
} finally {
  await browser?.close();
  await rm(directory, { recursive: true, force: true });
}
