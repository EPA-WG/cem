import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { parse } from '../parser/parse.js';
import { transform } from './transform.js';

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

describe('transform — snapshot each fixture', () => {
  for (const name of FIXTURE_NAMES) {
    it(`${name} matches snapshot`, () => {
      const doc = fixture(name);
      const output = transform(doc);
      expect(output).toMatchSnapshot();
    });
  }
});

describe('transform — login.html shape', () => {
  it('replaces main[data-cem-screen] with cem-screen[cem-id]', () => {
    const doc = fixture('login.html');
    const output = transform(doc);
    expect(output).toContain('<cem-screen cem-id="login"');
    expect(output).not.toContain('data-cem-screen');
  });

  it('replaces form[data-cem-form] with cem-form[cem-id]', () => {
    const doc = fixture('login.html');
    const output = transform(doc);
    expect(output).toContain('<cem-form cem-id="sign-in"');
    expect(output).not.toContain('data-cem-form');
  });

  it('replaces button[data-cem-action] with cem-action[variant]', () => {
    const doc = fixture('login.html');
    const output = transform(doc);
    expect(output).toContain('<cem-action variant="primary"');
    expect(output).not.toContain('data-cem-action');
  });

  it('preserves non-CEM attributes on transformed elements', () => {
    const doc = fixture('login.html');
    const output = transform(doc);
    expect(output).toContain('method="post"');
    expect(output).toContain('action="/session"');
    expect(output).toContain('aria-labelledby="login-title"');
  });

  it('preserves non-CEM elements unchanged', () => {
    const doc = fixture('login.html');
    const output = transform(doc);
    expect(output).toContain('<label for="email">');
    expect(output).toContain('<input');
    expect(output).toContain('<h1 id="login-title">');
  });
});

describe('transform — profile.html shape', () => {
  it('replaces section[data-cem-card] with cem-card[cem-id]', () => {
    const doc = fixture('profile.html');
    const output = transform(doc);
    expect(output).toContain('<cem-card cem-id="identity"');
    expect(output).toContain('<cem-card cem-id="preferences"');
  });

  it('replaces span[data-cem-badge] with cem-badge[variant]', () => {
    const doc = fixture('profile.html');
    const output = transform(doc);
    expect(output).toContain('<cem-badge variant="success"');
  });
});

describe('transform — message-thread.html shape', () => {
  it('replaces ol[data-cem-thread] with cem-thread[cem-id]', () => {
    const doc = fixture('message-thread.html');
    const output = transform(doc);
    expect(output).toContain('<cem-thread cem-id="support"');
  });

  it('replaces li[data-cem-message] with cem-message[variant]', () => {
    const doc = fixture('message-thread.html');
    const output = transform(doc);
    expect(output).toContain('<cem-message variant="received"');
    expect(output).toContain('<cem-message variant="sent"');
  });
});
