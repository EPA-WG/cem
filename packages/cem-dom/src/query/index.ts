import type { CemDocument, CemRole, DomNode, ElementNode, TextNode } from '../schema/types.js';

export function findByRole(doc: CemDocument, role: CemRole): ElementNode[] {
  return doc.cemNodes.filter(n => n.cemRole === role);
}

export function findById(doc: CemDocument, id: string): ElementNode | undefined {
  return doc.ids.get(id);
}

export function resolveLabel(doc: CemDocument, inputId: string): string | undefined {
  return doc.labels.get(inputId);
}

export function getTextContent(node: DomNode): string {
  if (node.nodeType === 'text') return (node as TextNode).value;
  return node.children.map(getTextContent).join(' ').trim();
}

export function hasAccessibleName(node: ElementNode, doc: CemDocument): boolean {
  if (node.attrs['aria-label']) return true;
  if (node.attrs['aria-labelledby'] && doc.ids.has(node.attrs['aria-labelledby'])) return true;
  if (node.attrs['id'] && doc.labels.has(node.attrs['id'])) return true;
  if (doc.implicitlyLabeled.has(node)) return true;
  if (node.attrs['title']) return true;
  if (getTextContent(node).trim()) return true;
  return false;
}

export function walkCem(doc: CemDocument, role: CemRole, fn: (node: ElementNode) => void): void {
  for (const node of doc.cemNodes) {
    if (node.cemRole === role) fn(node);
  }
}
