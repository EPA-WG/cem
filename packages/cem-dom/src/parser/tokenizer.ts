import type { SourceLocation } from '../schema/types.js';

interface TokenBase extends SourceLocation {
  type: string;
}

export interface DoctypeToken extends TokenBase { type: 'doctype'; value: string; }
export interface CommentToken extends TokenBase { type: 'comment'; value: string; }
export interface TextToken    extends TokenBase { type: 'text';    value: string; }
export interface OpenToken    extends TokenBase { type: 'open';    tag: string; attrs: Record<string, string>; selfClosing: boolean; }
export interface CloseToken   extends TokenBase { type: 'close';   tag: string; }

export type HtmlToken = DoctypeToken | CommentToken | TextToken | OpenToken | CloseToken;

const VOID_ELEMENTS = new Set([
  'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
  'link', 'meta', 'param', 'source', 'track', 'wbr',
]);

export function tokenize(html: string): HtmlToken[] {
  const tokens: HtmlToken[] = [];
  let pos = 0;
  let line = 1;
  let col = 1;

  function snap(): SourceLocation { return { line, column: col, byteOffset: pos }; }
  function ch(offset = 0): string { return html[pos + offset] ?? ''; }

  function step(n = 1): void {
    for (let i = 0; i < n; i++) {
      if (pos >= html.length) break;
      if (html[pos] === '\n') { line++; col = 1; } else { col++; }
      pos++;
    }
  }

  function skipWs(): void {
    while (pos < html.length && /\s/.test(ch())) step();
  }

  function readName(): string {
    let s = '';
    while (pos < html.length && !/[\s\/>]/.test(ch()) && ch() !== '>') { s += ch(); step(); }
    return s;
  }

  function readAttrValue(): string {
    const q = ch();
    if (q === '"' || q === "'") {
      step();
      let v = '';
      while (pos < html.length && ch() !== q) { v += ch(); step(); }
      if (pos < html.length) step();
      return v;
    }
    let v = '';
    while (pos < html.length && !/[\s>]/.test(ch())) { v += ch(); step(); }
    return v;
  }

  function readAttrs(): Record<string, string> {
    const attrs: Record<string, string> = {};
    for (;;) {
      skipWs();
      if (pos >= html.length || ch() === '>' || (ch() === '/' && ch(1) === '>')) break;
      let name = '';
      while (pos < html.length && !/[\s=\/>]/.test(ch()) && ch() !== '>') { name += ch(); step(); }
      if (!name) { step(); continue; }
      skipWs();
      if (ch() === '=') {
        step();
        skipWs();
        attrs[name] = readAttrValue();
      } else {
        attrs[name] = '';
      }
    }
    return attrs;
  }

  while (pos < html.length) {
    if (ch() !== '<') {
      const loc = snap();
      let text = '';
      while (pos < html.length && ch() !== '<') { text += ch(); step(); }
      const trimmed = text.trim();
      if (trimmed) tokens.push({ type: 'text', value: trimmed, ...loc });
      continue;
    }

    if (html.startsWith('<!--', pos)) {
      const loc = snap();
      step(4);
      let value = '';
      while (pos < html.length && !html.startsWith('-->', pos)) { value += ch(); step(); }
      if (html.startsWith('-->', pos)) step(3);
      tokens.push({ type: 'comment', value: value.trim(), ...loc });

    } else if (html.startsWith('</', pos)) {
      const loc = snap();
      step(2);
      let tag = '';
      while (pos < html.length && ch() !== '>' && !/\s/.test(ch())) { tag += ch(); step(); }
      while (pos < html.length && ch() !== '>') step();
      if (pos < html.length) step();
      tokens.push({ type: 'close', tag: tag.toLowerCase(), ...loc });

    } else if (html.startsWith('<!', pos)) {
      const loc = snap();
      step(2);
      let value = '';
      while (pos < html.length && ch() !== '>') { value += ch(); step(); }
      if (pos < html.length) step();
      tokens.push({ type: 'doctype', value, ...loc });

    } else {
      const loc = snap();
      step(); // skip <
      const tag = readName().toLowerCase();
      if (!tag) { step(); continue; }
      const attrs = readAttrs();
      const selfClosing = ch() === '/';
      if (selfClosing) step();
      if (ch() === '>') step();
      tokens.push({ type: 'open', tag, attrs, selfClosing: selfClosing || VOID_ELEMENTS.has(tag), ...loc });
    }
  }

  return tokens;
}
