# Native XDM handoff between transformation stages

The constructed CEM-to-XSLT and XSLT-to-XSLT regression initially failed with
`cem.xslt.bundle_argument`: constructed output was a flat native sequence,
including layout text, while XSLT requires one document focus. Explicit XML
encoding produces text and cannot supply node capabilities.

The shared CEM-QL adapter now retains constructed render values in an
`OwnedXdmDocument`. XSLT requests a bounded native document projection through
that owner. The existing typed in-memory value graph and retained XPath tree
supply the document; no serialized value artifact, XML export/import, JSON AST,
JavaScript parser or Studio-specific runtime path is involved. CEM query
bindings and JSON/YAML exporters keep their existing flat native value view.

An initial document wrapper exposed a second fault: absolute paths always used
arena node zero, which is an inaccessible sentinel for native forests. Validated
native trees now retain each node's semantic root index. Absolute paths and
ordering axes select that root in constant time. Its memory is included in the
native projection estimate. Empty and multiple constructed roots remain valid
children of one stage document.

The owner caches only its latest query-scope projection. Cache hits recheck
memory, values, depth, bytes and cancellation; source owners and memory permits
live as long as actual selected native nodes. Projection ownership retains raw
values rather than the cache-bearing wrapper, avoiding an owner cycle. Crossing
query scopes creates new projections; evicted projections survive only through
actual consumers. The fixtures cover stable current-scope identity, original
reference owners and source keys, exact source frames, typed attributes, weak
release and memory release.

Constructed document URIs remain absent. Existing source boundaries retain their
original identities without inventing missing line coordinates. XSLT lowering
now binds generated-template maps to the owning `#generated-cemt` URI. Bundle
reload validates present source boundaries, source IDs and source-range bounds;
generated offsets are never relabeled as original stylesheet offsets.

The ordinary **CEM transformation** project adds `build-xslt-report`: imported
XML → native CEM report model → XSLT HTML, alongside the existing JSON report
export. The existing project, resources, configurations, build selection and
shared preview components serve it; there is no dedicated example page or
production JavaScript addition. Its exact expected artifacts, independent
source/template edits, provenance and offline rebuilds have permanent native,
Node and Studio acceptance coverage. The optional schema-assets verifier also
uses a native two-stage XSLT graph online and offline.

## Completed checklist

- [x] Add failing native constructed CEM-to-XSLT and XSLT-to-XSLT graph fixtures with absolute XPath, namespaces, source provenance, repeated identity, owner lifetime/release, fresh-scope bounds/cancellation, semantic forest roots and encoded-text refusal.
- [x] Reuse the bounded native value/XPath ownership path for explicit stage documents, retaining original owners and cancellation/memory limits without serialized AST or implicit XML import.
- [x] Add a normal CEM transformation project configuration that feeds its constructed report model into XSLT; cover expected artifacts, parameter/data/template edits, provenance, independent copies and offline builds through the existing catalog acceptance.
- [x] Bind lowered XSLT output provenance to the owning generated CEMT URI; add reload/range and mismatched-boundary fixtures without relabeling generated offsets as original stylesheet coordinates.
- [x] Add CLI/WASM/Studio graph acceptance and docs, including Node command-service replay and optional-profile online/offline two-stage XSLT, rebuild consumers, verify credential-policy recovery, then commit and push.

## Verification

- Native XPath: 93 tests pass, including isolated logical document roots and
  ordering axes in a native forest.
- CEM-QL native ownership/value/XPath/XSLT cohort: 103 tests pass. The final
  XSLT cohort passes 50 tests after generated-source ownership validation.
- Owned XDM units: five tests pass in each of the default and external-schema
  native profiles; the retained pipeline fixture also passes without requesting
  a presentation tree.
- Native transformation graphs: three tests pass for CEM/XSLT producers,
  named/matched consumers, Unicode/namespaces, empty/multiple roots and encoded
  text refusal. Both original-owned XSLT parser-contract tests pass.
- CLI transformation/schema projects: all five tests pass, including exact new
  HTML bytes, complete JSON values, source maps, parameter/source/template
  changes, repairs and refusal without partial publication.
- Studio package lint and the changed Node/UI verification modules pass lint.

- Default and optional-schema WASM/Studio builds pass. The default browser and
  Node runtime ABI, all 658 integrity records and 32 schema packages verify.
- Node worker command-service cohort: 55 tests pass, including repeated native
  CEM-to-XSLT builds and a changed original XML source with exact HTML/JSON.
- Catalog audit passes with 22 authored projects, 247 configurations, 252
  documented workflows and 32 native schema packages.

A premature concurrent build/test launch caused generated staging collisions,
not a source or runtime failure. The original runtime packager was rerun from a
clean output directory, reusing the measured optimized WASM only after matching
its input/output SHA-256, pinned optimizer/version/options, byte-size bound and
public import/export descriptors. Both profiles and the subsequent Node/ABI
checks above pass against that clean package. Temporary reuse tooling was kept
outside the repository; no production build code or validation guard changed.
The default verifier also found the optional build's existing untracked
`feature-tour.schema-assets.cem` scratch graph in the strict ordinary generated
file set. Removing that scratch input restores the check without changing its
assertion or either deployment. A separate TODO tracks isolating optional
build inputs from the canonical generator domain.

- Focused Studio graph browser cases: 2/2 pass, covering both CEM project graph
  configurations and the existing XSLT project. Complete output values/bytes,
  parameters, data/templates, original invalid input refusal, native maps and
  independent project identities are checked.
- Full Feature Tour seed, original project bundles, native configuration
  execution and all 32 schema-package examples pass. Ordinary Studio output
  remains deterministic across 333 files.

- Full Studio transformation/schema browser target passes: both original graph
  projects, native CEM-to-XSLT handoff, rendered HTML, JSON table, source maps,
  data/template/parameter edits, draft and last-good-output guards, independent
  copies, all three schema-authoring projects and offline reload/build/repair.
- Optional-profile acceptance passes online and after cold offline reload:
  `nativeXdmHandoff: true` in both XSLT demand reports, formatter/colorizer and
  selected-resource processing, 205 package assets, four-runtime package replay,
  factory pins, independent source reimport and credential/stale/retirement
  refusals. Automatic recovery remains disabled.
- Deployed demo verification passes: 898 total/managed files and exact ownership
  hashes, all 333 Studio files, shared/isolated runtime galleries, native
  rendering/highlighting, Studio subpath/copy/HTML-CSS/offline workflows, ten
  unchanged theme CSS generators, 120 canonical playgrounds and the live tree
  gallery. The four deployed WASMs are two copies each of 24,076,144 and
  24,773,662 bytes, all below 25 MiB; no source capability was removed.

- Final isolated live Login recovery: worker and fallback both pass (2/2 cases,
  six unrelated cases skipped), each with three recoveries, twelve edits, two
  credential refusals and unknown-owner refusal. Individual and group checkpoints
  remain within 4 MiB; no failed recoveries, budget increases or timeout changes.

Recovery remains subject to the existing credential refusal policy:
entered passwords and one-time codes block recovery until explicitly cleared.
The 4 MiB group checkpoint budget, witness bounds and allocation/resume deadlines
are unchanged. Generated screenshots and Vitest attachments are excluded.
