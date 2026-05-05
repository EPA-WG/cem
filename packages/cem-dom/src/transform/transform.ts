import type {
  CemDocument, DomNode, ElementNode, TextNode, CommentNode, DoctypeNode,
} from '../schema/types.js';

const VOID_ELEMENTS = new Set([
  'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
  'link', 'meta', 'param', 'source', 'track', 'wbr',
]);

function escapeAttr(v: string): string {
  return v.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');
}

function escapeText(v: string): string {
  return v.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function serializeAttrs(attrs: Record<string, string>): string {
  return Object.entries(attrs)
    .map(([k, v]) => v === '' ? ` ${k}` : ` ${k}="${escapeAttr(v)}"`)
    .join('');
}

function buildCeAttrs(node: ElementNode): Record<string, string> {
  const result: Record<string, string> = {};
  const cemAttrName = `data-cem-${node.cemRole}`;
  const cemValue = node.cemValue ?? '';

  // variant roles map cemValue → variant; others map → cem-id
  const isVariant = node.cemRole === 'action' || node.cemRole === 'badge' || node.cemRole === 'message';
  result[isVariant ? 'variant' : 'cem-id'] = cemValue;

  for (const [k, v] of Object.entries(node.attrs)) {
    if (k !== cemAttrName) result[k] = v;
  }
  return result;
}

function serializeNode(node: DomNode): string {
  switch (node.nodeType) {
    case 'document':
      return node.children.map(serializeNode).join('');
    case 'doctype':
      return `<!DOCTYPE ${(node as DoctypeNode).value}>`;
    case 'comment':
      return `<!--${(node as CommentNode).value}-->`;
    case 'text':
      return escapeText((node as TextNode).value);
    case 'element':
      return serializeElement(node as ElementNode);
  }
}

function serializeElement(node: ElementNode): string {
  const children = node.children.map(serializeNode).join('');

  if (node.cemRole !== null) {
    const ceTag = `cem-${node.cemRole}`;
    const attrStr = serializeAttrs(buildCeAttrs(node));
    return `<${ceTag}${attrStr}>${children}</${ceTag}>`;
  }

  const attrStr = serializeAttrs(node.attrs);
  if (VOID_ELEMENTS.has(node.tag)) return `<${node.tag}${attrStr}>`;
  return `<${node.tag}${attrStr}>${children}</${node.tag}>`;
}

export function transform(doc: CemDocument): string {
  return serializeNode(doc.root);
}
