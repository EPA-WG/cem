import { tokenize } from './tokenizer.js';
import type { HtmlToken } from './tokenizer.js';
import type {
  CemDocument, CemRole, DocumentNode, ElementNode, TextNode,
  CommentNode, DoctypeNode, DomNode, ParseError, SourceLocation,
} from '../schema/types.js';
import { CEM_ATTRS, CEM_ATTR_TO_ROLE } from '../schema/types.js';

const ZERO_LOC: SourceLocation = { line: 0, column: 0, byteOffset: 0 };

function makeDocument(): DocumentNode {
  return { nodeType: 'document', loc: ZERO_LOC, children: [] };
}

function makeElement(tag: string, attrs: Record<string, string>, loc: SourceLocation): ElementNode {
  const cemAttr = CEM_ATTRS.find(a => a in attrs);
  const cemRole: CemRole | null = cemAttr ? CEM_ATTR_TO_ROLE[cemAttr] : null;
  const cemValue = cemAttr != null ? (attrs[cemAttr] ?? null) : null;
  return { nodeType: 'element', tag, attrs, cemRole, cemValue, loc, children: [] };
}

function buildTree(tokens: HtmlToken[], uri: string): {
  root: DocumentNode;
  ids: Map<string, ElementNode>;
  errors: ParseError[];
} {
  const root = makeDocument();
  const stack: DomNode[] = [root];
  const ids = new Map<string, ElementNode>();
  const errors: ParseError[] = [];

  for (const tok of tokens) {
    const parent = stack[stack.length - 1];

    if (tok.type === 'doctype') {
      const node: DoctypeNode = { nodeType: 'doctype', value: tok.value, loc: tok, children: [] };
      parent.children.push(node);

    } else if (tok.type === 'comment') {
      const node: CommentNode = { nodeType: 'comment', value: tok.value, loc: tok, children: [] };
      parent.children.push(node);

    } else if (tok.type === 'text') {
      const node: TextNode = { nodeType: 'text', value: tok.value, loc: tok, children: [] };
      parent.children.push(node);

    } else if (tok.type === 'open') {
      const node = makeElement(tok.tag, tok.attrs, tok);
      parent.children.push(node);
      if (tok.attrs['id']) ids.set(tok.attrs['id'], node);
      if (!tok.selfClosing) stack.push(node);

    } else if (tok.type === 'close') {
      if (stack.length <= 1) continue;
      let i = stack.length - 1;
      while (i > 0 && (stack[i] as ElementNode).tag !== tok.tag) i--;
      if (i > 0) {
        stack.splice(i);
      } else {
        errors.push({
          uri, line: tok.line, column: tok.column, byteOffset: tok.byteOffset,
          code: 'unmatched-close', severity: 'warning',
          message: `Unmatched close tag </${tok.tag}>`,
        });
      }
    }
  }

  return { root, ids, errors };
}

function textContent(node: DomNode): string {
  if (node.nodeType === 'text') return (node as TextNode).value;
  return node.children.map(textContent).join(' ');
}

const FORM_CONTROLS = new Set(['input', 'textarea', 'select', 'button']);

function firstFormControl(node: DomNode): ElementNode | null {
  if (node.nodeType === 'element') {
    const el = node as ElementNode;
    if (FORM_CONTROLS.has(el.tag)) return el;
  }
  for (const child of node.children) {
    const found = firstFormControl(child);
    if (found) return found;
  }
  return null;
}

function collectLabelAssociations(root: DocumentNode): {
  labels: Map<string, string>;
  implicitlyLabeled: Set<ElementNode>;
} {
  const labels = new Map<string, string>();
  const implicitlyLabeled = new Set<ElementNode>();
  function visit(node: DomNode): void {
    if (node.nodeType === 'element') {
      const el = node as ElementNode;
      if (el.tag === 'label') {
        if (el.attrs['for']) {
          labels.set(el.attrs['for'], textContent(el).trim());
        } else {
          const ctrl = firstFormControl(el);
          if (ctrl) implicitlyLabeled.add(ctrl);
        }
      }
    }
    for (const child of node.children) visit(child);
  }
  visit(root);
  return { labels, implicitlyLabeled };
}

function collectCemNodes(node: DomNode, result: ElementNode[] = []): ElementNode[] {
  if (node.nodeType === 'element' && (node as ElementNode).cemRole !== null) {
    result.push(node as ElementNode);
  }
  for (const child of node.children) collectCemNodes(child, result);
  return result;
}

function findFirst(node: DomNode, pred: (n: ElementNode) => boolean): ElementNode | null {
  if (node.nodeType === 'element' && pred(node as ElementNode)) return node as ElementNode;
  for (const child of node.children) {
    const found = findFirst(child, pred);
    if (found) return found;
  }
  return null;
}

export function parse(html: string, uri = ''): CemDocument {
  const tokens = tokenize(html);
  const { root, ids, errors } = buildTree(tokens, uri);
  const { labels, implicitlyLabeled } = collectLabelAssociations(root);
  const cemNodes = collectCemNodes(root);
  const title = findFirst(root, n => n.tag === 'title');
  const htmlEl = findFirst(root, n => n.tag === 'html');
  return {
    uri,
    root,
    title: title ? textContent(title).trim() : '',
    lang: htmlEl?.attrs['lang'] ?? 'en',
    cemNodes,
    ids,
    labels,
    implicitlyLabeled,
    errors,
  };
}

export async function parseStream(
  input: AsyncIterable<Uint8Array | string>,
  uri = ''
): Promise<CemDocument> {
  const chunks: string[] = [];
  const decoder = new TextDecoder();
  for await (const chunk of input) {
    chunks.push(typeof chunk === 'string' ? chunk : decoder.decode(chunk, { stream: true }));
  }
  chunks.push(decoder.decode());
  return parse(chunks.join(''), uri);
}
