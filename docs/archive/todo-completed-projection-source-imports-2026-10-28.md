# Native projection-source imports — 2026-10-28

Complete the native DOM/AST/event projection source-adapter item using the shared
CEM import boundary. No application UI behavior or JavaScript component was added.

## Implemented contract

- Six explicit JSON/binary MIME identities load original sources into retained
  common CEM trees. DOM/AST retain semantic structure; events become ordered
  literal records, including expression and typed-prelude fields. This is not
  executable event replay or restoration of captured runtime state.
- Original bytes and native JSON owners stay retained. Reported source maps,
  ranges and schema/runtime IDs confer no ownership. Schema IDs are discarded;
  references retain expression text with cleared targets/context. Source ranges
  address actual JSON objects, binary records or envelope hex strings.
- Binary v1/v2 admission validates complete records, tags, booleans, UTF-8,
  checked lengths/ranges/counts, IDs, owning edges and exact consumption. Refuse
  truncation, trailing bytes, cyclic/shared/dangling/unreachable nodes and
  semantic error nodes. Bare binary has structural validation, not a checksum.
- Sealed JSON envelopes require payload bytes, matching BLAKE3 hashes, contiguous
  canonical record chunks and matching root/parent/child identities. Reported
  source-map deltas remain descriptive. Metadata-only envelopes are refused.
- Limits remain 16 MiB, 65,536 nodes/JSON values and 64 levels, with existing
  smaller string-import limits. JSON limits now apply during native parsing;
  this does not enable JSON-to-XML's separate codepoint compatibility profile.
- Lifecycle loading, validation, Parse/Inspect, data import and CEM/XML/HTML or
  named DOM/AST JSON/binary exports use the retained owner and native writers.
  No serialized AST handoff, fallback CEM tokenization or foreign URI grant is
  introduced. Namespace aliases are generated for passive exports, including
  expression records; exporting a namespace does not grant an application schema.
- Attribute value-node edges participate in native sealed chunk ownership.
- Ordinary `native-projections` 1.0.4 adds `parse-dom-input`, `inspect-ast-input`,
  `inspect-events-input`, `import-dom-html`, `import-ast-xml` and
  `import-events-cem`. Existing declarative editor/result/provenance controls are
  reused. Documentation and the capability/workflow inventory are updated.
- Studio binary sources remain read-only/non-runnable with exact downloads.
  Their owned command admission remains the next task. Entered Login passwords
  and one-time codes still block recovery until cleared; automatic recovery is
  disabled. Preserve 4 MiB checkpoints, 32,768-node witnesses and all deadlines.

## Verification

- New native projection import/integrity/ownership tests: 12/12.
- Projection export filter: 33/33, including the 17/17 registered-validator cohort.
- Native command-service filter: 70/70; import-boundary guards: 2/2.
- Native conversion filter: 161/161, excluding the previously recorded
  `conversion_output_pipeline_reads_cemt_artifacts_through_environment_reader`
  baseline failure; its required freshness reads were not weakened.
- Original native JSON tests: 2/2; CSV import contract tests: 4/4; document-limit,
  string-import and JSON-to-XML integration tests: 14/14.
- Final-code CLI tests: 550/550. Six new ordinary configurations also run
  directly through the CLI with clean diagnostics and expected output text.
- Catalog unit/audit tests: 48/48; authored Studio lint passes.
- Public Node/browser WASM tests: 28/28, including all six input identities,
  Parse/Inspect/conversion, source ownership and malformed binary refusals.
  Distribution verification passes with 657 integrity records and 32 packages.
- All 12 CEM-QL WASM scripts pass.
- Focused Studio workbench browser tests: 32/32; expanded native-project browser
  acceptance: 1/1, including actual JSON input provenance and binary refusals.
- Release build and inventory-based deployment: 898 files, all four WASMs below
  25 MiB. CEM-QL: 23,982,306 bytes per copy; CEM-ML: 24,659,811 bytes per copy.

- Complete native catalog: 277 configurations, 241 successes and 36 intentional
  diagnostics; zero known-limit/companion/policy exceptions. Catalog identity:
  `c180b09527f28220fc865c00ce6119651e5b0ce0e860ceadea040e18f5ba65c3`.
- Deployed navigation: catalog operations, repairs, exact binary transfers,
  original diagnostic selection, saved Unicode/range invalidation, independent
  copies, retained repairs and all-family offline exports/transfers pass.
  New JSON source workflows follow actual artifact ranges in each saved copy.
- Demo verification: all owned hashes, all 333 Studio files, independent saved
  HTML/CSS preview and offline rebuilds pass. All 10 theme generators have
  unchanged CSS; all 120 canonical playground pages and the live tree gallery pass.
- Isolated complete live Login recovery: 2/2 worker/fallback cases pass, retaining
  credential refusals, repeated recovery and editing under unchanged bounds.

## Separate baseline and remaining work

The broader lifecycle cohort exposes the existing
`builtins_load_versioned_module_maps_through_json_syntax_adapter` failure for v1.
It reproduces on unchanged `32144d1a` in a temporary clean checkout, removed
afterward. Record its fixture update separately; preserve version-specific
native semantic checks. A zero-test probe affected by a shared Cargo target cache
was discarded; verified source checks were rebuilt from clean native artifacts.

Owned binary Studio commands, direct configuration URL text admission and the
existing formatter artifact-reader trace remain separate TODOs. No executable
normalized-event replay or captured-owner deserialization is claimed.

## Completed checklist and principles audit

- [x] Add failing native six-identity, truncation/trailing/tag/count/graph,
  sealed-hash/topology, namespace, original-owner and passive-reference fixtures.
- [x] Decode once at shared native import and wire lifecycle, source operations,
  data import and native writers without serialized AST handoffs or runtime grants.
- [x] Verify ordinary JSON configurations, read-only binary refusals, public WASM,
  native catalog, saved/offline copies, demo inventory and isolated Login privacy.

The source/semantic boundary stays native. Studio uses existing production
CEM components and declarative result/source bindings. No component or application
UI JavaScript was added. No theme tokens, credential rules, recovery budgets,
witness bounds or timeouts were changed to obtain these results.
