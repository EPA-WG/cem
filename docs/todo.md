# Todo

This is the authoritative checklist for unfinished execution work. Product/module
sequencing lives in [roadmap.md](../roadmap.md); wishlist work lives in
[wishlist.md](wishlist.md).

Completed work and verification evidence are preserved in the
[2026-10-23 snapshot](archive/todo-snapshot-2026-10-23.md), including the native
schema-asset prototype, Studio examples, editor, previews and recovery work.
Earlier history remains linked from that snapshot. Historical checkbox states
do not reopen completed work; adopted contracts remain in their design documents.

Prioritize Studio functionality and presentation samples before unrelated helper
work and physical-device/browser/AT acceptance. Examples remain ordinary projects
with resources and configurations in the same workspace, without dedicated
example screens. Follow the [Studio milestone contract](cem-studio.md#next-delivery-milestone-user-owned-project-workspace).

Keep UI declarative in `cem-components` and Studio CEM-ML; reusable JavaScript
behavior belongs in `cem-elements`, and application JavaScript owns non-UI services
only. Alert the user before introducing a new shared JavaScript UI capability.
Apply the [canonical CEM theme principles](../packages/cem-theme/src/lib/tokens/index.md#core-cem-principles),
including inherited token ownership, accessibility and theme survivability.
Entered passwords or one-time codes continue to block recovery until explicitly
cleared by the user; automatic whole-Studio recovery remains disabled. Follow the
[credential privacy policy](committed-checkpoints.md#credential-privacy-policy-for-live-login-recovery).

- [x] Make local demo Studio links bypass stale installed-worker assets without clearing projects or drafts. [Implementation and verification](archive/todo-completed-demo-preview-refresh-2026-10-28.md).
  Local links and redirects select an inventory-based Studio revision on the
  same origin. Hosting, stale-cache/draft retention and complete demo acceptance
  pass; port 8081 is restarted with the new handler.

- [x] Clarify shared select labels and simplify Studio's output-tree presentation. [Implementation and verification](archive/todo-completed-studio-output-navigation-2026-10-28.md).
  Shared field captions use canonical tag typography and related spacing.
  Declarative output navigation hides redundant roots while preserving nested
  folders, full paths and existing URL/selection identities.

- [x] Add project-home README links and minimum sample file guides. [Implementation and verification](archive/todo-completed-studio-readme-navigation-2026-10-28.md).
  All 22 authored samples, the 32-type showcase and starter have short purpose
  paragraphs and linked essential files. Shared routed anchors and passive
  Markdown previews select known project resources, retaining builds, drafts
  and read-only originals online/offline. Catalog audits, host-view and navigation
  tests, all 23 catalog README views, demo verification and worker/fallback Login
  recovery pass. Credential policy and recovery budgets/deadlines are unchanged.

Completed [compact Studio layout and shared runtime work](archive/todo-completed-compact-studio-2026-10-23.md) is archived. The [font-relative tree spacing follow-up](archive/todo-completed-compact-studio-2026-10-23.md#font-relative-tree-spacing-follow-up) records the IDE reference refinement. The [page-level super-compact follow-up](archive/todo-completed-compact-studio-2026-10-23.md#page-level-super-compact-theme-follow-up) records header controls, mode boundaries and verification.

The [explicit content-sized super-compact policy](archive/todo-completed-compact-studio-2026-10-23.md#explicit-content-sized-super-compact-policy) supersedes its earlier pointer-gated geometry.

Completed [editor filename de-duplication](archive/todo-completed-compact-studio-2026-10-23.md#studio-editor-filename-follow-up) covers editable files and read-only sample/output views.

Completed [graph parameter-map authoring and Studio controls](archive/todo-completed-graph-parameters-2026-10-23.md) is archived.

Shared package transports, project/resource-driven Studio demand, opt-in
embedded-body exclusion, isolated profile measurements, editor responsiveness and
trusted optional-format adoption are complete. Prioritize the functional
content-type and schema samples below.

Completed [shared popup surfaces and indicator clearance](archive/todo-completed-compact-studio-2026-10-23.md#shared-popup-surfaces-and-indicator-clearance) is archived.

Completed [immediate persisted theme updates](archive/todo-completed-compact-studio-2026-10-23.md#immediate-persisted-theme-updates) covers storage notification independence and nested button value capture, including the earlier repeated Studio theme-change failure.

## Project builds and artifact previews

Completed 2026-10-27: [shared collection manifests](archive/todo-completed-collection-manifests-2026-10-27.md)
unifies native CLI/common publication with the established JSON/YAML/XML writers.
Explicit bytes, application data, typed owners, inline provenance and CLI sidecars
retain their contracts; common hosts add no implicit child or sidecar writes.
Focused native, Node/browser WASM, Studio worker, credential and saved/offline demo
checks pass. Deployment remains 898 files; WASMs are 22.43/23.05 MiB. The separate
formatter-profile discrepancy and module-asset conformance are completed below.

Completed 2026-10-27: [native XML reload-bundle roundtrip](archive/todo-completed-xml-reload-roundtrip-2026-10-27.md)
orders passive namespace/schema dictionaries only during native serialization,
preserving runtime lookup, declaration order, legacy decoding and authority.
All 113 focused native checks and 12 WASM boundary scripts pass, including the
unchanged byte-equality assertion. Studio XML, privacy and saved/offline demo
checks pass; deployment remains 898 files with four WASMs at 22.39/23.01 MiB.

Completed 2026-10-26: [complete-runtime WASM size budget](archive/todo-completed-wasm-size-budget-2026-10-26.md)
pins post-bindgen Binaryen `-Oz`, retains all exports/capabilities and embedded
schemas, and publishes 898 files with four WASM assets at 22.33/22.95 MiB.
Strict size/count guards refuse before deployment mutation. Measured and regular
build hashes match; deployed Studio, offline builds, galleries, themes and
credential privacy checks pass. The archive distinguishes the source cohort's
273/274 result and passing fallback rerun from the separate pre-existing XML
byte-roundtrip defect. The XML defect is repaired above; the aggregate module-fetch
follow-up remains below.

Completed 2026-10-26: [demo deployment file budget and owned cleanup](archive/todo-completed-demo-file-budget-2026-10-26.md)
reduces `cem-bin` from 1,142 to 898 files, including its deterministic SHA-256
inventory. Cleanup preserves modified obsolete and unrelated files; the 999-file
preflight limit includes them. Clean, migrated and repeated copies match.
Studio's complete bytes, offline builds, native gallery factories, all ten theme
generators, canonical playgrounds and credential refusal pass.

Completed 2026-10-26: [Studio in the assembled demo site](archive/todo-completed-demo-site-studio-2026-10-26.md)
adds Studio to `build:demo`, the home-page links and `/studio/`. Complete native
deployment bytes, scoped headers/service worker, gallery factory ownership,
native preview and saved/offline builds pass. `serve:demo` provides the local
site and `verify:demo` checks the assembled deployment without clearing user data.

Completed [Studio accordion delivery check](archive/todo-completed-compact-studio-2026-10-23.md#studio-accordion-delivery-check-2026-10-24)
confirms the layout source is unchanged, restores the stopped local preview, and
verifies styled accordion/collapse behavior across a stale-worker reload with
saved projects and unsaved drafts retained.

Completed [canonical source viewer repair](archive/todo-completed-canonical-source-2026-10-24.md) covers explicit XHTML endpoints, source/package viewer checks and the complete nine-gallery compact Studio gate.

Completed [gallery lifecycle and checkpoint-limit repairs](archive/todo-completed-gallery-imports-2026-10-24.md) covers delayed connection callbacks, inventory refusal versus physical retirement, and the targeted source/package import gate.

The optional native schema-asset prototype and
[owned package/formatter consumers](archive/todo-completed-owned-schema-inputs-2026-10-24.md)
are complete; the default release retains its complete embedded bodies.
The [shared acquisition and offline delivery contract](archive/todo-completed-schema-acquisition-2026-10-24.md)
is implemented, including persistent activation, coordinated host integration,
opt-in deployment and body exclusion. Measure delivery and runtime costs before
claiming binary, headroom or checkpoint savings. Prototype implementation,
measurements and verification are in the
[archived evidence](archive/todo-snapshot-2026-10-23.md#project-builds-and-artifact-previews).

Completed [native owned-package activation](archive/todo-completed-native-package-activation-2026-10-24.md) covers atomic publication, source ownership, ordinary engine consumers and strict body admission. Completed WASM/service wiring and declarative demand are recorded below.

Completed [native CSV parser contracts](archive/todo-completed-csv-parser-contracts-2026-10-24.md) enable owned CSV package activation and ordinary import/validation with checked fact bindings, original source ownership and separate structural-validation readiness.

Completed [persistent WASM package activation and declarative demand](archive/todo-completed-persistent-wasm-packages-2026-10-24.md) covers native owner inventory, ordinary command/import contexts, explicit admission versus full resource-validation phases, existing HTTP-resource binding and distinct-heap recompilation. No native owners or retirement authority cross status/checkpoint records.

Completed [opt-in Studio schema-asset deployment](archive/todo-completed-studio-schema-profile-2026-10-24.md)
covers native profile emission, authored URL expectations, both factory pins,
complete offline delivery, cache isolation and independent source preparation.
Explicit host admission and coordinated publication remain required; the normal
bootstrap does not activate packages implicitly.

Completed [shared package resource handoff and owned invocation transport](archive/todo-completed-package-resource-handoff-2026-10-24.md)
covers original physical identities, checked release, fresh compilation, complete
binding, policy/credential refusals, cleanup and consecutive offline handoffs.

Completed [command-service worker package transport](archive/todo-completed-command-worker-packages-2026-10-24.md)
covers pinned original-byte delivery, exact worker ownership, trusted startup
grants, asynchronous checked release, native empty-inventory confirmation,
consecutive offline handoffs, credential refusal and late/unknown-outcome guards.


Completed [rendering-worker and owned-fallback package adapters](archive/todo-completed-rendering-package-adapters-2026-10-24.md)
covers exact physical bindings, original configuration and pool/root isolation,
checked release, fresh publication, pressure/revocation guards and consecutive
offline handoffs with native imports and template compilation.


Completed [shared Studio package demand](archive/todo-completed-studio-package-demand-2026-10-24.md)
covers explicit declarative demand across invocation, command, rendering and owned
helper heaps; lazy preview admission; complete original recovery cohorts;
single-pass fresh import; closed-runtime cleanup; and offline Studio acceptance.
The default complete deployment, credential refusal and disabled automatic
whole-Studio recovery are preserved.

Completed [opt-in embedded source-body exclusion](archive/todo-completed-external-schema-bodies-2026-10-24.md)
covers five optional packages, a complete native identity catalog, missing-source
and alias refusal, deferred conversion registration, actual binary-body omission,
and online/offline Studio formatter/recovery acceptance. The default deployment
and credential policy remain unchanged; gzip delivery is currently larger.

Completed [profile delivery, latency and native memory measurements](archive/todo-completed-schema-profile-measurements-2026-10-24.md)
cover 30 fresh-browser samples and the [recorded method/results](schema-asset-profile-measurements.md).
The optional profile adds delivery/first-use cost and CSV capacity despite saving
256 KiB of initial/paint linear capacity. Keep the complete default; the demand
selection follow-up is complete below.

Completed [project/resource-driven optional package demand](archive/todo-completed-resource-package-demand-2026-10-24.md)
covers native metadata planning, effective export selection, selected-resource
admission, bounded shared preparation, complete physical publication despite
cancellation/host closure, offline acceptance and 30 fresh-browser measurements.
HTML startup downloads no optional packages; the first dependent command pays
activation. The frozen-baseline editor timeouts are addressed below.

Completed [shared host-view settlement](archive/todo-completed-host-view-settlement-2026-10-24.md)
covers the plaintext draft profile, bounded DOM-order discovery, asynchronous
parent payloads, inserted/replaced owners and disposal. Both original desktop
editor cases and all 140 Studio workbench/navigation/recovery tests pass with
unchanged deadlines, focus/selection/scroll assertions and credential policy.

Completed [optional Studio format demand and owned source Parse](archive/todo-completed-optional-format-demand-2026-10-24.md)
covers trusted grants for Markdown, SCSS, XSLT and Relax NG, retained native parser
contracts, strict typed-source Parse, source provenance and online/offline
activation/recovery acceptance. The default retains its complete embedded catalog
and the credential policy stays unchanged; the optional profile's supported
operation limits stay explicit.

## Studio samples: supported content types and schemas

Add an Examples subfolder named **Content types and schemas**, organized into
Data, Documents and UI, Queries and transformations, and Schema authoring.
Reuse the existing manifest-generated **Schema examples** resources and authored
projects where possible. Entries remain ordinary cloneable projects with resources
and configurations; collection folders are discovery metadata. Prioritize these
functional samples before unrelated diagnostic/helper acceptance.

Completed [content-type/schema capability inventory and checked sample mappings](archive/todo-completed-content-type-inventory-2026-10-24.md)
are archived. Use the [audited operation and sample map](studio-content-types.md)
when adding the folders and functional projects below; registered aliases alone
do not establish operation support.

Completed [nested catalog folders and contextual project discovery](archive/todo-completed-content-type-catalog-2026-10-24.md)
are archived. Content types and schemas now reuses existing ordinary projects;
continue with the missing functional workflows below.

Completed [JSON/YAML report workflows and shared import repair](archive/todo-completed-json-yaml-report-2026-10-24.md)
cover native configurations, three report artifacts, intentional syntax failure
and repair, shared previews, independent saved copies and offline rebuilding.
Source validation remains distinct from application-schema instance validation.

Completed [CSV inventory, portable validation configurations and owned string imports](archive/todo-completed-csv-inventory-2026-10-24.md)
cover native header/quote diagnostics and repairs, typed HTML/JSON reports, shared
previews, independent saved copies and offline saved header-policy execution.
Both default and optional deployments pass the CSV workflow; standard header-mode
and row-width diagnostics retain their warning severity.

Completed [XML/XHTML workflows and query namespace context](archive/todo-completed-xml-xhtml-2026-10-24.md)
cover Supplier validation/selection/repair, the ordinary XHTML document project,
shared retained import, explicit query identities and saved/offline preview acceptance.

Completed [Markdown article/site workflows and passive native import](archive/todo-completed-markdown-site-2026-10-25.md)
cover article inspection/query/preview, typed metadata repair, complete site assets,
native source-range navigation and independent saved/offline copies. Authored HTML
policy, shared declarative controls and credential privacy remain unchanged.

Completed [HTML/CEM card and shared component workflows](archive/todo-completed-html-cem-card-2026-10-25.md)
cover semantic comparison, production card/slot composition, native HTML builds,
source inspection and policy repair, saved/offline previews, explicit worker
Inspect formats and shared projected slice-event ownership. Existing Login
contracts, declaration authority and credential privacy remain unchanged.

Completed [SVG/MathML projects and shared foreign-content previews](archive/todo-completed-svg-mathml-2026-10-25.md)
cover retained family imports, native inspection/query/formatting, namespace repair,
exact HTML entries and provenance, real vector/formula rendering and independent
saved/offline copies. Source validation and bounded preview admission remain
separate; credential privacy and automatic recovery policies are unchanged.

Completed [CSS/SCSS/selector projects and native stylesheet artifacts](archive/todo-completed-stylesheet-projects-2026-10-25.md)
cover typed CSS graph export, original CSS/SCSS provenance, scoped browser styles,
module/variable/nesting expansion, native HTML/XML selection and source repairs.
Saved edits, independent copies and offline reruns pass with shared controls and
unchanged resolver authority, source guards and credential policy.

Completed [CEM-QL and XPath data projects and original query-source provenance](archive/todo-completed-data-query-projects-2026-10-25.md)
cover typed filtering/joins, declared and data-derived parameter maps, native
selected-node ranges, source operations where supported, exact repairs and JSON
table artifacts. Saved edits, independent copies and offline runs pass through
shared controls. Unknown query identities refuse navigation; resolver authority,
saved projects/drafts and credential privacy remain unchanged.

Completed [CEM/XSLT transformations and schema-authoring projects](archive/todo-completed-transformation-schema-projects-2026-10-25.md)
cover five ordinary projects, native multi-step HTML/JSON artifacts and provenance,
CEM schema/package instance validation, JSON Schema/RELAX NG source authoring and
eight diagnostic repairs. Shared source editing now admits their registered text
types while compiled binaries remain read-only. Saved edits, independent copies
and offline runs pass; source/instance limits, resolver authority, saved drafts
and credential privacy remain explicit and unchanged.

Completed [module maps and portable configurations](archive/todo-completed-module-config-projects-2026-10-25.md)
cover two ordinary projects and 28 configurations, native v1/v2/v3 deployments,
actual source/output asset hashes, command/run-plan inspection, Studio JSON/CEM
codecs and guarded source archives. Saved edits, independent imports/copies and
offline rebuilds pass with shared controls. Native source-operation limits,
read authority, saved projects/drafts and credential privacy remain explicit.

Completed [native DOM/AST/event projection projects and shared validation](archive/todo-completed-native-projection-projects-2026-10-25.md)
cover 15 resources and 19 configurations, native CEM-ML inspection, exact explicit
exports, original source ranges and four diagnostic repairs. Shared native
validation, catalog byte admission and editor publication/terminal-newline fixes
pass worker, deployed, saved-copy and offline checks. Native diagnostic selection
uses admitted source buffers; missing inspection-provenance publication, AST JSON
compatibility and binary input/replay remain explicit native follow-ups.
Credential privacy and existing saved projects/drafts are preserved.

Completed [catalog README and declared capability audit](archive/todo-completed-catalog-readme-capability-audit-2026-10-25.md)
covers all 22 authored projects, generated Schema examples/Feature Tour and the
starter, 203 authored configurations and 208 documented workflows. Deterministic
reference/identity checks, worker/native fixtures and deployed readonly, saved-copy
and offline README/draft checks pass. Registered selector gaps and native limits
remain explicit; no new UI behavior or credential recovery authority is added.

Completed 2026-10-26: [full catalog acceptance](archive/todo-completed-full-catalog-acceptance-2026-10-25.md)
accounts for all 23 projects, 565 resources and 240 configurations with qualified
native/worker receipts, independent copies, actual builds/previews and offline
coverage. Generic native Markdown now renders and preserves tables; the reported
CEM-QL README preview passes online/offline. Populated-workspace controls and the
combined long navigation run pass without changing admission limits, saved
projects/drafts or credential policy. Native source-operation limits and
concurrent release scheduling remain explicit follow-ups below.

## Native and release follow-ups

Completed 2026-10-27: [native module-asset attribute conformance](archive/todo-completed-native-module-attributes-2026-10-27.md)
corrects a stale fixture to read existing native document metadata through
`input.kind`, retaining the original exact HTML assertion. Authored attributes,
metadata and explicit parameters stay distinct. Native owner/source-span checks,
all eight CLI conformance cases, 14 WASM cases, five selected Studio worker cases,
credential refusal, full package integrity and saved/offline demo gates pass.
Production runtime and shared declarative UI behavior are unchanged.

- [x] Add shared native module-attribute fixtures and native owner/immutability
      checks for authored, changed and absent attributes, explicit parameters,
      imported public templates, secondary assets, exact bytes and provenance
      across native CLI, Node/browser WASM and Studio command workers.

Investigated 2026-10-27: [Studio module-load diagnostics](studio-module-load-diagnostics.md)
adds opt-in script/network/frame metadata capture through public test APIs and a
controlled failed-import/redaction/teardown fixture. Two complete current source
cohorts pass **275/275**, including recovery and credential privacy: observed with
the normal development cache, then unobserved with a separate fresh cache. The
historical fetch failure remains unattributed; its conditional recurrence task
is retained under the Studio startup watch below. Original assertions/deadlines,
4 MiB limits, shared declarative UI and disabled automatic recovery are preserved.

- [x] Add and verify the controlled HTTP-failure/fresh-iframe diagnostic fixture
      for original import-error propagation, metadata redaction, frame ownership,
      recorder teardown and absence of retries; keep it distinct from reproducing
      the historical Studio failure.

These remain explicit follow-ups after the functional Studio sample catalog.
Keep the original admission, provenance, privacy and verification bounds.

Completed 2026-10-26: [native CSS-selector source export and shared editor paint](archive/todo-completed-css-selector-source-export-2026-10-26.md)
covers typed formatter/colorizer ownership, exact lexical source, native ranges,
hard-diagnostic refusal, shared worker/fallback coloring and ordinary Studio
format/color configurations with saved-copy and offline acceptance. Cross-query
provenance and the complete selector-package gate are covered by the completed
follow-up below.

Completed 2026-10-26: [typed XPath Parse, Inspect and source export](archive/todo-completed-xpath-source-operations-2026-10-26.md)
covers native syntax/token projections, declared shared fields and monotonic
events, borrowed lexical formatter/colorizer ownership, original source ranges
and hard-diagnostic refusal. Five ordinary Studio source configurations pass
shared WASM and deployed saved/offline checks. Caller namespace bindings are
covered by the follow-up below; credential recovery refusal and disabled
automatic recovery stay unchanged.

Completed 2026-10-26: [caller namespaces for XPath source operations](archive/todo-completed-xpath-caller-namespaces-2026-10-26.md)
covers named/default static context on native owners, original ranges and hard
refusals, CLI and structured input scopes, preserved authored validation argv and
selected-command execution. The ordinary XPath project passes namespace-command
repair, independent-copy and offline checks without changing shared controls or
credential privacy policy.

Completed 2026-10-26: [cross-query original-source provenance](archive/todo-completed-cross-query-provenance-2026-10-26.md)
covers shared retained CEM input views for CSS-selector, native tree map accessors
for all three query languages, original owner/URI/range retention, literal input
profiles and import guards. The unchanged cross-language equality assertion and
complete selector-package gate pass. Rebuilt WASM, ordinary Studio commands,
Login map equality, saved/offline selector workflows and credential refusal pass
without changing shared declarative UI or enabling automatic recovery.

Completed 2026-10-26: [shared abstract JSON graph byte policy](archive/todo-completed-json-graph-byte-policy-2026-10-26.md)
covers one native UTF-8/two-space/LF encoder, explicit writer-byte priority,
complete application JSON values and native collection-role separation. Exact
native/worker bytes, large integers, hashes, pinned snapshots and Supplier 1.1.1
fixtures pass. All 250 catalog configurations pass online/offline; shared Studio
saved-copy/preview workflows and credential refusal pass. Native collection
manifest publication is now shared across hosts, as recorded above.

Completed 2026-10-26: [SCSS mixin default-argument binding](archive/todo-completed-scss-mixin-defaults-2026-10-26.md)
covers captured module variables and earlier parameters, explicit caller-scope
arguments, aliases/forwarding/repeated calls and original Unicode/CRLF ranges.
SCSS stylesheet 1.0.1 now uses the omitted default with unchanged exact CSS.
Native, CLI, worker, saved/offline preview and credential refusal checks pass;
full lexical/function/content-block compatibility remains outside this profile.

Completed 2026-10-26: [file-URI graph module-resolution bases](archive/todo-completed-file-uri-graph-bases-2026-10-26.md)
covers relative file-URI imports/dependencies, localhost-prefix grants, encoded
names, bounded local globs and safe rooted-folder bindings. SCSS stylesheet
1.0.2 documents native builds without an extra base URI; missing module grants
still refuse publication and original source URI/ranges remain intact. Native,
CLI, rebuilt WASM/worker, saved/offline preview and credential refusal checks
pass. Plain-path configs retain their existing identity behavior.

Completed 2026-10-27: [lossless mixed-quote CEM-ML inspection](archive/todo-completed-mixed-quote-inspection-2026-10-27.md)
admits line-fenced literal attributes through the shared native tokenizer/writer,
with collision-free fences, exact LF/CRLF payloads, original UTF-8 ranges and
literal ownership. Ordinary quotes/backslashes and control/unterminated-value
refusals remain intact. Native/editor parity and incremental edits pass; the
HTML/CEM card 1.0.1 uses default CEM inspection again. CLI, both WASM bindings,
Studio saved edits, credential refusal and assembled-demo/offline acceptance
pass, with all capabilities, 898 files and four WASMs below 25 MiB preserved.

Completed 2026-10-27: [Login element-template identity and recovery headroom](archive/todo-completed-login-element-template-identity-2026-10-27.md)
corrects Login 2.8.1's live resource and Parse configuration using the existing
registered selectors, with unchanged source bytes and stable IDs. Native Parse
and Validate succeed with 15 preserved advisory warnings; original generic-source
errors retain their severity and ranges, and old saved identities are not
rewritten. Declarative layout containers keep complete worker/fallback recovery
within the existing DOM/4 MiB guards. Native catalog, editing/live view,
credential refusal, independent saved/offline copies and assembled demos pass.
The separately observed CLI/worker undeclared-binding gap is corrected in the
[Login validation parity work](archive/todo-completed-login-validation-parity-2026-10-27.md).

Completed 2026-10-27: [JSON/YAML source Parse identities and diagnostics](archive/todo-completed-structured-data-source-identity-2026-10-27.md)
routes JSON, YAML and JSON Schema Parse/Inspect through retained native import,
with source identity/ranges distinct from AST/events presentation identity.
Literal data remains inert; original native refusals and strict reports survive
without fabricated trees or write failures. Ordinary project configurations,
worker/Node WASM, all 254 native catalog configurations and the complete
online/offline worker catalog pass. Five obsolete JSON/map companion exceptions
are retired; other source limits remain pinned. Complete Login recovery retains
credential refusal, 4 MiB/witness/deadline guards and disabled automatic recovery.
The assembled demo retains 898 files and four WASMs below 25 MiB.

Completed 2026-10-27: [CEMT profiles and writer encoding](archive/todo-completed-cemt-profile-encoding-2026-10-27.md)
separates package colorizer selection from source-preview encoding in the shared
native writer. Explicit output aliases, legacy HTML requests, selector precedence,
public MIME/schema, original spans and final-newline ownership remain consistent.
The original raw-CEM assertion passes, including stripped CLI; all 548 CLI library
tests pass. Native/WASM, Studio formatting and credential-recovery checks pass
without UI behavior changes or relaxed limits. Deployment retains 898 files and
four WASMs below 25 MiB.

Completed 2026-10-27: [source-operation diagnostics and original owners](archive/todo-completed-source-operation-diagnostics-2026-10-27.md)
uses admitted native source owners for Parse/Inspect and publishes diagnostics
with the selected Inspect operation through CLI reports and common services.
XML-family/data sources retain literal input semantics; Markdown source views
retain native events/ranges before HTML lowering. The obsolete Supplier XML and
four Inspect exceptions are retired. The separately tracked portable Studio
project CEM limitation is resolved by the typed control-source work below.
Native/WASM, Studio source and complete
worker/fallback Login recovery checks pass without relaxing credential privacy,
checkpoint budgets or deadlines. The demo retains 898 files and four WASMs below
25 MiB.

Completed 2026-10-27: [Login validation parity](archive/todo-completed-login-validation-parity-2026-10-27.md)
runs element-template embedding checks in the shared admitted-source engine path,
retiring the duplicate CLI Validate/Check pass. CLI and worker now reject the
same undeclared Login binding at original byte 655, preserving source maps,
15 advisory warnings, declared/runtime bindings and repair. Native admission,
reload ownership, cancellation and validation budgets remain guarded. All 549
CLI library tests, WASM/Studio source checks, complete worker/fallback Login
recovery, native and online/offline catalog acceptance and demo checks pass.
Credential refusal, original checkpoint/deadline limits and shared declarative
UI principles remain intact; deployment retains 898 files and four WASMs below
25 MiB.

Completed 2026-10-27: [typed command/configuration source adapters](archive/todo-completed-control-source-adapters-2026-10-27.md)
retain native command/run-config/project models and original JSON/CEM syntax for
source Parse/Inspect/format, with standalone run-config semantic validation.
Original selectors, URI/ranges, literal values, refusals and repair survive;
source operations do not execute argv, prepare plans or read project resources.
Portable configurations 1.0.1 adds ten ordinary source workflows and retires the
last portable-project Inspect exception. Native and online/offline catalogs
cover all 264 configurations with zero source-limit/companion exceptions.
Native/CLI/WASM, Studio source, independent saved/offline and demo checks pass;
worker/fallback Login recovery passes in isolation with unchanged credential,
checkpoint, witness and deadline guards. The parallel fallback resume timeout
remains recorded in the existing concurrent verifier scheduling item. Deployment
retains 898 files and four WASMs below 25 MiB.

Completed 2026-10-27: [module-map semantics and AST JSON export](archive/todo-completed-module-map-ast-json-2026-10-27.md)
shares native v1/v2/v3 source checks with deployment planning, retains original
JSON diagnostics and separate paired-map/read boundaries, and aligns explicit
semantic AST JSON exports with their unchanged source validator. Ordinary Studio
projects add semantic repairs and actual-export validation. Native CLI/WASM,
complete native catalog, saved/offline browser, demo and isolated Login recovery
checks pass. CEMT presentation, binary ownership, credential policy and all
recovery bounds are unchanged; native projection decoding/replay remains below.

Completed 2026-10-27: [owned native Parse/Inspect provenance](archive/todo-completed-owned-parse-inspect-provenance-2026-10-27.md)
binds original and writer bytes to typed native ranges, publishes bounded
artifact-owned references, and connects existing Studio source selection without
reading AST data fields as authority. Native CLI/WASM, complete catalog,
saved/offline navigation and isolated worker/fallback Login recovery pass.
Declaration whitespace and unused view metadata were reduced to keep every
provenance row available within the unchanged node/byte limits. Remaining
formatter/markup endpoints are completed below; native projection input adapters remain.

Completed 2026-10-28: [formatter and HTML/XML conversion provenance](archive/todo-completed-formatter-markup-provenance-2026-10-28.md)
names original native formatter views, retains typed final markup writer bytes
and spans, and preserves SCSS module/callsite coordinates. The ordinary native
projection project includes XML formatting and CEM-to-HTML/XML configurations
using the existing declarative result/source controls. Native CLI/WASM, all 271
native catalog configurations, saved/offline source selection, demo and isolated
worker/fallback Login recovery pass. Credential refusal and all recovery bounds
remain unchanged. The separate existing artifact-reader trace failure is
recorded below.

Completed 2026-10-28: [native projection-source imports](archive/todo-completed-projection-source-imports-2026-10-28.md)
loads six explicit JSON/binary identities into retained native common trees,
validates full payloads and sealed chunk hashes/topology, and preserves actual
artifact provenance with passive references/events. Ordinary Studio JSON input
configurations, all 277 native catalog configurations, saved/offline selection,
demo inventory and isolated worker/fallback Login recovery pass. Binary Studio
command admission remains below; credential refusal and all recovery bounds are
unchanged. Deployment retains 898 files with all four WASMs below 25 MiB.

Completed 2026-10-28: [owned binary Studio commands](archive/todo-completed-owned-binary-commands-2026-10-28.md)
admits explicit saved configurations through shared native byte/identity checks
and execution revalidation. Sources remain immutable, with exact downloads and
read-only declarative controls. All six binary configurations pass online and
offline; native/WASM, all 283 catalog configurations, deployed demos and isolated
Login recovery pass. Layout-only whitespace was removed to retain the unchanged
32,768-unit witness limit. Credential privacy, 4 MiB groups and all deadlines
remain unchanged; deployment stays at 898 files with all four WASMs below 25 MiB.

Completed 2026-10-28: [direct configuration URL text admission](archive/todo-completed-configuration-text-admission-2026-10-28.md)
shares catalog file admission: available bytes, declared preview policy and full
strict UTF-8 decoding. Missing bytes remain unavailable; invalid/unsupported
sources retain exact downloads, and missing resources preserve selection/drafts.
All 37 navigation tests, deployed configuration/draft/offline checks, demo
verification and isolated worker/fallback Login recovery pass. Existing shared
CEM controls, native parsing, credential policy and all limits are unchanged.
Deployment remains 898 files, with all four WASMs below 25 MiB.

Completed 2026-10-28: [multiline editor-driver attribution](archive/todo-completed-editor-driver-performance-2026-10-28.md)
reproduces the dominant browser edit/style cost with runtime listeners absent.
The permanent six-case diagnostic records original bytes, events, nodes, latency
and optional CPU profiles while preserving the 30-second action deadline. Live
fill remains 23.9 seconds with 1,987 nodes/events; fresh native paste takes 38 ms
and one node/event. Shared CEM publishes once on both paths. All ten editor
stories (including large native paste/selection/undo), deployed diagnostics,
demo checks and isolated Login recovery pass. Production editor/CSS behavior,
credential refusal, all recovery limits and the 898-file deployment are unchanged.

Completed 2026-10-28: [version-specific module-map lifecycle fixtures](archive/todo-completed-versioned-module-map-fixtures-2026-10-28.md)
replaces the obsolete shared success buffer with canonical v1/v2/v3 examples and
covers 28 source/admission/refusal cases with original native diagnostics/ranges.
All 103 lifecycle tests, both 33-test module-map cohorts, three CLI checks,
Node/browser-worker WASM, Studio configuration/artifact checks and isolated Login
recovery pass. All Rust edits are test-only; production validation, shared
components, credential refusal and recovery budgets/deadlines are unchanged.

Completed 2026-10-28: [formatter artifact-reader freshness](archive/todo-completed-formatter-reader-freshness-2026-10-28.md)
traces the eight discovery/execution reads and closes the obsolete four-read
fixture without removing admission checks. The expanded fixture exercises 58
pipeline runs, including late source changes/refusals, re-admission and URI/MIME
cache qualification. Pipeline, asset/cache, native provenance, public WASM,
Studio formatting and isolated Login recovery checks pass. All edits are tests
and documentation; production code, shared components, credential privacy,
cache bounds and recovery limits/deadlines remain unchanged.

Completed 2026-10-28: [constructed document/provenance URI separation](archive/todo-completed-constructed-document-uris-2026-10-28.md)
fixes the shared native accessor used by CEM-QL and XPath. Constructed string
documents report no document URI while retaining original source owners, ranges,
diagnostics and independent base URIs; loaded-document URIs remain intact.
Native, public WASM, Studio command/provenance, deployed-demo and isolated Login
recovery checks pass. The deployment retains 898 files with all four WASM files
below 25 MiB. Shared components, credential refusal and all recovery
budgets/deadlines are unchanged.

Completed 2026-10-28: [Studio generated WASM glue lint scope](archive/todo-completed-studio-generated-glue-lint-2026-10-28.md)
excludes only the build-owned `generated/schema-assets/` staging directory.
The original 238 errors/five warnings reproduce before the repair; package-wide
lint now passes with all 86 tracked authored JS/TS files covered. Source, script
and root-config rules and error/warning probes remain intact. Uncached/cached
lint, shared declarative components and isolated worker/fallback Login recovery
pass. Runtime/UI behavior, credential refusal and all recovery limits are unchanged.

Completed 2026-10-28: [Original-owned XSLT compilation](archive/todo-completed-owned-xslt-compilation-2026-10-28.md).
Shared engine compilation, import discovery, name resolution and every imported
stylesheet now retain the admitted immutable native parser contract. Complete
and external native fixtures, physical-runtime package replay, online/offline
Studio graphs, deployed demos and isolated worker/fallback Login recovery pass.
The stale trusted bootstrap pin is refreshed with strict equality preserved;
credential refusal and all recovery budgets/deadlines remain unchanged. The
malformed-CSV default Parse mismatch found during acceptance is resolved below.

Completed 2026-10-28: [Default Parse refusals and original CSV diagnostics](archive/todo-completed-default-parse-refusal-2026-10-28.md).
CLI and common command execution now share native refused-presentation policy:
exit 1 with original diagnostics and no output when no source tree is admitted.
Recoverable source inspection, schema-owned classifications and strict output
verification remain intact. Native, Node worker, Studio online/offline, deployed
demo and isolated worker/fallback Login recovery checks pass. The strict census
fixture includes the existing thirtieth package-owner registry; no authority or
recovery bounds change. Relative Node output canonicalization is tracked below.

- [x] Provide owned native XDM handoff for constructed transformation-stage results. [Implementation and verification](archive/todo-completed-native-xdm-handoff-2026-10-28.md).
  CEM/XSLT output now supplies one bounded native document to XSLT, preserving
  source owners and typed contracts without encoding/reimport. Native forest
  roots and generated-template provenance are corrected; the ordinary CEM
  project demonstrates saved edits and offline previews. Native, Node, full
  Studio, optional-profile, deployed-demo and isolated worker/fallback Login
  recovery checks pass. Credential refusal and all recovery bounds are unchanged.

- [x] Make full schema-package validation honor supplied resource readers for local identities. [Implementation and verification](archive/todo-completed-schema-package-reader-2026-10-28.md).
  Artifact, converter-template and example reads use the supplied reader before
  filesystem resolution; refusals retain full validation diagnostics. Legacy
  no-reader filesystem checks, metadata-only admission and strict runtime
  artifact consumption are preserved. Native, CLI, public WASM, Studio worker,
  isolated credential recovery and deployed/offline demo checks pass.

- [ ] Diagnose relative Node `--out` destination canonicalization before common command-service URI admission. `createNodeCommandService({ cwd })` plus `parse inventory.csv --out inspection.cem` currently refuses `runPlan.outputs.resolvedDestination=inspection.cem` as `cem.command_service.uri_invalid` before reaching Parse; an explicit `file:` destination works and preserves refused output. Add lowering/Node/executable fixtures first, determine the supported argv-path contract and retain strict prepared-URI/publication authority checks. Keep this separate from native Parse refusal policy.

- [ ] Diagnose the three existing linked package preparation source-map failures at `schema_package_lifecycle/attribute_datatypes.rs:473`: pretyped handoff, callback context/publication guards and checked preparation replacement retain an additional `SourceBoundary` frame. All three reproduce on unchanged `eca54d20`; preserve source provenance and intended assertions when resolving the mismatch. The broader cohort passes 46/49 cases; see the [CSV contract verification](archive/todo-completed-csv-parser-contracts-2026-10-24.md#verification).

- [ ] Isolate the optional Studio sample graph from the ordinary Feature Tour generated directory. `build-schema-assets-profile.mjs` leaves `generated/feature-tour/feature-tour.schema-assets.cem`, causing the subsequent strict default `generate-feature-tour.mjs --check` file-set assertion to fail. Add default → optional → default verification fixtures, move the optional build input into its own generated domain with correct relative resource bases, and retain exact default file-set checking. Removing only that untracked build scratch file permits default verification; saved projects/drafts are unrelated.

- [ ] Diagnose the intermittent worker-static-change fresh-allocation timeout in StaticPreviewRefusals observed in the frozen 111-case credential-policy cohort (110 pass); the unchanged isolated case passes. Preserve the original five-second allocation deadline and static-presentation refusal assertions.

- [ ] Repair standalone runtime typecheck preparation for the transitive CLI generated schema: retain or restore the exact native-generated TypeScript input through the CLI build/cache contract, so `tsc --build --emitDeclarationOnly` works without explicit schema preparation. Keep this separate from semantic editor/widget recovery.

- [ ] Stabilize concurrent release-verifier scheduling around the existing 30-second cold Login source-paint case; retain its assertions and deadline. The unchanged complete release passes with `--parallel=1`. The 283-case Studio source invocation overlapping deployed browser gates passes 276 cases with seven deadline failures; all seven pass in an isolated Nx rerun, including complete live Login recovery on worker/fallback. Pending-build cancellation then passes three isolated runs after public navigation-readiness ordering is repaired. Keep the original five-second resume deadline, test bounds, privacy assertions and 4 MiB checkpoint budget; do not treat overlapping execution as a green full-cohort invocation. The typed control-source acceptance adds a concurrent fallback `cem.checkpoint.resume.timeout` at resume 0 (worker passes); its unchanged worker/fallback rerun passes 2/2 in isolation. Preserve this separate scheduling evidence; see the [control-source verification](archive/todo-completed-control-source-adapters-2026-10-27.md#verification).

- [ ] Repair the existing native conversion baseline independently of recovery: `cross_surface_conversion_rules::cem_fixture_preserves_anonymous_scope_expression_avt_comments_and_rich_content` loses the expected `{.busy}` value in both default and stripped tests. Preserve the intended native semantics and assertion when resolving the mismatch. The separate CEMT profile/writer mismatch is completed above.

- [ ] Investigate the existing stripped WASM binary-size gate: current stripped output is 27,264,508 bytes versus 27,033,499 default bytes, despite the stripped capability contract rejecting debug control. Establish actual debugger-code omission and the correct size baseline without weakening the gate to hide retention.

## Amsterdam presentation samples: active Studio priority

The Login, Supplier-pipeline and ordinary Markdown + CEM-ML site projects are
implemented in the shared workspace. Preserve projects/resources/configurations,
source provenance and independent copies. The
[required tutorial checkpoints](declarative-amsterdam-2026/presentation.md#required-tutorial-checkpoints),
[Login demonstration](declarative-amsterdam-2026/presentation-plan.md#required-login-demonstration-and-companion-examples)
and [sample audit](presentation/declarative-amsterdam-2026-handoff.md#studio-sample-audit-2026-10-17)
remain the scope references. Completed sample work is in the
[archive](archive/todo-snapshot-2026-10-23.md#amsterdam-presentation-samples-active-studio-priority).
These remaining diagnostics/helper actions do not lead the functionality queue.

- [ ] Diagnose the existing `DOM Stylesheet Adoption / Readiness And Native Diagnostics` held-worker timeout without weakening readiness assertions. The stylesheet ownership work reproduces the 30-second timeout with the unchanged `6f1c8dca` registry as well as the repair, both alone and in the sequential browser group; the other two DOM-adoption cases pass. Keep this separate from completed replacement/collection ownership checks.

- [ ] Audit the five pre-existing strict schema-definition diagnostics in the Studio project package (`id` pattern datatype and `path`/`reference-path` prefix facets); preserve its native portable project contract. The explicit-entry work verifies the same baseline diagnostics before/after and introduces no additional one.

## Declarative UI Architecture Correction

### Remaining declarative UI migration

The canonical component migration is complete. Keep these unresolved diagnostic
follow-ups separate from completed migration and checkpoint acceptance. Follow
the [declarative UI principle](declarative-ui-principle.md) and
[component development pattern](component-development-pattern.md).

- [ ] Conditional diagnostic follow-up: if slider property-page root readiness
      fails again, use the new failure capture to attribute it before changing
      behavior. One initial aggregate timeout remains unexplained; the unchanged
      source/packed gate, 20 reloads under load and final full gate pass. Passing
      repeats do not establish a cause or a specific slider fix.

- [ ] Diagnose the isolated suggestions-controller baseline failures reproduced on 2026-10-20 against committed HEAD: ProductionFieldsExternalSourcesAndTrustedAncestorResume (child visibility) and PlatformCloseCancelsPendingAndPreservesEditorRouting (missing committed native row map). Keep assertions and deadlines intact; distinguish these from checkpoint capture acceptance.

- [ ] Conditional diagnostic follow-up: if datepicker/suggestions failures
      recur during concurrent browser runs, capture their focus, dialog and
      processing state before changing behavior. Running the canonical cohort
      alongside the package gate produced two 30-second play timeouts and two
      following focus/open failures (262/266); the standalone cohort passes
      266/266 with unchanged deadlines and assertions. No runtime cause is
      attributed. Preserve this distinction in subsequent acceptance reports.

### Native-surface delivery follow-ups

- [ ] Verify both pinned [ARIA profiles](cem-element-aria-reference-profile.md)
      in the documented browser/assistive-technology matrix. Record actual
      browser/OS/AT versions, accessibility relation exposure, ordered/repeated
      details targets and error-message announcements. Keep untested combinations
      explicit and the draft profile opt-in. Single-relation accessibility APIs
      may expose only the first target; exported DOM sequences must stay intact.

## Autocomplete and suggestions design for CEM inputs

The shared controller, declarative field composition, native profiles and
host-integrated fixtures are implemented. Remaining acceptance uses the
[suggestions acceptance record](../packages/cem-components/docs/suggestions-acceptance.md)
and its repeatable device procedures; automated input does not establish
physical-device or spoken assistive-technology coverage.

- [ ] Complete physical touch/pen, real keyboard/IME, screen-reader and mobile
      viewport runs for both field providers. Record browser/OS/device/AT versions
      and exact outcomes. Verify tap preserves the editor/caret, pan scrolls,
      terminal IME keys neither select nor submit, and active-row/status
      announcements preserve the field's label/help. Run the plain overflow-box
      touch baseline: the automated gesture did not scroll it in this environment,
      and a wheel-scroll pass does not substitute for touch-pan evidence.

- [ ] Record supported-browser native datalist selection, filtering,
      accessibility and device evidence before claiming additional-profile
      acceptance. Demonstrate numeric option selection stores/submits `1` rather
      than its label One; cover email/URL sanitization, range/step mismatches,
      source withdrawal, physical IME and mobile keyboard/AT. Headed Chromium
      and Firefox probes recorded value/FormData and withdrawal, but did not
      establish visible picker selection; numeric Arrow Down/Enter produced `0`.
      WebKit probes remain blocked by missing host libraries. A connected list
      alone is not selection evidence; unsupported native UI preserves ordinary
      editing without a custom popup fallback.

- [ ] Record supported-device Back/dismiss-gesture evidence before advertising
      platform close support. Use the host-enabled acceptance procedure; the
      ordinary local gallery supplies no platform-close authority. Verify one
      accepted request dismisses the admitted child, preserves text and parent
      state, and releases the watcher so normal navigation resumes. Existing
      Chromium protocol evidence does not establish physical Back/gesture support.

- [ ] **Only when requested:** design field-owned label/submission conversion,
      multiline/email-multiple token completion and full surface-provider
      adoption. Carry explicit value/provenance, selection/history, native
      semantics and relationship grants into each separate contract. Labels must
      not rewrite numeric values; token edits must preserve caret, separators and
      composition; one provider owns each adopted surface.

## Studio startup failure watch

- [ ] On recurrence, attribute the historical aggregate fallback entry-module
      fetch failure before changing the loader or test lifecycle. The old cohort
      passed 273/274 and an isolated rerun passed; current observed and fresh-cache
      full cohorts each pass 275/275. Use the
      [script/network/frame capture](studio-module-load-diagnostics.md) to retain
      the actual failed request and owning frame, add a failing cause-specific
      regression, then repair and rerun the complete cohort. Passing repeats and
      a later HTTP 200 do not establish the historical cause. Preserve recovery
      assertions/deadlines, credential refusal, user storage and the 4 MiB budget.

## Browser failures observed during placement verification

Preserve meaningful assertions and existing story/global deadlines. Capture
failing lifecycle state before changing readiness or runtime behavior; passing
repeats alone do not attribute a historical failure.

- [ ] Fixture: attribute the historical aggregate-only data-table/set-URL and
      native suggestions pointer-tap deadlines recorded on 2026-10-08. Establish
      isolated controls, then capture the failing step under load. The separate
      pan-opening readiness defect and provider story split are complete; the
      final aggregate passed 487/487. Retain every tap, pan-cancellation and
      outside-focus assertion. See the
      [archived investigation](archive/todo-snapshot-2026-10-10.md#browser-failures-observed-during-placement-verification).

- [ ] Investigate runtime aggregate instability on recurrence. The historical
      action-cutover run passed 311/312 stories: `Payload Css Readiness And
      Hydration` expected one style but observed zero; its complete 12-story
      file passed in isolation. Later reader-admission fixture corrections do
      not attribute a post-settlement missing style. A full playground run on
      2026-10-10 also hit the gallery sample-readiness deadline once; all 19
      galleries passed in isolation and the complete source/packed rerun passed.
      Use the verifier's component, URL, missing-host and runtime diagnostics;
      preserve the readiness assertion and reproduce before changing behavior.

## Immediate: Legacy Demo Case Coverage

The static readiness inventory and reproduced scoped-CSS, external-src, retry,
NPM and string-demo fixture corrections are complete. Remaining work is traced
recurrence attribution, not wholesale replacement of frame waits. See the
[current boundary inventory](browser-stabilization-review.tmp.md#reconciliation-of-the-remaining-readiness-inventory).
These historical investigations remain behind presentation functionality.

- [ ] Attribute the source-loaded cell-overrides stock sample's historical
      initial-warning timeout on traced recurrence. Retain its diagnostic target
      and current deadline. After refreshing packaged WASM, all 32 concurrent
      helper/real-component probes passed on 2026-10-08; warning publication took
      5.65–9.35 seconds under overlapping load without reproducing the failure.

- [ ] Capture declaration/render/worker state on recurrence of the location
      samples' 180-frame wait for both readers. Focused overlapping load passes
      without attributing the historical failure. Preserve exact per-card output
      and resource predicates; inspect authored instances and available lifecycle
      signals before changing the fixture. See the
      [NPM/location evidence](browser-stabilization-review.tmp.md#npm-and-location-readiness-under-overlapping-load).

- [ ] Capture HTTP's historical 300-frame initial article-count timeout using
      the existing opt-in lifecycle/per-card tracing, which rethrows the original
      error. Distinguish declaration/library startup, current rendering and
      future HTTP completion before choosing a correction. If startup polling
      is premature, await existing lifecycle settlement before checking each
      authored instance; preserve resource assertions and the 30-second story
      deadline. Traced follow-up workloads pass without reproducing the failure;
      see the [HTTP observation](browser-stabilization-review.tmp.md#http-startup-observation-without-a-reproduced-failure).

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final [Phases 10 and 11](../roadmap.md#phase-10---figma-ui-kit). Their
checklists live only in the roadmap. Finish and verify `cem-elements` and the
declarative `cem-components` migration first, then the remaining non-Figma phases
through Phase 9 release governance.

- [ ] Implement the general whole-page DCE CLI compilation pipeline from the
      [whole-page SSR proposal](cem-ml-cli-ssr-proposal.md), using config-driven
      CEMT transform graphs and the shared native DCE lifecycle. Accept HTML,
      CEM-ML and opted-in `.cem.md` pages; keep source URI, deployed page/base URL
      and output destination distinct. Support scoped declaration/module
      resolution, canonical islands, scoped CSS, bounded nested settling and
      registered CEMT participants such as `cem-demo-element`. Emit deterministic
      HTML, hydration metadata, diagnostics and source maps with atomic
      single-page/directory fan-out. Add native, Node and browser fixtures for
      identity-matched hydration without an initial rerender, mismatch/privacy
      refusals and source/client parity, without executing arbitrary page
      JavaScript. Keep this behind the current Studio sample priorities.

## Deferred work: Declarative Amsterdam 2026 CEM presentation

Presentation preparation started 2026-10-10 and was deferred at the user's
request on 2026-10-11. On 2026-10-17 the user prioritized Studio functionality
and availability of the presentation's sample set. The active sample checklist
above takes precedence over device acceptance and helper work; the remaining
deck, website, release and rehearsal actions here remain preserved. Read the
[presentation handoff](presentation/declarative-amsterdam-2026-handoff.md) after
clearing chat context; it records the starting material, implementation
checkpoint and return path. The existing
[session proposal](declarative.amsterdam.2026.md) is the starting brief. The
[working presentation document](declarative-amsterdam-2026/presentation.md)
records the 2026-10-11 program/discussion review, current agenda and
[presentation minutes](declarative-amsterdam-2026/presentation.md#presentation-minutes).
The user requested adoption-focused framing on October 10 and alignment with
the public agenda on October 11. Use the pitch to explain why the promised
document activities matter; preserve the login walkthrough, platform subjects,
and six proposal intervals. Correspondence and release checks remain open.
The [presentation plan](declarative-amsterdam-2026/presentation-plan.md) owns
ideas, preliminary design, and adoption planning; the
[platform pitch](declarative-amsterdam-2026/cem-platform-pitch.md) owns the
audience-facing message. This file remains the sole execution checklist.

- [ ] Apply the canonical CEM theme light palette throughout the slide deck.
      Keep physical slide 2 (the original title slide) in dark mode as contrast.
      Use CEM dark mode for terminal output samples. Preserve semantic syntax
      colors and verify their contrast. The new opening teaser uses light mode.

- [ ] Copy the prepared animated opening into the main deck and verify the
      26-second sequence in Slideshow. Import currently requires one starting
      click; try After previous in the Motion panel for automatic slide-entry
      playback, then recheck timing. The live deck remains unchanged.

- [ ] Use the presentation plan to map the manifesto and platform pitch into
      the presentation website and slides, following the current presentation
      minutes. Use familiar side-by-side representations and visible outcomes;
      prioritize XML/XSLT developers, web UI developers and platform/integration
      owners. Keep the published activities and 60-minute slot; use broader
      pipeline and security examples as companions.

- [ ] Verify the selected exercise against current CEM implementation and
      resolve remaining delivery requirements. Published title, slot, audience,
      activities, and working minutes are recorded in the presentation document.
      Gmail history, session-specific submission requirements, participant
      releases, and the complete demonstration path remain unresolved.

- [ ] Prepare a reproducible tutorial and presenter demo with source files,
      exact commands, expected results, an intentional error and repair, and a
      presenter-led fallback. Reuse current examples and native capabilities;
      add an explicit fixture action here before any new fixture work.

- [ ] Produce the presentation, speaker notes, diagrams and participant setup
      instructions. Check the proposed Node/Linux installation paths against
      actual available releases, and record requirements for any other platforms
      included in the session. Prepare resource entries for discussion #24;
      add verified links as materials become available. Use that stable URL
      throughout the deck; Studio remains a future participant route.

- [ ] Rehearse the timed walkthrough and verify every shown command, example,
      output and demo path. Check slide readability and the fallback flow;
      record the tested versions and remaining limitations with the materials.

- [ ] Finalize the presentation and supporting files, record their locations
      and verification in the handoff, and archive completed presentation actions.
      Then continue with **CEM Studio** below. An outline alone does not complete
      this set.

## CEM Studio delivery planning

Completed workspace, source-editor, node and edge authoring work remains in the
[prior snapshot](archive/todo-snapshot-2026-10-23.md#cem-studio-delivery-planning).
Native parameter maps, shared form controls and their verification are in the
[graph authoring completion record](archive/todo-completed-graph-parameters-2026-10-23.md).

Continue with persistent native package activation, opt-in deployment/body
exclusion and measurements under [Project builds and artifact previews](#project-builds-and-artifact-previews).
Preserve the [bounded project lifecycle](cem-studio.md#next-delivery-milestone-user-owned-project-workspace):
100 graph nodes / 1 MiB source and 20 history snapshots / 8 MiB. Authored ID
changes, aliases and expression-valued attributes remain source-only. Keep
credential recovery refusal, the aggregate 4 MiB checkpoint budget and separate
physical-device/spoken AT acceptance.

## Component application WASM profile (separate target)

Planned 2026-10-28 at the user's request. Implement the
[profile design](cem-components-wasm-profile.md) and
[roadmap workstream](../roadmap.md#313-minimal-component-runtime-wasm-profile) as
an explicit bundle for `cem-elements` plus `cem-components`, leaving existing
full-runtime/CLI/Studio targets and defaults intact. No profile target is created
by this planning update. Start with the audit and native boundary proof; establish
size budgets from measurements before extraction is accepted. Preserve every
component capability, shared UI ownership and the credential policy above. Keep
the current Studio execution order; run this as a separate component-library
workstream when selected.

- [ ] Audit the generated component catalog, declaration/capability metadata and
  native import/call/export/registry roots. Map every component mode to required
  core, source assets and candidate language/conversion/tooling modules; include
  dynamic imports, providers, source-editor coloring, media and table workflows.
- [ ] Add baseline fixtures for core-style UI cold start and demanding component
  workflows. Record exact hashes/toolchain, raw/gzip-9/Brotli-9 bytes, requests,
  first-use latency, linear/live/host memory and all-features totals. Reproduce
  the full-profile comparator from the same commit; choose fixed size budgets
  after the reachability/prototype report, not an invented minimum.
- [ ] Define and prototype the versioned core/module ABI and capability catalog
  with native fixtures for one isolated optional operation and one importer
  feeding a retained component. Prove typed CEM-tree/source-owner handoff or
  original native reimport/recompile without cross-heap handles, JSON AST
  conversion or JavaScript parsing; retain types, namespaces/base URIs, ranges,
  diagnostics, cancellation and lease disposal. Keep a required parser in core
  until this boundary passes, and record the selected module-loading mechanism.
- [ ] Split native dependency/registry/export roots behind a separate component
  profile; add proposed `cem_ql:build:wasm-components`. Keep existing full builds
  and rendering/admission/recovery guards unchanged. Add native fixtures and
  binary/import-graph evidence that optional compiled code and debug/tooling
  initialization are absent, rather than merely hiding their JavaScript exports.
- [ ] Add the reusable `cem-elements` module loader and proposed
  `cem-elements:build:components-runtime` target. Alert the user before new shared
  JavaScript capability implementation. Cover pinned ABI/version/hash/features,
  dependency closure, bounded deduplicated acquisition, worker/owned-fallback
  activation, exact-owner disposal and late/aborted/revoked publication. No
  Studio/component-local JavaScript loader workaround is allowed.
- [ ] Connect generated native requirements and existing declarative resource
  demand to module readiness/preload. Add fixtures for external/dynamic template
  languages, mode/resource changes, pending/failing demand, retry, concurrent
  consumers and source-editor selection/IME/focus preservation. Preserve current
  committed UI and exactly-once side effects; document explicit async readiness
  for this new entry point without silently changing existing synchronous APIs.
- [ ] Extract optional language/conversion/tooling modules one group at a time,
  with native and browser parity fixtures for each mapped component mode and
  original diagnostics/provenance. Reuse owned source packs for declarative
  schemas/formatters/colorizers; do not emit a separate WASM per source template
  or pull a monolithic full engine into every optional module.
- [ ] Add proposed `@epa-wg/cem-components:build:components-runtime` assembly and
  explicit package export/import-map wiring for declarations, matching shared
  core/loader/modules and theme dependencies. Add clean npm-consumer/bundler,
  standalone/subpath and full -> component -> full output/cache-isolation
  fixtures; assert one canonical core asset and no eager full-runtime dependency.
- [ ] Add offline/preload/update fixtures proving core-only installation fetches
  no optional modules or packs, explicit feature sets remain usable offline, and
  uncached offline demand fails without losing state or contacting an undeclared
  service. Preserve integrity, cache scope, native source authority and saved
  projects/drafts; never put credentials in public deployment caches.
- [ ] Enroll every optional endpoint, active work and retained owner in existing
  recovery groups. Add worker/fallback capture, drain, fresh-generation reimport,
  re-demand and lease/late-acknowledgment fixtures. Re-run entered-password/OTP
  refusal, require explicit clearing, keep automatic whole-Studio recovery off,
  and preserve the 4 MiB budget and all current deadlines.
- [ ] Run the complete catalog/mode, accessibility, theme/forced-color and shared
  component suites on full and component profiles without skipping capabilities
  or weakening assertions. Publish reproducible core/first-use/all-modules
  byte/latency/memory and duplication reports; enforce agreed fixed budgets,
  retained default builds and profile-specific optimizer/import/export checks.
  Document installation, demand, preload/offline choices and measured limits
  before advertising a minimum or considering a separate default-adoption step.

## Component gallery source-link maintenance (after Studio presentation work)

- [ ] Migrate remaining extensionless canonical-source `cem-module-url` references to full declaration paths under the folder import-map convention. Add source/isolated-package fixtures without assuming a template has only id/type attributes. The current artifact-tree increment repairs and verifies cem-media-preview; a filtered passive-component gate keeps that verification scoped while this older bulk maintenance remains separate.

- [x] Halve normal tree row padding through theme-owned `--cem-data-tree-padding-factor` (normal/dense 0.5, sparse 1). Verified `yarn nx run @epa-wg/cem-theme:build:css` and Chromium gallery styles across all five themes: normal 4px/6px (58px rows), sparse 8px/12px (66px rows), condensed unchanged. Ancestor sparse, nested normal reset and inherited control-padding override verified. Coupling minima and color/forced-color rules remain unchanged; forced-colors emulation was not separately run.

- [x] Group tree marker and icon in a zero-gap `part="leading"` span. Updated shared disclosure hit testing for nested markers and existing story assertions for the wrapper and zero gap. Verified `cem-elements:build`, `@epa-wg/cem-components:verify-declarative`, and Chromium gallery measurements (normal/sparse/condensed gap 0, row heights unchanged); marker click emits toggle and icon click emits activation.
