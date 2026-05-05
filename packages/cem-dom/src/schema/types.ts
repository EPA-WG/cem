export const CEM_ATTRS = [
  'data-cem-screen',
  'data-cem-form',
  'data-cem-action',
  'data-cem-card',
  'data-cem-badge',
  'data-cem-list',
  'data-cem-row',
  'data-cem-thread',
  'data-cem-message',
] as const;

export type CemAttr = (typeof CEM_ATTRS)[number];

export const CEM_ROLES = [
  'screen', 'form', 'action', 'card', 'badge', 'list', 'row', 'thread', 'message',
] as const;

export type CemRole = (typeof CEM_ROLES)[number];

export const CEM_ATTR_TO_ROLE: Record<CemAttr, CemRole> = {
  'data-cem-screen': 'screen',
  'data-cem-form':   'form',
  'data-cem-action': 'action',
  'data-cem-card':   'card',
  'data-cem-badge':  'badge',
  'data-cem-list':   'list',
  'data-cem-row':    'row',
  'data-cem-thread': 'thread',
  'data-cem-message':'message',
};

export const ACTION_VARIANTS = ['primary', 'secondary', 'destructive', 'quiet'] as const;
export type ActionVariant = (typeof ACTION_VARIANTS)[number];

export const BADGE_VARIANTS = ['success', 'info', 'warning', 'error'] as const;
export type BadgeVariant = (typeof BADGE_VARIANTS)[number];

export const MESSAGE_VARIANTS = ['sent', 'received'] as const;
export type MessageVariant = (typeof MESSAGE_VARIANTS)[number];

export interface SourceLocation {
  line: number;
  column: number;
  byteOffset: number;
}

export type DomNodeType = 'document' | 'element' | 'text' | 'comment' | 'doctype';

export interface DomNode {
  nodeType: DomNodeType;
  loc: SourceLocation;
  children: DomNode[];
}

export interface DocumentNode extends DomNode {
  nodeType: 'document';
}

export interface ElementNode extends DomNode {
  nodeType: 'element';
  tag: string;
  attrs: Record<string, string>;
  cemRole: CemRole | null;
  cemValue: string | null;
}

export interface TextNode extends DomNode {
  nodeType: 'text';
  value: string;
}

export interface CommentNode extends DomNode {
  nodeType: 'comment';
  value: string;
}

export interface DoctypeNode extends DomNode {
  nodeType: 'doctype';
  value: string;
}

export interface ParseError {
  uri: string;
  line: number;
  column: number;
  byteOffset: number;
  code: string;
  severity: 'warning' | 'error' | 'fatal';
  message: string;
}

export interface ValidationMessage {
  rule: string;
  severity: 'warning' | 'error';
  message: string;
  loc: SourceLocation;
}

export interface CemDocument {
  uri: string;
  root: DocumentNode;
  title: string;
  lang: string;
  cemNodes: ElementNode[];
  ids: Map<string, ElementNode>;
  labels: Map<string, string>;
  /** Form controls that have an accessible name via a wrapping `<label>` (implicit association). */
  implicitlyLabeled: Set<ElementNode>;
  errors: ParseError[];
}
