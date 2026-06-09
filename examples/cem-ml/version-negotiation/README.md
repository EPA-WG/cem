# Version-Negotiation Fixtures

Per `docs/cem-ml-ac.md` §AC-P-V-5 (and the document-format directive rules
of §AC-F-8). Each fixture exercises one outcome of **per-dispatched-namespace
version negotiation** for the core CEM-ML content type, driven by the
`packages/cem_ml/src/parser/format.rs` resolver and asserted by the
schema-machine integration tests in
`packages/cem_ml/tests/version_negotiation_fixtures.rs`.

These fixtures back fitness function **FF-2** (`tools/fitness/fitness-gates.json`).

| Fixture                       | `@doc` constraint | Expected outcome |
| ----------------------------- | ----------------- | ---------------- |
| `core-major-forgiving.cem`    | `cem-ml 1`        | Forgiving same-MAJOR load: binds embedded 1.0.0, records `cem.doc.version_resolved` (Info). Clean-parsing evidence fixture. |
| `core-major-unsupported.cem`  | `cem-ml 2`        | Strict reject — unsupported MAJOR: `cem.doc.version_unsupported` (Error), no format identity. |
| `core-future-minor.cem`       | `cem-ml 1.2`      | Strict reject — future MINOR within the supported MAJOR: `cem.doc.version_unsupported` (Error). |

## Scope

The embedded Tier A profile is **1.0.0** (`SUPPORTED_VERSION` in
`format.rs`). The "forgiving load (same MAJOR, higher MINOR)" axis of
AC-P-V-5 is demonstrated by the open-MAJOR constraint `1` binding the
embedded `1.0.0`; the "strict reject (unsupported MAJOR)" axis by `2`, with
`1.2` additionally pinning the future-minor reject.

At Tier A the only dispatched content type whose version is negotiated is
the core CEM-ML document format itself (the `@doc cem-ml <version>`
directive). Per-`@ns` schema-URI version negotiation for *other* dispatched
namespaces is future work; when it lands, add its fixtures here and extend
the integration test.
