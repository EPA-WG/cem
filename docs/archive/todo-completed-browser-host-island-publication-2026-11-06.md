# Browser host/island publication bridge — 2026-11-06

The browser now publishes generated host writes and canonical hydration,
attribute and dataset sections within its existing owned render transaction.
The earlier four host/island snapshot differences are resolved. Original island,
context-root and payload ownership remain, and host synchronization does not
schedule a recursive render or mint a new processing data revision.

The shared native `prepare_host_write_control` coalesces HTML names in first
encounter order, retains final values and computes DOMStringMap keys. SSR uses
the same coalescing under its stricter mapped-source/domain admission. Flat work
is bounded to the existing 16,384 input visits, 64 distinct writes and 1 MiB
lexical bytes, reduced by the caller's memory ceiling and checked under its
original operation. The same-pass renderer dataset view now folds HTML ASCII
case. The versioned primitive `hostAttributeCommit` control is exported beside
the unchanged name/value fields. It contains no AST, source owner or adoption
permission. Named WASM descriptors remain unchanged; the paired browser/core
build is required for host-writing results.

The browser validates and owns the command metadata, stages three canonical
sections through the existing writer and applies them with the host writes.
It checks token, connection, source attributes, canonical section ownership and
fields. Pending external source mutations invalidate the old frame, while native
CSS-context synchronization is excluded from recursive invalidation. Existing
native bindings overwritten by scalar host writes are retired on commit and
restored on rollback. Attribute/dataset dictionaries retain literal own keys.

The full-plan fallback uses the existing prepared patch reconciler for
host-writing results. Worker patches and authoritative patch recovery join the
same host publication. Retained CSS wraps the complete CSS/DOM publication,
including activation callbacks; unused patch/style preparations are cancelled if
an outer publication fails before delegation. Cleanup attempts all preparations,
records cleanup errors and keeps recovery required after uncertain publication.
A rejected patch or synchronous publication failure restores host values and
original metadata section nodes. Existing dirty CSS/DOM recovery remains
necessary when a publication has already changed other owned state; arbitrary
script reactions or external actions are not claimed reversible.

Successful publication refreshes current host/native binding fields through the
existing named browser snapshot codec. Data revision remains the one submitted
for processing, and recovery advances only the existing render-attempt field.
Resource/event work retains its existing admission order after patch success.
This increment does not replace the legacy browser slot/snapshot ingress model,
add an AST JSON transport, enable broader SSR domains or prove SSR adoption.
Native SSR output remains `Partial(Incomplete)`.

Tests first reproduced three native write-control failures, a separate folded
same-pass dataset regression and five publication-unit failures. A browser
observation failed on the old pre-write canonical snapshot. A further failing
CSS cleanup test exposed unused preparation handles after an outer publication
exception. Browser prototype-like-key observations use an explicit JSON string
export because the browser test transport normalizes object keys; no production
AST uses that sidecar. Captured product state and the native observation export
are compared independently.

The four browser profiles are full-plan fallback, fallback processing patches,
retained CSS fallback, and retained CSS with actual worker dispatch. Fixture-only
routing isolates the production publishers without introducing resource effects.
Failure interception occurs after native planning, then checks original metadata
nodes, original payload reference, unchanged output and preserved external input.
Recovery is bounded to the existing single/full retry behavior. A source edit
between preparation and publication proves stale input cannot replace the latest
frame. Worker/fallback tracing verifies the intended transport path.

`cem-elements:test:host-island-publication` runs native observations, actual
stripped native control cases, focused unit cases, four publication profiles and
the shared native/browser observation suite. It depends on the paired package
build, uses bounded Cargo settings, and emits named test-control JSON sidecars
under `dist/reports`. Independent build, targeted lint, native Clippy, stripped
SSR regressions, native cache and optimized WASM template/CSS consumers pass.
Exact code/artifact hashes are in the [verification receipt](browser-host-island-publication-2026-11-06.json).

- [x] Native write controls **4/4**, including actual stripped features; earlier
      host commit **11/11**, native observations **17/17**, stripped capture
      **15/15**, preparation **13/13**, retained rendering **17/17**, cache **4/4**.
- [x] Focused publication, CSS queue and processing-engine units **54/54**.
- [x] All four browser publication profiles pass, including rollback, bounded
      recovery, current revision, source churn and original payload identity.
- [x] Native/browser observation contracts **13/13**: 12 complete matches,
      13 payload matches, and one explicit retained-slot ingress difference.
- [x] Production package build/type checks, focused ESLint and native Clippy pass
      with existing warnings. Existing optimized WASM template and CSS consumers pass.
- [x] Optimized default WASM **24,193,342 bytes**, SHA-256
      `d4965a4b5d41bd9a9a25694c9a8e4cd1994a8877d54b8fde9a07b94f6c585fec`,
      under **26,214,400 bytes**, with matching named descriptors (28 imports,
      171 exports). No new WASM function ABI or default feature is introduced.
- [x] Frozen presentation checksum remains
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

No full-workspace, public deployment or general hydration acceptance is claimed.
Next: shared retained-node browser slot ingress, broader payload/CSS/resource
admission, then bounded nested settling and native completion proof.
