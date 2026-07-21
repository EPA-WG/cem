# CEM Component Examples

These package-local examples show how the MVP declarations compose into workflow-shaped UI. Executable workflow
fixtures live in `../tests/workflows/` and are covered by `src/lib/workflows.browser.spec.ts`; primitive-family browser
coverage remains in `src/lib/primitives.browser.spec.ts`.

| Example | Purpose |
| --- | --- |
| [`auth-form.html`](./auth-form.html) | Auth form with action and input families. |
| [`profile-editor.html`](./profile-editor.html) | Profile editor with avatar, editable fields, preferences, validation feedback, and save action. |
| [`asset-browser.html`](./asset-browser.html) | Navigation plus content components for asset browsing. |
| [`discussion-thread.html`](./discussion-thread.html) | Discussion thread with message list, composer, status badge, toast, alert, and post action. |
| [`settings.html`](./settings.html) | Settings and feedback flow using switches, radio choices, dialog, sheet, toast, progress, skeleton, and alert. |
| [`feedback-settings.html`](./feedback-settings.html) | Legacy combined settings example retained as an alias for existing links. |

Examples assume the page has installed the CEM theme CSS, `@epa-wg/cem-elements`, and `@epa-wg/cem-components`.
