# Canonical CEM-ML source payload admission — 2026-11-07

The bounded native SSR profile now admits a direct unmarked
`template[type="text/cem-ml"]` as a source payload. Its entire original template
remains in the canonical payload under CEM-ML content-type/schema metadata.
The source is neither relabeled as HTML payload content nor replaced by a
serialized second AST. The optional `data-cem-schema` must identify the canonical
CEM-ML schema. Shorthand normalization, other source/schema profiles, foreign DOM
and instance CSS closure admission remain open.

The native import boundary exposes `retain_inline_cem_payload` and its closed
`RetainedCemPayloadSource`. It retains the original native HTML tree/template,
uses the existing mapped source handoff, and derives a namespace-aware native
CEM tree through the controlled tokenizer and schema machine. Namespace-ready
structural source admission does not evaluate references, load resources or
install CSS. Unbound or unadmitted namespaces are refused. HTML entity decoding
is applied once to the CEM source view. Authored maps are retained through the
handoff; decoded-buffer lines never become authored lines. Identical code in
two original templates has different source-selection identity.

Parsing happens explicitly during preparation. The shared receiver consumes an
already admitted native owner and never starts a source compiler from packet
metadata. Portable graphs with matching strings and provenance cannot supply
that original owner. The native `TemplateData` field stays outside authored
bindings and codecs. Host/island publication reuses the exact admitted AST only
when source tree, template node and source-admission operation match. A new
operation is refused before SSR evaluation. Source parsing is bounded to 32 KiB,
16,384 nodes and depth 64, reduced by explicit limits and the original operation's
stack/memory policy. Accounted memory survives while the last derived tree owner
is retained and is released on failure or final drop.

Tests first failed for the missing native import and source-owner contract.
Further native fixtures caught acceptance of an unbound lexical prefix; the
importer now refuses it rather than treating it as a namespace URI. Negative
cases cover malformed source, unknown namespaces, HTML child markup, lowered
input/node/depth and caller memory/stack bounds, cancellation, changed source or
operation, and portable graph imitation. Positive cases verify entity decoding,
source maps, source identities/parents, memory lifetime, and AST reuse through
an actual host write. A source expression containing division by zero stays
inert throughout the pass.

The shared browser observation export now visits inert template contents. Its
new canonical source case proves original source/island content parity without
claiming browser execution of the derived AST. Native observations use one
original operation across preparation and application.

`yarn nx run cem-elements:test:ssr-source-admission` depends on the paired build,
uses bounded Cargo settings, and runs the native import, receiver and SSR tests
in default and actual stripped profiles, plus the browser suites. An already
prepared paired build can use `--excludeTaskDependencies --skip-nx-cache`.

- [x] Source import: **3/3**, default and actual stripped.
- [x] Explicit receiver proof/graph refusal: **1/1**, default and stripped.
- [x] SSR/native regressions: **61/61**, each profile (2 retained slots, 3 source
      payloads, 11 host commit, 15 capture, 13 preparation, 17 rendering).
      Combined source gate: **130 native checks**.
- [x] Compiled component receiver regressions: **17/17** native cases.
- [x] Browser retained slots **4/4**, host/island publication **4/4**, shared
      native/browser observations **15/15 complete matches**, no divergences.
- [x] Paired production build, native Clippy, focused Rust formatting and ESLint,
      optimized WASM template cache and retained/instance stylesheet consumers pass.
- [x] Default optimized WASM: **24,015,219 bytes**, below **26,214,400**;
      SHA-256 `8b31d179d64f5a133b8ee54175554ce69655aa79a34199e7e5e6d7e24e1ce58f`.
      Named descriptors remain 28 imports/175 exports. No new JS/WASM function
      ABI or default feature is introduced; packaged browser/core hashes match.
- [x] Frozen presentation checksum remains
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

Exact code/artifact hashes and browser observations are in the
[verification receipt](ssr-cem-source-admission-2026-11-07.json).

This is canonical source admission, not general source execution or hydration
acceptance. A successful pass remains `Partial(Incomplete)`. No deployment,
full-workspace acceptance or component-WASM size claim is made. Next topics:
source ingress normalization and foreign/CSS closure admission; bounded nested
instance/resource coordination; native completion proof before export/adoption.
