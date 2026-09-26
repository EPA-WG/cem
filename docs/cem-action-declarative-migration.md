# cem-action declarative migration

Status: explicit submit/reset support and colocated native-pointer tests accepted;
v3 XHTML deployment design accepted; implementation precedes migration cutover.

`cem-action` is the next migration candidate in the accepted
[component MVP](component-mvp.md) order. It has no component-specific behavior
module: the legacy registry contains a CEM-ML native button, attribute bindings,
slot projection and a release-time `pressed` slice. No new runtime capability
has been identified as necessary for preserving that behavior.

## Scope decision

The MVP component table describes text action, submit and command buttons.
The narrower [component reference](../packages/cem-components/docs/component-reference.md#actions)
describes a native command button. The current registry hardcodes
`type="button"`, and the sign-in workflow test explicitly asserts that type.
Changing it to submit by default would change form behavior during a migration.

Accepted scope: add explicit submit/reset support in this migration. Preserve
`type="button"` as the default so existing workflows remain non-submitting.

| Host attribute | Rendered native button contract |
| --- | --- |
| `type` | Default `button`; explicit `submit` and `reset` delegate to native HTML form behavior. Other explicit values use the browser's native button-type normalization. |
| `name`, `value` | Forward to the native button; submitter name/value participates in `FormData(form, submitter)`. |
| `form` | Forward for external form ownership. |
| `formaction`, `formenctype`, `formmethod`, `formtarget` | Forward native submitter overrides. |
| `formnovalidate` | Presence enables the native validation override, including an empty attribute value. |
| `disabled` | Presence disables the native button, including an empty or `"false"` attribute value, following native boolean-attribute semantics. |
| `aria-label` | Forward an explicit accessible name. Default slot and `label` fallback remain supported. |
| `loading`, `expanded` | Retain existing value bindings to `aria-busy` and `aria-expanded`; loading does not imply disabled. |

The browser owns required-input validation, submit/reset default actions,
submitter identity, external form ownership, and cancellation through
`preventDefault()`. Reset restores native control defaults; it does not invent a
CEM state-reset operation. The existing release-time `pressed` slice remains.
No component-local JavaScript or new form behavior installer is needed.

## Native-pointer test decision

The normative colocated-story policy currently permits `storybook/test`, the
shared preview loader, and the component's raw declaration imports. Its
synthetic `userEvent.hover` dispatches events but leaves `button.matches(':hover')`
false in Chromium. Synthetic pointer presses likewise cannot prove native
`:active` paint. The legacy action suite uses the real `vitest/browser` driver.

Accepted: permit a guarded dynamic `vitest/browser` import inside colocated
plays solely for native input checks. Assertions and ordinary interactions keep
using `storybook/test`. An `import.meta.env.MODE !== 'test'` early return protects
ordinary Storybook from loading the browser driver. Story descriptions identify
which native-pointer assertions run only in Vitest; the same rendered fixture
remains available for manual interaction in regular Storybook. The normative
policy and declarative gate now permit this narrow test-only exception.

The prototype passes trusted hover, held pointer/Space active colors, contrast,
geometry, focus, node/payload identity, disabled suppression, and release-time
activation checks. Its static Storybook build also passes without loading the
Vitest driver.

## XHTML asset-delivery decision

CEM Site deploys runtime assets through paired module-map v2 documents. The
canonical action declaration must be delivered to its search and interactive
pages, and those pages must explicitly load `cem-action.xhtml#cem-action`.
Removing the registry entry before providing this asset would break both pages.

A trial v2 resource with `contentType: "application/xhtml+xml"` fails Site's
build with `cem.module_map.resource_type_unsupported`. The
[module-map v2 schema](../packages/cem_ml/schema-packages/module-map-v2/v1/schema/module-map-v2.cem)
allows only JavaScript, CSS and WASM resources and explicitly declares future
asset kinds `reject-unless-a-later-schema-version-declares-it`. Expanding v2
silently or adding an application-local JavaScript copy workaround is not the
accepted contract.

Accepted: extend the existing worker-safe module-map v3 with deployment-only
XHTML resources. The [v3 XHTML design](module-map-v3-xhtml-assets.md) defines the
narrow MIME/extension contract, v1/v2 compatibility, native acceptance sequence,
Site adoption and cutover order. V3 already owns typed JavaScript/JSON imports
and declared module-edge rewriting; the XHTML addition preserves those rules.
No separate application asset-copy path is planned.

## Existing behavior to preserve

- Default slot supplies the visible label; the `label` fallback is `Action`.
- `variant` defaults to `primary`. Only the primary color intent is currently
  accepted by the action hover/active contracts; do not invent other intents.
- The generated native button receives disabled, loading/busy and expanded
  state from the existing attributes. Preserve loading/expanded value bindings;
  use an explicit presence query for native boolean attributes as specified above.
- Native activation records the existing release-time `pressed` slice. Held
  pointer/keyboard state belongs to CSS `:active`, not a new runtime field.
- Default, enabled hover and enabled active paint use their existing primary
  action token pairs. Disabled controls exclude hover/active treatment.
- Preserve the button node, authored payload, accessible name and geometry
  across held/released activation. Keep focus and native disabled semantics.

The accepted hover and active contracts are
[action-hover-contract.md](../packages/cem-components/docs/action-hover-contract.md)
and [action-active-contract.md](../packages/cem-components/docs/action-active-contract.md).
Their state semantics remain binding; migrate CSS ownership from the global
stylesheet into the declaration instead of preserving obsolete global selectors.

## Implementation sequence

1. Add `src/components/cem-action/cem-action.xhtml` with one `<cem-element>`
   declaration, `<template id="cem-action" type="text/cem-ml">`, embedded static
   token CSS and a stable `part="control"` on the native button. Use native
   declaration-owned `@scope` handling and retain the public slot contract.
2. Add the colocated CSF Next story through the shared declaration loader.
   Put component unit assertions in its `play` functions. Cover fallback and
   rich labels, state reflection, activation, disabled behavior, pointer/keyboard
   interactions, node identity and once-per-declaration style installation.
3. Remove the legacy registry member and migrated global selectors. Update the
   migration inventory to 47 legacy tags only after all consumers and gates
   reflect the canonical declaration. Do not add an installer or duplicate the
   template in JavaScript to retain the old registry shape.
4. Audit affected workflow/demo loaders and package assets. Explicitly load the
   canonical XHTML where consumers previously relied on the legacy registry.
   Keep legacy workflow integration tests for the resulting composition; move
   action-specific unit assertions out of the separate legacy component suite.
   Document the shrinking legacy installer surface.
5. Update style/state-matrix/catalog verification to read the embedded action
   CSS and colocated evidence while retaining exact token/state checks. Do not
   relax global-selector rejection or increase the frozen legacy-code baseline.
6. Run the declarative gate, focused action stories and affected consumer/style/
   state/catalog/package checks. Use broader tests only for shared modules
   actually changed. Stop before an unsupported declarative capability or an
   unresolved API change.

## Audit evidence

The legacy source is `packages/cem-components/src/lib/primitives.ts`; global
paint rules are in `src/styles.css`. Existing action coverage spans
`primitives.browser.spec.ts`, the hover/active fixtures and workflow tests.
`workflows.browser.spec.ts` asserts the sign-in action remains `type="button"`.
The package README already describes `installCemComponentPrimitives` as a
shrinking legacy API; migrated components are canonical XHTML package assets.
The architecture gate currently reports one migrated component, 48 legacy tags
and 61 legacy authored code files. This planning change does not alter those
counts or claim the action migration is complete.

## Prototype findings

The focused Chromium prototype passed three stories: fallback/rich labels,
state reflection, and explicit native forms (validation, submitter name/value,
external form overrides, disabled suppression, and canceled/normal reset).
After the test-policy decision, all five prototype stories pass, including
native pointer/keyboard states and live type/disabled attribute updates.
No global tests were run. Presence checks must use
`if seq:count(datadom.attributes.disabled) > 0 { true } else { null }` (and the
same pattern for `formnovalidate`), following canonical `cem-select`. Binding an
empty host attribute directly omitted the native boolean attribute. Assertions
must check `HTMLButtonElement.disabled`: a matcher can treat the disabled custom
ancestor as sufficient even when its native child is enabled.

Use `:scope > button[part~='control']:where(:enabled:hover)` and the corresponding
active selector. Without `:where`, the selectors exceed the accepted `0-2-1`
specificity ceiling and are suppressed with a diagnostic. This is existing
runtime policy, not a missing CSS capability.

The migration remains unshipped until v3 XHTML deployment is implemented
and consumer/gate cutover is complete. No legacy registry or global style has
been removed by the planning change.

## Cutover trial verification

The unshipped cutover draft passed the declarative gate (2 canonical / 47 legacy),
style contract, state matrix, Figma inventory, Material parity, catalog build,
package build/typecheck, packed-XHTML check, and five focused action stories.
The ordinary static Storybook build passed. The legacy source remains in place
until Site's XHTML deployment path is resolved; these draft results do not claim
a completed migration.

The affected component package suite reported 80 passing and 49 failing tests.
Two representative failures reproduce using the committed pre-migration registry
and state tests: `cem-icon-button[disabled]` leaves its native button enabled,
and `cem-nav[collapsible]` lacks its disclosure. Other failures include legacy
required/boolean state and datepicker cases; their individual baseline status
has not yet been established. Preserve these failures as follow-up work rather
than weakening assertions or changing frozen behavior modules. No workspace-wide
tests were run.
