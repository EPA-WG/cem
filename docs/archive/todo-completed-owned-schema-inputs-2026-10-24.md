# Completed owned native schema inputs — 2026-10-24

- [x] Migrate native schema/package and formatter/colorizer consumers from static embedded references to owned bootstrap/source inputs; qualify artifact caches by admitted source identity before excluding optional bodies from an opt-in WASM build. Keep the default complete release unchanged.
- [x] Add native regressions for dynamically owned package inputs, packed package consumer parity and source-qualified formatter cache reuse/refusal; migrate the shared package boundary and verify default catalogs remain complete.

## Shared native input boundary

`SchemaPackage` accepts owned manifest/schema inputs and checks their native
metadata against the selected bootstrap descriptor. Admitted inputs use shared
owned text and survive release of the original store. The complete default
adapter borrows compiled text without copying it. The old `BuiltinSchemaPackage`
name remains a compatibility alias; consumers use the shared package boundary.

`SchemaAssetPack::owned_package` returns independently owned, validated source
inputs. `conversion_reader` supplies original admitted formatter/colorizer
bytes, canonical source provenance and content types. It refuses missing sources
without embedded fallback. Authoritative package metadata does not validate
against an unrelated embedded template.

## Formatter/colorizer cache admission

Every output artifact read revalidates admission before cache lookup. Parsed
options are keyed by the complete artifact descriptor, source URI, content type
and SHA-256 of the actual bytes. Both legacy and authoritative readers can reuse
matching admitted options; changed sources and revoked admission cannot reuse a
descriptor's earlier body. Each pipeline cache holds at most 256 entries.

The stale-source regression failed before this repair. Additional tests cover
strict reader refusal after cache warming, URI/type/descriptor separation,
bounded revisions, dynamically owned input lifetime and packed formatter/
colorizer output/provenance parity with the complete default catalog.

## Verification and remaining scope

116 native tests passed:

- Seven pack-consumer/cache tests and six package-loader tests.
- Twelve asset-pack tests and two registry-assembly tests.
- 65 built-in conversion/formatter/colorizer cases and five CEM-QL tree
  inspection integration tests.
- Seven pack-consumer/cache and twelve asset-pack tests with
  `--no-default-features --features schema-asset-packs`.

Both profiles verify the complete 32-package, 200-source pinned catalog. The
profiling-only tree-render fixture also compiled; its ignored case is not counted
as an executed test.

The 22-task Studio build passed, including native CLI, both browser/WASM packages
and generated catalogs. Scoped JavaScript lint reported no errors (existing
non-null assertion warnings remain). The rebuilt outputs also passed the
14-page source/package import cohort and all eight shared gallery-page stories,
including persisted theme updates and independent example themes. Five focused
theme-storage recovery and entered-credential refusal stories also passed.
All 13 shared demo-element lifecycle/behavior stories passed. The gallery repair
and its 62 processing-host/checkpoint-domain unit checks are recorded in the
[gallery completion record](todo-completed-gallery-imports-2026-10-24.md).

This changes shared native consumers, not application UI. Studio remains
declarative; entered credentials still block recovery, the aggregate checkpoint
budget remains 4 MiB and automatic recovery is not enabled by this work.
Shared runtime acquisition, deployment/service-worker inventory, fresh-generation
reimport and opt-in embedded-body exclusion remain open. Both current profiles
retain embedded bodies; no binary, allocator, linear-memory or checkpoint savings
are claimed.
