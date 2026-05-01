# CEM Component MVP

This document defines the first component surface and state matrix for CEM fixtures and Figma planning.

## Component List

| Component | Primary Use | Required Tokens |
| --- | --- | --- |
| App shell | Page structure, landmarks, skip target | palette, gap, inset, typography |
| Top bar | Product title and primary actions | palette, stroke, gap, typography |
| Navigation list | Section and task navigation | palette, gap, inset, bend, typography |
| Form | Grouped user input and submission | gap, inset, typography |
| Text field | Text entry, validation, help text | palette, stroke, bend, gap, typography |
| Select field | Bounded choice entry | palette, stroke, bend, gap, typography |
| Checkbox | Binary consent and filters | palette, stroke, bend, control, typography |
| Button | Primary, secondary, quiet, destructive actions | palette, action, control, bend, typography |
| Card | Summary container for profile/assets/messages | palette, stroke, bend, gap, inset |
| Data list | Asset rows, search results, settings lists | palette, stroke, gap, typography |
| Badge | Status, count, and priority labels | palette, bend, inset, typography |
| Message thread | Conversation list and message composer | palette, gap, inset, bend, typography |
| Alert | Error, warning, success, and info feedback | palette, action, stroke, gap, typography |

## State Matrix

| State | Applies To | Required Behavior |
| --- | --- | --- |
| default | All components | Uses mode-aware palette, type, shape, spacing, and stroke variables. |
| hover | Interactive components | Uses action hover recipes or generated action color variables. |
| focus-visible | Interactive components | Shows focus ring with CEM stroke/ring tokens and does not rely on color alone. |
| active | Buttons, nav items, selectable rows | Uses active action treatment and keeps text contrast. |
| selected | Navigation, checkbox, list rows | Uses selected semantics distinct from hover/focus. |
| disabled | Inputs, buttons, nav items | Reduces affordance without dropping below readable contrast for labels. |
| invalid | Form fields, forms | Exposes error message relationship and error color/stroke tokens. |
| required | Form fields | Exposes required semantics without relying on visual mark alone. |
| loading | Buttons, lists, forms | Preserves layout dimensions while status changes. |
| empty | Lists, threads, assets | Provides visible empty-state content and action path. |

## First Validation Flow

Use the semantic fixture set in `examples/semantic/` as the first cross-check:

1. Render each fixture through the DOM/XSLT pipeline.
2. Confirm every component maps to an MVP component row above.
3. Confirm every visible component state maps to a state row above.
4. Confirm every visual value comes from CEM token CSS or native Figma variables.
