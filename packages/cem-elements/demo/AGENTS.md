# `cem-elements` demo authoring instructions

These instructions apply to this directory and its subdirectories. The
repository-level `AGENTS.md` and `CLAUDE.md` instructions continue to apply.

## Treat demos as documentation

- A demo page must explain the featured capability before presenting examples.
  State what the feature does, why or when it is useful, and the result the
  reader should expect. Include important limits or non-use cases when they
  prevent a likely misunderstanding.
- Write legends and descriptions for readers, not for the test suite. A legend
  identifies the case; its description states the behavior being demonstrated.
- Keep prose, source, and rendered output focused on the featured behavior.
  Remove incidental markup and values that compete with the lesson.
- Add a **See also** section with relative links to closely related demo pages.
  Cross-reference both directions when two pages split one conceptual topic.
- Use repository-owned, deterministic fixtures and assets. Depend on a remote
  resource only when remote resolution itself is the behavior under
  demonstration.

## Isolate cases

- Put exactly one authored use case in each `cem-demo-element`. Do not combine
  naked, wrapped, overridden, referrer, input-form, or state variants merely to
  make a visual comparison. Give each variant its own legend and sample.
- Keep each sample self-contained: place its declaration, instance, payload,
  local data, and supporting resource elements in that sample's inert
  `<template>` whenever practical.
- Prefer an anonymous `<cem-element>` without `tag` when the declaration is
  rendered only in place. Add a tag and an explicit instance only when the case
  requires reuse, attributes or payload on an instance, composition by another
  DCE, selector/referrer lookup, or a named declaration as the subject of the
  example. Never add a tag solely so a test can locate the result.
- A reusable display helper may be named when several cases genuinely consume
  it. Keep the helper small, document it as a helper, and do not let it obscure
  the feature being demonstrated.
- Give legends stable, descriptive names. Number sequential cases when order
  aids reading, and update every inventory assertion when cases move or split.

## Keep the gallery compact

- Design ordinary samples so at least two `cem-demo-element` cards can fit in
  one row at desktop widths. Avoid unnecessary fixed widths, many table
  columns, long labels, and unbroken output.
- Treat whitespace inside an `cem-demo-element`'s direct `<template>` as demo
  content. `cem-demo-element` preserves that indentation when it presents the
  sample, so leading spaces widen the displayed source and can prevent two
  cards from fitting in a row.
- Start the sample payload at column 1 on the line immediately after the direct
  `<template>` tag. Do not indent the whole payload to match the surrounding
  page HTML. Keep the declaration's internal indentation because that expresses
  its own structure:

  ```html
  <cem-demo-element legend="Example" description="What this case proves">
      <template>
  <cem-element>
      <template type="text/cem-ml">
  {p | Compact demo content}
      </template>
  </cem-element>
      </template>
  </cem-demo-element>
  ```

- Shorten long displayed URLs while preserving access to the complete value;
  for example, use the existing expanding-link pattern and `str:shorten`.
- Move intrinsically wide material, such as a URL/referrer matrix or a broad
  table, to a focused demo file and cross-link it from the parent topic.
- Prefer semantic and accessible HTML: associate labels with controls, provide
  useful image alternatives, use table headings, and make interactive results
  understandable without relying on color alone.

## Author CEM-ML for reading

- Use the tabular CEM-ML layout. Put nested closing braces together on the last
  line when that makes the hierarchy scannable, and align multiline sequences
  and attributes instead of producing wide lines. For example:

  ```cem-ml
  {article @class=demo-card |
      {ul |
          {cem:for-each
              @select='
                  (   "Apple"
                  ,   "Banana"
                  )'
              @as=fruit |
              {li | {$fruit}}
  }   }   }
  ```

- Put static component presentation in a `{style | ... }` node at the
  top of the DCE template. Use inline style attributes only when the changing
  style value is itself part of the demonstrated data behavior.
- Break long attribute sets across aligned lines. Keep short elements on one
  line when they remain readable.
- Preserve the established formatting of cases unrelated to the edit. Do not
  run a broad formatter over a hand-arranged demo page. In particular, do not
  let an HTML formatter re-indent payload content under an
  `cem-demo-element > template` boundary.
- Escape URLs correctly for the containing syntax, including `&amp;` in HTML
  attributes, and keep resolution inputs explicit enough that a reader can see
  whether a URL is relative, absolute, or import-map based.

## Keep demos testable without test markup

- Demo markup is the product under test. Do not add test-only classes, IDs,
  `data-*` attributes, state outputs, wrapper elements, or hidden anchors.
- Tests should find cases by their public legend and interact through natural
  elements, labels, attributes, visible content, and feature-owned semantics.
  If a test is difficult to write, improve the public example rather than
  inserting a private selector hook.
- Every demo must work both as a standalone HTTP page and when its authored
  document is source-loaded by the Storybook fixture. Resolve relative assets
  from the demo/declaration URL, not from an assumed test runner URL.
- When adding, removing, renaming, or splitting a demo page or case:
  - add or update the source-contract unit test;
  - add or update the source-loaded Storybook interaction;
  - update the standalone/source-loaded fixture inventory;
  - include the demo file in the relevant Nx target inputs; and
  - add and complete the fixture checklist item required by `CLAUDE.md`.
- Assert behavior and reader-visible outcomes. Do not freeze whitespace,
  incidental wrapper structure, or formatting unless source presentation is
  the explicit contract.

## Before finishing

- Confirm that each `cem-demo-element` teaches only one case and has a useful
  description.
- Confirm that tags, helper components, markup, and styling are necessary for
  the demo rather than its tests.
- Check the page at a desktop width for a two-card row and for overflow.
- Check that each direct demo `<template>` starts its payload at column 1 and
  does not contribute leading presentation whitespace.
- Check related-page links and standalone relative resource resolution.
- Run the focused unit and Storybook coverage, the demo-fixture verifier, lint,
  and typecheck through the existing Nx targets when the change affects them.
