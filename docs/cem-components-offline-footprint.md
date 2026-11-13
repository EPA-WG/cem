# Delivery and offline cached-body footprint

The all-catalog benchmark preload delivers and caches 26.80 MiB. The existing
default Studio caches 53.80 MiB of response bodies; the optional schema-assets
Studio caches 63.04 MiB. These are explicit asset scopes and decoded response
body totals, not browser disk allocation, native memory or a minimum runtime.

The [original per-file/group evidence](cem-components-offline-footprint.json)
records actual CacheStorage entries, offline response verification, artifact
hashes, cache/deployment identities and source-pinned measurement tooling.

| Scope | Delivered files | Delivery MiB | Per-file gzip estimate MiB | Cached entries | Cached body MiB | Unique offline URLs | Unique content MiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| All 59 canonical components, benchmark preload | 223 | 26.80 | 7.84 | 223 | 26.80 | 223 | 26.74 |
| Default Studio | 350 | 53.75 | 16.05 | 351 | 53.80 | 348 | 53.71 |
| Schema-assets Studio | 555 | 63.00 | 16.55 | 556 | 63.04 | 553 | 35.14 |

One URL cached in two groups counts twice in cached body bytes, once in unique
URL bytes. Different URLs with identical SHA-256 content count once only in the
unique-content diagnostic. That diagnostic is hypothetical content deduplication,
not evidence of physical browser storage sharing or permission to merge native
instances, source leases or profile authorities. The gzip column sums individual
level-9 compressed files; actual HTTP verification serves uncompressed bodies.
It does not measure CDN wire traffic, headers or archive compression.

## Capability and artifact scope

The full-catalog scope uses the existing isolated benchmark assembler: all 59
canonical source declarations, built `cem-elements` runtime, component/theme CSS
and the full default renderer. Every declared managed file, including the
deployment inventory, is preloaded into a temporary audit cache. This measures
a conservative preload of this browser asset set; it is not an npm release
package, a production PWA policy or an upper bound for arbitrary applications.
Dynamic/external documents, optional command services and user projects require
their own scope and closure checks. Clean npm loading remains the next gate.

Studio measurements use the actual built deployments, production service workers
and their shell/runtime/samples cache inventories. The optional profile also uses
its schema-asset integrity inventory and all 205 immutable pack assets. It retains
32 schema catalog packages with five explicit external-package grants: CSV,
Markdown, SCSS, XSLT and Relax NG. Inventory presence and successful byte reads
do not grant package execution or prove every format's semantics; existing native
package admission, privacy, resolver and recovery checks remain authoritative.

Both Studio trees include 57 of the 59 canonical declaration filenames. They
omit standalone `cem-fieldset` and `cem-source-editor` declaration files; this is
why their body totals are kept separate from the all-catalog preload. This is a
file-inventory observation, not a claim that an active Studio control is missing.
Both trees deliver two files outside their offline inventory:
`assets/@epa-wg/cem-theme/package.json` and `fonts/LICENSE.material-icons.txt`.
Their sizes remain included in delivery totals and listed as uncached paths.

| Native artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Full renderer, catalog/default Studio | 24,476,684 | `3fc8939ca8eef749b01e9a582dc642cf098ee7d7b0ef61275036f9d6d11e2f76` |
| Default Studio command runtime | 25,669,508 | `a0b3ad9a0d044b6ffc234682735ce43abce8d7105182d9de45713397b1b31d91` |
| Optional Studio renderer and command runtime | 29,172,552 each | `b435cecc91a600b9d86569c06d793ccbb92d39081dfeed3a77090d8e254747f3` |

The optional deployment stores identical 29,172,552-byte native bodies at two
distinct renderer/command URLs. This explains most of the difference between
its cached-body and unique-content totals. Investigate immutable-code/asset
aliasing separately while preserving each binding/profile and independent heap;
no loader, cache policy or deployment artifact is changed by this audit.

## Measurement and verification

For each profile, an isolated browser context registers its worker from a thin
audit probe. Default/optional Studio use their actual workers. The full-catalog
case uses a cache-only audit worker because the library has no corresponding
production PWA inventory. The probe and this synthetic worker are harness
overhead excluded from the artifact/body totals. No application UI is booted,
no credentials or saved projects are used, and no user deployment is overwritten.

The collector independently expands built control metadata, refuses out-of-scope
or missing files, verifies optional pack/index hashes, and records every file's
bytes/hash and per-cache membership. It reads every actual cached body and
compares it with the build. It then disables network access, blocks the local
HTTP server and fetches every distinct offline URL through the controlled page.
All 223/348/553 offline URL hashes match; no offline request reaches the server.
Cache entries and byte counts match the independently expanded plans exactly.

Online artifact bodies served during fresh installation total 26.80/53.88/63.24
MiB for catalog/default/optional respectively. These include repeated metadata
reads and cache population, but not a complete application startup or first-use
session. The report distinguishes these served bodies from installed cache
copies, unique URLs, content hashes and total delivered files.

Five focused tests verify alias/cross-group/content accounting, actual-body
mismatch refusal, missing/external/escaped resources, optional pack/index pins,
canonical declaration completeness and original inventory drift. Installation
has a bounded audit deadline. All three real cache/offline verification scopes
pass. This complements the earlier [readiness/memory observations](cem-components-browser-readiness-timing.md)
and [optional profile functional verification](cem-components-wasm-code-sharing.md);
it does not rerun every component/format UI, provide physical disk use, prove
native link closure or select fixed performance/memory budgets.

```bash
yarn nx run cem_ql:measure:offline-footprint
```

Use current matched `cem-elements`/full-renderer outputs plus already built
ordinary and schema-assets Studio trees. The target stages its own full-catalog
benchmark and refuses stale package/core cohorts. It writes the complete local
record to `dist/reports/components-wasm/offline-footprint.json` and the committed
evidence path above. No Rust rebuild is required for this byte audit.

Next verify clean npm consumers: package asset/import closure, browser worker and
fallback first use, and combined memory with the new readiness metric. Fixed
budgets, extracted-module loading, arbitrary external resources and whole-app
capability conformance remain separate gates.
