# Browser adoption of canonical SSR islands — 2026-11-09

Base: `5c0ccc0820911a859e32e06e35c05b10db8eac8e`.

The readiness-checked CLI HTML export marks its canonical island templates with
`data-cem-ssr-profile="cem-native-inline/1"` through the shared HTML writer.
This marker routes browser admission. It confers no reuse authority and leaves
the retained native island and report readiness fields unchanged.

The browser imports the actual host, island and retained range through the shared
native DOM capture and CEMV boundary. Literal namespace bindings are scoped to
their DOM ancestors and preserve native source identity. Native admission checks
the canonical schema, host inputs, passive profile, range and declaration
version. Compatible versions use a fresh native projection to verify the live
range. Missing, malformed or incompatible declaration versions render using the
admitted island payload. Unsupported or inconsistent islands stay frozen.

The browser rechecks DOM freshness, declaration identity, connection and
instance identity before publishing the decision. Adoption keeps the rendered
nodes, starts the usual lifecycle and supports later payload invalidation. The
retained root records declaration fidelity without inventing source coordinates.

Reproduce the CLI-to-Chromium fixture with
`yarn nx run cem-elements:test:ssr-canonical-hydration`. It builds the native CLI
and the browser runtime, serves the CLI output unchanged, and checks node
identity, reconnect and payload invalidation, version fallback, unknown schema
and profile refusal, tampered or stale ranges, duplicate identity, and zero
external requests. It also resumes the resulting browser snapshot through the
existing unmarked path.

Native verification: 6 canonical admission, 3 namespace view, 5 DOM import,
122 stripped SSR, 17 component-runtime, 6 adapter integration, 3 export proof
unit, and 4 CLI integration checks. Clippy and browser package lint passed.

Final paired package: 9/9 Chromium scenarios with zero external requests and
4/4 existing browser hydration stories. Checked template cache, retained CSS
and instance CSS WASM consumer regressions passed.
The source-admission gate passed 312 native checks and 30 Chromium cases:
4 slot profiles, 4 host publication profiles, 19 native/browser render
observations and 3 passive pages, with no divergences.
The optimized core and its browser vendor copy match: 24,049,225 bytes below
the 26,214,400-byte limit, with 28 imports and 177 exports. Core SHA-256:
`01a36538b1cc4c9d9c05f8254cf7d5dbc1ef2769a9a3474fb8a091b7f37efd03`.
Frozen presentation SHA-256 remains
`bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

Scope: default browser policies, passive CEM-ML in light DOM and HTML payloads.
Nested participants, scoped CSS, foreign markup, CEM source payloads, live
slices and form controls still need end-to-end parity. CLI host resource and
source admission, including acquisition and parser CPU accounting, remains open.
Browser WASM does not issue or reconstruct a server CPU proof.
