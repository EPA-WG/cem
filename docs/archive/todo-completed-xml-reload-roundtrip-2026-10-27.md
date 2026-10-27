# Native XML reload-bundle roundtrip — 2026-10-27

- [x] Reproduce unstable namespace/schema snapshot bytes with failing native fixtures before changing serialization.
- [x] Cover fresh hash maps, inherited/shadowed/deferred namespaces, authored declaration order and legacy wire-map decoding.
- [x] Cover byte-exact XML export/reload and passive attachment, fresh owners, original payload/source provenance and rejection of altered metadata.
- [x] Fix the shared native serialization boundary without changing runtime lookup behavior, wire shape, protocol versions or the existing WASM assertion.
- [x] Rebuild optimized runtimes and verify native/CLI/WASM consumers, Studio XML workflows, credential refusal and saved/offline demo delivery within both deployment budgets.

## Cause and correction

`NsContext.active` and `inherited_overrides` are runtime hash maps. Their derived
serialization emitted entries in hash iteration order. Decoding reconstructed
those maps with fresh hash state, so unchanged XML lexical snapshots produced
different bundle bytes on re-export. `SchemaScopeFrame.declared_inlines` and
`inherited_inlines` had the same issue, including when snapshots were compared
during passive attachment and typed-prelude validation.

The private shared `schema::serialization::serialize_string_map` borrows and
sorts entries by key only while serializing those four dictionaries. Runtime
hash lookup, authored declaration vectors, binding IDs, shadowing and deferred
namespace semantics are unchanged. Historical unordered map encodings still
decode; subsequent exports normalize dictionary ordering. There is no wire-shape
or protocol-version change, AST payload rewrite, source reparse, transported
runtime context or new authority grant.

The serializer is in `cem-ml`; CEM-QL, CLI and WASM consumers reuse it. No Studio
or component JavaScript, parser substitute, cached-byte workaround or component
declaration change is involved.

## Verification

Both new native deterministic-snapshot tests fail before the fix and pass after
it. Final focused native coverage is **113 passing checks**:

- Namespace and schema unit modules: **18** cases, including legacy unordered
  wire-map decoding, retained parent values, deferred prefixes and declaration order.
- Bundle/checkpoint/typed-prelude integration: **70** cases across
  `checkpoint_runtime_state`, `reference_reload_bundle`,
  `typed_prelude_transport` and `typed_preludes`.
- CEM-QL retained transport/lifecycle/attachment: **21** cases across
  `reference_transport_api`, `reference_lifecycle_api`,
  `reference_reload_attachment` and `lexical_scope_handoff`.
- CLI transport/reload validation: **4** cases.

The unchanged `packages/cem_ql/tests/reference-transport-wasm.mjs`, including
its strict byte equality at line 105, passes against both an isolated copy of
the new unoptimized engine and the regular optimized build. Its separate worker
heaps retain XML slots/entities, independent contexts, explicit crossing grants
and disposal guards. The complete **12/12 CEM-QL WASM boundary scripts** pass,
including checkpoints, native constraints/values, generation ownership, caches,
stylesheet lifetimes, memory/headroom and the 30-case URL matrix.

Browser/Node package tests pass **10/10**; verification retains **657 integrity
records and all 32 schema packages**. Studio's focused XML/XHTML source/query,
repair and native HTML build cases pass **2/2**. Shared checkpoint acceptance
passes **Empty Credential Controls And Entered Value Refusal**, exercising both
worker and fallback. Entered password/OTP controls still block recovery until
explicitly cleared; the existing deadlines, 4 MiB checkpoint budget and disabled
automatic whole-Studio recovery remain unchanged.

The complete demo builds and publishes through the ordinary shared pipeline.
Deployed acceptance passes: all **333 Studio files** match their build bytes,
saved-copy navigation and native HTML/CSS previews work, offline reload/rebuild
matches online output, service-worker scope stays under `/studio/`, all ten
theme generators emit unchanged CSS, all 120 canonical playground URLs respond,
the live tree renders and independent factory memories remain separate.

## Deployment inventory

**898 files, 114,563,382 bytes (109.26 MiB)**, including the inventory. The four
WASM files remain below 25 MiB with all capabilities enabled:

| Runtime, two copies each | Bytes | MiB | SHA-256 |
| --- | ---: | ---: | --- |
| CEM-QL | 23,481,853 | 22.39 | `e4f923cbb544a1dd3cba73025b74526398a7a7e34172eed2e121e714cd685df3` |
| CEM-ML bridge | 24,131,701 | 23.01 | `2bdde146f4364fd45e5ca8ab074a48612960631f4a519ad985f40e0153b6b87f` |

Optimized artifacts preserve **169/170 exports and 28 imports per module**.
Native release/LTO, pinned Binaryen 132.0.0 `-Oz`, default `debug-control`, full
embedded schemas, independent factories and integrity-before-publication remain
in place. Generated binaries, reports, screenshots and Vitest attachments are
excluded from the commit.

The initial Nx invocation failed to process the project graph; its verbose
retry succeeded without configuration changes. The unrelated aggregate Studio
fallback dynamic-module fetch failure from the earlier full cohort remains in
[todo.md](../todo.md). This task's focused acceptance does not claim that
aggregate issue is repaired or that the entire source cohort was rerun.
