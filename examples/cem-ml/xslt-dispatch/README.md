# XSLT-Dispatch Fixtures

Per `docs/cem-ml-ac.md` §AC-P-6.8 / §AC-P-V-4 / §AC-P-V-7. Each fixture embeds an
`xsl:`-namespace region inside CEM-ML and exercises **XSLT region dispatch**:
the version-stable XSLT namespace is dispatchable as an embedded content type,
version-pinned from `xsl:stylesheet/@version` (not the URI, not the CEM-ML core
version), isolated from CEM-ML interpretation, and **explicit opt-in**.

Driven by `packages/cem_ml/src/schema/xslt.rs` (decision-core) applied in
`packages/cem_ml/src/schema/machine.rs` (`with_xslt_dispatch` opt-in +
`emit_xslt_dispatch` + the region-isolation guard), asserted by
`packages/cem_ml/tests/xslt_dispatch_fixtures.rs`.

| Fixture                         | Opt-in | Expected outcome |
| ------------------------------- | ------ | ---------------- |
| `embedded-xslt.cem`             | on     | `cem.handoff.xslt_dispatched` (Info), version-pinned to XSLT 1.0; the `xsl:template` descendant is isolated (no `cem.schema.unresolved_namespace`). |
| `embedded-xslt.cem`             | off    | AC-P-V-7 default: the `xsl:` region falls to the AC-P-6.7 unknown-namespace disposition → `cem.schema.unresolved_namespace` (reject, application run). |
| `embedded-xslt-no-version.cem`  | on     | `cem.xslt.version_invalid` (Error) — dispatch cannot version-pin without `@version`. |

## Scope

AC-P-6.8 dispatch / isolation / version-pinning are independent of which XSLT
versions the engine can **execute** (AC-P-6.9 — a deferred Tier-C wishlist). The
opt-in here is the `with_xslt_dispatch` machine flag standing in for
host-provided namespace metadata / a scope-policy rule. Tier A realizes the
dispatch as a diagnostic and isolates the subtree via the region-depth guard;
the full Layer-5 child-parser handoff (opaque-text materialization) lands with
the other content-type handoffs in Phase 11.
