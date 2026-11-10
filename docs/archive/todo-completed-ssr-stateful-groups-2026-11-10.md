# Nested and styled stateful SSR hydration

Base commit `c9f711b355a02ed15f9d630721dc8448bda4e4ea`.

- [x] Admit compatible stateful participants with inline declarations, HTML
  payloads, and verified declaration and instance stylesheets.
- [x] Prepare every processing baseline before writing hydration metadata or
  attaching listeners. Bind parent inputs in the nested child's inert payload
  and bind child controls inside the child's own rendered range.
- [x] Recheck form ownership and mirrors, stylesheet rules, and captured DOM
  before publication. Keep all server ranges on refusal.
- [x] Verify preserved focus, edits, independent form and slice updates, and
  retained render and stylesheet nodes in Chromium. Cover worker and
  main-thread processing and verified same-origin imports.

The native browser admission and stripped SSR suites passed. Chromium covered
80 cases, including seven stateful group cases, with zero off-origin requests.
