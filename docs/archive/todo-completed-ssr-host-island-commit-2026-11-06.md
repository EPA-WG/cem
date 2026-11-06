# Shared native host attributes and canonical island commit — 2026-11-06

The one-inline-DCE SSR path now stages generated host writes, canonical
attributes, the derived dataset and the rendered children in one native
publication. `instance_island::host_commit::prepare_host_commit` uses the shared
canonical constructor and admission path, without HTML serialization/reparse,
JSON DOM transport or an SSR-specific slot/payload model. Its input must retain
the exact original host tree and node, even when another owner's strings match.

The existing renderer selects defaults and writes. Supplied empty strings win
over defaults. Selected writes replace existing values in place; new names
append in first encounter order. Repeated names coalesce to their final value.
The renderer now updates its semantic DOMStringMap view for same-pass `data-*`
reads, preserving native values for untouched entries. This view is an index;
canonical attributes remain authoritative, and canonical dataset is rebuilt and
validated from their final values. No second render is scheduled.

`HostAttributeUpdate` retains declaration/query source maps separately from its
existing name/value effect equality and transport fields. Native pure-cache
output also retains these maps. Older cached host-write records without source
provenance become misses; no map is fabricated on decode. Original compiled
owners remain retained. This changes the private native output-cache tuple,
not the public browser wire fields, command ABI or default feature profile.

`cem_ml::parser::tree_apply::apply_element` publishes one `CemElementEdit` with
attribute writes and child replacement. It rejects duplicate expanded write
names and retains input values, source/producer owners and operation memory
permits. Untouched attributes retain source identity/maps and order. Changed
attributes retain write-origin maps without invented physical author lines.
A late child or budget/control failure publishes neither half and releases
staged permits. `apply_children` remains the compatible no-attribute-write
wrapper. The generic applier does not take over HTML or island policy.

The shared island constructor's value helpers are extracted from preparation,
without changing first-connection capture behavior. Only canonical attributes
and dataset sections are replaced; other parts remain native values. Payload
references keep the exact original reference handles and source owners instead
of copying their authoritative subtrees. The staged candidate is rebound through
existing topology, alias, source, sensitive-input and dataset-equality admission.
Existing host `data-cem-render-scope` stays outside canonical input as before.

The shared host profile accepts source-mapped passive lower-case HTML writes.
Namespace changes, internal `data-cem-*` marker writes, active handlers, resource
requirements, reserved aliases, sensitive controls and unmapped writes are
refused. Writes exceeding 64 total host attributes or the shared byte/work
ceilings fail before application; caller policy may only reduce those limits.
This is bounded accounted work, not total-heap accounting of retained inputs.
Removal writes and generalized mutation/event/resource state are later work.

`AppliedInlineDce::committed_island()` exposes the committed template, context,
bindings and effective writes. `input()` retains original capture for provenance.
The derived page and committed native island have equal final host attributes
and dataset strings, while source text and original payload ownership remain
unchanged. The result still records one render pass as `Partial(Incomplete)`.
No complete revisioned settling coordinator, adoption stamps, ready proof,
production HTML writer, CLI lowering, publication or hydration is introduced.

Tests first reproduced six failures among eight host-commit cases and three
failures among seven generic AST cases. Additional failing regressions exposed
lost source maps in the native output cache and acceptance of a different host
owner with identical strings. The final native cases cover supplied empty
values, default/selected/repeated writes, ordering, same-pass reads, canonical
binding freshness, exact payload reference identity, maps, ownership, rollback,
limits and original cancellation/completed scopes. Pure-cache tests explicitly
check provenance, independently of name/value effect equality.

The browser observation fixtures now include five host-state cases and inspect
both public host attributes and canonical attribute/dataset sections. The
existing browser bridge reflects writes after capturing its processing snapshot;
four cases retain pre-write canonical state. This is a separate browser bridge
gap, recorded by explicit expected differences rather than claimed as full
parity. The new native transaction resolves it for SSR. Browser integration must
retain the processing revision and publication/rollback guards; merely taking a
fresh snapshot or adding a second render would not satisfy that contract.
The earlier browser slot-metadata/native-node difference remains recorded too.
Trusted runtime code runs; authored scripts and external requests do not.

Focused checks are `cem_ql:test --test=ssr_host_commit`,
`cem_ql:test:ssr-render:stripped`, `cem_ql:lint:ssr-render`, generic ML transaction
checks, private-cache tests and existing template/module-default regressions.
The browser gate consumes the explicit native observation sidecar and the final
rebuilt core through the existing built browser runtime. Verification retains
incremental/debug-symbol suppression, bounded Cargo jobs and skipped Nx cache to
avoid the earlier disk exhaustion. No full-workspace gate is claimed.

Exact checks, final artifact hashes and source hashes are retained in the
[verification receipt](ssr-host-island-commit-2026-11-06.json). Frozen presentation,
Site/UI/theme and npm packages are unchanged. Next: wire the browser host/island
transaction with revision and rollback checks, then shared retained-slot ingress,
broader payload/CSS/resource admission and bounded settling before adoption.

- [x] Native host commit **11/11** and generic AST transaction **7/7**, in both
      default and actual stripped profiles. Native output-cache cases **4/4**.
- [x] Module defaults **6/6**, template rendering **55/55**, retained rendering
      **17/17**, stripped capture **15/15** and preparation **13/13**; isolated
      component receiver **17/17** with actual component features.
- [x] Both native Clippy targets and focused formatting/JavaScript lint pass,
      with existing warnings outside the changed core modules.
- [x] Chromium **12/12** explicit observation contracts: 11 matching rendered
      outputs, 12 matching payloads, seven full matches, four recorded browser
      host/island-state differences and one recorded slot-ingress difference.
- [x] Final optimized default core **24,209,768 bytes**, SHA-256
      `89b773240f53c3d6fe279a125ceaa0e2669974fa3c89746857c23c99a7a9e03f`,
      below **26,214,400 bytes**. Named descriptors match (28 imports, 171 exports).
      Existing template-cache and retained declaration/instance CSS consumers pass
      on that final optimized binary.
- [x] Frozen presentation remains **17,321,262 bytes**, SHA-256
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.
