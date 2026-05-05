import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { parse } from './parse.js';

const FIXTURES_DIR = join(import.meta.dirname, '../../../../examples/semantic');

function fixture(name: string): string {
  return readFileSync(join(FIXTURES_DIR, name), 'utf8');
}

describe('parse — login.html', () => {
  let doc: ReturnType<typeof parse>;

  beforeEach(() => {
    doc = parse(fixture('login.html'), 'login.html');
  });

  it('extracts title', () => {
    expect(doc.title).toBe('CEM Semantic Fixture - Login');
  });

  it('extracts lang', () => {
    expect(doc.lang).toBe('en');
  });

  it('finds data-cem-screen', () => {
    const screens = doc.cemNodes.filter(n => n.cemRole === 'screen');
    expect(screens).toHaveLength(1);
    expect(screens[0].cemValue).toBe('login');
  });

  it('finds data-cem-form', () => {
    const forms = doc.cemNodes.filter(n => n.cemRole === 'form');
    expect(forms).toHaveLength(1);
    expect(forms[0].cemValue).toBe('sign-in');
  });

  it('finds data-cem-action', () => {
    const actions = doc.cemNodes.filter(n => n.cemRole === 'action');
    expect(actions).toHaveLength(1);
    expect(actions[0].cemValue).toBe('primary');
  });

  it('builds id map', () => {
    expect(doc.ids.has('email')).toBe(true);
    expect(doc.ids.has('password')).toBe(true);
    expect(doc.ids.has('login-title')).toBe(true);
  });

  it('builds label map', () => {
    expect(doc.labels.get('email')).toBe('Email');
    expect(doc.labels.get('password')).toBe('Password');
  });

  it('has no parse errors', () => {
    expect(doc.errors).toHaveLength(0);
  });
});

describe('parse — registration.html', () => {
  let doc: ReturnType<typeof parse>;

  beforeEach(() => {
    doc = parse(fixture('registration.html'), 'registration.html');
  });

  it('finds screen, form, and action', () => {
    expect(doc.cemNodes.map(n => n.cemRole)).toEqual(['screen', 'form', 'action']);
  });

  it('screen id is registration', () => {
    expect(doc.cemNodes[0].cemValue).toBe('registration');
  });
});

describe('parse — profile.html', () => {
  let doc: ReturnType<typeof parse>;

  beforeEach(() => {
    doc = parse(fixture('profile.html'), 'profile.html');
  });

  it('finds two card nodes', () => {
    const cards = doc.cemNodes.filter(n => n.cemRole === 'card');
    expect(cards).toHaveLength(2);
    expect(cards.map(c => c.cemValue)).toEqual(['identity', 'preferences']);
  });

  it('finds badge with success variant', () => {
    const badges = doc.cemNodes.filter(n => n.cemRole === 'badge');
    expect(badges).toHaveLength(1);
    expect(badges[0].cemValue).toBe('success');
  });
});

describe('parse — assets-list.html', () => {
  let doc: ReturnType<typeof parse>;

  beforeEach(() => {
    doc = parse(fixture('assets-list.html'), 'assets-list.html');
  });

  it('finds list and row nodes', () => {
    const list = doc.cemNodes.filter(n => n.cemRole === 'list');
    const rows = doc.cemNodes.filter(n => n.cemRole === 'row');
    expect(list).toHaveLength(1);
    expect(rows).toHaveLength(2);
  });
});

describe('parse — message-thread.html', () => {
  let doc: ReturnType<typeof parse>;

  beforeEach(() => {
    doc = parse(fixture('message-thread.html'), 'message-thread.html');
  });

  it('finds thread and message nodes', () => {
    const thread = doc.cemNodes.filter(n => n.cemRole === 'thread');
    const messages = doc.cemNodes.filter(n => n.cemRole === 'message');
    expect(thread).toHaveLength(1);
    expect(messages).toHaveLength(2);
    expect(messages.map(m => m.cemValue)).toEqual(['received', 'sent']);
  });
});
