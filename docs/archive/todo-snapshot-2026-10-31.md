# TODO snapshot — 2026-10-31

Historical snapshot before completed-work cleanup: 149 completed checkboxes and
62 unfinished checkboxes. States and verification evidence below are historical;
the [active TODO](../todo.md) is the authority for unfinished execution work.
All unfinished items and their ordering were preserved in that active checklist.
Relative Markdown links have been rebased for this archive location.

The archived checklist links product/module sequencing to
[roadmap.md](../../roadmap.md) and wishlist work to [wishlist.md](../wishlist.md).

Completed work and verification evidence are preserved in the
[2026-10-23 snapshot](../archive/todo-snapshot-2026-10-23.md), including the native
schema-asset prototype, Studio examples, editor, previews and recovery work.
Earlier history remains linked from that snapshot. Historical checkbox states
do not reopen completed work; adopted contracts remain in their design documents.

Prioritize Studio functionality and presentation samples before unrelated helper
work and physical-device/browser/AT acceptance. Examples remain ordinary projects
with resources and configurations in the same workspace, without dedicated
example screens. Follow the [Studio milestone contract](../cem-studio.md#next-delivery-milestone-user-owned-project-workspace).

Keep UI declarative in `cem-components` and Studio CEM-ML; reusable JavaScript
behavior belongs in `cem-elements`, and application JavaScript owns non-UI services
only. Alert the user before introducing a new shared JavaScript UI capability.
Apply the [canonical CEM theme principles](../../packages/cem-theme/src/lib/tokens/index.md#core-cem-principles),
including inherited token ownership, accessibility and theme survivability.
Entered passwords or one-time codes continue to block recovery until explicitly
cleared by the user; automatic whole-Studio recovery remains disabled. Follow the
[credential privacy policy](../committed-checkpoints.md#credential-privacy-policy-for-live-login-recovery).

- [x] Make local demo Studio links bypass stale installed-worker assets without clearing projects or drafts. [Implementation and verification](../archive/todo-completed-demo-preview-refresh-2026-10-28.md).
  Local links and redirects select an inventory-based Studio revision on the
  same origin. Hosting, stale-cache/draft retention and complete demo acceptance
  pass; port 8081 is restarted with the new handler.

- [x] Clarify shared select labels and simplify Studio's output-tree presentation. [Implementation and verification](../archive/todo-completed-studio-output-navigation-2026-10-28.md).
  Shared field captions use canonical tag typography and related spacing.
  Declarative output navigation hides redundant roots while preserving nested
  folders, full paths and existing URL/selection identities.

- [x] Add project-home README links and minimum sample file guides. [Implementation and verification](../archive/todo-completed-studio-readme-navigation-2026-10-28.md).
  All 22 authored samples, the 32-type showcase and starter have short purpose
  paragraphs and linked essential files. Shared routed anchors and passive
  Markdown previews select known project resources, retaining builds, drafts
  and read-only originals online/offline. Catalog audits, host-view and navigation
  tests, all 23 catalog README views, demo verification and worker/fallback Login
  recovery pass. Credential policy and recovery budgets/deadlines are unchanged.

Completed [compact Studio layout and shared runtime work](../archive/todo-completed-compact-studio-2026-10-23.md) is archived. The [font-relative tree spacing follow-up](../archive/todo-completed-compact-studio-2026-10-23.md#font-relative-tree-spacing-follow-up) records the IDE reference refinement. The [page-level super-compact follow-up](../archive/todo-completed-compact-studio-2026-10-23.md#page-level-super-compact-theme-follow-up) records header controls, mode boundaries and verification.

The [explicit content-sized super-compact policy](../archive/todo-completed-compact-studio-2026-10-23.md#explicit-content-sized-super-compact-policy) supersedes its earlier pointer-gated geometry.

Completed [editor filename de-duplication](../archive/todo-completed-compact-studio-2026-10-23.md#studio-editor-filename-follow-up) covers editable files and read-only sample/output views.

Completed [graph parameter-map authoring and Studio controls](../archive/todo-completed-graph-parameters-2026-10-23.md) is archived.

Shared package transports, project/resource-driven Studio demand, opt-in
embedded-body exclusion, isolated profile measurements, editor responsiveness and
trusted optional-format adoption are complete. Prioritize the functional
content-type and schema samples below.

Completed [shared popup surfaces and indicator clearance](../archive/todo-completed-compact-studio-2026-10-23.md#shared-popup-surfaces-and-indicator-clearance) is archived.

Completed [immediate persisted theme updates](../archive/todo-completed-compact-studio-2026-10-23.md#immediate-persisted-theme-updates) covers storage notification independence and nested button value capture, including the earlier repeated Studio theme-change failure.

## Project builds and artifact previews

Completed 2026-10-27: [shared collection manifests](../archive/todo-completed-collection-manifests-2026-10-27.md)
unifies native CLI/common publication with the established JSON/YAML/XML writers.
Explicit bytes, application data, typed owners, inline provenance and CLI sidecars
retain their contracts; common hosts add no implicit child or sidecar writes.
Focused native, Node/browser WASM, Studio worker, credential and saved/offline demo
checks pass. Deployment remains 898 files; WASMs are 22.43/23.05 MiB. The separate
formatter-profile discrepancy and module-asset conformance are completed below.

Completed 2026-10-27: [native XML reload-bundle roundtrip](../archive/todo-completed-xml-reload-roundtrip-2026-10-27.md)
orders passive namespace/schema dictionaries only during native serialization,
preserving runtime lookup, declaration order, legacy decoding and authority.
All 113 focused native checks and 12 WASM boundary scripts pass, including the
unchanged byte-equality assertion. Studio XML, privacy and saved/offline demo
checks pass; deployment remains 898 files with four WASMs at 22.39/23.01 MiB.

Completed 2026-10-26: [complete-runtime WASM size budget](../archive/todo-completed-wasm-size-budget-2026-10-26.md)
pins post-bindgen Binaryen `-Oz`, retains all exports/capabilities and embedded
schemas, and publishes 898 files with four WASM assets at 22.33/22.95 MiB.
Strict size/count guards refuse before deployment mutation. Measured and regular
build hashes match; deployed Studio, offline builds, galleries, themes and
credential privacy checks pass. The archive distinguishes the source cohort's
273/274 result and passing fallback rerun from the separate pre-existing XML
byte-roundtrip defect. The XML defect is repaired above; the aggregate module-fetch
follow-up remains below.

Completed 2026-10-26: [demo deployment file budget and owned cleanup](../archive/todo-completed-demo-file-budget-2026-10-26.md)
reduces `cem-bin` from 1,142 to 898 files, including its deterministic SHA-256
inventory. Cleanup preserves modified obsolete and unrelated files; the 999-file
preflight limit includes them. Clean, migrated and repeated copies match.
Studio's complete bytes, offline builds, native gallery factories, all ten theme
generators, canonical playgrounds and credential refusal pass.

Completed 2026-10-26: [Studio in the assembled demo site](../archive/todo-completed-demo-site-studio-2026-10-26.md)
adds Studio to `build:demo`, the home-page links and `/studio/`. Complete native
deployment bytes, scoped headers/service worker, gallery factory ownership,
native preview and saved/offline builds pass. `serve:demo` provides the local
site and `verify:demo` checks the assembled deployment without clearing user data.

Completed [Studio accordion delivery check](../archive/todo-completed-compact-studio-2026-10-23.md#studio-accordion-delivery-check-2026-10-24)
confirms the layout source is unchanged, restores the stopped local preview, and
verifies styled accordion/collapse behavior across a stale-worker reload with
saved projects and unsaved drafts retained.

Completed [canonical source viewer repair](../archive/todo-completed-canonical-source-2026-10-24.md) covers explicit XHTML endpoints, source/package viewer checks and the complete nine-gallery compact Studio gate.

Completed [gallery lifecycle and checkpoint-limit repairs](../archive/todo-completed-gallery-imports-2026-10-24.md) covers delayed connection callbacks, inventory refusal versus physical retirement, and the targeted source/package import gate.

The optional native schema-asset prototype and
[owned package/formatter consumers](../archive/todo-completed-owned-schema-inputs-2026-10-24.md)
are complete; the default release retains its complete embedded bodies.
The [shared acquisition and offline delivery contract](../archive/todo-completed-schema-acquisition-2026-10-24.md)
is implemented, including persistent activation, coordinated host integration,
opt-in deployment and body exclusion. Measure delivery and runtime costs before
claiming binary, headroom or checkpoint savings. Prototype implementation,
measurements and verification are in the
[archived evidence](../archive/todo-snapshot-2026-10-23.md#project-builds-and-artifact-previews).

Completed [native owned-package activation](../archive/todo-completed-native-package-activation-2026-10-24.md) covers atomic publication, source ownership, ordinary engine consumers and strict body admission. Completed WASM/service wiring and declarative demand are recorded below.

Completed [native CSV parser contracts](../archive/todo-completed-csv-parser-contracts-2026-10-24.md) enable owned CSV package activation and ordinary import/validation with checked fact bindings, original source ownership and separate structural-validation readiness.

Completed [persistent WASM package activation and declarative demand](../archive/todo-completed-persistent-wasm-packages-2026-10-24.md) covers native owner inventory, ordinary command/import contexts, explicit admission versus full resource-validation phases, existing HTTP-resource binding and distinct-heap recompilation. No native owners or retirement authority cross status/checkpoint records.

Completed [opt-in Studio schema-asset deployment](../archive/todo-completed-studio-schema-profile-2026-10-24.md)
covers native profile emission, authored URL expectations, both factory pins,
complete offline delivery, cache isolation and independent source preparation.
Explicit host admission and coordinated publication remain required; the normal
bootstrap does not activate packages implicitly.

Completed [shared package resource handoff and owned invocation transport](../archive/todo-completed-package-resource-handoff-2026-10-24.md)
covers original physical identities, checked release, fresh compilation, complete
binding, policy/credential refusals, cleanup and consecutive offline handoffs.

Completed [command-service worker package transport](../archive/todo-completed-command-worker-packages-2026-10-24.md)
covers pinned original-byte delivery, exact worker ownership, trusted startup
grants, asynchronous checked release, native empty-inventory confirmation,
consecutive offline handoffs, credential refusal and late/unknown-outcome guards.


Completed [rendering-worker and owned-fallback package adapters](../archive/todo-completed-rendering-package-adapters-2026-10-24.md)
covers exact physical bindings, original configuration and pool/root isolation,
checked release, fresh publication, pressure/revocation guards and consecutive
offline handoffs with native imports and template compilation.


Completed [shared Studio package demand](../archive/todo-completed-studio-package-demand-2026-10-24.md)
covers explicit declarative demand across invocation, command, rendering and owned
helper heaps; lazy preview admission; complete original recovery cohorts;
single-pass fresh import; closed-runtime cleanup; and offline Studio acceptance.
The default complete deployment, credential refusal and disabled automatic
whole-Studio recovery are preserved.

Completed [opt-in embedded source-body exclusion](../archive/todo-completed-external-schema-bodies-2026-10-24.md)
covers five optional packages, a complete native identity catalog, missing-source
and alias refusal, deferred conversion registration, actual binary-body omission,
and online/offline Studio formatter/recovery acceptance. The default deployment
and credential policy remain unchanged; gzip delivery is currently larger.

Completed [profile delivery, latency and native memory measurements](../archive/todo-completed-schema-profile-measurements-2026-10-24.md)
cover 30 fresh-browser samples and the [recorded method/results](../schema-asset-profile-measurements.md).
The optional profile adds delivery/first-use cost and CSV capacity despite saving
256 KiB of initial/paint linear capacity. Keep the complete default; the demand
selection follow-up is complete below.

Completed [project/resource-driven optional package demand](../archive/todo-completed-resource-package-demand-2026-10-24.md)
covers native metadata planning, effective export selection, selected-resource
admission, bounded shared preparation, complete physical publication despite
cancellation/host closure, offline acceptance and 30 fresh-browser measurements.
HTML startup downloads no optional packages; the first dependent command pays
activation. The frozen-baseline editor timeouts are addressed below.

Completed [shared host-view settlement](../archive/todo-completed-host-view-settlement-2026-10-24.md)
covers the plaintext draft profile, bounded DOM-order discovery, asynchronous
parent payloads, inserted/replaced owners and disposal. Both original desktop
editor cases and all 140 Studio workbench/navigation/recovery tests pass with
unchanged deadlines, focus/selection/scroll assertions and credential policy.

Completed [optional Studio format demand and owned source Parse](../archive/todo-completed-optional-format-demand-2026-10-24.md)
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

Completed [content-type/schema capability inventory and checked sample mappings](../archive/todo-completed-content-type-inventory-2026-10-24.md)
are archived. Use the [audited operation and sample map](../studio-content-types.md)
when adding the folders and functional projects below; registered aliases alone
do not establish operation support.

Completed [nested catalog folders and contextual project discovery](../archive/todo-completed-content-type-catalog-2026-10-24.md)
are archived. Content types and schemas now reuses existing ordinary projects;
continue with the missing functional workflows below.

Completed [JSON/YAML report workflows and shared import repair](../archive/todo-completed-json-yaml-report-2026-10-24.md)
cover native configurations, three report artifacts, intentional syntax failure
and repair, shared previews, independent saved copies and offline rebuilding.
Source validation remains distinct from application-schema instance validation.

Completed [CSV inventory, portable validation configurations and owned string imports](../archive/todo-completed-csv-inventory-2026-10-24.md)
cover native header/quote diagnostics and repairs, typed HTML/JSON reports, shared
previews, independent saved copies and offline saved header-policy execution.
Both default and optional deployments pass the CSV workflow; standard header-mode
and row-width diagnostics retain their warning severity.

Completed [XML/XHTML workflows and query namespace context](../archive/todo-completed-xml-xhtml-2026-10-24.md)
cover Supplier validation/selection/repair, the ordinary XHTML document project,
shared retained import, explicit query identities and saved/offline preview acceptance.

Completed [Markdown article/site workflows and passive native import](../archive/todo-completed-markdown-site-2026-10-25.md)
cover article inspection/query/preview, typed metadata repair, complete site assets,
native source-range navigation and independent saved/offline copies. Authored HTML
policy, shared declarative controls and credential privacy remain unchanged.

Completed [HTML/CEM card and shared component workflows](../archive/todo-completed-html-cem-card-2026-10-25.md)
cover semantic comparison, production card/slot composition, native HTML builds,
source inspection and policy repair, saved/offline previews, explicit worker
Inspect formats and shared projected slice-event ownership. Existing Login
contracts, declaration authority and credential privacy remain unchanged.

Completed [SVG/MathML projects and shared foreign-content previews](../archive/todo-completed-svg-mathml-2026-10-25.md)
cover retained family imports, native inspection/query/formatting, namespace repair,
exact HTML entries and provenance, real vector/formula rendering and independent
saved/offline copies. Source validation and bounded preview admission remain
separate; credential privacy and automatic recovery policies are unchanged.

Completed [CSS/SCSS/selector projects and native stylesheet artifacts](../archive/todo-completed-stylesheet-projects-2026-10-25.md)
cover typed CSS graph export, original CSS/SCSS provenance, scoped browser styles,
module/variable/nesting expansion, native HTML/XML selection and source repairs.
Saved edits, independent copies and offline reruns pass with shared controls and
unchanged resolver authority, source guards and credential policy.

Completed [CEM-QL and XPath data projects and original query-source provenance](../archive/todo-completed-data-query-projects-2026-10-25.md)
cover typed filtering/joins, declared and data-derived parameter maps, native
selected-node ranges, source operations where supported, exact repairs and JSON
table artifacts. Saved edits, independent copies and offline runs pass through
shared controls. Unknown query identities refuse navigation; resolver authority,
saved projects/drafts and credential privacy remain unchanged.

Completed [CEM/XSLT transformations and schema-authoring projects](../archive/todo-completed-transformation-schema-projects-2026-10-25.md)
cover five ordinary projects, native multi-step HTML/JSON artifacts and provenance,
CEM schema/package instance validation, JSON Schema/RELAX NG source authoring and
eight diagnostic repairs. Shared source editing now admits their registered text
types while compiled binaries remain read-only. Saved edits, independent copies
and offline runs pass; source/instance limits, resolver authority, saved drafts
and credential privacy remain explicit and unchanged.

Completed [module maps and portable configurations](../archive/todo-completed-module-config-projects-2026-10-25.md)
cover two ordinary projects and 28 configurations, native v1/v2/v3 deployments,
actual source/output asset hashes, command/run-plan inspection, Studio JSON/CEM
codecs and guarded source archives. Saved edits, independent imports/copies and
offline rebuilds pass with shared controls. Native source-operation limits,
read authority, saved projects/drafts and credential privacy remain explicit.

Completed [native DOM/AST/event projection projects and shared validation](../archive/todo-completed-native-projection-projects-2026-10-25.md)
cover 15 resources and 19 configurations, native CEM-ML inspection, exact explicit
exports, original source ranges and four diagnostic repairs. Shared native
validation, catalog byte admission and editor publication/terminal-newline fixes
pass worker, deployed, saved-copy and offline checks. Native diagnostic selection
uses admitted source buffers; missing inspection-provenance publication, AST JSON
compatibility and binary input/replay remain explicit native follow-ups.
Credential privacy and existing saved projects/drafts are preserved.

Completed [catalog README and declared capability audit](../archive/todo-completed-catalog-readme-capability-audit-2026-10-25.md)
covers all 22 authored projects, generated Schema examples/Feature Tour and the
starter, 203 authored configurations and 208 documented workflows. Deterministic
reference/identity checks, worker/native fixtures and deployed readonly, saved-copy
and offline README/draft checks pass. Registered selector gaps and native limits
remain explicit; no new UI behavior or credential recovery authority is added.

Completed 2026-10-26: [full catalog acceptance](../archive/todo-completed-full-catalog-acceptance-2026-10-25.md)
accounts for all 23 projects, 565 resources and 240 configurations with qualified
native/worker receipts, independent copies, actual builds/previews and offline
coverage. Generic native Markdown now renders and preserves tables; the reported
CEM-QL README preview passes online/offline. Populated-workspace controls and the
combined long navigation run pass without changing admission limits, saved
projects/drafts or credential policy. Native source-operation limits and
concurrent release scheduling remain explicit follow-ups below.

## Native and release follow-ups

Completed 2026-10-27: [native module-asset attribute conformance](../archive/todo-completed-native-module-attributes-2026-10-27.md)
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

Investigated 2026-10-27: [Studio module-load diagnostics](../studio-module-load-diagnostics.md)
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

Completed 2026-10-26: [native CSS-selector source export and shared editor paint](../archive/todo-completed-css-selector-source-export-2026-10-26.md)
covers typed formatter/colorizer ownership, exact lexical source, native ranges,
hard-diagnostic refusal, shared worker/fallback coloring and ordinary Studio
format/color configurations with saved-copy and offline acceptance. Cross-query
provenance and the complete selector-package gate are covered by the completed
follow-up below.

Completed 2026-10-26: [typed XPath Parse, Inspect and source export](../archive/todo-completed-xpath-source-operations-2026-10-26.md)
covers native syntax/token projections, declared shared fields and monotonic
events, borrowed lexical formatter/colorizer ownership, original source ranges
and hard-diagnostic refusal. Five ordinary Studio source configurations pass
shared WASM and deployed saved/offline checks. Caller namespace bindings are
covered by the follow-up below; credential recovery refusal and disabled
automatic recovery stay unchanged.

Completed 2026-10-26: [caller namespaces for XPath source operations](../archive/todo-completed-xpath-caller-namespaces-2026-10-26.md)
covers named/default static context on native owners, original ranges and hard
refusals, CLI and structured input scopes, preserved authored validation argv and
selected-command execution. The ordinary XPath project passes namespace-command
repair, independent-copy and offline checks without changing shared controls or
credential privacy policy.

Completed 2026-10-26: [cross-query original-source provenance](../archive/todo-completed-cross-query-provenance-2026-10-26.md)
covers shared retained CEM input views for CSS-selector, native tree map accessors
for all three query languages, original owner/URI/range retention, literal input
profiles and import guards. The unchanged cross-language equality assertion and
complete selector-package gate pass. Rebuilt WASM, ordinary Studio commands,
Login map equality, saved/offline selector workflows and credential refusal pass
without changing shared declarative UI or enabling automatic recovery.

Completed 2026-10-26: [shared abstract JSON graph byte policy](../archive/todo-completed-json-graph-byte-policy-2026-10-26.md)
covers one native UTF-8/two-space/LF encoder, explicit writer-byte priority,
complete application JSON values and native collection-role separation. Exact
native/worker bytes, large integers, hashes, pinned snapshots and Supplier 1.1.1
fixtures pass. All 250 catalog configurations pass online/offline; shared Studio
saved-copy/preview workflows and credential refusal pass. Native collection
manifest publication is now shared across hosts, as recorded above.

Completed 2026-10-26: [SCSS mixin default-argument binding](../archive/todo-completed-scss-mixin-defaults-2026-10-26.md)
covers captured module variables and earlier parameters, explicit caller-scope
arguments, aliases/forwarding/repeated calls and original Unicode/CRLF ranges.
SCSS stylesheet 1.0.1 now uses the omitted default with unchanged exact CSS.
Native, CLI, worker, saved/offline preview and credential refusal checks pass;
full lexical/function/content-block compatibility remains outside this profile.

Completed 2026-10-26: [file-URI graph module-resolution bases](../archive/todo-completed-file-uri-graph-bases-2026-10-26.md)
covers relative file-URI imports/dependencies, localhost-prefix grants, encoded
names, bounded local globs and safe rooted-folder bindings. SCSS stylesheet
1.0.2 documents native builds without an extra base URI; missing module grants
still refuse publication and original source URI/ranges remain intact. Native,
CLI, rebuilt WASM/worker, saved/offline preview and credential refusal checks
pass. Plain-path configs retain their existing identity behavior.

Completed 2026-10-27: [lossless mixed-quote CEM-ML inspection](../archive/todo-completed-mixed-quote-inspection-2026-10-27.md)
admits line-fenced literal attributes through the shared native tokenizer/writer,
with collision-free fences, exact LF/CRLF payloads, original UTF-8 ranges and
literal ownership. Ordinary quotes/backslashes and control/unterminated-value
refusals remain intact. Native/editor parity and incremental edits pass; the
HTML/CEM card 1.0.1 uses default CEM inspection again. CLI, both WASM bindings,
Studio saved edits, credential refusal and assembled-demo/offline acceptance
pass, with all capabilities, 898 files and four WASMs below 25 MiB preserved.

Completed 2026-10-27: [Login element-template identity and recovery headroom](../archive/todo-completed-login-element-template-identity-2026-10-27.md)
corrects Login 2.8.1's live resource and Parse configuration using the existing
registered selectors, with unchanged source bytes and stable IDs. Native Parse
and Validate succeed with 15 preserved advisory warnings; original generic-source
errors retain their severity and ranges, and old saved identities are not
rewritten. Declarative layout containers keep complete worker/fallback recovery
within the existing DOM/4 MiB guards. Native catalog, editing/live view,
credential refusal, independent saved/offline copies and assembled demos pass.
The separately observed CLI/worker undeclared-binding gap is corrected in the
[Login validation parity work](../archive/todo-completed-login-validation-parity-2026-10-27.md).

Completed 2026-10-27: [JSON/YAML source Parse identities and diagnostics](../archive/todo-completed-structured-data-source-identity-2026-10-27.md)
routes JSON, YAML and JSON Schema Parse/Inspect through retained native import,
with source identity/ranges distinct from AST/events presentation identity.
Literal data remains inert; original native refusals and strict reports survive
without fabricated trees or write failures. Ordinary project configurations,
worker/Node WASM, all 254 native catalog configurations and the complete
online/offline worker catalog pass. Five obsolete JSON/map companion exceptions
are retired; other source limits remain pinned. Complete Login recovery retains
credential refusal, 4 MiB/witness/deadline guards and disabled automatic recovery.
The assembled demo retains 898 files and four WASMs below 25 MiB.

Completed 2026-10-27: [CEMT profiles and writer encoding](../archive/todo-completed-cemt-profile-encoding-2026-10-27.md)
separates package colorizer selection from source-preview encoding in the shared
native writer. Explicit output aliases, legacy HTML requests, selector precedence,
public MIME/schema, original spans and final-newline ownership remain consistent.
The original raw-CEM assertion passes, including stripped CLI; all 548 CLI library
tests pass. Native/WASM, Studio formatting and credential-recovery checks pass
without UI behavior changes or relaxed limits. Deployment retains 898 files and
four WASMs below 25 MiB.

Completed 2026-10-27: [source-operation diagnostics and original owners](../archive/todo-completed-source-operation-diagnostics-2026-10-27.md)
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

Completed 2026-10-27: [Login validation parity](../archive/todo-completed-login-validation-parity-2026-10-27.md)
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

Completed 2026-10-27: [typed command/configuration source adapters](../archive/todo-completed-control-source-adapters-2026-10-27.md)
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

Completed 2026-10-27: [module-map semantics and AST JSON export](../archive/todo-completed-module-map-ast-json-2026-10-27.md)
shares native v1/v2/v3 source checks with deployment planning, retains original
JSON diagnostics and separate paired-map/read boundaries, and aligns explicit
semantic AST JSON exports with their unchanged source validator. Ordinary Studio
projects add semantic repairs and actual-export validation. Native CLI/WASM,
complete native catalog, saved/offline browser, demo and isolated Login recovery
checks pass. CEMT presentation, binary ownership, credential policy and all
recovery bounds are unchanged; native projection decoding/replay remains below.

Completed 2026-10-27: [owned native Parse/Inspect provenance](../archive/todo-completed-owned-parse-inspect-provenance-2026-10-27.md)
binds original and writer bytes to typed native ranges, publishes bounded
artifact-owned references, and connects existing Studio source selection without
reading AST data fields as authority. Native CLI/WASM, complete catalog,
saved/offline navigation and isolated worker/fallback Login recovery pass.
Declaration whitespace and unused view metadata were reduced to keep every
provenance row available within the unchanged node/byte limits. Remaining
formatter/markup endpoints are completed below; native projection input adapters remain.

Completed 2026-10-28: [formatter and HTML/XML conversion provenance](../archive/todo-completed-formatter-markup-provenance-2026-10-28.md)
names original native formatter views, retains typed final markup writer bytes
and spans, and preserves SCSS module/callsite coordinates. The ordinary native
projection project includes XML formatting and CEM-to-HTML/XML configurations
using the existing declarative result/source controls. Native CLI/WASM, all 271
native catalog configurations, saved/offline source selection, demo and isolated
worker/fallback Login recovery pass. Credential refusal and all recovery bounds
remain unchanged. The separate existing artifact-reader trace failure is
recorded below.

Completed 2026-10-28: [native projection-source imports](../archive/todo-completed-projection-source-imports-2026-10-28.md)
loads six explicit JSON/binary identities into retained native common trees,
validates full payloads and sealed chunk hashes/topology, and preserves actual
artifact provenance with passive references/events. Ordinary Studio JSON input
configurations, all 277 native catalog configurations, saved/offline selection,
demo inventory and isolated worker/fallback Login recovery pass. Binary Studio
command admission remains below; credential refusal and all recovery bounds are
unchanged. Deployment retains 898 files with all four WASMs below 25 MiB.

Completed 2026-10-28: [owned binary Studio commands](../archive/todo-completed-owned-binary-commands-2026-10-28.md)
admits explicit saved configurations through shared native byte/identity checks
and execution revalidation. Sources remain immutable, with exact downloads and
read-only declarative controls. All six binary configurations pass online and
offline; native/WASM, all 283 catalog configurations, deployed demos and isolated
Login recovery pass. Layout-only whitespace was removed to retain the unchanged
32,768-unit witness limit. Credential privacy, 4 MiB groups and all deadlines
remain unchanged; deployment stays at 898 files with all four WASMs below 25 MiB.

Completed 2026-10-28: [direct configuration URL text admission](../archive/todo-completed-configuration-text-admission-2026-10-28.md)
shares catalog file admission: available bytes, declared preview policy and full
strict UTF-8 decoding. Missing bytes remain unavailable; invalid/unsupported
sources retain exact downloads, and missing resources preserve selection/drafts.
All 37 navigation tests, deployed configuration/draft/offline checks, demo
verification and isolated worker/fallback Login recovery pass. Existing shared
CEM controls, native parsing, credential policy and all limits are unchanged.
Deployment remains 898 files, with all four WASMs below 25 MiB.

Completed 2026-10-28: [multiline editor-driver attribution](../archive/todo-completed-editor-driver-performance-2026-10-28.md)
reproduces the dominant browser edit/style cost with runtime listeners absent.
The permanent six-case diagnostic records original bytes, events, nodes, latency
and optional CPU profiles while preserving the 30-second action deadline. Live
fill remains 23.9 seconds with 1,987 nodes/events; fresh native paste takes 38 ms
and one node/event. Shared CEM publishes once on both paths. All ten editor
stories (including large native paste/selection/undo), deployed diagnostics,
demo checks and isolated Login recovery pass. Production editor/CSS behavior,
credential refusal, all recovery limits and the 898-file deployment are unchanged.

Completed 2026-10-28: [version-specific module-map lifecycle fixtures](../archive/todo-completed-versioned-module-map-fixtures-2026-10-28.md)
replaces the obsolete shared success buffer with canonical v1/v2/v3 examples and
covers 28 source/admission/refusal cases with original native diagnostics/ranges.
All 103 lifecycle tests, both 33-test module-map cohorts, three CLI checks,
Node/browser-worker WASM, Studio configuration/artifact checks and isolated Login
recovery pass. All Rust edits are test-only; production validation, shared
components, credential refusal and recovery budgets/deadlines are unchanged.

Completed 2026-10-28: [formatter artifact-reader freshness](../archive/todo-completed-formatter-reader-freshness-2026-10-28.md)
traces the eight discovery/execution reads and closes the obsolete four-read
fixture without removing admission checks. The expanded fixture exercises 58
pipeline runs, including late source changes/refusals, re-admission and URI/MIME
cache qualification. Pipeline, asset/cache, native provenance, public WASM,
Studio formatting and isolated Login recovery checks pass. All edits are tests
and documentation; production code, shared components, credential privacy,
cache bounds and recovery limits/deadlines remain unchanged.

Completed 2026-10-28: [constructed document/provenance URI separation](../archive/todo-completed-constructed-document-uris-2026-10-28.md)
fixes the shared native accessor used by CEM-QL and XPath. Constructed string
documents report no document URI while retaining original source owners, ranges,
diagnostics and independent base URIs; loaded-document URIs remain intact.
Native, public WASM, Studio command/provenance, deployed-demo and isolated Login
recovery checks pass. The deployment retains 898 files with all four WASM files
below 25 MiB. Shared components, credential refusal and all recovery
budgets/deadlines are unchanged.

Completed 2026-10-28: [Studio generated WASM glue lint scope](../archive/todo-completed-studio-generated-glue-lint-2026-10-28.md)
excludes only the build-owned `generated/schema-assets/` staging directory.
The original 238 errors/five warnings reproduce before the repair; package-wide
lint now passes with all 86 tracked authored JS/TS files covered. Source, script
and root-config rules and error/warning probes remain intact. Uncached/cached
lint, shared declarative components and isolated worker/fallback Login recovery
pass. Runtime/UI behavior, credential refusal and all recovery limits are unchanged.

Completed 2026-10-28: [Original-owned XSLT compilation](../archive/todo-completed-owned-xslt-compilation-2026-10-28.md).
Shared engine compilation, import discovery, name resolution and every imported
stylesheet now retain the admitted immutable native parser contract. Complete
and external native fixtures, physical-runtime package replay, online/offline
Studio graphs, deployed demos and isolated worker/fallback Login recovery pass.
The stale trusted bootstrap pin is refreshed with strict equality preserved;
credential refusal and all recovery budgets/deadlines remain unchanged. The
malformed-CSV default Parse mismatch found during acceptance is resolved below.

Completed 2026-10-28: [Default Parse refusals and original CSV diagnostics](../archive/todo-completed-default-parse-refusal-2026-10-28.md).
CLI and common command execution now share native refused-presentation policy:
exit 1 with original diagnostics and no output when no source tree is admitted.
Recoverable source inspection, schema-owned classifications and strict output
verification remain intact. Native, Node worker, Studio online/offline, deployed
demo and isolated worker/fallback Login recovery checks pass. The strict census
fixture includes the existing thirtieth package-owner registry; no authority or
recovery bounds change. Relative Node output canonicalization is tracked below.

- [x] Provide owned native XDM handoff for constructed transformation-stage results. [Implementation and verification](../archive/todo-completed-native-xdm-handoff-2026-10-28.md).
  CEM/XSLT output now supplies one bounded native document to XSLT, preserving
  source owners and typed contracts without encoding/reimport. Native forest
  roots and generated-template provenance are corrected; the ordinary CEM
  project demonstrates saved edits and offline previews. Native, Node, full
  Studio, optional-profile, deployed-demo and isolated worker/fallback Login
  recovery checks pass. Credential refusal and all recovery bounds are unchanged.

- [x] Make full schema-package validation honor supplied resource readers for local identities. [Implementation and verification](../archive/todo-completed-schema-package-reader-2026-10-28.md).
  Artifact, converter-template and example reads use the supplied reader before
  filesystem resolution; refusals retain full validation diagnostics. Legacy
  no-reader filesystem checks, metadata-only admission and strict runtime
  artifact consumption are preserved. Native, CLI, public WASM, Studio worker,
  isolated credential recovery and deployed/offline demo checks pass.

- [ ] Diagnose relative Node `--out` destination canonicalization before common command-service URI admission. `createNodeCommandService({ cwd })` plus `parse inventory.csv --out inspection.cem` currently refuses `runPlan.outputs.resolvedDestination=inspection.cem` as `cem.command_service.uri_invalid` before reaching Parse; an explicit `file:` destination works and preserves refused output. Add lowering/Node/executable fixtures first, determine the supported argv-path contract and retain strict prepared-URI/publication authority checks. Keep this separate from native Parse refusal policy.

- [ ] Diagnose the three existing linked package preparation source-map failures at `schema_package_lifecycle/attribute_datatypes.rs:473`: pretyped handoff, callback context/publication guards and checked preparation replacement retain an additional `SourceBoundary` frame. All three reproduce on unchanged `eca54d20`; preserve source provenance and intended assertions when resolving the mismatch. The broader cohort passes 46/49 cases; see the [CSV contract verification](../archive/todo-completed-csv-parser-contracts-2026-10-24.md#verification).

- [ ] Isolate the optional Studio sample graph from the ordinary Feature Tour generated directory. `build-schema-assets-profile.mjs` leaves `generated/feature-tour/feature-tour.schema-assets.cem`, causing the subsequent strict default `generate-feature-tour.mjs --check` file-set assertion to fail. Add default → optional → default verification fixtures, move the optional build input into its own generated domain with correct relative resource bases, and retain exact default file-set checking. Removing only that untracked build scratch file permits default verification; saved projects/drafts are unrelated.

- [ ] Diagnose the intermittent worker-static-change fresh-allocation timeout in StaticPreviewRefusals observed in the frozen 111-case credential-policy cohort (110 pass); the unchanged isolated case passes. Preserve the original five-second allocation deadline and static-presentation refusal assertions.

- [ ] Repair standalone runtime typecheck preparation for the transitive CLI generated schema: retain or restore the exact native-generated TypeScript input through the CLI build/cache contract, so `tsc --build --emitDeclarationOnly` works without explicit schema preparation. Keep this separate from semantic editor/widget recovery.

- [ ] Stabilize concurrent release-verifier scheduling around the existing 30-second cold Login source-paint case; retain its assertions and deadline. The unchanged complete release passes with `--parallel=1`. The 283-case Studio source invocation overlapping deployed browser gates passes 276 cases with seven deadline failures; all seven pass in an isolated Nx rerun, including complete live Login recovery on worker/fallback. Pending-build cancellation then passes three isolated runs after public navigation-readiness ordering is repaired. Keep the original five-second resume deadline, test bounds, privacy assertions and 4 MiB checkpoint budget; do not treat overlapping execution as a green full-cohort invocation. The typed control-source acceptance adds a concurrent fallback `cem.checkpoint.resume.timeout` at resume 0 (worker passes); its unchanged worker/fallback rerun passes 2/2 in isolation. Preserve this separate scheduling evidence; see the [control-source verification](../archive/todo-completed-control-source-adapters-2026-10-27.md#verification).

- [ ] Repair the existing native conversion baseline independently of recovery: `cross_surface_conversion_rules::cem_fixture_preserves_anonymous_scope_expression_avt_comments_and_rich_content` loses the expected `{.busy}` value in both default and stripped tests. Preserve the intended native semantics and assertion when resolving the mismatch. The separate CEMT profile/writer mismatch is completed above.

- [ ] Investigate the existing stripped WASM binary-size gate: current stripped output is 27,264,508 bytes versus 27,033,499 default bytes, despite the stripped capability contract rejecting debug control. Establish actual debugger-code omission and the correct size baseline without weakening the gate to hide retention.

## Amsterdam presentation samples: active Studio priority

The Login, Supplier-pipeline and ordinary Markdown + CEM-ML site projects are
implemented in the shared workspace. Preserve projects/resources/configurations,
source provenance and independent copies. The
[required tutorial checkpoints](../declarative-amsterdam-2026/presentation.md#required-tutorial-checkpoints),
[Login demonstration](../declarative-amsterdam-2026/presentation-plan.md#required-login-demonstration-and-companion-examples)
and [sample audit](../presentation/declarative-amsterdam-2026-handoff.md#studio-sample-audit-2026-10-17)
remain the scope references. Completed sample work is in the
[archive](../archive/todo-snapshot-2026-10-23.md#amsterdam-presentation-samples-active-studio-priority).
These remaining diagnostics/helper actions do not lead the functionality queue.

- [ ] Diagnose the existing `DOM Stylesheet Adoption / Readiness And Native Diagnostics` held-worker timeout without weakening readiness assertions. The stylesheet ownership work reproduces the 30-second timeout with the unchanged `6f1c8dca` registry as well as the repair, both alone and in the sequential browser group; the other two DOM-adoption cases pass. Keep this separate from completed replacement/collection ownership checks.

- [ ] Audit the five pre-existing strict schema-definition diagnostics in the Studio project package (`id` pattern datatype and `path`/`reference-path` prefix facets); preserve its native portable project contract. The explicit-entry work verifies the same baseline diagnostics before/after and introduces no additional one.

## Declarative UI Architecture Correction

### Remaining declarative UI migration

The canonical component migration is complete. Keep these unresolved diagnostic
follow-ups separate from completed migration and checkpoint acceptance. Follow
the [declarative UI principle](../declarative-ui-principle.md) and
[component development pattern](../component-development-pattern.md).

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

- [ ] Verify both pinned [ARIA profiles](../cem-element-aria-reference-profile.md)
      in the documented browser/assistive-technology matrix. Record actual
      browser/OS/AT versions, accessibility relation exposure, ordered/repeated
      details targets and error-message announcements. Keep untested combinations
      explicit and the draft profile opt-in. Single-relation accessibility APIs
      may expose only the first target; exported DOM sequences must stay intact.

## Autocomplete and suggestions design for CEM inputs

The shared controller, declarative field composition, native profiles and
host-integrated fixtures are implemented. Remaining acceptance uses the
[suggestions acceptance record](../../packages/cem-components/docs/suggestions-acceptance.md)
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
      [script/network/frame capture](../studio-module-load-diagnostics.md) to retain
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
      [archived investigation](../archive/todo-snapshot-2026-10-10.md#browser-failures-observed-during-placement-verification).

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
[current boundary inventory](../browser-stabilization-review.tmp.md#reconciliation-of-the-remaining-readiness-inventory).
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
      [NPM/location evidence](../browser-stabilization-review.tmp.md#npm-and-location-readiness-under-overlapping-load).

- [ ] Capture HTTP's historical 300-frame initial article-count timeout using
      the existing opt-in lifecycle/per-card tracing, which rethrows the original
      error. Distinguish declaration/library startup, current rendering and
      future HTTP completion before choosing a correction. If startup polling
      is premature, await existing lifecycle settlement before checking each
      authored instance; preserve resource assertions and the 30-second story
      deadline. Traced follow-up workloads pass without reproducing the failure;
      see the [HTTP observation](../browser-stabilization-review.tmp.md#http-startup-observation-without-a-reproduced-failure).

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final [Phases 10 and 11](../../roadmap.md#phase-10---figma-ui-kit). Their
checklists live only in the roadmap. Finish and verify `cem-elements` and the
declarative `cem-components` migration first, then the remaining non-Figma phases
through Phase 9 release governance.

- [ ] Implement the general whole-page DCE CLI compilation pipeline from the
      [whole-page SSR proposal](../cem-ml-cli-ssr-proposal.md), using config-driven
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
[presentation handoff](../presentation/declarative-amsterdam-2026-handoff.md) after
clearing chat context; it records the starting material, implementation
checkpoint and return path. The existing
[session proposal](../declarative.amsterdam.2026.md) is the starting brief. The
[working presentation document](../declarative-amsterdam-2026/presentation.md)
records the 2026-10-11 program/discussion review, current agenda and
[presentation minutes](../declarative-amsterdam-2026/presentation.md#presentation-minutes).
The user requested adoption-focused framing on October 10 and alignment with
the public agenda on October 11. Use the pitch to explain why the promised
document activities matter; preserve the login walkthrough, platform subjects,
and six proposal intervals. Correspondence and release checks remain open.
The [presentation plan](../declarative-amsterdam-2026/presentation-plan.md) owns
ideas, preliminary design, and adoption planning; the
[platform pitch](../declarative-amsterdam-2026/cem-platform-pitch.md) owns the
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
[prior snapshot](../archive/todo-snapshot-2026-10-23.md#cem-studio-delivery-planning).
Native parameter maps, shared form controls and their verification are in the
[graph authoring completion record](../archive/todo-completed-graph-parameters-2026-10-23.md).

Continue with persistent native package activation, opt-in deployment/body
exclusion and measurements under [Project builds and artifact previews](#project-builds-and-artifact-previews).
Preserve the [bounded project lifecycle](../cem-studio.md#next-delivery-milestone-user-owned-project-workspace):
100 graph nodes / 1 MiB source and 20 history snapshots / 8 MiB. Authored ID
changes, aliases and expression-valued attributes remain source-only. Keep
credential recovery refusal, the aggregate 4 MiB checkpoint budget and separate
physical-device/spoken AT acceptance.

## Component application WASM profile (separate target)

Planned 2026-10-28 at the user's request. Implement the
[profile design](../cem-components-wasm-profile.md) and
[roadmap workstream](../../roadmap.md#313-minimal-component-runtime-wasm-profile) as
an explicit bundle for `cem-elements` plus `cem-components`, leaving existing
full-runtime/CLI/Studio targets and defaults intact. No profile target is created
by this planning update. Start with the audit and native boundary proof; establish
size budgets from measurements before extraction is accepted. Preserve every
component capability, shared UI ownership and the credential policy above. Keep
the current Studio execution order; run this as a separate component-library
workstream when selected.

Clarified 2026-10-29: prioritize the app build/compiled-load boundary below over
broader benchmark and optional-authoring-module work. Start with shared compiled
UI + JSON for the shopping-cart app; the catalog remains the complete capability
inventory, not the minimum set loaded by every app.

- [x] Refine the deployment design around app-specific compiled UI plus JSON,
  using a shopping cart with predefined GraphQL operations. Separate build-only
  source validation/compilation from required runtime evaluation, JSON decoding
  and app-specific dynamic-input checks. Keep all other component capabilities
  available on demand rather than forcing the full catalog into initial core.
- [x] Add a shopping-cart reference fixture with an explicit dependency manifest,
  canonical shared-component roster, native CEM-ML view, illustrative GraphQL
  schema/operations, JSON cart/update/remove/empty/partial-error/failure payloads
  and a README listing essential files. This is an executable native deployment
  contract, not a functioning storefront or a registered Studio project yet.
- [x] Prove native build-time compilation and source-free artifact reload for
  that fixture in dev/prod modes. Assert changed JSON updates the same compiled
  view, native source ownership survives import, text is escaped, missing/error
  results render correctly, manifest/component drift is refused, and compiler
  profiling records no CEM-ML/CEM-QL compilation during reload/render. Retain
  hash/binding/version refusal and document CSS/browser-source dependencies.
  See the [shopping-cart reference](../cem-components-shopping-cart-profile.md).
  Four cart tests and nine existing artifact reload tests pass; both dev/prod
  runs render six response states with zero instrumented source-compiler calls.
  Runtime general-registry assembly still occurs
  and remains a partitioning requirement. Production runtime, shared UI and
  credential handling are unchanged; no reduced WASM or browser app is claimed.
- [x] Add strict native manifest-pinned template admission and a source-free
  processing input shared by worker/fallback hosts. Add native/WASM and shared
  host fixtures for missing/corrupt/mismatched hashes, versions, bindings and
  scope, no source/compiler fallback, input mutation, cache identity, superseded
  work, native ownership/disposal and replay into fresh processing owners.
  Preserve the existing optional-cache source compilation path and test its
  compatibility. This transport step does not yet register a whole application.
  Implemented `importPinnedTemplateArtifact`, the shared render adapter and
  `pinned-template-artifact-v1` input under processing protocol v30. Bounds are
  checked before buffer copies; only accepted inputs are published for replay,
  so rejected replacements preserve the committed template. Verification: six
  native app-boundary tests, nine existing reload tests, separate-heap WASM
  admission/disposal, 808 shared unit tests, worker/fallback browser admission
  and live Login privacy/recovery acceptance. Native CSS parsing, source-driven
  declaration registration and the full WASM remain in place. No generated
  screenshots/attachments or user storage are included in this work.
- [x] Connect source-free compiled artifacts to shared component registration,
  worker/fallback processing and recovery, using manifest-pinned artifact/source
  hashes rather than downloading the original template source. Preserve all
  existing source-driven/full-runtime entry points. Prove a compiled stylesheet
  path or retain required native CSS parsing until that boundary is complete.
  - [x] Generate hash-pinned native declaration metadata alongside template IR,
    preserving tag/alias, version, intrinsic capability, scope, original URI and
    XML bases, link policy, module maps and binding/source-map contracts. Add
    native fixtures for canonical cart components, metadata tampering, unsupported
    declarations/imports, capability precedence and build errors.
    Add a reproducible native fixture builder and separate-heap WASM checks for
    metadata admission, invalid pins and binary-owner disposal.
  - [x] Add shared registration from admitted metadata and binary bytes; reuse
    collision checks, form association, retained styles, rendering and recovery.
    Add worker/fallback browser fixtures for canonical field/action composition,
    defaults, edits, FormData, scoped styles, source-loader refusal, failed pins,
    registration conflicts, disconnected/remounted declarations and scope disposal.
    Implemented native `cem-declaration-metadata/1`, the build example and
    `registerCompiledDeclaration`; both worker/fallback paths use strict pinned
    IR. Four native declaration tests cover five canonical components in dev/prod,
    aliases/bases/capabilities, module maps, strict metadata and build refusal.
    Separate-heap WASM admission/disposal, 808 units and six browser stories pass.
    Both complete live Login worker/fallback recovery/privacy cases pass within
    the existing 4 MiB budget. Credential refusal and deadlines are unchanged.
    Existing CSS parsing and source-driven registration remain supported.
  - [x] Add declarative compiled-asset loading and a generated complete component
    closure; prove whole-app recovery from that source-free deployment. Keep
    XPath companions/external CEMT imports and legacy routes refused until their
    compiled contracts exist. Do not claim a reduced runtime from metadata alone.
    - [x] Add declarative compiled metadata/binary URLs and an independent metadata
      hash to `cem-element`; share bounded transport and native admission. Add
      transport unit fixtures and worker/fallback browser fixtures for pin/network
      refusal, retry, URL/header changes, scope disposal, source-loader refusal,
      real forms and preserved edits. Keep recovery budgets/privacy unchanged.
    - [x] Generate a bounded static declaration dependency closure and loader HTML
      natively. Add fixtures for transitive/conditional references, missing or
      mismatched dependencies, cycles, duplicate/oversized graphs and unsupported
      dynamic declaration construction; support explicit projected-content roots.
      Implemented the native-only closure builder/example and shared loader;
      a generated six-component form uses canonical components in worker/fallback
      browsers without XHTML downloads. Three native closure tests, five bounded
      transport tests, and real pin/HTTP/import-map/retry/edit/disposal browser
      checks pass. Verification also passed 813 shared units, eight component
      browser cases, both complete live Login privacy/recovery cases, native
      declaration/reload checks, separate-heap WASM checks and the refreshed audit.
      The full WASM is unchanged at 24,076,286 bytes; no code omission is claimed.
      The helper derives static render-tag dependencies, including
      branches, not arbitrary query results or a runtime function/resource closure.
    - [x] Prove fresh-generation checkpoint/recovery for the complete declaratively
      loaded compiled form, including changed data, edits, focus/selection and
      source-free reimport. Keep credential blocking, 4 MiB and existing deadlines.
      Add isolated worker/fallback browser fixtures for two complete retire/allocate/
      resume cycles, fresh publication and changed input, exact DOM/styles and
      FormData, credential refusal/clear, no component refetch/compiler fallback,
      and rejection of newer property edits or changed declaration pins.
      Six isolated cases pass: worker/fallback each perform two real complete
      retire/allocate/resume cycles, or refuse a newer property edit/changed pin
      without altering retained controls or committed bytes. All six application
      owners publish fresh state and accept later edits; scoped styles, FormData,
      exact control identity, focus and selection survive without component
      refetch or source-compiler fallback. Password/OTP capture and ticket issuance
      refuse property-only and accepted secrets, then recover after explicit clear
      with empty authored/reset defaults. A previous safe checkpoint may remain;
      refusal cannot replace it with credential-bearing bytes. Existing budgets,
      5-second native handoff deadlines and runtime implementation are unchanged.
      This closes the reviewed static six-component contract. Unsupported source
      companions/imports/dynamic construction remain explicit build refusals, and
      full-runtime source loading continues to provide those capabilities.
      Verification: 14 component browser cases, 813 shared units, both complete
      Studio Login privacy/recovery cases and the unchanged native/WASM audit pass.
- [x] Add native compiled stylesheet/scoping admission with pinned source and
  artifact identities, URL bases and scope contracts; prove equivalent imports,
  diagnostics, DOM/style reuse and separate-heap worker/fallback recovery. Keep
  native CSS parsing in the deployed profile until the compiled replacement passes.
  - [x] Add a bounded native compiled CSS tree capsule, preserving exact source
    identity/ranges and decoded nodes. Add native fixtures for deterministic
    round trips, parser-call refusal, corrupt/hash/version/graph failures and
    identical scoping/resource/import diagnostics.
  - [x] Add an opt-in compiled-style template format, strict v2 payload admission
    and build-generated declarations; retain legacy v1 and native source loading.
    Add native/WASM, shared processing and worker/fallback load/recovery fixtures
    proving pinned format agreement, no inline CSS reparsing, retained DOM/styles,
    source URI bases and existing credential/budget policies.
    Implemented `CEMCSS1` native tree capsules and strict `cem-template-artifact/2`
    with legacy v1 compatibility. The existing shared transport compares actual
    native format results before publication and replay; missing/invalid compiled
    styles never reparse source as a fallback. Node/string/source-map interning
    keeps the generated six-component fixture under the unchanged 1 MiB response
    ceiling (the largest artifact is about 544 KiB). Native tests prove identical
    nodes, source keys/ranges, all three scope modes, import/URL resolution,
    effective identities and diagnostics, with zero inline CSS-parser calls on
    capsule admission. Dictionary expansion, cycles, invalid indices, versions,
    body hashes and source-only payloads fail closed. Original CSS is retained
    for provenance/inspection. Full/source/dynamic CSS and authorized text/css
    imports remain supported; the import-closure gate below stays open.
    The optimized full reference is 24,172,150 bytes (23.05 MiB), about 94 KiB
    above the preceding build, with the same 28 imports/171 exports. Sharing the
    v1/v2 slice-reader decoder avoided about 1.04 MiB of duplicated decoder code
    in the intermediate build. No native source-parser code has been excluded.
    Verification passes: 21 native CSS/template/declaration/closure/reload tests,
    814 shared unit tests, 14 component browser cases, separate-heap v2 stylesheet
    admission/emission plus legacy v1/instance stylesheet WASM checks, both full
    Studio Login credential/recovery cases, TypeScript, lint (no errors) and the
    refreshed size/interface/source audit. The 4 MiB checkpoint budget, source
    loader limits and 5-second native handoff deadlines remain unchanged.
  - [x] Admit compiled imported stylesheet closures before excluding CSS source
    parsing; preserve current authorized text/css imports until that gate passes.
    - [x] Add native fixtures for compiled full stylesheet capsules and transitive,
      repeated and conditional import closures; prove parser-free admission,
      equivalent emission/provenance, URL/MIME/SRI/byte policy checks and refusal
      of missing, corrupt, remapped, cyclic or oversized dependencies.
    - [x] Add strict opt-in v3 template/declaration closure builds and generated
      imported-CSS form fixtures; verify shared worker/fallback loading and real
      generation recovery with unchanged credential and resource budgets.
    - [x] Add CLI build fixtures refusing outside-root URLs, symlink escapes,
      missing and oversized CSS files before publishing any closure assets.
    Implemented `CEMCSS2` full-stylesheet capsules and an opt-in
    `cem-template-artifact/3` build (`--compiled-css`). Inline fields keep their
    scoped-block capsule contract; v1/v2 source imports remain supported. Native
    build loaders are explicit, and the CLI limits CSS to the canonical source
    directory. Runtime replay uses the existing resolver and import state machine,
    rechecking final URLs, MIME, original-byte SRI, cycles, depth and byte bounds.
    Missing/unused responses, changed bases or mappings, corrupt capsules and
    policy failures refuse without source fallback. Emission, diagnostics,
    resource rewriting, style reuse and recovery use the existing shared paths.
    The form's transitive/repeated conditional imports take effect without CSS
    fetches and survive two actual worker/fallback generation replacements.
    No application/Studio UI JavaScript or credential-policy exceptions were added.
    Verification passes: 92 native cases (including 52 existing source-CSS cases),
    four CLI refusal fixtures, 815 shared units, 14 component browser cases,
    separate-heap v2/v3 and legacy/instance stylesheet WASM checks, both complete
    Studio Login credential/recovery cases, TypeScript, lint (0 errors; 818
    warnings), eight audit-tool cases and the refreshed native/source audit.
    The optimized full WASM is 24,197,177 bytes (23.08 MiB), up 25,027 bytes;
    gzip-9 is 7,373,853 bytes and Brotli-9 is 5,418,257 bytes. Its 28 imports and
    171 exports are unchanged; canonical/vendored copies match. All generated
    form binaries stay below the unchanged 1 MiB response limit. The 4 MiB
    checkpoint budget and 5-second handoff deadlines stay unchanged. Native CSS
    parsing remains in the full deployment; core code partitioning is still open.
- [ ] Add app-build dependency generation and GraphQL schema/operation validation,
  including variable/response contracts and compact runtime input checks. Treat
  application manifests as requirements, not native grants or permissions; retain
  JSON decoding and validation for API/user data arriving after deployment.
  Generate the selected runtime function/behavior/module closure from native
  compiled metadata rather than assembling the complete general registry.
  - [x] Generate bounded native build requirements from compiled template/query
    IR and retained CSS closures. Add fixtures for branch/lambda/attribute calls,
    import aliases and arities, data-format demand, dynamic/native/provider
    blockers, malformed IR, deterministic hashes and no query execution or
    query-source reparsing. Emit pinned requirements beside the declaration-closure assets.
    Add CLI fixtures for deterministic pinned sidecars and strict refusal of
    open requirements before any output is published.
    Implemented native-only `application_requirements` and pinned
    `requirements.json`/`requirements.hash` sidecars. Reports bind exact declaration
    metadata/source/template hashes, roots/component edges, call module/name/arity
    and dispatch kind, literal format/schema demands and retained CSS source/URL
    requirements. All branches, attributes and function bodies are conservatively
    inspected without query execution. Host/provider, shared-behavior, dynamic
    dispatch, HTML resource/style and source-import gaps stay explicit; optional
    `--require-static-requirements` refuses them before output publication.
    `nativeLinkClosureVerified` remains false. Selected transitive implementation
    closure, parser/profile validation and native registry generation remain open.
    Verification: ten new native inventory cases, 17 declaration/closure/reload
    regressions, six CLI admission/refusal fixtures, 14 component browser cases
    (including real worker/fallback credential-safe recovery), eight audit-tool
    tests, Rust formatting and script lint pass. Generated native template bins
    and existing declaration JSON remain byte-identical. A rebuilt post-bindgen
    WASM exactly matches the preceding optimizer input (27,480,746 bytes,
    SHA-256 `d2623d68bbab6e0e70c4b47a110c7067e3e66c3c6ff734503c12353ac534c5dc`);
    native JavaScript/types also match. Build-only code is excluded on wasm32:
    shipped WASM stays 24,197,177 bytes (23.08 MiB), with unchanged ABI, ownership,
    source loading and credential policy. The regenerated source audit matches.
  - [ ] Map the collected function/module requirements to transitive native
    implementation dependencies and build a selected registry. Keep unresolved
    dispatch/provider/source requirements explicit; inventory is not linked-code
    omission evidence or runtime authority.
    - [x] Add a native selected-registry prototype for reviewed pure sequence/text
      functions, with explicit transitive execution-unit requirements. Add fixtures
      proving default/selected output equivalence, callbacks and pipeline checks,
      denial before argument effects, cancellation and cache-receipt refusal.
      Render the existing cart JSON cases with zero full-registry assemblies.
      Add deterministic pinned CLI recipe/Rust generation and refusal fixtures;
      keep unreviewed functions and behavior/provider/source contracts open.
      Implemented immutable selection for 15 reviewed pure functions and a native
      dispatcher sharing ordinary value/callback helpers. Exact module/name/arity
      and pipeline adapters are checked before argument evaluation; native/URL
      special routes, nested calls and preexisting receiver chains cannot escape
      selection. The cart entry renders all six native JSON fixtures with identical
      HTML and zero full function-catalogue assemblies. Transitive execution-unit
      closure/source paths are recorded, including recursive evaluator/lambda
      contracts; this is not a complete native symbol or link-liveness proof.
      Optional `--selected-registry` emits pinned JSON plus generated Rust only
      for closed/reviewed requirements; CLI fixtures compile and execute that Rust
      and reject open/unreviewed projects before writing assets. Selected renders
      cannot obtain read receipts or hit/populate pure-render caches; an existing
      full-registry cache hit cannot hide a refusal. Full-runtime defaults and
      worker/fallback admission/ownership remain unchanged. The prototype is
      native-only; `nativeLinkClosureVerified` stays false.
      Verification: 166 native library tests (four existing profiling cases
      ignored), 45 declaration/reload/style/cache regressions, seven CLI cases
      plus generated-Rust compilation/execution, bridge compilation, native WASM
      admission/legacy stylesheet checks, 14 component browser cases, both full
      Studio Login privacy/recovery cases, eight audit-tool cases, formatting and
      script lint pass. The rebuilt full WASM remains 24,197,177 bytes (23.08 MiB),
      with 28 imports/171 exports; gzip-9 is 7,373,854 bytes and Brotli-9 is
      5,418,366 bytes. Source/interface/vendor audits pass. The 4 MiB checkpoint
      limit, 5-second native handoff deadlines and entered-credential refusal
      policy are unchanged; no application/component UI JavaScript was added.
    - [x] Add an isolated selected component WASM prototype and native/WASM
      fixtures for pinned v2/v3 admission, six changing JSON responses, malformed
      input, denied functions, missing build selection and independent heaps.
      Measure its actual interface and linked bytes against the full runtime;
      keep provider/type/source closure and complete component integration open.
      Add JSON-only native-import parity/ownership/diagnostic fixtures before
      using that shared boundary to avoid the all-format runtime dispatcher.
      Implemented opt-in `component-runtime` Cargo/build-script mode with separate
      target/output directories, build-owned generated Rust selection (empty and
      default-deny without a recipe), one compiled v2/v3 native JSON/render entry,
      no full tooling exports and no evaluator general/native/URL/chain fallback.
      Shared JSON-only import preserves ordinary parser/limits/projection, source
      identities and native ownership; no JS object/AST handoff or UI behavior was
      added. Input bounds, native admission/memory accounting, controlled render
      timeouts and selected-cache refusal remain active. This is an entry-template
      prototype: CSS installation, shared component/provider ownership, generated
      input/type contracts and recovery integration are still open. The fixture's
      selection is build-input/behavioral evidence, not an embedded registry stamp
      or complete application link proof; `nativeLinkClosureVerified` stays false.
      Matched Binaryen 132.0.0 -Oz builds: prototype 10,301,667 bytes (9.82 MiB),
      gzip-9 2,855,039 and Brotli-9 2,156,757; full default 24,198,709 bytes
      (23.08 MiB), gzip-9 7,376,804 and Brotli-9 5,422,643. Raw reduction is 57.4%;
      code payload 21,438,119 -> 8,029,605 and data 2,731,590 -> 2,260,125 bytes.
      Prototype has 10 imports/9 exports (one app entry plus support), full has
      28/171. This measures whole-file/section omission, not individual languages.
      Verification: 166 native QL tests (four existing profiling cases ignored),
      two native default-deny/admission tests, 19 shared import regressions,
      45 declaration/reload/style/cache cases, CLI recipe/Rust fixtures and bridge
      compilation pass. Final optimized WASM matches all six native cart outputs
      on two independent heaps and rejects bad pins, JSON, sizes, v1/source input
      and unselected functions. Full native CSS/pinned-admission WASM checks,
      14 component browser cases, both focused full Studio Login worker/fallback
      recovery cases, eight audit-tool cases, formatter/script lint and matching
      source/interface/vendor audits pass. Entered-credential refusal, the 4 MiB
      checkpoint budget and five-second native handoff deadlines are unchanged.
    - [ ] Complete the separate application WASM target beyond the bounded
      compiled-template/JSON prototype. Close required shared behavior/provider/
      source and native input/type contracts before treating a component app as
      complete; measure remaining linked roots without claiming per-language
      omission from whole-file size alone.
      - [x] Add retained compiled-declaration owners to the selected profile,
        with native/WASM fixtures for metadata/template pins, scoped v2/v3 CSS,
        closed-import policy refusal, consumer release and disposal. Add a shared
        cem-elements generation/stylesheet-lease adapter and browser fixtures for
        native CSS installation, independent heaps, foreign-root refusal and stale
        lease cleanup without loading the full WASM bootstrap. Keep unsupported
        widget/provider/recovery contracts explicit and fail closed.
        Implemented pinned passive `CompiledDeclarationOwner` admission, actual
        format/module-map/scope checks, repeated native JSON rendering, closed
        compiled CSS replay and native consumer leases. Unsupported capabilities
        and source fallback refuse. Internal per-attempt consumers bound failed
        load bookkeeping and preserve old valid output. Counter-only inventories
        include inactive-but-unfreed native allocations; limits are 64 owners,
        64 styles and 128 leases per owner, with existing native admission/memory
        guards. Shared `cem-elements/component-generation` exposes asynchronous
        isolated bindings and existing DOM stylesheet-lease installation. It
        hides native objects, blocks disposal during calls/new roots during close,
        handles reentrant/stale cleanup, and refuses foreign allocations before
        retiring binding caches. Chromium's >8 MiB sync-instantiation limit is
        respected with async instantiation, without browser flags.
        Verification: seven profile native tests, 166 full native tests (four
        existing profiling cases ignored), 42 declaration/reload/style/cache
        regressions, native fixture generation, two factory/eight audit tests,
        optimized profile/owner WASM acceptance, shared pinned/CSS WASM checks,
        15 component browser cases and both focused full Studio Login recovery
        cases pass. Browser acceptance confirms private/shared/imported styles,
        stable old nodes on failed replacement, disconnect cleanup and one reduced
        WASM fetch without full/source downloads. Node acceptance covers separate
        heaps, foreign inactive roots and busy/reentrant close fences. Rust format,
        script/TypeScript lint, package build and fresh source/vendor audits pass.
        The freshly built full target's post-bindgen bytes exactly match the
        existing optimizer comparator input; full WASM remains 24,198,709 bytes.
        Extended prototype: 10,826,394 bytes (10.32 MiB), gzip-9 3,053,945 and
        Brotli-9 2,300,625, 10 imports/17 exports; whole-file reduction is 55.3%.
        Code/data payloads are 8,540,321/2,273,432 bytes. No individual-language
        omission or complete application closure is claimed. The adapter is not
        yet the CEM processing worker/backend or a recovery endpoint; widget/
        provider/attribute/slice bindings and checkpoint/journal handoff stay open.
        Credential privacy, the 4 MiB checkpoint budget and five-second native
        handoff deadlines remain unchanged; no component/application UI JS added.
      - [x] Fix explicit CSS stylesheet response entry-mode forcing. Recognized
        `mode=stylesheet` now precedes `:host`, `@scope` and `scope` inference;
        `entry=stylesheet` works when `mode` is absent. Existing default inference
        and mode-over-entry precedence stay intact. Original MIME, UTF-8 source,
        URI, source keys and byte ranges survive compiled response admission;
        full-stylesheet capsule restrictions and current resource policy remain.
        - [x] Add parser precedence/default fixtures, native import/capsule MIME
          and source-identity parity/refusal fixtures, source-versus-compiled
          import replay/emission fixtures and explicit-`:host` owned-declaration
          WASM/browser acceptance before marking the mode fix complete.
        Verification: 77 CSS/SCSS and 20 import native tests, seven profile owner
        tests, 166 full QL tests (four existing profiling cases ignored), 42 focused
        declaration/reload/style/cache regressions, bridge check, native fixtures,
        ten factory/audit unit tests, full/profile WASM acceptance and 15 component
        browser cases pass. Both full Studio Login privacy/recovery cases pass
        (worker and fallback). Package build, Rust formatting and fresh matched
        source/vendor audits pass. Reduced WASM: 10,826,438 bytes (10.32 MiB),
        gzip-9 3,053,960 and Brotli-9 2,300,840; full comparator: 24,206,012 bytes,
        gzip-9 7,377,289 and Brotli-9 5,424,626. Interfaces stay 10/17 and 28/171
        imports/exports respectively; whole-file reduction stays 55.3%. This does
        not complete application link closure, widget/provider bindings or recovery
        handoff for the reduced backend. Credential privacy, the 4 MiB checkpoint
        budget and five-second handoff deadlines remain unchanged. No component
        or application UI JavaScript was added.
      - [x] Bind native attribute and slice inputs to passive compiled declarations
        in the reduced ABI, using the shared CEMV codec and native TemplateData
        binding APIs. Bound bytes/values/depth/bindings and refuse duplicate,
        reserved, malformed or mismatched inputs without retaining failed state.
        `renderInputs` takes one opaque multi-root CEMV artifact and routing
        controls, with no JSON data/AST adapter. Attribute wrappers must match;
        slices keep native roots or unwrap an explicitly named attribute. Typed
        values, references, source maps/keys and native identity remain intact.
        Explicit attribute/slice envelopes make input order irrelevant. Inputs
        live only for the call, with 1 MiB encoded/accounted graph budget, 16 KiB
        controls, 64 bindings, 16,384 values/references, depth 64 and five-second
        native execution control. Shared cem-elements guards disposal/reentrancy;
        unsupported widget/provider capabilities still refuse admission.
        - [x] Add native input parity, order-independent attribute/slice channels,
          typed/reference/provenance, refusal and post-failure fixtures, plus
          selected-WASM and shared generation browser acceptance with independent
          heaps and no full/source runtime download. Per-story network evidence
          excludes earlier stories while still requiring exactly one profile
          fetch and no source/full downloads for each scenario.
        Verification: 11 profile native tests, 166 full native tests (four existing
        profiling cases ignored), 30 declaration/style/cache regressions, native
        fixture generation, ten factory/audit unit tests, optimized profile/owner
        WASM acceptance and 16 component browser cases pass. Both full Studio
        Login privacy/recovery cases pass in a quiet rerun; an initial concurrent
        release-build run hit existing resume deadlines. No timeout was increased.
        Package build, Rust formatting, script/TypeScript lint and fresh matched
        source/vendor audits pass. Reduced WASM is 11,153,481 bytes (10.64 MiB),
        gzip-9 3,142,830 and Brotli-9 2,365,084, with 10 imports/18 exports.
        Code/data payloads are 8,862,296/2,278,291 bytes, a 53.9% whole-file
        reduction against the unchanged 24,206,012-byte full comparator. Linked
        application closure is still unverified. Complete data-island/receiver
        contracts, multiple independent input artifacts, widget/provider ownership
        and worker/recovery handoff remain open. Credential privacy, the 4 MiB
        checkpoint budget and five-second handoff deadlines remain unchanged;
        no component/application UI JavaScript or credential capture was added.
      - [x] Bind host-owned instance identity and prove compiled receiver
        declarations through the reduced native input ABI. Preserve existing
        no-context calls, receiver conversion/constraints/defaults and refusal
        without widening widget/provider or island admission.
        The optional third `renderInputs` argument delegates to a separate native
        `renderInstanceInputs` entry. Identity needs a pinned host binding, is
        limited to 1,024 UTF-8 bytes, and rejects empty/whitespace/control input.
        The shared adapter rejects unpaired UTF-16 surrogates before encoding;
        Unicode/punctuation remain native-escaped values. Identity lives only in
        the call's input frame. Owned declaration JSON input now refuses runtime
        roots `instanceID`, `island` and `suggestions`; native attribute/slice
        routes still cannot replace them. Pinned receiver attribute declarations
        enforce required values, defaults, normalization and facets through the
        existing native renderer. Unresolved named types and invalid defaults
        fail closed with fixed diagnostics; no schema override is added.
        - [x] Add native required/default/normalization/range/named-type refusal
          and reserved-root/instance-boundary fixtures, plus native-built receiver
          inputs and WASM/browser parity across changing identities, failures and
          independent heaps. Keep metadata/template pins and privacy budgets.
        - [x] Keep per-story resource timing observable after Storybook module
          and fixture downloads fill the default buffer; assert exactly one
          profile WASM fetch and no full/source downloads for each scenario.
          Increase fixture-only timing capacity without clearing other entries
          or relaxing per-story fetch assertions.
        Verification: 14 profile native tests, 166 full native tests (four
        existing profiling cases ignored), 33 receiver/declaration regressions,
        native fixture generation, ten factory/audit unit tests, optimized
        profile/owner WASM acceptance, 17 component browser cases and both full
        Studio Login privacy/recovery cases pass. Studio ran with optimization
        paused; the existing deadlines were preserved. Package build, Rust format,
        script/TypeScript lint and fresh source/vendor audits pass. Reduced WASM:
        11,155,464 bytes (10.64 MiB), gzip-9 3,143,400 and Brotli-9 2,366,057;
        code/data payloads are 8,864,286/2,278,244 bytes, 10 imports/19 exports.
        This adds 1,983 bytes and remains 53.9% smaller than the unchanged full
        comparator. Identity is a host value, not an ownership/recovery grant.
        Named schema contracts, canonical native island/projection, live shared
        controls and worker/recovery integration remain open. Credential privacy,
        the 4 MiB checkpoint budget and five-second handoff limits are unchanged;
        no component/application UI JavaScript or credential capture was added.
      - [ ] Add canonical native island/projection contracts before enrolling
        the reduced ABI in shared processing. Reuse qualified lifecycle topology,
        native payload/slot/value identities and projection ownership; refuse
        unsupported island/schema input. Add native/WASM parity and malformed
        topology/namespace/source fixtures before admitting live shared controls.
        - [x] Implement bounded passive native CEMV island admission and native
          projection export: qualified ordered parts, lifecycle/payload identity,
          host identity, retained payload/slot references and empty unimplemented
          state domains. Refuse credential controls without capture proof.
        - [x] Add native schema-drift, malformed topology/namespace/version,
          source/reference/slot and privacy refusal fixtures; native-built
          islands plus WASM/browser parity and independent generation acceptance.
          Implemented the qualified lifecycle `0.2.0` passive gate, exact
          host/hydration identity, HTML payload identity, lowercase names and
          required source metadata. Unimplemented domains must be empty; missing
          source metadata, payload references, alternate schemas and credential
          controls without live capture proof refuse. Native payload/slot groups
          retain node identity/order, namespace, source key/URI and source-parent
          edges. `renderIsland` returns HTML; `projectIsland` exports opaque CEMV
          through the shared native writer, including valid empty output. Derived
          projections cannot be reused as island state. The shared cem-elements
          adapter transports bytes with existing reentrancy/disposal guards.
          Verification: 20 profile native tests, 166 full native tests (four
          existing profiling cases ignored), 35 value/receiver/declaration/export
          regressions, native fixtures, ten factory/audit unit tests, optimized
          profile/owner WASM acceptance, 18 component browser cases and both full
          Studio Login privacy/recovery cases pass. Rust format, script/TypeScript
          lint, package build and fresh source/vendor audits pass. Each browser
          scenario fetches one profile WASM and no source/full runtime. HTML and
          binary projections match independent full-native reference output on
          separate heaps; refusal preserves later valid calls and no native roots
          survive disposal. Reduced WASM: 11,249,302 bytes (10.73 MiB), gzip-9
          3,178,420 and Brotli-9 2,387,834; code/data 8,953,469/2,282,732 bytes,
          10 imports/21 exports, 53.5% smaller than the unchanged full comparator.
          This is structural passive admission, not full domain/schema validation,
          live registration, DOM installation or a checkpoint/recovery permit.
          Parent delivery gate remains open. Credential privacy, the 4 MiB
          checkpoint budget and five-second handoff deadlines are unchanged;
          no component/application UI JavaScript or credential capture was added.
        - [ ] Extend beyond passive islands with native attribute/dataset and
          slice domain adapters, including type/source preservation, payload
          references and schema-dependent input contracts, before live controls.
          - [x] Preserve atomic native value provenance across CEMV re-export;
            add a focused native roundtrip fixture before changing the codec.
            The shared v3 writer now retains source URI/key/line metadata for
            native atomic values, as it already did for nodes. Source-map and
            provenance roundtrips pass without changing the binary contract.
          - [x] Bind canonical string attributes, exact dataset projection and
            declared scalar/array/record slice views without JSON AST conversion.
            Add native type/source/alias/duplicate/credential/schema refusal
            fixtures and native-built WASM/browser parity cases.
            Native views retain original value/container source maps and keys;
            DOMStringMap values share their source attribute identity. Receiver
            conversion updates semantic attributes while preserving lexical
            dataset strings. Strict scalar kinds, contiguous array indices,
            duplicate/reserved/cross-channel aliases, 64-alias bounds and
            credential host metadata are checked before publication. Whole
            semantic record/array CEMV export remains refused. Resources/forms/
            validation/events still require empty state; no checkpoint authority
            or component/application UI JavaScript was added.
            Verification: 25 profile native cases, 166 full native unit cases
            (four existing profiling cases ignored), 38 focused regressions,
            native fixtures, ten factory/audit tests, profile/owner and full
            declaration/pinned-template/retained-CSS WASM checks, 19 component
            browser cases and both complete Studio Login recovery/privacy
            scenarios pass. Package build, changed-block Rust formatting,
            script/TypeScript lint and source/vendor audits pass. Final reduced
            WASM is 11,268,127 bytes (10.75 MiB), gzip-9 3,184,884 and Brotli-9
            2,396,344; code/data 8,973,695/2,281,312 bytes, 10 imports/21 exports.
            Matched full WASM is 24,207,650 bytes; reduction remains 53.5%.
            Native link closure remains unproven. Credential privacy, the 4 MiB
            checkpoint budget and five-second handoff deadlines remain unchanged.
          - [x] Admit reviewed native payload references while preserving target,
            source-parent and slot identity. Add bounded cycle/dangling-target,
            namespace/schema and credential-refusal fixtures before admission.
            - [x] Add native reference graph fixtures for repeated/multi-target,
              nested and empty projections, source/slot/parent preservation,
              cycle/dangling/depth/expansion refusal and referenced credential
              metadata; verify native-built WASM and browser parity.
            Payload keeps original native reference occurrences and target edges.
            Slot indexes follow direct rendered targets in order, append repeats
            without cloning source nodes and omit empty references; nested
            references remain inside their HTML element. Full rendered target/
            descendant closure passes namespace/source/credential checks before
            publication. Document/atomic/attribute targets remain refused. Shared
            CEMV graph admission checks source backlinks, dangling edges, DAG,
            depth and expanded work; native traversal also polls/bounds visits.
            Verification: 31 profile native cases, 25 native value/reference/
            serialization regressions, ten factory/audit unit cases, native
            fixtures, optimized profile/owner WASM acceptance, 20 component
            browser cases and both complete Studio Login recovery/privacy cases
            pass. HTML and opaque CEMV projections match an independent full-
            native renderer on separate heaps; no source/full WASM fetches occur
            in profile scenarios and disposal leaves no declaration owners.
            The first Studio run hit fallback/resume-2's five-second deadline
            during concurrent native build work; pausing Binaryen for the rerun
            passed both modes without changing deadlines, assertions or policy.
            Package build, changed Rust formatting, TypeScript/script lint and
            source/vendor audits pass. Optimized profile: 11,281,592 bytes
            (10.76 MiB), gzip-9 3,189,385, Brotli-9 2,398,485; code/data
            8,986,698/2,281,737 bytes, 10 imports/21 exports. The full comparator
            stays 24,207,650 bytes; reduction is 53.4%. Only the opt-in profile's
            admission changed; no component/application UI JavaScript, checkpoint
            authority or Studio backend switching was added. Entered credentials
            still block live recovery; 4 MiB and five-second limits remain.
          - [ ] Bind app-specific named receiver schema/type contracts to pinned
            build artifacts; verify unavailable/changed contracts fail before
            publication without loading the general schema catalog.
            - [x] Pin native named scalar facet contracts in an application-only
              declaration envelope; add native build/admission, source identity,
              missing/changed/duplicate/unreferenced type, unsupported executable
              and stale-pin fixtures, plus separate-heap WASM/browser parity.
              Added native-only `application_declaration::build` and the separate
              CEMAPPD1 / cem-application-declaration/1 envelope. The independently
              trusted outer metadata pins unchanged v2/v3 IR, the binary payload
              and each scalar model's schema/source URI, source hash and compiled
              contract hash. Exact IR type requirements must match the manifest
              and provided definitions; changed/missing/duplicate/unused types,
              unsupported policies, dynamic selection, reserved built-in names,
              unknown/trailing payload fields and stale pins refuse before owner
              publication. Full-runtime declaration admission refuses this
              envelope. Existing generation `open` needs no new JS/API surface.
              Fresh JSON/input/island frames bind pinned native models; defaults,
              conversion, patterns/whitespace and receiver-local facet intersection
              reuse the shared renderer. Models retain original schema source
              maps and are released with the declaration. This first capsule
              admits reviewed scalar facets over five primitive bases, not
              executable/tokenizer/node-schema contracts; pattern evaluation uses
              the existing native facet validator, not precompiled regex IR.
              Limits: 64 types, 16 restrictions/type, 64 KiB accounted/model,
              1 MiB aggregate models and each deployment input, 100,000 inspected
              IR nodes/64 levels. No schema/catalog fetches or source fallback.
              Verification: 56 native application/build/profile tests, 26 shared
              value/declaration/reload/serialization regressions, ten factory/audit
              unit cases, native fixtures, pre-optimized and final optimized WASM
              parity on separate heaps, 21 component browser cases and both full
              Studio Login recovery/privacy scenarios pass. Studio ran with
              Binaryen paused, retaining existing deadlines and assertions.
              Package build, changed Rust formatting, script/TypeScript lint and
              source/vendor audits pass. Final profile: 11,471,605 bytes (10.94
              MiB), gzip-9 3,240,672 and Brotli-9 2,429,535; code/data
              9,176,661/2,281,758 bytes, 10 imports/21 exports. The unchanged full
              comparator is 24,207,650 bytes; reduction is 52.6%. Native link
              closure is still unproven. No component/application UI JavaScript,
              backend switching or checkpoint grant was added; entered-credential
              refusal, 4 MiB checkpoints and five-second handoffs remain unchanged.
            - [x] Define portable native executable/node-schema contract capsules
              before admitting callback/tokenizer or node-valued named schemas.
              [Design contract](../cem-native-schema-capsules.md). The proposed wire
              component-facing formats remain unavailable; v1 admits only its reviewed
              scalar subset. Executable export/linking is separate work below.
              - [x] Specify native build/export, inert decoding, fresh host linking,
                original source identity, bounded dependency closure and atomic
                publication; keep live callbacks, leases and grants off the wire.
              - [x] Add fully repinned refusal fixtures for node/list/content-type
                contracts, unsupported inherited restrictions/model policies,
                executable extensions and a future envelope version. Verify native
                admission and separate WASM/browser heaps preserve live owners and
                accept a subsequent valid render.
              - [x] Repair the preparation-evidence and required-candidate fixture
                inputs to distinguish original lexical maps from retained-view
                URI provenance; keep exact source/text mismatch refusal and
                independent candidate identity/provenance assertions.
              Verification: 58 native application/profile tests, all 234 native
              datatype execution tests, 12 named-receiver/declaration regressions,
              ten factory/audit unit cases, generated native fixtures, both
              existing optimized WASM drivers, 21 component browser cases and
              both full Studio Login recovery/privacy scenarios pass. Changed
              Rust formatting, TypeScript/script lint, document links and source/
              vendor audits pass. Eight new refusal fixtures retain existing owners and valid
              subsequent renders on separate heaps; native checks prove all
              integrity/type requirement pins match for unsupported models.
              The broad native run initially exposed two older fixtures mixing
              lexical parser maps with URI-enriched retained views; corrected
              fixture inputs keep exact map/text refusal and independent source
              assertions. No production runtime or WASM binary changed; profile
              remains 11,471,605 bytes (10.94 MiB). No app/component UI JavaScript
              or credential/checkpoint/admission policy changes were introduced;
              entered credentials still block live recovery, with the existing
              4 MiB checkpoint budget and five-second handoff deadlines.
            - [x] Implement a native structural node-contract exporter/verifier
              over shared retained validation models, with original source
              occurrences and exact dependency pins; prove native reload parity
              before adding it to the application envelope.
              - [x] Add native literal structural fixtures covering scalar fields,
                required/allowed attributes, child bounds, retained source/namespace
                occurrences and independent full-model reload parity.
              - [x] Add refusal fixtures for unsupported source/model policies,
                stale/mixed pins, altered occurrences/dependencies, cycles,
                retained accounting, oversized/malformed binary collections and
                cancellation.
              - [x] Prove reloaded structural models preserve native reference
                readiness, directed cross-owner grants, repeated placements and
                original candidate diagnostics through the shared query host.
              - [x] Generate a native structural capsule fixture and prove existing
                component WASM/browser generations refuse it without changing
                live owners or preventing the next valid render.
              Added native-only `cem_ml::schema::structural_capsule` CEMSC1
              build/verification. It pins the logical schema, original physical
              source/hash/extent and selected registry identity, preserves a flat
              native arena and passive lexical namespaces, and checks exact local
              attribute/child dependency hashes. The compiled projection must
              equal the shared literal element/attribute/total-child-count model;
              unsupported source and executable policies cannot disappear. Reload
              parses no source and creates no callback, grant or runtime owner.
              Bounds include a 64 KiB binary, 64 combined definitions, 16 count
              rules/element, 16,384 records/edges, 64 levels, one 100,000-visit
              verification budget, 64 KiB accounted per consuming root and 1 MiB
              aggregate retained accounting. Source ownership, original ranges,
              canonical binary shape and expanded dependency work are checked.
              The shared parser's source-ID-0 document trivia is retained exactly;
              it cannot substitute for a declaration/field's pinned source.
              Verification: ten new native capsule cases, 11 shared reference/
              diagnostic regressions, 58 selected-profile tests and 246 datatype/
              receiver/declaration regressions pass (325 native tests). Native
              build fixtures, both optimized WASM drivers, 21 component browser
              cases, ten factory/audit cases, formatting, lint, document links and
              source/vendor audits pass. The generated structural fixture is
              21,312 bytes. Separate browser/WASM heaps refuse that native schema
              as a component declaration, preserve their live owner and render
              successfully afterward. Both deployed WASM binaries are unchanged;
              the selected profile remains 11,471,605 bytes (10.94 MiB). Component
              receiver linking, executable registration and worker/recovery
              admission remain open. No app/component UI JavaScript, backend
              switching, checkpoint grant or credential-policy change was added.
              Both full Studio Login recovery/privacy scenarios pass with native
              builds finished before acceptance. Entered credentials still block
              recovery; the 4 MiB checkpoint budget and five-second handoff
              deadlines remain unchanged.
            - [ ] Add an explicit fresh-host linker and receiver adapter for
              verified node contracts, then pure query/exact native exports.
              Keep unavailable callbacks/tokenizers pending or refused; never
              infer a registration from an export name or source identity.
              - [x] Link verified CEMSC1 models to concrete fresh native hosts and
                exact literal node receiver declarations; test registry mismatch,
                foreign hosts, epoch changes, disposal and failed installation.
              - [x] Stamp native captured attribute access with its live context
                and original invocation; add receiver validation fixtures for
                structural/scalar/count failures, local cardinality, duplicate/
                cross-owner targets, stale captures, strings/defaults, nested
                references, cancellation and original diagnostic provenance.
              - [x] Verify native full/reloaded parity and current WASM refusal,
                component browser behavior and Studio credential recovery policy.
              - [x] Reuse bounded literal-source inspection for receiving schemas;
                test duplicate namespace headers and unknown source policies so
                compilation cannot silently discard required semantics.
              Native linking now uses a non-cloneable `FreshStructuralHost` over
              the real query host, explicit scopes/grants, pinned registry
              identity and live context. It constructs a `StructuralNodeReceiver`
              only for the original registered literal node declaration and
              returns a new model after liveness checks; failure mutates no
              existing binding. Access receipts retain the current context,
              exact caller attribute/tree and selected declaration/schema/local
              metadata. Untyped, stale, relabelled or foreign captures cannot
              supply an invocation, and changed receiver snapshots require
              relinking. Closed/dropped hosts cannot reopen; retirement and
              cancellation cannot reuse a verdict. The adapter traverses the
              authorized literal forest through shared validation, preserves
              repeated/mixed-owner roots and full-native diagnostic provenance,
              applies local cardinality, and leaves nested expressions incomplete.
              Limits: 64 roots, 16,384 placements, 64 levels, 1 MiB accounted
              literal/name/source-map content and 100,000 shared preflight/name
              visits. Strings, defaults and unsupported source/executable policies
              never become ready node inputs. Context proofs stay native and
              process-local; no source parsing, callback deserialization or
              template named-node/WASM admission was added.
              Verification: 13 receiver cases, ten capsule cases, 86 shared core
              cases, 260 additional query/datatype/value/declaration regressions
              and 58 profile cases pass (427 native tests). Independent original
              compilation matches linked validation including URI/ranges/line/
              column; closed-host reopening and changed-metadata/source-policy
              cases were first observed failing and then repaired. Both current
              WASM drivers, 21 component browser cases, ten factory/audit cases
              and both full Studio Login recovery/privacy scenarios pass. Native
              builds finished before Studio acceptance. Changed formatting,
              documentation links and source/vendor audits pass. Existing WASM
              artifacts/ABI and the 11,471,605-byte profile are unchanged; no new
              production JavaScript, checkpoint grant or backend switching.
              Entered credentials still block recovery; 4 MiB checkpoints and
              five-second handoff deadlines remain unchanged.
              - [x] Add verified pure-query validator capsules for an explicit
                reviewed subset, with complete native function/signature closure
                and no source recompilation at verification or registration.
                - [x] Add native roundtrip/parity fixtures for typed validator IR,
                  checked original declaration dependencies, exact function and
                  input/result signatures, and original diagnostic provenance.
                - [x] Reject repinned malformed/unsupported IR, dynamic or native
                  calls, missing/extra function requirements, foreign declaration
                  occurrences, bounds and cancellation before registration.
                - [x] Verify the shared result adapter and cumulative invocation
                  budget on reloaded programs; preserve existing WASM refusal
                  and Studio's entered-credential recovery policy.
                Native `CEMQV1` now carries typed query tables, original source
                maps, fixed value/datatype/candidate roles, a source/declaration
                fingerprint and exact reviewed function requirements. Verification
                is inert; explicit registry registration requires checked original
                native declarations and actual available host functions. Matching
                hashes alone cannot supply an implementation. Execution keeps the
                ordinary scoped native views, input checks, schema-owned results
                and selected dispatcher; there is no general-catalog fallback.
                Supported instructions include bounded literals, static records,
                arrays/sequences, comparisons/binary operators, conditionals and
                field reads, plus nine reviewed sequence/string functions. Dynamic
                calls, callbacks/lambdas, loops, resource/state/type/template
                providers and unsupported helpers refuse. The source dependency
                remains separately supplied; this is not whole-schema or
                compiled-component admission and creates no generation grant.
                Wire cap: 64 KiB; instructions: 4,096; original query: 16 KiB;
                graph/type depth: 64/32; one 100,000-visit verification allowance.
                Structural and query capsules share bounded native MessagePack
                preflight. Expanded instruction cost is charged cumulatively to
                the enclosing invocation's native validator-work budget, capped
                at the smaller of 100,000 and 64 times effective queue size.
                Verification: seven capsule and seven query integration cases,
                427 existing native regressions (441 distinct native cases),
                both current WASM drivers, 18 component browser cases, ten
                factory/audit cases and both Studio Login recovery/privacy
                scenarios pass. Renamed headers, unavailable host functions and
                invalid qualified-name spans were observed failing before repair.
                Source audit freshness, changed formatting and documentation links
                pass. No production JavaScript, WASM ABI/export, application wire
                version, source fallback or cached verdict was added. Existing
                optimized profile remains 11,471,605 bytes. Entered credentials
                still block recovery; 4 MiB checkpoints and five-second handoff
                deadlines remain unchanged. Studio acceptance ran without
                concurrent native builds; the final span-only native guard
                passed focused native coverage afterward.
              - [x] Transport original query declaration/source dependencies as
                bounded native capsules, restore occurrence mappings and bind
                query registrations to concrete native host context lifetimes.
                Refuse expired/foreign/replaced owners without replaying source
                queries or widening retained view grants; keep new worker/
                fallback transport and recovery admission gated.
                - [x] Add native declaration-source roundtrip and diagnostic
                  parity fixtures, preserving original occurrences, passive
                  lexical/namespace metadata and physical/base source URIs
                  without source parsing or query recompilation on reload.
                - [x] Reject malformed/repinned source graphs, mixed program
                  pins, unrelated captures, dynamic source inputs, allocation/
                  traversal limits and cancellation before linking.
                - [x] Exercise actual host scope/context epochs, pending/foreign
                  scopes, replacement/retirement, close/drop, fresh owners and
                  cumulative invocation budgets; keep worker/fallback admission
                  gated and rerun existing Login credential-privacy acceptance.
                - [x] Prove retained pure-query invocations do not retain unused
                  host-native callback capabilities after their host is dropped.
                Native `CEMQD1` carries the verified program and original typed
                source arena/passive lexical metadata. Reload creates new owners,
                preserves original occurrences/ranges/physical URIs, checks the
                original declarations through the shared contract source checker
                and never parses, lowers, fetches or evaluates source queries.
                Structural/query sources share the native arena codec with
                separate whitespace/text profiles; CEMSC1's wire shape and
                original admission profile remain unchanged. Pending namespace
                declarations, dynamic source references, unrelated captures,
                mixed program pins and foreign node/namespace provenance refuse.
                The full envelope is capped at 64 KiB, source arenas at 16,384
                records/64 levels and source coordinates at 1 MiB. Outer/nested
                verification shares one 100,000-visit allowance and control.
                Fresh bindings use the actual registered source tree/scope and
                private host epoch, actual available functions and source kind.
                Invocations derive their scope/policy from the concrete host and
                share their query budget. New scopes, context/grant changes,
                retirement and close/drop expire receipts. Failed replacement
                leaves the existing binding live; successful staging atomically
                retires it. Ambient callbacks/resolvers/inputs are not inherited
                or retained by a closed validator invocation. This binds one
                validator, not a complete effective datatype or component receiver.
                Verification: three native source cases, seven program cases,
                14 query integration cases, 427 existing native regressions
                (451 distinct native cases), both current WASM drivers, 18
                component browser cases, ten factory/audit cases and both Studio
                Login recovery/privacy scenarios pass. Foreign namespace
                provenance and unavailable callback retention were observed
                failing before repair. Formatting, documentation links and audit
                freshness pass. No production JavaScript, WASM ABI/export,
                application envelope change, source fallback, cached verdict or
                checkpoint grant was added. Existing optimized component WASM
                remains 11,471,605 bytes. Entered credentials still block recovery;
                the 4 MiB checkpoint budget and five-second deadlines remain.
                Native builds finished before Studio acceptance.
              - [x] Link exact reviewed native validator/tokenizer registrations
                on fresh hosts through native-only requirement capsules;
                preserve unavailable and checked-replacement states.
                - [x] Add exact native validator/tokenizer requirement and
                  source roundtrip fixtures, including ABI/module/export/
                  signature mismatch, missing/duplicate implementations and
                  source-owner refusal without deserializing callbacks.
                - [x] Exercise real fresh-host epochs, retirement/replacement,
                  cancellation and callback release; check unavailable verdicts
                  and preserve the shared validator result adapter.
                - [x] Integrate guarded tokenizer selections with the existing
                  datatype compiler; prove inherited admission, original decoded
                  token spans, checked replacement, limits and stale handles.
                - [x] Prove explicit source-owner sharing uses the complete
                  verified source snapshot and identity, allowing related native
                  requirements to feed one compiler without pointer/name guessing.
                - [x] Keep preparation/conversion/equality/serializer adapters
                  explicitly pending and rerun browser/WASM and Login privacy
                  acceptance before publishing this native-only increment.

                Native `CEMNR1` now pins the original typed source and declaration
                occurrences, exact implementation ID/kind ABI/module hash/export
                and passive signature. The embedding explicitly supplies actual
                implementations; this association is trusted embedding metadata,
                not code attestation or linked-symbol omission proof. Source
                verification is inert and bounded; callbacks and live contexts
                remain in-process. Weak proxy ownership prevents retained
                selections from keeping retired callbacks alive. Successful
                registration/removal advances the context; duplicates and failed
                staged replacements preserve live bindings. The shared compiler
                retains inherited tokenization, original decoded spans, checked
                replacement and limits. Availability changes during separator
                checks remain unavailable, including through the host proxy.
                Canonical accepted-record validator results are supported;
                diagnostics-only legacy result policies are not transported yet.
                Verification: nine new native registration cases and 451 existing
                native cases pass (460 total), including source/program/receiver,
                datatype, structural and component-profile regressions. The
                component-runtime WASM target compiles; both current WASM drivers,
                18 component browser cases, ten factory/audit cases and both
                worker/fallback Studio Login recovery/privacy cases pass.
                Proxy availability loss during separator checking was observed
                failing before repair. Formatting, documentation links and audit
                freshness pass. Native builds finished before Studio acceptance.
                No production JavaScript, application envelope, WASM ABI/export,
                source fallback, cached verdict or checkpoint grant was added.
                Current optimized component WASM remains 11,471,605 bytes; it was
                exercised, not rebuilt for these native-only adapters. Entered
                credentials still block recovery; automatic whole-Studio recovery
                stays disabled, with the 4 MiB budget and five-second deadlines.
              - [x] Link exact direct native preparation/conversion/equality/
                list-serializer registrations on fresh hosts; preserve their
                typed admission, source ownership, unavailable states and
                checked replacement contracts without serializing callbacks.
                - [x] Add native operation requirement roundtrip/refusal fixtures
                  for exact ABI/module/export/signatures, operation modes and
                  original datatype ownership, including incompatible repinned bytes.
                - [x] Exercise fresh-host native preparation, conversion, equality
                  and list serialization through the ordinary datatype compiler;
                  preserve candidates, source attribution, order and explicit
                  boundaries, with no implicit conversion or inferred capability.
                - [x] Prove checked preparation replacement retains inherited
                  admission and output compatibility; test budgets and unavailable
                  outcomes through real compiled descriptors.
                - [x] Check required native candidates, invalid output shapes,
                  original constant owners and compiler representation mismatch;
                  refuse invalid input before invoking the registered callbacks.
                - [x] Exercise operation retirement, registration/removal epochs,
                  cancellation and weak callback release; refuse stale publication
                  while keeping duplicate selections and failed installs atomic.
                - [x] Rerun native/component/WASM and Studio Login recovery/privacy
                  acceptance, document remaining admission/transport gates and
                  preserve the 4 MiB budget and existing credential refusal.

                `CEMNR1` now carries direct typed native operation signatures,
                exact original datatype occurrences and preparation mode. A fresh
                native host explicitly registers actual implementations and links
                an opaque operation owner. Its installed selections use the
                existing datatype compiler, candidate checks, output validation,
                budgets and enumeration/list export machinery. Checked preparation
                preserves inherited admission and compatible output. Known invalid
                Ready/derived, checked/root and scalar/whole-list combinations
                refuse before compilation; the ordinary compiler remains the
                authority over resolved representation compatibility. Proxies
                share weak host/context/active-binding ownership and check before
                and after calls. Stale or cancelled implementations publish no
                value/verdict/text, even when a callback returned success. Keeping
                compiled descriptors does not retain disposed native callbacks.
                Duplicate/foreign selections leave existing registry entries
                unchanged. Inert metadata does not attest code or constitute
                complete datatype/prepared-token/component admission. Constant,
                facet, list-item orchestration and source-backed result-profile
                closures remain explicit follow-up work below.
                Verification: ten new operation cases and 460 existing native
                cases pass (470 distinct); the final registration suite has 19
                cases, including 28 lifecycle/callback-publication scenarios and
                32 repinned operation-record refusals. Unsupported Ready/derived
                preparation was observed failing the early-refusal fixture before
                repair. Both current WASM drivers, the component-runtime WASM
                compile check, 18 component browser cases, ten factory/audit cases
                and both worker/fallback Studio Login recovery/privacy cases pass.
                Formatting, documentation links and audit freshness pass. Native
                builds finished before Studio acceptance. No production JavaScript,
                application envelope, WASM ABI/export, cached verdict or checkpoint
                grant was added. Current optimized component WASM remains
                11,471,605 bytes and was exercised, not rebuilt for the native-only
                adapters. Entered credentials still block recovery; automatic
                whole-Studio recovery stays disabled, with the 4 MiB checkpoint
                budget and five-second deadlines unchanged. Generated screenshots
                and Vitest attachments remain excluded from the commit.
              - [x] Complete bounded native capability adapters for constant
                interpretation, facet profiles, explicit shared list-item lexical
                orchestration and source-backed operation result profiles before
                complete compiled-datatype/component admission. Pin exact
                implementations and original sources; never infer registrations,
                serialize prepared tokens or replay source queries as a fallback.
                - [x] Add native constant/facet/list-orchestration requirement
                  roundtrip and mismatch fixtures; require explicit actual
                  registrations and preserve original source allocations.
                - [x] Exercise constants, checked facet intersection and list
                  orchestration through the ordinary compiler, including empty
                  lists, inherited policies, original spans and owner retirement.
                - [x] Add shared availability checks for retained facet rules,
                  declaration readiness and sealed preparation; refuse stale
                  owners before/after execution without retaining native code;
                  intersect additional guards without erasing a prior refusal.
                - [x] Export/verify original native source-backed conversion,
                  equality and constant profiles; bind checked result adapters
                  and explicit native callbacks without source-query replay.
                - [x] Test result shape/status/source diagnostics, missing and
                  incompatible adapters, cancellation, budgets, metadata/source
                  mismatches and weak lifetime release for all new contracts.
                - [x] Run native/WASM/component and both Studio Login recovery/
                  privacy acceptance checks; retain credential refusal, the 4 MiB
                  budget and five-second deadlines, then document remaining gates.

                Native `CEMNR1` now pins constant representation, exact facet
                family and list-item orchestration mode against original types.
                Actual constant callbacks and shared family/item algorithms must
                be registered explicitly. Existing compiler preparation, facet
                intersection, constant validation and list tokenization remain
                authoritative, including empty lists, absent dependencies and
                inherited restrictions. Shared availability guards intersect and
                propagate through retained facet/declaration readiness, sealed
                preparation, typed producers and result processing. An expired
                owner cannot publish a value/verdict even when retirement occurs
                during lazy native result-field reads. Caller control stops retain
                precedence. Native source conversion/equality/constant profiles
                restore original behavior roles/aliases and canonical result
                contracts, then use explicitly supplied fresh checked adapters.
                Callback/adapter/runtime objects and prepared tokens stay local;
                no source query is compiled or replayed by these adapters.
                Constant values remain immutable compiler data, not cached runtime
                validation or portable preparation proofs. The 64 KiB per-record
                limit stays unchanged: larger arenas refuse, with explicit source
                modules rather than silent pruning or increased limits. Custom
                result QNames, cross-source/imported profiles and query operation
                profiles beyond the existing validator subset remain outside this
                bounded format. Full APIs retain their capabilities; complete
                application closure admission and physical runtime transport are
                still pending below.
                Verification: eleven new closure cases and 470 existing native
                cases pass (481 distinct), including 30 registration cases and
                48 repinned additional-record refusals. Late native record reads
                explicitly retire their owner during adaptation and cannot publish.
                Retained sealed preparations and typed producers refuse retired
                facet owners; empty lists retain orchestration guards. Shared
                caller cancellation remains a control stop. The combined large
                source fixture exceeded 64 KiB and refused as required; it was
                split into explicit source modules without changing limits. Native
                builds finished before both Studio Login worker/fallback recovery/
                privacy scenarios passed. Both current WASM drivers, the WASM
                compile check, 18 component browser cases, ten factory/audit cases,
                formatting, local documentation links and audit freshness pass.
                No production JavaScript, application envelope, WASM endpoint,
                callback serialization or cached runtime verdict was introduced.
                Current optimized component WASM remains 11,471,605 bytes; it was
                exercised, not rebuilt for the native adapters. Entered credentials
                still block recovery; automatic whole-Studio recovery remains
                disabled, with the 4 MiB checkpoint budget and five-second
                deadlines unchanged. Generated screenshots and Vitest attachments
                remain excluded from the commit.
              - [ ] Carry node contract requirements into a new application
                envelope and reduced WASM/worker receiver transport with fresh
                generation ownership, recapture and privacy acceptance.
                - [x] Build and inertly verify a separate bounded native
                  application dependency envelope with content-addressed blobs,
                  independent manifest/build pins and typed existing capsules.
                - [x] Add exact compiled declaration/component/receiver closure
                  fixtures, including missing/orphan/duplicate/cyclic dependencies,
                  dynamic unsupported forms and registry/ABI/source mismatches.
                - [x] Exercise structural, portable query-source and native
                  requirement dependencies, aggregate limits, cancellation,
                  deterministic packing and complete-source owner sharing.
                - [x] Preserve legacy scalar refusal and prior verified owners
                  on verification failure; keep verification distinct from native
                  linking, publication, code liveness and recovery permission.
                - [x] Exercise the 1,024/1,025-edge graph boundary and reject
                  alternate per-byte array encodings for native binary blobs.
                - [x] Run native/WASM/component and both Studio Login privacy
                  acceptance checks; document bounds, trusted assertions and the
                  remaining reduced-runtime transport/ownership gates.

                Native `application_envelope` now builds/verifies CEMAPPD2 /
                cem-application-dependencies/2. It pins exact ML/QL/registry,
                manifest, compiled IR requirements and registry recipe, and
                deduplicates opaque native declaration/template/SC1/QD1/NR1
                blobs. Static component references and exact structural receiver
                names/schema URIs must match. Only records reachable from
                component roots or explicit standalone contract roots are allowed;
                missing, unused, cyclic or conflicting records refuse. Dependency
                order is deterministic. Native sources share only after complete
                snapshot agreement; conflicting same-ID implementations refuse.
                Bounds: 4 MiB encoded package and unique blob bytes, 32 components,
                64 contracts, 128 blobs, 1,024 edges and depth 32; per-template,
                metadata, nested capsule and operation limits remain enforced.
                The outer verifier has 100,000 checks; nested capsules retain
                their own budgets with the same cancellation control. This is
                neither a peak-heap measurement nor a higher checkpoint budget.
                Declared contract edges are trusted build assertions; complete
                semantic/native code closure is not proven. Verification remains
                inert, and native_link_closure_verified() remains false. Scalar
                CEMAPPD1 and current WASM admission remain unchanged.

                Verification: 15 new envelope cases and 484 existing native cases
                (499 total), the WASM compile check, both separate-heap WASM
                drivers, 18 component browser cases, ten audit/factory cases and
                both complete live Login worker/fallback privacy recovery checks
                pass. Formatting, 228 local documentation links, audit freshness
                and diff checks pass. Current optimized component WASM is
                11,471,605 bytes; it was exercised, not rebuilt for this native-only
                package adapter. No production JavaScript, WASM endpoint, callback
                serialization or live permission receipt was added. Entered
                credentials still block recovery; automatic whole-Studio recovery
                remains disabled, with the 4 MiB checkpoint budget and five-second
                deadlines unchanged. Generated screenshots and Vitest attachments
                remain excluded from this commit.

                - [x] Assemble bounded fresh-native-host named-node bindings from
                  the verified envelope with exact original occurrences, explicit
                  query/native registrations and all-or-nothing native slot
                  replacement. Keep scalar/executable receiver substitutions
                  unavailable; component/runtime admission remains separate.
                  - [x] Add native assembly fixtures for exact compiled named-node
                    occurrences and literal local cardinality, multiple component
                    models, missing/extra/relabelled sources, scalar substitutions
                    and unsupported receiver policies.
                  - [x] Stage every portable query/native dependency through the
                    existing explicit host linkers; cover missing implementations,
                    wrong roles/scopes and retained binding liveness.
                  - [x] Refuse publication/access after a consumer retires a
                    staged structural model; retain the exact typed node guard
                    without replacing source identity or adding callback proxies.
                  - [x] Add atomic native assembly-slot replacement, stale/foreign
                    context and cancellation refusals, staged cleanup and prior
                    committed owner preservation before runtime publication.
                  - [x] Verify fixed assembly/source bounds, legacy refusal and
                    native/WASM/component/Studio credential privacy regressions;
                    document the remaining runtime transport/admission gates.

                  Native `structural_receiver::application` now stages exact
                  compiled named-node occurrences/local cardinality against actual
                  registered source trees and one host context. Per-component
                  models retain exact typed node guards; all QD1/NR1 dependencies
                  require explicit existing linker requests/registrations. The
                  checked native slot replaces complete assemblies together and
                  retires superseded/abandoned models and proxies. Failed/stale/
                  foreign/cancelled candidates preserve the prior slot; no host
                  registry/context is mutated by staging. Assembly caps receiving
                  tree nodes at 16,384 each/65,536 distinct total, with 64 receiver
                  occurrences per component/256 total. Scalar/executable named
                  receiver substitutions remain unavailable. This is native
                  binding assembly, not complete executable datatype installation,
                  semantic/native code closure, runtime publication or permission.

                  Verification: 13 new assembly/boundary cases and 499 existing
                  native cases (512 total), WASM compile checking, both existing
                  separate-heap WASM drivers, 18 component browser cases, ten
                  audit/factory cases and both complete live Login worker/fallback
                  credential privacy checks pass. The retired-model regression
                  fails before retaining/checking the exact typed node guard and
                  passes with it. Formatting, 231 local documentation file links,
                  audit freshness and diff checks pass. Existing optimized
                  component WASM is 11,471,605 bytes; it was exercised, not rebuilt
                  for this native-only assembly. No production JavaScript, WASM
                  endpoint, callback serialization, cached verdict or runtime
                  permission receipt was added. Entered credentials still block
                  recovery; automatic whole-Studio recovery remains disabled,
                  with 4 MiB checkpoints and five-second deadlines unchanged.
                  Native builds finished before Studio acceptance. Generated
                  screenshots and Vitest attachments remain excluded.

                - [ ] Extend the portable application receiver contracts and
                  assembly to scalar/executable named types with exact original
                  base/item/behavior dependencies and explicit compiler
                  installations. Add mixed node/scalar/executable, missing/stale
                  implementation, closure and atomic replacement fixtures before
                  enabling component admission; keep native code closure unproven
                  until linked implementation evidence exists.
                  - [x] Export/verify a bounded native original datatype-source
                    capsule with exact literal local base/list-base/item closure,
                    original field occurrences, source snapshots and trusted pins.
                    Add deterministic/reload, missing/cyclic/duplicate/qualified,
                    unsupported reference/constant/kind and malformed wire fixtures.
                  - [x] Compile verified local source closures through the shared
                    native datatype compiler under the actual fresh host's source,
                    context, control and traversal policy. Require explicit actual
                    implementations; cover inherited scalar/list typed validation,
                    limits, cancellation, foreign/pending scopes and stale owners.
                  - [x] Share a complete verified source with NR1 requirements
                    before explicit operation installation; test exact snapshot
                    agreement, registration/source mismatches and retirement.
                  - [x] Run native/WASM/component and isolated Studio worker/fallback
                    credential privacy acceptance; document native source closure
                    separately from full executable/behavior/receiver/code closure.

                  Native CEMDT1 / cem.native-datatype-sources/1 now pins original
                  whole source snapshots, selected roots and exact shared-plan
                  literal local base/list-item dependencies with original field
                  occurrences. Native verification is inert and bounded; it does
                  not deserialize executable types. The fresh host's compiler
                  adapter preserves actual source/scope/context/control/traversal
                  policy, requires explicit implementations and retains private
                  executable snapshots for checked root-only typed validation.
                  Complete verified source agreement enables original NR1
                  operation installation. Scalar/list inheritance, original
                  owners, missing registrations, cancellation, foreign/stale/
                  pending scopes and prior-owner refusal are covered. Bounds:
                  64 KiB wire, 32 roots, 64 local type declarations and depth 32,
                  with existing source/preflight/compiler budgets unchanged.
                  Whole behavior/imported/query closure, guarded preparation,
                  mixed scalar/executable receiver formats and atomic receiver
                  installation remain open. Existing CEMAPPD1/CEMAPPD2, WASM
                  endpoints and native code closure flags are unchanged.

                  Verification: nine new integration and three wire/boundary
                  cases plus 512 existing native cases (524 total) pass, including
                  exact same-source operation installation and a different arena
                  claiming the same source identity refusing to share. A regression
                  first exposed inherited dependencies as unintended validation
                  entrypoints; root-only checks now refuse them. WASM compilation,
                  both existing separate-heap WASM drivers, 18 component browser
                  cases, ten audit/factory cases and both full Login worker/fallback
                  credential privacy checks pass. Formatting, 238 local
                  documentation file links, audit freshness and diff checks pass.
                  Current optimized component WASM is 11,471,605 bytes; it was
                  exercised, not rebuilt for the native-only contracts. No new
                  production JavaScript, WASM endpoint, callback serialization,
                  cached verdict or permission receipt was added. Entered
                  credentials still block recovery; automatic whole-Studio recovery
                  stays disabled, with 4 MiB checkpoints and five-second deadlines
                  unchanged. Native builds finished before Studio acceptance.
                  Generated screenshots and Vitest attachments are excluded.

                  - [ ] Complete behavior/query/imported type dependencies and
                    guarded lexical/typed preparation handoff before defining the
                    new mixed receiver format and enabling receiver installation.
                    - [x] Define a separately versioned behavior/query/imported
                      source dependency contract with original declaration/field
                      occurrences, complete lexical snapshots and exact QD1/NR1/
                      datatype-module pins. Keep CEMDT1 refusal unchanged; inert
                      references/query text cannot grant execution or source reuse.
                      - [x] Add a distinct passive reference-source snapshot with
                        exact attribute value/reference/context edges and lexical
                        snapshots. Test original names/spans, fresh allocation,
                        legacy refusal, stale provenance, contexts, graph ownership,
                        unresolved/retained targets, bounded preflight and cancellation.
                      - [x] Add an inert datatype dependency capsule binding exact
                        original dependency fields to local types or independently
                        verified DT1/QD1/NR1 pins. Test scalar/list/rule/imported
                        requirements, deterministic reload, missing/orphan/conflicting
                        modules, repinning, cycles, bounds and source agreement.
                        Keep intended target assertions distinct from host selection,
                        crossing grants, compilation, registration and publication.
                      - [x] Run native/WASM/component and isolated Studio Login
                        privacy acceptance; document the remaining linking and
                        receiver/runtime gates without changing existing formats.

                      Native CEMDP1 / cem.native-datatype-dependencies/1 now pins
                      original datatype field occurrences and intended local,
                      DT1 exported-root, QD1 declaration or NR1 validator targets.
                      A distinct versioned passive source profile retains original
                      reference expressions, lexical containing-node indices,
                      attribute value edges, names, namespaces and spans. The
                      shared source/lexical validators check original ownership;
                      existing literal snapshots and CEMDT1 still reject references.
                      Nested artifacts are supplied explicitly and verified against
                      exact pins; missing, extra, duplicate, wrong-role and corrupt
                      inputs refuse. Complete source snapshots must agree for one
                      source URI, even when identity/hash claims match. No allocation
                      sharing follows from agreement. Intended selections remain
                      trusted build assertions: selections_verified() and
                      executable_closure_verified() both remain false. No parser,
                      callback, host binding, crossing grant, executable datatype,
                      component admission or receiver publication is created.
                      Bounds: 64 KiB capsule, 32 roots, 64 source declarations,
                      128 bindings, local depth 32 and 64 supplied artifacts within
                      4 MiB total bytes. Nested formats keep their existing limits
                      and share cancellation, not a new aggregate work allowance.
                      Dependency-bearing datatype modules, additional operation/
                      constant profiles and actual imported-source linking remain
                      separate pending gates below.

                      Verification: seven new integration, three new wire/graph
                      and two new native source cases pass alongside 540 prior
                      native cases (552 distinct). Original fields, namespaces,
                      fresh owners, pins, source agreement/conflicts, actual context
                      ownership independent of integrity hashes, local cycles,
                      cardinality/depth/wire bounds and cancellation are covered.
                      The WASM compile check, both current separate-heap WASM
                      drivers, 18 component browser cases, ten audit/factory cases
                      and both complete live Login worker/fallback credential
                      privacy scenarios pass. Native builds drained before Studio
                      acceptance. Formatting, 250 local documentation file links,
                      refreshed audit metadata and diff checks pass. All production
                      changes are native-only; the current optimized component
                      (11,474,094 bytes) and full (24,202,382 bytes) WASM assets
                      were exercised, not rebuilt. No production JavaScript or new
                      WASM endpoint was introduced. Entered credentials still block
                      recovery; automatic whole-Studio recovery stays disabled,
                      with 4 MiB checkpoints and five-second deadlines unchanged.
                      Generated screenshots and Vitest attachments are excluded.
                    - [x] Bind imported QName/type selections through actual fresh
                      source scopes, per-source policies and directed crossing
                      grants. Add missing/stale/unavailable/import-cycle and
                      original-namespace fixtures before cross-source compilation;
                      never substitute defaults or consumer-side aliases.
                      - [x] Add checked source registration and explicit captured
                        lexical-context preparation; test exact owners/host scopes,
                        pending contexts, original namespace aliases and export roots.
                        Refuse generic tree registration without the exact verified
                        capture already attached; do not attach metadata implicitly
                        during selection or invalidate an unchanged prior result.
                      - [x] Resolve intended dependency sites through the actual
                        host's shared name catalog/traversal and directed grants.
                        Test exact QD1/NR1 rule targets without implementation
                        execution, missing/reverse grants, missing exports, mismatched
                        targets, reference cycles, destination/request limits,
                        cancellation and callback-time stops without compilation.
                      - [x] Keep selection results local and host/context-bound;
                        test stale/foreign/closed owners, failed-attempt preservation
                        and callback lifetime release, then run native/WASM/component
                        and isolated Studio Login privacy acceptance.

                      Native FreshStructuralHost now registers exact verified
                      source captures with explicitly supplied contexts and
                      policies, and prepares saved lexical occurrences without
                      inheriting a ready root's inputs. Selection borrows the actual
                      resolver and uses shared original name discovery/dependency
                      traversal. DT1 exports are explicit, restricted to exported
                      roots and named from their own original schema/declaration.
                      Original XML/CEM namespace snapshots and use aliases remain
                      authoritative; consumer aliases/target assertions cannot
                      repair mismatches. Literal/native crossings require actual
                      directed grants. Actual selected fields/targets must match
                      original CEMDP1/DT1 identities, including QD1/NR1 rule targets.
                      No validator implementation is executed or registered.
                      A red fixture exposed implicit capture admission through raw
                      tree registration; preparation/selection now require the
                      exact capture already attached, and a refused attempt cannot
                      bootstrap it through discovery. No persistent catalog or
                      host clone is installed. Pending, missing, foreign, reloaded,
                      ambiguous, wrong/multiple/empty and cyclic selections refuse.
                      Current native reference cycles across distinct prepared
                      occurrence scopes are covered; future dependency-bearing
                      module cycles remain a separate format/linking gate.

                      FreshDatatypeDependencySelection retains inspection counts,
                      immutable capsule data and a weak actual-host context lease.
                      Context/scope/grant changes, successful occurrence preparation,
                      close/drop, foreign hosts, explicit retirement and operation
                      stops invalidate use. Unchanged failed selection/preparation
                      attempts preserve prior results. Inspection does not replay
                      selectors or retain actual host callbacks. This result is
                      not permission to compile/publish and does not freeze mutable
                      callback/provider data. Portable selection/code closure flags
                      remain false; a future compiler must select in its checked
                      invocation. Name discovery and dependency walks keep their
                      separate existing bounds; walks share remaining request work
                      across roots, with actual per-destination policies per walk.
                      No aggregate peak-heap or global phase budget is claimed.

                      Verification: thirteen new selection cases plus 552 prior
                      native cases pass (565 distinct). The final regression run
                      includes the explicit capture-admission repair. WASM
                      compilation, both existing separate-heap WASM drivers,
                      18 component browser cases, ten audit/factory cases and both
                      complete live Login worker/fallback credential privacy
                      scenarios pass. Native builds drained before Studio
                      acceptance. Formatting, 255 local documentation file links,
                      refreshed audit metadata and diff checks pass. All production
                      changes are native-only; current optimized component
                      (11,474,094 bytes) and full (24,202,382 bytes) WASM assets
                      were exercised, not rebuilt. No production JavaScript,
                      application wire change, new WASM endpoint or receiver
                      admission was introduced. Entered credentials still block
                      recovery; automatic whole-Studio recovery stays disabled,
                      with 4 MiB checkpoints and five-second deadlines unchanged.
                      Generated screenshots and Vitest attachments are excluded.
                    - [x] Compile cross-source scalar/list datatype dependencies inside the
                      actual checked host invocation, with fresh original source
                      contexts, explicit implementations and renewed selection.
                      Do not accept a historical selection inspection as permission;
                      test native/query rule ownership, original diagnostics,
                      callback changes, cancellation, budgets and stale contexts
                      before exposing mixed receivers or a WASM entry point.
                      - [x] Add shared native selection admission before compiler
                        construction; test changed/mismatched targets and fresh
                        selector execution instead of historical inspection reuse.
                      - [x] Compile original cross-source scalar/list roots using
                        the actual host and explicit primitive metadata. Test source
                        scopes/grants, inherited facets, root-only validation,
                        missing/foreign implementations, budgets and cancellation.
                      - [x] Refuse unbound QD1/NR1 rule modules and executable
                        tokenizer/validator metadata in the primitive compiler;
                        test that correct source selection cannot activate code.
                      - [x] Bind exact QD1/NR1 rule modules explicitly, preserve
                        original source contexts/policies/diagnostics and weak native
                        ownership; test missing/incorrect registrations, fresh
                        outcomes, source budgets and callback-time retirement/stops.
                        - [x] Add fresh imported scalar query/native rule fixtures
                          for explicit registrations, original datatype/diagnostic
                          ownership, live outcomes and missing/foreign/wrong owners.
                        - [x] Test actual rule-source contexts/policies and shared
                          query/rule/diagnostic budgets across inherited/item rules.
                        - [x] Test retired/dropped owners, removed implementations,
                          stale hosts and callback-time retirement/cancellation;
                          prove compiled snapshots do not retain native callbacks.
                        - [x] Refuse reentrant imported native-rule validation
                          without replaying callbacks or resetting invocation work;
                          prove same-host copied rule sources cannot substitute owners.
                        - [x] Test a combined imported query/native inheritance
                          chain with exact datatype/source ownership, intersected
                          outcomes and shared rule/diagnostic limits.
                        - [x] Run native/WASM/component and isolated worker/fallback
                          Studio Login privacy acceptance; document the supported
                          native-only boundary without enabling receiver admission.
                      - [x] Keep primitive compiled descriptors private and context-bound;
                        test stale/foreign/closed owners and failed-attempt preservation,
                        then run native/WASM/component and isolated Studio Login
                        credential privacy acceptance without enabling admission.

                      The [primitive compiler](../cem-native-schema-capsules.md#fresh-host-primitive-dependency-compilation)
                      renews original source selection inside the actual shared native
                      compiler invocation; it never accepts the inspection result as
                      permission. Exact source scopes/grants and explicit primitive
                      metadata produce private typed scalar/list snapshots with
                      root-only validation. Unbound QD1/NR1 rule sites and executable
                      hooks refuse in the primitive-only path. The explicit
                      imported-rule path now binds QD1/NR1 dependencies with caller
                      owners, actual source contexts and weak native implementations;
                      portable selection/code-closure flags remain false. No new
                      WASM endpoint, wire format,
                      receiver admission or application JavaScript is introduced.

                      Primitive-stage verification: native tests-first checks
                      reproduced missing
                      compiler/admission APIs and the node-metadata admission gap.
                      Nine new fixtures cover compiler/admission behavior (eight
                      compiler cases and one shared traversal case). All 587
                      distinct native cases passed: 394 integration, 179 core and
                      14 shared dependency
                      traversal cases. The WASM-target check, 18 component browser
                      cases, ten audit/factory cases and both WASM drivers passed.
                      Isolated Studio Login recovery passed in worker and fallback
                      modes (two selected cases; six filtered), after native builds
                      drained. The source audit, 262 local documentation targets,
                      focused Rust formatting and diff checks passed. Existing
                      component (11,474,094 bytes) and full (24,202,382 bytes) WASM
                      assets were exercised, not rebuilt. Entered passwords/OTP
                      still block recovery; automatic whole-Studio recovery stays
                      disabled, with 4 MiB checkpoints and five-second deadlines
                      unchanged. Generated screenshots and Vitest attachments
                      are excluded.

                      The [imported-rule contract](../cem-native-schema-capsules.md#explicit-imported-rule-owners)
                      requires exact caller-held rule owners; compilations keep weak
                      owners and private descriptors. Each validation obtains original
                      source query contexts from the actual host, shares query work
                      across inherited/item rules and preserves datatype/diagnostic
                      source arenas. Missing or copied sources, wrong registrations,
                      missing grants, stale/retired/dropped owners and removed native
                      implementations refuse. Callback-time stops and retirement
                      cannot publish success. Native tests exposed and repaired a
                      reentry gap: the entry guard now refuses a nested call before
                      replaying its native callback or starting another allowance.
                      Other executable hooks and portable/runtime admission stay
                      behind the separate gates below.

                      Imported-rule verification: the binding API and native reentry
                      boundary had failing-first fixtures. Twelve new actual-host
                      cases now pass, including combined query/native inheritance,
                      source arenas, wrong registrations, source policies, shared
                      budgets, retirement/stops and weak callback release. All 599
                      distinct native cases passed (406 integration and 193 core/
                      shared traversal). The WASM-target check, 18 component browser
                      cases, ten audit/factory cases and both WASM drivers passed.
                      Isolated Studio Login recovery passed in worker and fallback
                      modes (two selected cases, six filtered), with native builds
                      drained beforehand. Source audit, focused Rust formatting,
                      268 local documentation targets and diff checks passed.
                      Existing component (11,474,094 bytes) and full (24,202,382
                      bytes) WASM assets were exercised, not rebuilt. No application
                      JavaScript, wire change, new WASM endpoint or receiver
                      admission was added. Entered credentials still block recovery;
                      automatic whole-Studio recovery stays disabled, with 4 MiB
                      checkpoints and five-second deadlines unchanged. Generated
                      screenshots and Vitest attachments are excluded.
                    - [ ] Extend separately versioned dependency records for
                      dependency-bearing datatype modules and additional native
                      operation/constant profiles before claiming complete executable
                      or code closure. Preserve original source snapshots and nested
                      format limits; test cross-module cycles and source agreement.
                      - [x] Add a separately versioned, flat native datatype-module
                        exporter/verifier with stable module IDs and original source
                        snapshots; test dependency-bearing chains and shared diamonds
                        without changing CEMDP1 or granting runtime admission.
                      - [x] Reuse shared datatype classification/field checks and
                        leaf verifiers; test local/imported roots, aliases, literal
                        and native references, rule pins and whole-source agreement.
                      - [x] Reuse query/native rule-capsule fixture builders across
                        legacy and module graph tests; prove pins stay inert and
                        malformed leaf capsules retain their own rejection rules.
                      - [x] Reject self/cross-module/local cycles with bounded
                        original-source witnesses; test missing/unreachable modules,
                        exports, aggregate depth/work/bytes and cancellation.
                      - [x] Add repinned malformed/version/unknown/trailing/deep/
                        oversized capsule fixtures and preserve nested leaf limits.
                      - [x] Wire verified module graphs into actual fresh-host
                        selection/compilation with explicit imports, scopes, grants
                        and rule owners before enabling module execution.
                        - [x] Add actual-host module chain/list/diamond fixtures,
                          preserving original sources, root-only validation and
                          fresh selector evaluation through the shared compiler.
                        - [x] Test explicit source/occurrence readiness, imports,
                          per-hop directed grants, foreign/copy/extra scopes and
                          aliases without implicit registration or catalog retention.
                        - [x] Test reachable query/native rule owners, unused exports,
                          original diagnostics, shared limits, retirement/cancellation,
                          reentry and weak callback lifetime release.
                        - [x] Run native/WASM/component and isolated Studio Login
                          privacy acceptance; document the native-only boundary.
                      - [x] Add separately versioned operation/constant dependency
                        profiles, with exact original sources, ownership, limits and
                        compatibility checks before claiming complete code closure.
                        - [x] Add inert CEMOP1 profiles reusing exact CEMNR1
                          operations, original datatype/constant sources and
                          source-backed result contracts; test role, representation,
                          registry, whole-source agreement and owner isolation.
                        - [x] Test repinned malformed/version/unknown/trailing/deep/
                          oversized profiles, leaf limits and cancellation; verify
                          explicit fresh-host linking still requires live owners.
                      - [x] Add a separately versioned dependency-aware operation
                        source format for native-reference fields and constant
                        slots; preserve CEMNR1's closed original-source contract.
                        - [x] Add CEMOS1 export/verification using shared dependency
                          snapshots, operation/signature checks and datatype plans;
                          test native base/list/rule fields and constant slots with
                          original occurrences, contexts and no selector execution.
                        - [x] Test direct/source-backed operation modes, explicit
                          source sharing, foreign/copied inputs and old-format
                          refusal, versions, malformed/repinned records, bounds,
                          resolved targets, source disagreement and cancellation.
                        - [x] Run native/WASM/component and isolated Studio Login
                          privacy acceptance; document the source-only boundary.

                      CEMOS1 verification: a missing-API fixture failed first;
                      nine new integration cases now pass. All 648 native cases
                      passed (450 integration and 198 core/shared traversal), as
                      did the WASM-target check, 18 component browser cases, ten
                      audit/factory cases and both WASM drivers. Isolated complete
                      Studio Login recovery passed through Nx in worker/fallback
                      modes (two selected cases, six filtered), after native builds
                      drained. Focused Rust formatting and source audit passed.
                      Original fields, constant slots and contexts stay passive;
                      existing readers still refuse this separate native format.
                      No application/component JavaScript, runtime entry point or
                      receiver admission was added. Existing component (11,474,094
                      bytes) and full (24,202,382 bytes) WASM assets were exercised,
                      not rebuilt. Entered credentials still block recovery;
                      automatic whole-Studio recovery remains disabled, with the
                      4 MiB budget and five-second deadlines unchanged. Generated
                      screenshots and Vitest attachments are excluded.

                      - [ ] Add separately versioned operation-profile pins for
                        dependency-aware source records; preserve CEMOP1/CEMNR1
                        admission and independently check whole-source agreement.
                      - [ ] Integrate operation/constant profiles with actual-host
                        module compilation and the application dependency envelope
                        through separately versioned contracts; preserve CEMDM1,
                        original selectors, explicit owners and weak lifetimes.
                      - [x] Run native/WASM/component and isolated Studio Login
                        privacy acceptance; document the inert source-backed boundary.

                      The [CEMDM1 module graph](../cem-native-schema-capsules.md#dependency-bearing-datatype-module-graphs)
                      is a separate flat source-backed format. Stable module IDs
                      avoid recursive capsule loading and circular content hashes.
                      Shared datatype inspection and leaf verification preserve
                      original snapshots, exported roots and whole-source agreement.
                      Deterministic module/declaration orders reject local, self and
                      cross-module cycles, with original source-field witnesses for
                      module cycles. Combined depth counts through modules and DT1
                      leaves; outer work and total bytes stay bounded, while nested
                      leaf decoders keep their own existing limits. This remains
                      inert metadata in the wire verifier: selections/code closure
                      are not proven and verification runs no callbacks. Explicit
                      native module entry points now borrow these sources through
                      the same actual-host resolver/compiler as CEMDP1, with original
                      source scopes, per-hop grants, imports and weak reachable rule
                      owners. Existing CEMDP1 APIs do not accept the distinct verified
                      type implicitly. Dependency-aware operation sources and
                      actual-host module/envelope integration remain open above.

                      The [CEMOP1 profiles](../cem-native-schema-capsules.md#versioned-operation-and-constant-dependency-profiles)
                      independently group exact CEMNR1 operation/constant pins by
                      original datatype. Shared native verification and explicit
                      snapshot-checked occurrence mapping preserve original sources,
                      constant provenance, source-result contracts and fresh owners.
                      Roles, kinds, representations, registry/compiler compatibility,
                      duplicate/missing/extra artifacts and phase/byte limits are
                      checked without execution. Literal constants are preserved;
                      native-reference constant slots retain CEMNR1's refusal.
                      Existing fresh-host linkers still require explicit registrations
                      and source scopes; retirement disables compiled proxies and
                      metadata retains no callback owner. Portable closure remains
                      false. Module/envelope installation and runtime admission are
                      separate gates, rather than implicit wire upgrades.

                      The [actual-host module path](../cem-native-schema-capsules.md#actual-host-datatype-module-binding)
                      uses borrowed verified inputs, not a fabricated CEMDP1 capsule
                      or a replacement host. It reuses ordinary source discovery,
                      traversal, compiler and guards. All original source scopes and
                      captured occurrences remain explicit; imports name actual
                      exported roots and grants follow each declaring hop. A bounded
                      metadata closure identifies required fields and reachable
                      query/native owners, while compilation renews actual selections.
                      Unused exported rules require no owner or prepared occurrence.
                      Private typed validation preserves original source/diagnostic
                      arenas, shared limits, reentry protection and weak callback
                      ownership. Portable closure flags, wire formats and runtime
                      admission remain unchanged.

                      Operation/constant-profile verification: a missing-API fixture
                      failed first; nine new integration cases now pass. All 639
                      distinct native cases passed (441 integration and 198 core/
                      shared traversal), including existing module/source/compiler
                      and owner contracts. The WASM-target check, 18 component
                      browser cases, ten audit/factory cases and both WASM drivers
                      passed. Isolated Studio Login recovery passed through Nx in
                      worker and fallback modes (two selected cases, six filtered),
                      after native builds drained. Source audit, focused Rust
                      formatting, 286 local documentation targets and diff checks
                      passed. Existing component (11,474,094 bytes) and full
                      (24,202,382 bytes) WASM assets were exercised, not rebuilt.
                      This is a separate native-only format; existing wire semantics,
                      runtime endpoints and receiver admission remain unchanged.
                      No application/component JavaScript was added. Entered
                      credentials still block recovery; automatic whole-Studio
                      recovery remains disabled, with 4 MiB checkpoints and
                      five-second deadlines unchanged. Generated screenshots and
                      Vitest attachments are excluded.

                      Module-graph verification: a missing-API fixture failed first;
                      eleven new integration and five native verifier/order cases
                      now pass. All 615 distinct native cases passed (417 integration,
                      198 core/shared traversal), including the legacy CEMDP1 source,
                      selection, compilation and ownership contracts. The WASM-target
                      check, 18 component browser cases, ten audit/factory cases and
                      both WASM drivers passed. Isolated Studio Login recovery passed
                      in worker and fallback modes (two selected cases, six filtered),
                      after native builds drained. Source audit, focused Rust
                      formatting, 278 local documentation targets and diff checks
                      passed. Existing component (11,474,094 bytes) and full
                      (24,202,382 bytes) WASM assets were exercised, not rebuilt.
                      No application JavaScript, new WASM endpoint or receiver
                      admission was added. Entered credentials still block recovery;
                      automatic whole-Studio recovery stays disabled, with 4 MiB
                      checkpoints and five-second deadlines unchanged. Generated
                      screenshots and Vitest attachments are excluded.
                    - [x] Add native datatype-root preparation invocations using
                      shared evidence immutability/metadata checks and cumulative
                      preparation/validation budgets. Add scalar/list, missing/
                      empty/rejected/pending/unavailable and untracked-view fixtures.
                    - [x] Bind sealed datatype evidence to the exact compilation,
                      root, frozen input/candidate and invocation; cover foreign
                      host/root/invocation, source mismatch, stale context, dropped/
                      retired owner and callback-time cancellation/retirement.
                    - [x] Reject retained root and attribute evidence after a
                      selected/inherited tokenizer retires; add a shared bounded
                      availability check without repeating tokenization.
                    - [x] Verify fresh validation on every consumption, cumulative
                      budget exhaustion and reentrancy refusal without callback
                      replay, raw token export or wire-format changes.
                    - [x] Run native/WASM/component and isolated Studio Login
                      credential privacy acceptance; retain behavior/query/imported
                      closure and mixed receiver format as separate pending gates.

                    Native datatype-root preparation now exposes checked frozen
                    invocations with sealed immutable values, lexical text, decoded
                    spans, original candidate/source metadata and no cached verdict.
                    Consumption reruns the shared validator under one cumulative
                    reservation for preparations, output/input visits, rules,
                    comparisons and diagnostics. Shared immutable/metadata checks
                    and refund accounting are reused by attribute preparation.
                    Compilation ownership is weak; closing/dropping/retiring the
                    compilation, invocation or actual host/context invalidates
                    evidence. Wrong roots/hosts/invocations and untracked views
                    refuse. Readiness hooks and preparation/validation callbacks
                    cannot obtain reentrant allowances; unwind spends reservations.
                    A reproduced tokenizer-retirement gap is repaired for root and
                    attribute evidence through read-only checks of selected and
                    inherited tokenizers, without tokenization replay. Callback
                    cancellation/retirement cannot complete a handoff. No wire
                    format, endpoint, component admission or permission is added.
                    Complete behavior/query/imported closure, mixed receiver
                    contracts, local facets and external typed-producer admission
                    remain separate pending gates.

                    Verification: 15 new root preparation cases and one shared
                    attribute regression pass alongside 524 prior native cases
                    (540 distinct total). Red fixtures reproduced tokenizer-owner
                    retirement and readiness-hook reentrancy gaps before repair.
                    Selected and inherited tokenizer retirement both refuse retained
                    receipts. WASM compilation, both optimized separate-heap WASM
                    drivers, 18 component browser cases, ten audit/factory cases
                    and both full Login worker/fallback credential privacy scenarios
                    pass. Both WASM profiles were rebuilt with matched Binaryen
                    132.0.0 -Oz optimization: selected component 11,474,094 bytes,
                    full 24,202,382 bytes. The selected build uses the documented
                    generated fixture registry; a build without it correctly denied
                    the fixture calls. Canonical/vendor full assets match. Native
                    builds drained before Studio acceptance. Formatting, 245 local
                    documentation file links, audit freshness and diff checks pass.
                    No production JavaScript, new WASM endpoint, wire codec or
                    cached validation verdict was added. Entered credentials still
                    block recovery; automatic whole-Studio recovery stays disabled,
                    with 4 MiB checkpoints and five-second deadlines unchanged.
                    Generated WASM, screenshots and Vitest attachments are excluded.
                - [ ] Enroll portable query source/binding/invocation owners in
                  actual worker/owned-fallback generation and inventory groups;
                  add fresh-heap transport, shutdown/replacement, stale receipt,
                  drain/cancellation and entered-credential recovery fixtures
                  before exposing a new endpoint or enabling admission.
      - [ ] Bind compiled declarations, retained CSS installation and shared
        behavior/provider ownership to the reduced ABI. Prove the compiled form's
        worker/fallback edits, credential refusal and generation handoff without
        substituting the full tooling bootstrap.
      - [ ] Bind generated registry/artifact/input contracts to the selected
        application build and specialize remaining execution/type/source roots.
        Audit linked symbols and optional-module handoff before claiming unused
        language/schema/function implementations are absent.
- [ ] Assemble and measure the compiled-UI + JSON shopping-cart profile first.
  Add declarative shared GraphQL transport only if existing resource capabilities
  cannot express it; alert the user before implementing new shared JavaScript.
  Register the usable app as an ordinary Studio example project when complete,
  with real quantity/remove actions and build/preview/offline fixtures. No local
  UI JavaScript, fake action buttons or credential-policy bypass counts as done.
- [x] Add reproducible audit tooling and fixtures for catalog/declaration drift,
  capability inheritance and shared behavior imports; conservative static/dynamic
  import evidence; matched WASM section/interface/compression measurements and
  Cargo/native registry roots. Keep source references distinct from linked-code
  liveness and candidate module labels distinct from removability proof.
- [x] Add isolated baseline fixtures for a small control/form flow, native
  JSON/CSV data-view parsing and source-editor highlighting. Capture cold/first-use
  requests and sizes, correctness, native live/linear memory where available,
  and host-memory availability without collecting user content. Verify baseline
  fixture loading never adds component/application runtime behavior or resets
  user projects/drafts.
- [x] Audit the generated catalog/declarations, shared behavior imports and native
  dependency/export/registry roots. Record required core, mode selectors and
  candidate language/tooling boundaries for all 59 components, including dynamic
  provider/template, editor, media and table extension points. Publish the
  [source audit](../cem-components-wasm-audit.md) and
  [six worker/fallback reference runs](../cem-components-wasm-baseline.md).
  Verification: eight audit tests, six isolated component flows, audit freshness,
  script lint and live Login credential-refusal/clear-and-recover acceptance.
  No production runtime/UI behavior or credential policy changed. This source
  inventory does not prove native liveness, exhaustive mode coverage or removal.
- [ ] Extend the baseline with repeated cold/first-use latency, active-worker and
  aggregate host/native memory, all-capabilities/offline totals and clean npm
  consumer loading. Keep unavailable counters explicit. Diagnose duplicate full
  WASM acquisition observed in local no-store worker runs (two transfers of one
  unique binary; fallback one). Retain matched hashes/toolchains and fixed budget
  selection after the native boundary/prototype report, not an invented minimum.
- [ ] Prove native call reachability and dynamic mode/provider/template closure
  alongside module extraction. Keep the conservative source matrix separate
  from linked-code omission evidence; map required source packs and compiled
  modules for each requested capability before moving its implementation.
- [ ] Define and prototype the versioned core/module ABI and capability catalog
  with native fixtures for one isolated optional operation and one importer
  feeding a retained component. Prove typed CEM-tree/source-owner handoff or
  original native reimport/recompile without cross-heap handles, JSON AST
  conversion or JavaScript parsing; retain types, namespaces/base URIs, ranges,
  diagnostics, cancellation and lease disposal. Keep a required parser in core
  until this boundary passes, and record the selected module-loading mechanism.
  Use editor paint/highlighting as the isolated-operation candidate and CSV
  feeding `cem-data-view` as the typed retained-data candidate from the audit.
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
