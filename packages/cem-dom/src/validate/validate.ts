import type { CemDocument, DomNode, ElementNode, ValidationMessage } from '../schema/types.js';
import { ACTION_VARIANTS, BADGE_VARIANTS, CEM_ATTRS, MESSAGE_VARIANTS } from '../schema/types.js';
import { hasAccessibleName } from '../query/index.js';

function err(rule: string, message: string, node: ElementNode): ValidationMessage {
  return { rule, severity: 'error', message, loc: node.loc };
}

function warn(rule: string, message: string, node: ElementNode): ValidationMessage {
  return { rule, severity: 'warning', message, loc: node.loc };
}

function validateAriaRef(
  node: ElementNode,
  attr: string,
  doc: CemDocument,
  out: ValidationMessage[]
): void {
  const ref = node.attrs[attr];
  if (!ref) return;
  if (!doc.ids.has(ref)) {
    out.push(err('broken-aria-ref', `${attr}="${ref}" references unknown id`, node));
  }
}

function validateForRef(
  node: ElementNode,
  doc: CemDocument,
  out: ValidationMessage[]
): void {
  const forVal = node.attrs['for'];
  if (!forVal) return;
  if (!doc.ids.has(forVal)) {
    out.push(err('broken-for-ref', `for="${forVal}" references unknown id`, node));
  }
}

function collectAll(node: DomNode, result: ElementNode[] = []): ElementNode[] {
  if (node.nodeType === 'element') result.push(node as ElementNode);
  for (const child of node.children) collectAll(child, result);
  return result;
}

export function validate(doc: CemDocument): ValidationMessage[] {
  const out: ValidationMessage[] = [];

  for (const node of doc.cemNodes) {
    switch (node.cemRole) {
      case 'screen':
        if (!node.attrs['aria-labelledby'] && !node.attrs['aria-label']) {
          out.push(warn('missing-screen-label', 'data-cem-screen should have aria-labelledby or aria-label', node));
        }
        validateAriaRef(node, 'aria-labelledby', doc, out);
        break;

      case 'form':
        validateAriaRef(node, 'aria-labelledby', doc, out);
        break;

      case 'action': {
        const v = node.cemValue;
        if (v && !(ACTION_VARIANTS as readonly string[]).includes(v)) {
          out.push(err('invalid-action-variant',
            `data-cem-action="${v}" is not a valid variant (${ACTION_VARIANTS.join(', ')})`, node));
        }
        if (!hasAccessibleName(node, doc)) {
          out.push(err('missing-accessible-name', 'data-cem-action must have an accessible name', node));
        }
        break;
      }

      case 'badge': {
        const v = node.cemValue;
        if (v && !(BADGE_VARIANTS as readonly string[]).includes(v)) {
          out.push(err('invalid-badge-variant',
            `data-cem-badge="${v}" is not a valid variant (${BADGE_VARIANTS.join(', ')})`, node));
        }
        break;
      }

      case 'message': {
        const v = node.cemValue;
        if (v && !(MESSAGE_VARIANTS as readonly string[]).includes(v)) {
          out.push(err('invalid-message-variant',
            `data-cem-message="${v}" is not a valid variant (${MESSAGE_VARIANTS.join(', ')})`, node));
        }
        break;
      }

      case 'card':
      case 'list':
      case 'row':
      case 'thread':
        validateAriaRef(node, 'aria-labelledby', doc, out);
        break;
    }

    // Flag any unknown data-cem-* attributes
    for (const attr of Object.keys(node.attrs)) {
      if (attr.startsWith('data-cem-') && !(CEM_ATTRS as readonly string[]).includes(attr)) {
        out.push(warn('unknown-cem-attr', `Unknown attribute ${attr}`, node));
      }
    }
  }

  // Validate label[for] references and form field accessible names
  for (const node of collectAll(doc.root)) {
    if (node.tag === 'label') {
      validateForRef(node, doc, out);
    }
    if ((node.tag === 'input' || node.tag === 'textarea' || node.tag === 'select') &&
        node.attrs['type'] !== 'hidden' && node.attrs['type'] !== 'submit' && node.attrs['type'] !== 'button') {
      if (!hasAccessibleName(node, doc)) {
        out.push(err('missing-accessible-name',
          `<${node.tag}> must have an accessible name (label, aria-label, or aria-labelledby)`, node));
      }
    }
  }

  return out;
}
