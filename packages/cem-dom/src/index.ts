export { parse, parseStream } from './parser/parse.js';
export { validate } from './validate/validate.js';
export { transform } from './transform/transform.js';
export { findByRole, findById, resolveLabel, getTextContent, hasAccessibleName, walkCem } from './query/index.js';
export type {
  CemDocument, CemRole, CemAttr,
  ElementNode, DocumentNode, TextNode, CommentNode, DoctypeNode, DomNode,
  ParseError, ValidationMessage, SourceLocation,
  ActionVariant, BadgeVariant, MessageVariant,
} from './schema/types.js';
export { CEM_ATTRS, CEM_ROLES, CEM_ATTR_TO_ROLE, ACTION_VARIANTS, BADGE_VARIANTS, MESSAGE_VARIANTS } from './schema/types.js';
