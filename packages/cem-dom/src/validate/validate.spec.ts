import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { parse } from '../parser/parse.js';
import { validate } from './validate.js';

const FIXTURES_DIR = join(import.meta.dirname, '../../../../examples/semantic');

function fixture(name: string) {
  return parse(readFileSync(join(FIXTURES_DIR, name), 'utf8'), name);
}

const FIXTURE_NAMES = [
  'login.html',
  'registration.html',
  'profile.html',
  'assets-list.html',
  'message-thread.html',
];

describe('validate — all fixtures clean', () => {
  for (const name of FIXTURE_NAMES) {
    it(`${name} has zero hard violations`, () => {
      const doc = fixture(name);
      const messages = validate(doc);
      const errors = messages.filter(m => m.severity === 'error');
      expect(errors).toHaveLength(0);
    });
  }
});

describe('validate — login.html', () => {
  it('emits no messages at all', () => {
    const doc = fixture('login.html');
    expect(validate(doc)).toHaveLength(0);
  });
});

describe('validate — invalid variants', () => {
  it('flags unknown action variant', () => {
    const html = `<!doctype html><html lang="en"><body>
      <main data-cem-screen="test" aria-labelledby="t"><h1 id="t">T</h1>
        <button data-cem-action="bogus">Go</button>
      </main></body></html>`;
    const doc = parse(html, 'test.html');
    const msgs = validate(doc);
    expect(msgs.some(m => m.rule === 'invalid-action-variant')).toBe(true);
  });

  it('flags unknown badge variant', () => {
    const html = `<!doctype html><html lang="en"><body>
      <main data-cem-screen="test" aria-labelledby="t"><h1 id="t">T</h1>
        <span data-cem-badge="purple">Status</span>
      </main></body></html>`;
    const doc = parse(html, 'test.html');
    const msgs = validate(doc);
    expect(msgs.some(m => m.rule === 'invalid-badge-variant')).toBe(true);
  });
});

describe('validate — broken references', () => {
  it('flags aria-labelledby pointing to missing id', () => {
    const html = `<!doctype html><html lang="en"><body>
      <main data-cem-screen="test" aria-labelledby="nonexistent"></main>
    </body></html>`;
    const doc = parse(html, 'test.html');
    const msgs = validate(doc);
    expect(msgs.some(m => m.rule === 'broken-aria-ref')).toBe(true);
  });

  it('flags label[for] pointing to missing id', () => {
    const html = `<!doctype html><html lang="en"><body>
      <label for="ghost">Name</label>
      <input id="name" type="text">
    </body></html>`;
    const doc = parse(html, 'test.html');
    const msgs = validate(doc);
    expect(msgs.some(m => m.rule === 'broken-for-ref')).toBe(true);
  });
});
