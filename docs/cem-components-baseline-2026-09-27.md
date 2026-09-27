# Legacy component baseline — 2026-09-27

## Result

At `4c467348`, with the packaged `cem-elements` build restored through Nx,
the unchanged legacy suite reports **81 passing / 48 failing / 129 total**.
This matches the counts recorded during native CSS closure. The command below
runs only the component suite; it does not claim the aggregate package gate is
green. The runtime build was run first because these tests import its package.

After the shared harness fixture repair, the suite reports **82 passing /
47 failing / 129 total**. Comparing full test names removes exactly the harness
failure listed below and adds no failures. The aggregate package gate remains red.

```sh
yarn nx run cem-elements:build
yarn nx run-many --targets=test --projects=@epa-wg/cem-components --excludeTaskDependencies --skipNxCache --args='--run --reporter=json --outputFile=/tmp/cem-components-baseline.json'
```

The JSON report and console log are local diagnostic outputs under `/tmp`;
the failure inventory below preserves their useful contents in the repository.

## Attribution and next work

- **Empty boolean attributes:** legacy templates use value bindings such as
  `@disabled={datadom.attributes.disabled}` or truthiness conditions such as
  `datadom.attributes.collapsible`. An empty string is omitted by the former
  and false in the latter. Canonical components must explicitly test presence
  with `seq:count(...) > 0` where the attribute contract is presence-based.
  The native `host_boolean_attribute_presence_is_distinct_from_value_truthiness`
  fixture covers absent, empty, `"false"`, and `"true"` values. This explains
  the source-level mismatch behind disabled controls and missing navigation,
  selectable-list, checkable-chip, transient-dialog, and busy/empty branches.
  It does not establish that every later assertion will pass after migration.
- **Datepicker structure:** the failure screenshot shows a literal `}` after
  the authored input/toggle. In `primitives.ts`, the `datepicker-day` template's
  `cem:otherwise` closing sequence also closes the outer module. Its following
  `datepicker-week` and main `body` are consequently outside that module, and
  the final closing sequence contains an unmatched brace. All seven datepicker
  tests fail after roughly their 15-second deadlines; the JSON reporter supplies
  `STACK_TRACE_ERROR`, not a useful inner assertion. The brace defect is a
  concrete source finding; its contribution to all seven failures still needs
  a focused native render and browser check during owner migration.
- **Separate unresolved failures:** autocomplete payload replacement, expansion
  activation, paginator focus ownership, sort-header owner identity, and input
  paint need individual attribution. Do not
  classify these from test names alone or weaken their assertions.
- **Harness required-input fixture (resolved):** the isolated harness failure
  occurs when an empty input incorrectly passes native form validation. Its
  test-owned field template bound `required` directly to the host value, so
  the empty attribute was omitted on the native input. Switching that fixture
  to the already-proven presence expression restores the existing assertion.
  The browser fixture also checks absent, empty, `"false"`, and `"true"` values,
  live removal/reintroduction, native `validity.valueMissing`, and input identity.
  No production component, shared runtime behavior, or assertion was relaxed.
  The full `verify-phase3-harness` gate passes, including lint and typechecking.

Continue with the accepted `cem-action` declarative cutover. Site's v3 XHTML
asset delivery prerequisite is complete. Preserve this baseline when comparing
consumer tests, and migrate each remaining owner with its existing contracts.
The legacy behavior modules remain frozen; this investigation changes no
production semantics or test expectations.

## Failure inventory

This is the original 48-failure snapshot; resolved cases remain here as evidence.

Counts are per test file. Each entry records the first reported failure only;
assertions after it were not exercised successfully.

| Test file | Failed |
| --- | ---: |
| `src/lib/autocomplete.browser.spec.ts` | 2 |
| `src/lib/datepicker.browser.spec.ts` | 7 |
| `src/lib/divider.browser.spec.ts` | 1 |
| `src/lib/expansion.browser.spec.ts` | 4 |
| `src/lib/feedback-expanded.browser.spec.ts` | 7 |
| `src/lib/paginator.browser.spec.ts` | 4 |
| `src/lib/sort-header.browser.spec.ts` | 2 |
| `src/lib/states.browser.spec.ts` | 18 |
| `src/lib/workflows.browser.spec.ts` | 2 |
| `src/lib/testing/component-harness.browser.spec.ts` | 1 |

### autocomplete.browser.spec.ts

- **refreshes live payload without replacing focus, input identity, committed value, or events** — `Error: Timed out waiting for replacement option renders`
- **supports native migration and projects disabled, readonly, busy, required, and invalid states safely** — `AssertionError: expected false to be true // Object.is equality`

### datepicker.browser.spec.ts

- **keeps one direct native text input and optional native toggle as the exact owners** — `Error: STACK_TRACE_ERROR`
- **keeps canonical value, locale calendar, validation, form, and reset ownership on the input** — `Error: STACK_TRACE_ERROR`
- **moves one roving grid focus owner through day, week, month, and year navigation before confirmation** — `Error: STACK_TRACE_ERROR`
- **keeps pointer drafts and cancel, Escape, and backdrop dismissal silent until Apply commits** — `Error: STACK_TRACE_ERROR`
- **suppresses every disabled route and rejects malformed owner vocabulary without substitution** — `Error: STACK_TRACE_ERROR`
- **keeps today, selected, focus, hover, and disabled paint independent and geometry-stable** — `Error: STACK_TRACE_ERROR`
- **supports a silent expanded property while live bounds update validity and calendar suppression** — `Error: STACK_TRACE_ERROR`

### divider.browser.spec.ts

- **exposes exact semantic orientation and an explicit decorative boundary** — `AssertionError: expected null to be 'true' // Object.is equality`

### expansion.browser.spec.ts

