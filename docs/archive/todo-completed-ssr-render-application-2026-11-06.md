# One retained shared render-plan application — 2026-11-06

`cem_ql::ssr_render::render_and_apply_inline_dce` now consumes a prepared native
island and evaluates its retained template with the existing shared renderer and
bindings. Existing element-reference ID projection and native reference
projection feed the common `cem_ml::parser::tree_apply::apply_children`
transaction. No external parser walk, HTML serialization/reparse, JSON DOM,
second evaluator or application-specific payload renderer is introduced.

The derived native CEM page replaces the admitted host's children with the
canonical marked inert island, `cem-render-start`, rendered output, and
`cem-render-end`. Empty output preserves the island and both boundaries.
Original source/page/declaration owners, original payload query identities and
parents, envelope identity, maps, namespaces and deployed base/document metadata
remain retained. Output materialization is a derived view; the original payload
remains authoritative. Inserted qualified names survive native handoff, and
generated nodes acquire no fabricated author lines. The original HTML owner
retains source syntax including doctype; a production HTML serializer is still
required before publishing a page from this AST.

The generic transaction admits already projected common native values, refusing
unexpanded references or source structured attribute values. It preserves
untouched decoded strings without double escaping. Original-to-output node IDs
are available for retained source nodes outside the replaced children. Native
producer/source owners and memory permits stay with the derived tree, including
after the outer application object is dropped. Cancellation/completed-scope
codes and the caller's deadline remain authoritative. The whole derived page is
bounded to 16 MiB, 65,536 nodes and depth 64, reduced by caller policy; projection
retains the shared 1 MiB/16,384-value/depth-64 limits. These accounted storage
bounds are not total-heap accounting of already-retained input owners.

The checked SSR artifact transition requires the exact original HTML owner,
matching source/page/base identity and exactly one additional render pass. It
uses the existing native extension-artifact channel and stays
`Partial(Incomplete)` without hydration readiness. It does not introduce a ready
constructor, completion proof, settle loop, CLI intrinsic, wire codec or export.

The generated-output profile accepts passive HTML, text and comments. Active
handlers, script/style/document nodes, foreign or nested custom participants,
resource-policy requirements, structured attribute contracts and invalid void
children fail before AST application. Shared native payload source and sensitive
input admission is reused. Host writes are explicitly refused with
`cem.ssr.host_updates_unavailable`: canonical attributes/dataset and outer host
attributes need one shared commit before these writes are admitted.

A failing source-text regression exposed a native decoder limitation outside the
replaced host. `RetainedElementImport::validate_html_text_projection` now checks
that profile at the import boundary before render/application. Unsupported named
references, HTML numeric replacement cases and literal attribute carriage returns
fail with `cem.ssr.source_projection_unsupported`, rather than silently changing
untouched content. Earlier lossless import/capture remains available. This
conservative text admission does not claim full WHATWG tree construction.

Four generic native transaction cases and eight initial DCE application cases
failed before implementation. Expanded tests cover exact owners, decoded text,
source maps, slot projection, empty output, namespace/QName retention, native
extension handoff, invalid progress, physical-output-independent rendering,
admission, cancellation, caller ceilings and lifetime memory accounting.

Seven explicit observation fixtures run through both native application and the
existing browser runtime with the rebuilt default core WASM. Six rendered cases
match; all seven payload observations match. One fixture records the existing
browser direct-slot-node difference: its bridge supplies metadata records and
refuses record insertion with `cem.ql.render.expression_type`, while the native
shared island binds retained nodes. The difference remains visible in the report
and has a dedicated follow-up task. Semantic output observations omit only
internal render stamps, namespace-declaration spelling and zero-length browser
text placeholders; they do not claim equal runtime node identity or hydration
stamps. Trusted runtime code executes, authored scripts do not, and external
requests are blocked. JSON is an explicit test-observation sidecar.

Focused Nx targets are `cem_ql:test --test=ssr_render`,
`cem_ql:test:ssr-render:stripped`, `cem_ql:lint:ssr-render` and
`cem_ql:test:ssr-render:browser`. The browser gate requires the native observation
export, built `cem-elements` runtime and rebuilt default core. Core transaction
checks use `cem_ml:test --lib --args=tree_apply` and actual
`cem_ml:test:stripped --args='--lib tree_apply'`. Existing template-cache and
retained stylesheet WASM consumers remain regression gates.

Exact results and final code/artifact hashes are recorded in the
[verification receipt](ssr-render-application-2026-11-06.json). Builds retain
`CARGO_INCREMENTAL=0`, disabled development/test debug symbols, bounded Cargo
jobs and `--skip-nx-cache`. No full-workspace gate is claimed. The default core
feature profile and 25 MiB size limit remain unchanged. Frozen presentation,
public Site/UI, theme/CSS and npm packages are not repackaged or deployed.

Next: implement the shared host/island attribute commit, then admit shared native
browser slot ingress. Additional payload/CSS/resource domains and bounded nested
settling follow before completion proof, serialization, CLI publication and
hydration can be enabled.

- [x] Native DCE application **17/17** in default and actual stripped profiles;
      generic AST transaction **4/4** in each profile.
- [x] Earlier capture **15/15**, preparation **13/13**, native artifact **14/14**,
      renderer **28/28** and import-boundary **2/2** regressions pass. Stripped
      capture/preparation also pass **15/15** and **13/13**.
- [x] Chromium **7/7** observation contracts: six matching rendered cases,
      seven matching payloads, one explicitly recorded slot-ingress difference.
- [x] Focused formatting/JavaScript lint and both native Clippy targets pass;
      existing warnings are outside the changed core modules.
- [x] Optimized default core WASM **24,220,012 bytes**, SHA-256
      `5236de4ec19180c945730112e9be4811a569f22a35b94b6fd6bf8e664a7d8ea7`,
      below **26,214,400 bytes**. Named public descriptors match (28 imports,
      171 exports); three function imports change order, with matching generated
      glue. Existing template-cache and retained stylesheet consumers pass again
      on the final optimized binary.
- [x] Frozen presentation stays **17,321,262 bytes**, SHA-256
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.
