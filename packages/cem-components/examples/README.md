# CEM Component Examples

These package-local examples show how the MVP declarations compose into workflow-shaped UI. Executable workflow
fixtures live in `../tests/workflows/` and are covered by `src/lib/workflows.browser.spec.ts`; primitive-family browser
coverage remains in `src/lib/primitives.browser.spec.ts`.

| Example | Purpose |
| --- | --- |
| [`auth-form.html`](./auth-form.html) | Auth form with action and input families. |
| [`asset-browser.html`](./asset-browser.html) | Navigation plus content components for asset browsing. |
| [`feedback-settings.html`](./feedback-settings.html) | Settings and feedback flow using switches, dialog, sheet, toast, progress, skeleton, and alert. |

Examples assume the page has installed the CEM theme CSS, `@epa-wg/cem-elements`, and `@epa-wg/cem-components`.