- **exposes one exact heading, button, and persistent controlled-panel relationship** — `AssertionError: expected null to be 'region' // Object.is equality`
- **uses one native click path for pointer, Enter, and Space while preserving owner identity** — `Error: Timed out waiting for expansion state true`
- **keeps disabled activation suppressed while allowing silent programmatic state control** — `AssertionError: expected false to be true // Object.is equality`
- **keeps hover, focus-visible, and active paint on the header with stable transient geometry** — `Error: Timed out waiting for expansion state true`

### feedback-expanded.browser.spec.ts

- **uses native owners for transient initialization and live state transitions** — `Error: Expected fixture to contain dialog`
- **delegates modal focus, keyboard dismissal, return value, and restoration to the native dialog** — `Error: Expected fixture to contain dialog`
- **keeps focus-visible ownership on native dialog fallbacks and authored descendants** — `Error: Expected fixture to contain dialog`
- **preserves native fallback focus, modal boundaries, and state through cancel and close** — `Error: Expected fixture to contain dialog`
- **paints only focused native dialog fallbacks with the D5 zebra outline** — `Error: Expected fixture to contain dialog`
- **keeps a transient sheet non-modal, focus-neutral, and application-controlled** — `AssertionError: expected false to be true // Object.is equality`
- **preserves open-dialog identity and state while cleaning up close, replacement, and reconnect paths** — `Error: Expected fixture to contain dialog`

### paginator.browser.spec.ts

- **renders exact landmark, page-size, range, action, normalization, and optional-control semantics** — `AssertionError: expected [ 'previous', 'next' ] to deeply equal [ 'first', 'previous', 'next', 'last' ]`
- **navigates once through pointer, Enter, and Space with focus-stable suppressed boundaries** — `Error: Missing paginator last action`
- **keeps global disabled and live programmatic control silent while retaining surviving owners** — `AssertionError: expected false to be true // Object.is equality`
- **keeps hover, focus-visible, and active paint on native controls with stable transient geometry** — `AssertionError: expected <select …(6)>…(1)</select> to be <button type="button" …(8)>…(1)</button> // Object.is equality`

### sort-header.browser.spec.ts

- **renders exact column-header, native action, direction, and accessible-name ownership** — `AssertionError: expected false to be true // Object.is equality`
- **suppresses disabled activation and keeps programmatic state silent with stable owners** — `AssertionError: expected <button type="button" …(7)>…(2)</button> not to be <button type="button" …(7)>…(2)</button> // Object.is equality`

### states.browser.spec.ts

- **reflects action, loading, disabled, expanded, selected, and focus states on native controls** — `AssertionError: expected false to be true // Object.is equality`
- **applies shared native hover treatment without changing action geometry or semantics** — `AssertionError: expected false to be true // Object.is equality`
- **styles only navigation hover owners without changing current selection or component state** — `Error: Expected harness fixture to contain cem-nav[collapsible] > nav > .cem-nav__disclosure`
- **moves keyboard focus through navigation owners without changing selection or component state** — `Error: Expected harness fixture to contain cem-nav[label="Focus workspace navigation"] > nav > .cem-nav__disclosure`
- **applies navigation active treatment during trusted pointer and native keyboard activation** — `Error: Expected harness fixture to contain cem-nav[label="Active workspace navigation"] > nav > .cem-nav__disclosure`
- **applies shared native active treatment during pointer and keyboard activation** — `AssertionError: expected false to be true // Object.is equality`
- **composes tokenized input indicators across appearance, hover, focus, validation, and selection states** — `AssertionError: expected 30 to be less than or equal to 1`
- **moves keyboard focus through every enabled input indicator without changing component state** — `AssertionError: expected false to be true // Object.is equality`
- **projects explicit busy state across every input without taking over its lifecycle** — `AssertionError: expected null to be 'loading' // Object.is equality`
- **reflects form disabled, invalid, required, readonly, checked, and indeterminate states** — `AssertionError: expected false to be true // Object.is equality`
- **toggles checkable content chips without changing passive chip semantics** — `Error: Expected state render output matching cem-chip[checkable] button`
- **styles only interactive content hover owners without changing selection or component state** — `Error: Expected harness fixture to contain #interactive-content-list > select`
- **moves keyboard focus through interactive content owners without changing selection or checked state** — `Error: Expected harness fixture to contain #focus-selectable-list > select`
- **toggles collapsible navigation without changing passive landmark semantics** — `Error: Expected state render output matching cem-nav[collapsible] button`
- **selects declarative list options without changing passive list semantics** — `Error: Expected state render output matching cem-list[selectable] select`
- **marks explicit busy cards without making nested content primitives loading owners** — `AssertionError: expected null to be 'loading' // Object.is equality`
- **marks explicit empty workflow surfaces without inferring layout emptiness** — `AssertionError: expected null to be 'empty' // Object.is equality`
- **marks explicit busy workflow surfaces without making formatting containers loading owners** — `AssertionError: expected null to be 'loading' // Object.is equality`

### workflows.browser.spec.ts

- **renders the registration auth workflow with required, invalid, and loading states** — `AssertionError: expected false to be true // Object.is equality`
- **renders the password reset workflow with help, error, and loading feedback** — `AssertionError: expected false to be true // Object.is equality`

### component-harness.browser.spec.ts

- **proves action and field contracts through real CEM-ML substrate declarations** — `AssertionError: expected true to be false // Object.is equality`

## Canonical action cutover comparison

After moving action to canonical XHTML and colocated stories, the full legacy
suite still reports **82 passing / 47 failing / 129 total**, with exactly the
same failing test names and no skipped tests. Six canonical action stories pass
separately. Workflow fixtures explicitly register the canonical declaration;
legacy action unit assertions now live in its stories. This establishes no new
legacy-suite failures, while the existing owner repairs remain open.
