# Native CEM-ML ingress normalization — 2026-11-08

Native SSR now accepts the `cem-ml` source shorthand, ASCII case variants and
browser `String.trim` whitespace around either admitted CEM-ML type. Native
bindings and saved islands expose `text/cem-ml`; the original authored template
and its source text retain their spelling. U+0085, MIME parameters and unrelated
content types are not admitted as CEM-ML.

The shared native import boundary owns type classification. It derives the CEM
source AST from the original HTML owner as before. Canonical input reuses its
source tree. An alias uses the new generic `apply_attributes` native CEM edit to
replace only the derived envelope's type while retaining its content, source
keys, maps and parent provenance. The edit uses the existing whole-page limits
and caller operation; the tighter source parser limits remain independent. No
HTML serialization/reparse, JavaScript AST encoder or JSON DOM is introduced.

`RetainedCemPayloadSource` retains both authored and canonical views. The receiver
requires the exact canonical tree/node and original operation, rather than
accepting an equivalent tree or matching provenance strings. The canonical view
is used before template evaluation and host publication, so a native slot query
also sees the normalized type. The authored source remains the authority and
its derived AST is still not serialized into the canonical island. Source
compilation stays in explicit preparation; compiled packet receivers obtain no
implicit source compiler. Source expressions remain inert and SSR remains
`Partial(Incomplete)`.

A failing native fixture first established the missing attribute-only edit.
Layered cases verify child retention, original-owner immutability, original
source keys/maps and parent relationships, lowered bounds and cleanup, shorthand
and case/Unicode whitespace variants, canonical no-op identity, and refusal of
forged canonical views and portable graphs. SSR cases verify the actual bound
canonical view and original authored type after host publication. Two additional
shared native/browser observations compare shorthand and case/whitespace ingress
with the existing canonical source case. The mixed-case browser case first failed
because exact-string ingress unwrapped the source envelope as HTML. The shared
`cem-elements` ingress predicate now matches the native case/trim normalization;
first connection keeps the envelope inert and stores its canonical type.

The existing `cem-elements:test:ssr-source-admission` gate now includes the
attribute-edit suite in default and actual stripped profiles. With a prepared
paired build it runs with `--excludeTaskDependencies --skip-nx-cache`.

- [x] Native source cases **4/4**, attribute-edit cases **9/9**, receiver proof
      cases **2/2**, and SSR/native cases **62/62**, in each feature profile:
      **154 native checks** in the complete gate.
- [x] Native compiled component receiver regressions **17/17**.
- [x] Retained-slot browser profiles **4/4**, publication profiles **4/4**,
      shared native/browser observations **17/17 complete matches**, no divergences.
- [x] Paired production build, native Clippy, focused formatting and ESLint, and
      existing optimized WASM template/CSS consumers pass.
- [x] Optimized default WASM: 24,027,304 bytes, SHA-256 `cc2a4ab6b1ffb4858fd3e7adb1602daaef953d61fcee4eb7827335c8ef787495`;
      below the 26,214,400-byte per-file ceiling with 28 imports/175 exports.
      No named JS/WASM ABI or default feature change; packaged assets match.
- [x] Frozen presentation checksum remains
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

Exact source/artifact hashes and observations are in the
[verification receipt](ssr-cem-ingress-normalization-2026-11-08.json).

Foreign payloads and retained instance CSS still need explicit closure admission.
Nested instance/resource coordination follows, then native completion proof
before CLI/export and hydration adoption. No deployment, full-workspace
acceptance, component-WASM size claim or general hydration acceptance is made.
