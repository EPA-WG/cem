# Namespace Rebinding Fixtures

Per `docs/cem-ml-ac.md` AC-P-10 / AC-P-V-1 and
`packages/cem_ml/src/schema/namespace.rs`. Each fixture exercises a
binding scenario covered by the `NsContext` scope-chain implementation
and the integration tests in
`packages/cem_ml/tests/namespace_rebinding_fixtures.rs`.

| Fixture                                | Scenario                                                                                  |
| -------------------------------------- | ----------------------------------------------------------------------------------------- |
| `default-html-svg-html.cem`            | Default-namespace rebinding round trip: HTML → SVG → HTML in one CEM-ML document.         |
| `default-html-svg-html.xml`            | XML parity rendering of the same document via `xmlns="..."` attributes (Phase 11 target). |
| `prefix-rebind.cem`                    | Repeated prefix bindings: outer `x = uri:outer`, inner `x = uri:inner`; outer restored on close. |
