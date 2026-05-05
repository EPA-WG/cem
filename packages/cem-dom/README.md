# `@epa-wg/cem-dom`

Schema, parser, validator, and transform pipeline for CEM semantic documents.
Parses HTML annotated with `data-cem-*` attributes into a typed in-memory model,
validates the model against the CEM vocabulary, and transforms it to light-DOM
custom-element markup compatible with `@epa-wg/custom-element`.

## Install

```bash
yarn add @epa-wg/cem-dom
```

## Primary exports

| Export | Description |
| ------ | ----------- |
| `parse(html, uri?)` | HTML string → `CemDocument` |
| `parseStream(input, uri?)` | `AsyncIterable` → `CemDocument` (streaming) |
| `validate(doc)` | `CemDocument` → `ValidationMessage[]` |
| `transform(doc)` | `CemDocument` → HTML string (semantic → custom-element markup) |
| `findByRole(doc, role)` | Returns all CEM nodes for the given role |
| `findById(doc, id)` | Looks up a node by its `id` attribute |
| `resolveLabel(doc, inputId)` | Returns the label text associated with an input |
| `hasAccessibleName(node, doc)` | Checks that a node has a computed accessible name |
| Types | `CemDocument`, `CemRole`, `ElementNode`, `ValidationMessage`, … |

## Round-trip example: `login.html`

```typescript
import { parse, validate, transform } from '@epa-wg/cem-dom';
import { readFileSync } from 'node:fs';

const html = readFileSync('examples/semantic/login.html', 'utf8');
const doc = parse(html, 'login.html');

// Validate — zero hard violations expected for well-formed fixtures
const messages = validate(doc);
const errors = messages.filter(m => m.severity === 'error');
console.log(`Violations: ${errors.length}`); // 0

// Transform: data-cem-* → cem-* custom elements
const output = transform(doc);
// Produces: <cem-screen cem-id="login" aria-labelledby="login-title">
//             <cem-form cem-id="sign-in" method="post" action="/session">
//               …<cem-action variant="primary" type="submit">Sign in</cem-action>
//             </cem-form>
//           </cem-screen>
```

## Build & test

```bash
yarn build                                # build all packages including cem-dom
nx run @epa-wg/cem-dom:build              # build this package
nx run @epa-wg/cem-dom:test               # unit tests
nx run @epa-wg/cem-dom:lint               # lint
nx run @epa-wg/cem-dom:validate-fixtures  # validate examples/semantic/*.html
nx run @epa-wg/cem-dom:build:docs         # compile schema markdown → XHTML
```

## Key paths

| Purpose | Path |
| ------- | ---- |
| Schema vocabulary | `src/schema/cem.schema.md` |
| TypeScript types | `src/schema/types.ts` |
| HTML tokenizer | `src/parser/tokenizer.ts` |
| Parser | `src/parser/parse.ts` |
| Query helpers | `src/query/index.ts` |
| Validator | `src/validate/validate.ts` |
| XSL transform spec | `src/transform/cem-to-ce.xsl` |
| Transform helper | `src/transform/transform.ts` |
| Fixture validation script | `scripts/validate-fixtures.mjs` |
| Built output | `dist/` |
| Validation reports | `dist/cem-dom.report.{md,json}` |

## Related docs

- [CEM DOM acceptance criteria](../../docs/cem-dom-ac.md) — testable AC for the parser/transform stack.
- [CEM DOM library plan](../../docs/dom-library-plan.md) — package responsibility and non-goals.
- [CEM component MVP](../../docs/component-mvp.md) — component list and state matrix that drives the schema vocabulary.
- [CEM components package](../cem-components/README.md) — Phase 3 runtime that will consume this package's transform output.
- [Repository documentation index](../../docs/index.md) — full project map.
