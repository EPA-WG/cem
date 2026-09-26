# Storybook theme switcher placement

Status: proposed; placement decision required before implementation.

The active requirement is the **Remaining declarative UI migration** item in
[todo.md](todo.md): a Storybook-owned accessible switcher composed from
production `cem-select`, exact five-mode switching on the preview root, and
per-mode interaction/token evidence. Stories must not own or persist global
state. This note proposes implementation boundaries; it does not change the
component or theme contracts.

## Current integration

`packages/cem-elements/.storybook/main.ts` includes both runtime stories and
colocated `cem-components` stories. Its `preview.ts` installs the CEM runtime,
loads public theme CSS and provides the shared declaration loader. There is
currently no theme global or manager configuration in that Storybook.

Production `cem-select` is an XHTML/CEM-ML declaration with the shared
`choice-select` capability. Its story uses a real keyboard tab to reach its
combobox and checks focus. Adding another focusable control ahead of that
canvas changes that interaction boundary. Stories also return both HTML strings
and elements, so a preview decorator must support both without replacing nodes
or resetting their state.

## Placement decision

| Location | Benefit | Cost |
| --- | --- | --- |
| Storybook manager toolbar (recommended) | Keeps story DOM and keyboard order unchanged; naturally owns global controls | Manager is a separate document and needs its own CEM runtime, production declaration and theme CSS; real manager/preview browser coverage is required |
| Preview-owned toolbar outside the story body | Reuses the existing preview runtime and declaration loader | Adds a focusable control to the preview document; requires an explicit toolbar/canvas focus boundary and regression checks for existing keyboard stories |

Both choices must use the production `cem-select`; a native Storybook toolbar
menu would not satisfy the component-composition requirement. Neither choice
adds component-local behavior. Storybook integration may bridge the production
control's public change event to Storybook globals and apply the global to its
owned preview root. Selection, keyboard handling and popup behavior remain in
`cem-select` and `choice-select`.

## Proposed behavior after placement is accepted

- One Storybook global, `cemTheme`, with exactly `native`, `light`, `dark`,
  `contrast-light` and `contrast-dark`; initial mode `native`.
- The control is named **Theme**, with labels Native, Light, Dark, Contrast
  light and Contrast dark. Values map explicitly to the public
  `cem-theme-*` selectors.
- Storybook globals are the source of truth. Control changes update that
  global; external global changes update the control. No story-local storage
  or separate local-storage preference is introduced.
- Apply `data-theme="cem-theme-<mode>"` to the preview document root, preserving
  unrelated root attributes. Do not rewrite component declarations or inject
  copied token values. Explicitly themed examples keep their local scope.
- Missing or unrecognized global values render the safe initial `native` mode
  without creating a sixth option. Avoid update-event feedback loops.
- Keep the control available through story navigation, with listener cleanup
  and no duplicate runtime/declaration installation in either document.

## Implementation and verification plan

1. Add Storybook-owned composition and the host adapter for the chosen location.
   Load the production declaration and public CSS; implement no replacement
   select behavior or component-specific CSS in application code.
2. Add the global and preview-root integration. Keep generic runtime/declaration
   helpers separate from manager setup so importing a helper cannot install
   preview-only hooks in the manager document.
3. Add a Storybook-owned integration fixture that selects every mode, checks
   exact options and global/root synchronization, and verifies representative
   resolved background/text and control tokens against the public theme CSS.
   Include keyboard access, accessible naming, focus retention, external global
   changes and navigation. Component unit coverage remains colocated.
4. Run the relevant declarative gate, focused switcher/select browser checks,
   and the affected Storybook configuration checks. Scope broader checks to any
   shared preview modules actually changed. Do not publish artifacts.

Next action: select manager-toolbar or preview-toolbar placement before adding
runtime/UI integration code.
