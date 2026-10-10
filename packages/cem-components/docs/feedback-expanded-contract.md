# Feedback visibility contract

Dialogs now follow the [native dialog contract](./dialog-contract.md), and
tooltips follow the [native tooltip contract](./tooltip-contract.md).
Their former registry behavior modules, `transient`/`expanded` dialog branches
and `cem-dismiss` adapter are retired. Shared `native-surface` owns their
command, preparation, close and focus lifecycle.

## Focus-neutral sheet

`cem-sheet` remains one named `<aside role="region">`. Without `transient`, it
is visible and ignores `expanded`. With `transient`, presence of `expanded`
removes native `hidden`; removing `expanded` hides it. Both attributes use
presence semantics. The application owns opener `aria-expanded` and
`aria-controls`, and any recovery after removing a focused descendant.

The sheet never traps or moves focus, intercepts Escape, makes the document
inert or emits dialog dismissal events. Rerender preserves the region and its
native descendant state. Its canonical declaration and colocated
[stories](../src/components/cem-sheet/cem-sheet.stories.ts) cover persistent and
transient state, native keyboard focus, forms and worker/fallback reconnect.
See the [sheet contract](./sheet-contract.md) for naming and composition.
