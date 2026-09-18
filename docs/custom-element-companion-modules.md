# `@epa-wg/custom-element` Companion Modules And Resource Primitives

Updated 2026-09-17: standalone HTTP is retired under the accepted
[CEM loader plan](cem-data-loader-plan.md).

This records the Phase 3.6 policy for the browser modules that ship with
`@epa-wg/custom-element` beside `custom-element.js`.

It follows the package baseline in
[`custom-element-package-baseline.md`](custom-element-package-baseline.md), the
adapter boundary in
[`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md), and the
bridge policy in
[`custom-element-bridge-template-policy.md`](custom-element-bridge-template-policy.md).

## Decision

Preserve the remaining companion module files and registrations. HTTP is the
explicit exception: remove its standalone module and root export, and migrate
consumers to the `cem-elements` CEM document loader.

The HTTP resource primitive now has host policy, lifecycle, native import, owner
retention and executable fixtures. Remaining standalone companion shims keep
their existing compatibility contracts.

## Module Policy

| Module | Next-major status | Runtime policy |
| --- | --- | --- |
| `module-url.js` | Keep as shipped browser file and side-effect registration. | Substrate-backed when rendered inside declarations: inert `<module-url slice src>` helpers are removed from light-DOM output, resolved through `CemElementRuntimeOptions.resolveModuleUrl`, and exposed under `datadom.slices.<slice>`. The standalone `module-url` custom element remains a compatibility shim. |
| `http-request.js` | Retired. | Remove standalone imports and `HttpRequestElement`. Migrate templates/examples to `cem-elements` with CEM-ML declarations and explicit XPath libraries. HTTP response bytes become retained CEM nodes; JavaScript snapshots contain metadata with `data: null`. See the [loader contract](cem-data-loader-plan.md). |
| `local-storage.js` | Keep as published companion shim. | Do not promote the current global `localStorage` monkeypatch into the render substrate. Migration templates may wire the element's `change` event explicitly. A future primitive must define same-tab/cross-tab update policy, value coercion, storage-denied behavior, and edge/SSR non-availability diagnostics. |
| `location-element.js` | Keep as published companion shim. | Do not promote the current `history`/`location` monkeypatch into the render substrate. Migration templates may wire the element's `change` event explicitly. A future primitive must define host-window policy, navigation mutation permissions, live update sources, and non-browser behavior. |
| Demo-only resource helpers | Migrate or drop per fixture need. | Demo resources are examples, not package API. Keep only those needed for package-local verification or migration documentation. |

`index.js` keeps the following re-exports after HTTP retirement:

- re-export `custom-element.js`;
- re-export `local-storage.js`;
- re-export `location-element.js`;
- do not add `module-url.js` to the root barrel unless the publish-readiness pass
  intentionally changes the package API.

## Compatibility Rules

The next-major package should preserve side-effect definitions:

- importing `local-storage.js` defines `local-storage`;
- importing `location-element.js` defines `location-element`;
- importing `module-url.js` defines `module-url`.

The companion shims may continue exposing a `value` property and dispatching
`change` for browser consumers. That compatibility does not imply automatic
resource-to-slice behavior inside `CemElementRuntime`. Automatic slice writes must
be either:

- the existing substrate-backed `module-url` path; or
- explicit rendered-element event wiring through `slice`, `slice-event`, and
  `slice-value`; or
- a future substrate primitive with its own fixture coverage.

## Future Primitive Requirements

A browser resource primitive must be accepted only after these questions are
answered in docs and fixtures:

- how the resource is keyed in render-plan identity and edge cache identity;
- which host policy hook authorizes the side effect;
- what data shape is written into `DataIslandSnapshot.slices`;
- which fields are allowed, omitted, or redacted by browser-to-edge export policy;
- how asynchronous completion, aborts, stale responses, and diagnostics are
  ordered against render revisions;
- how the primitive behaves in browser, worker, edge, SSR, and non-browser hosts;
- how tests wait on completion without timing sleeps.

## Migration Notes

- `module-url` templates should prefer the substrate resource form already covered
  by material parity fixtures.
- Existing `local-storage` and `location-element` demos should be
  rewritten as migration examples with explicit slice-event wiring before they are
  used as package-local adapter fixtures.
- Legacy demos that depend on XSLT `for-each`, broad XPath, or implicit
  resource-to-slice behavior are not acceptance fixtures until those dependencies
  are migrated.
- The shims stay browser-only. Edge/SSR snapshots must not serialize live
  `Response`, `Storage`, `Location`, `History`, `Window`, or DOM objects.

## Implementation TODO

- Keep baseline smoke tests for import side effects and default/named exports of
  each companion file.
- Add one `module-url` adapter fixture that proves rendered helpers use
  `resolveModuleUrl` and do not remain in output.
- Add explicit-event migration fixtures for `local-storage` and
  `location-element` only after the adapter fixture harness exists.
- Add diagnostics for unsupported implicit resource-slice patterns if legacy demos
  are loaded through the next-major adapter.
- Keep the HTTP retirement guard and migrated source/dist/packed examples in the
  package verification gate.
