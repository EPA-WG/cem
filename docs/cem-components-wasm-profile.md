# Minimal component runtime WASM profile

Status: separate compiled-template/JSON WASM prototype with passive declaration
and retained-style ownership and bounded native attribute/slice inputs implemented;
complete live component distribution and runtime switching remain in progress. Requested 2026-10-28. Product sequencing belongs to
[roadmap phase 3.1.3](../roadmap.md#313-minimal-component-runtime-wasm-profile);
implementation steps belong to the [execution checklist](todo.md#component-application-wasm-profile-separate-target).

The goal is a smaller initial WASM download for live `cem-element` applications
using `@epa-wg/cem-elements` and `@epa-wg/cem-components`. Every current component
capability remains available through the core or a module loaded when needed.
Static HTML output alone does not meet this live-component contract. Keep the
current full-runtime build, CLI, Studio defaults and package entry points intact.

## Application build and deployment boundary

Clarified 2026-10-29: determine the initial runtime from the deployed app's
requirements. Use the [shopping-cart reference](cem-components-shopping-cart-profile.md)
as the first compiled-UI + JSON acceptance case. The complete component catalog
is the capability/parity inventory; it does not require every parser, schema or
component behavior to be present at startup in every application.

The app build validates authored components, templates, styles, static data and
declared operations, compiles native template/expression artifacts and records
the exact required component/module/source closure. Runtime loads pinned native
artifacts and executes the compiled plans against changing data. General schema
authoring/validation, source parsers, converters and tooling can be omitted from
initial download when they are unnecessary for that app's admitted inputs.
Additional formats and source-driven compilation remain available through demand
modules. Preserve the native evaluator's supported semantics and explicit module
requirements for dynamic function/resource choices.
Reuse a shared compiled-UI + JSON core across apps; the manifest selects required
capabilities and preload policy. Components continue to share one runtime rather
than embedding a private engine for each control.

Build-time validation cannot cover API responses, user edits or resources that
arrive later. Retain bounded native JSON decoding/encoding, artifact/ABI/integrity
checks, necessary runtime type/input contracts, and compact app-specific variable
and response checks generated during the build. These checks should not require
shipping the full general-purpose schema catalog. GraphQL operation validation
is build tooling for fixed operations; the cart client sends their strings and
JSON variables, then handles JSON data/errors. Server pricing, stock and mutation
validation remain authoritative.

The current native `CompiledTemplateArtifact` already reloads binary template
and expression IR without invoking the CEM-ML or CEM-QL source compiler. Legacy
v1 stylesheet fields reload CSS source through the native importer; opt-in v2
reloads pinned inline CSS trees without reparsing source. Opt-in v3 additionally
carries build-closed imported stylesheets and rechecks current URL/MIME/integrity
and byte policy without CSS parsing. The [compiled CSS contract](cem-components-shopping-cart-profile.md#compiled-imported-stylesheet-closures)
preserves v1/v2 authorized source imports and dynamic/source styles; the full
binary still contains their parser.
Native closure builds also emit a [pinned requirements inventory](cem-components-shopping-cart-profile.md#native-compiled-requirements-inventory)
from compiled template/query IR and retained CSS: exact call arities, literal
format demands, source/artifact identities and unresolved host/provider contracts.
The native [selected-registry prototype](cem-components-shopping-cart-profile.md#native-selected-registry-prototype)
now generates exact reviewed entries and transitive execution-unit requirements,
and renders the cart entry without full function-catalogue assembly. It remains
available in an [isolated WASM prototype](cem-components-shopping-cart-profile.md#isolated-compiled-component-wasm-prototype); complete component ABI, behavior/provider and input/type closure stay open.
The [retained declaration/style layer](cem-components-shopping-cart-profile.md#retained-passive-declarations-and-stylesheet-leases)
now owns pinned passive declarations and native scoped v2/v3 style leases. The
small shared `cem-elements/component-generation` adapter initializes asynchronously,
uses existing DOM style ownership, isolates heaps and refuses foreign roots during
manual cleanup. Passive declarations also render shared CEMV attribute/slice
inputs with typed/source/reference retention, fixed admission limits and reserved
runtime-root refusal. Optional host instance identity uses its own bounded input
and cannot be supplied through JSON/attribute/slice roots. Pinned receiver
declarations preserve required/default/conversion/facet checks. Each render owns
a temporary native input frame. Passive native island admission now checks
qualified lifecycle/payload identity and produces HTML or a native CEMV projection
with retained payload/slot/source references. Native island domain views now bind
canonical string attributes, an exact DOMStringMap dataset and explicitly typed
scalar/record/array slices without JSON AST conversion. Receiver normalization
preserves lexical dataset strings; CEMV scalar re-export retains original source
keys. Alias collisions, malformed types/dataset/array indices, nonempty unsupported
state domains and host/payload credential metadata without capture proof refuse.
Reviewed native payload references now preserve their occurrences and original
target/source-parent identities. Slots follow direct reference targets in order;
empty references add no entry and nested references remain inside their element.
The complete rendered target closure is checked before publication, including
credential refusal, with native cycle/depth/expanded-work admission guards.
The native-only application declaration wrapper now pins reviewed named scalar
facet contracts alongside unchanged v2/v3 IR. The existing generation `open` entry
verifies exact named-type requirements and schema/source/contract identities
before publishing an owner. JSON, binary-input and island frames bind those
contracts natively without schema/catalog fetches; local facets still intersect.
Executable/tokenizer/node-schema contracts, remaining island domains,
widget/provider input and recovery-group contracts remain open;
no default Studio backend or credential policy changes are implied.
The browser's `retainCemMlTemplateArtifact` path still takes original
template source for its optional source-cache behavior. A separate strict
`retainPinnedCemMlTemplateArtifact` adapter and `pinned-template-artifact-v1`
processing input now admit native binary IR from build-pinned source/artifact
hashes without source text. Worker/fallback processing shares this path and never
falls back to source compilation when a deployment pin is refused. The processing
protocol is v30; old source/cache inputs remain supported.

Native build-generated declaration metadata and explicit shared
`CemElementRuntime.registerCompiledDeclaration` now register canonical components
from hash-pinned metadata and binary IR, preserving capability/form association,
styles and collision checks. Worker/fallback form/action composition is covered
with source downloads forbidden. See the [build and registration contract](cem-components-shopping-cart-profile.md#build-generated-declarations).
Declarative `compiled-src`/`compiled-template-src`/`compiled-hash` loading and a
native static declaration-closure builder are also implemented. The six-component
form fixture loads with no XHTML downloads and now passes two complete physical
generation recovery cycles per mode, preserving edits/forms/styles and credential
refusal. Stale input/pin handoffs also refuse without overwriting retained state.
See the [recovery acceptance](cem-components-shopping-cart-profile.md#compiled-application-recovery-acceptance).
A complete function/resource closure remains pending. Ordinary `cem-element src`
continues to support source. Keep native CSS parsing until compiled imported closures are admitted;
compiled inline styles alone do not complete that exclusion gate.
The isolated prototype separates exports and removes general dispatch references
from its WASM evaluator. Its measured reduction is evidence for this bounded
entry-template/JSON surface; it does not establish per-language omission or a
complete live-component replacement. The full runtime retains its general registry.

## Baseline and evidence

The [capability, dependency and size audit](cem-components-wasm-audit.md)
covers all 59 canonical component declarations, 24 shared behaviors and the
current native binding/dependency roots. The [isolated browser baseline](cem-components-wasm-baseline.md)
checks form editing, native JSON/CSV rendering and editor coloring in both
worker and fallback modes. These are reference measurements of the full runtime;
they are not measurements of the isolated compiled-template/JSON prototype.
Complete live-component integration into that reduced target remains open.
The browser baseline retains the original full-profile comparator identified by
its recorded hash. The pinned-admission browser/WASM tests use the newly rebuilt
runtime; the audit records that current binary separately.

The audited binary is dominated by its code section; data is a much smaller
share. Those sections do not attribute bytes to removable languages or assets. The
audit identifies CSV parsing within native `data:read` evaluation and editor
highlighting as capabilities whose module boundaries must be proven for apps
that request them. The JSON-only cart does not request either at startup.
Source metadata is conservative evidence, not a complete native call graph or
an exhaustive dynamic-mode proof. The local no-store worker runs also fetch the
same WASM twice, so unique file size and repeated transfer cost are reported
separately. Repeated timing runs, active-worker/aggregate memory and the full
capability/offline total remain follow-up measurements.

The component runtime currently vendors the CEM-QL binary, which depends on the
common CEM-ML engine. The native binding facade imports that generated runtime
statically. Default Cargo features include debug control, and the common crate
includes multiple language implementations. Moving schema/template source bodies
into asset packs does not by itself remove those compiled implementations.

The original full-profile reference after `9c74b08c`, measured on 2026-10-28:

| WASM-only measure | Bytes | MiB |
| --- | ---: | ---: |
| Uncompressed | 24,075,631 | 22.96 |
| Gzip level 9 | 7,332,268 | 6.99 |
| Brotli quality 9 | 5,394,379 | 5.14 |

Artifact: `packages/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_bg.wasm`.
SHA-256: `b37fad5b65edd6a4510e7e9fc2751abc49a9913b40886a163b151834b4eab47a`.
These are reference measurements, not a promised minimum, RAM measurements or
proof that the current local server supplies HTTP compression. Reproduce them
with the same source, toolchain, optimizer and compression settings before
setting a core download budget. Record the complete-runtime comparator from the
same commit; do not compare a new optimized core against an old debug binary.

Reproduce the audit from the repository root with Node and the installed Rust
WASM toolchain available on `PATH`, generated catalog/full-runtime artifacts,
the matching `dist/reports/wasm-optimization/cem-ql.json`, and cached locked Cargo
dependencies. The browser baseline also requires Playwright Chromium and an
assembled demo deployment with its inventory (default `~/cem-bin`; pass a path
as the first argument to use another deployment). It opens isolated contexts
and leaves existing browser sessions, projects and drafts untouched.

```bash
node --test tools/scripts/audit-components-wasm.test.mjs
node tools/scripts/audit-components-wasm.mjs --write-doc
node tools/scripts/audit-components-wasm.mjs --check
node tools/scripts/measure-components-wasm.mjs --write-doc
```

The first script refuses declaration/catalog drift and mismatched canonical,
vendored or optimizer hashes. The baseline verifies deployed declaration bytes
and published response hashes against the deployment inventory. Generated JSON
evidence stays under `dist/reports/components-wasm/`; the Markdown reports are
committed reference observations, not timing thresholds.

Source owners:

- [CEM-QL native features/dependencies](../packages/cem_ql/Cargo.toml) and
  [common engine dependencies](../packages/cem_ml/Cargo.toml).
- [Native component binding facade](../packages/cem-elements/src/lib/internal/runtime-support/native-runtime.ts).
- [Owned package loader and demand transport](../packages/cem-elements/src/lib/schema-asset-pack.ts).
- [Existing asset-pack measurements](schema-asset-profile-measurements.md) and
  [release optimizer contract](wasm-deployment-size.md).

## Separate target and packaging contract

Proposed target names, to implement through the existing build graph:

| Target | Ownership and output |
| --- | --- |
| `cem_ql:build:wasm-components` | Native core profile, matching generated bindings/types/factory, versioned optional module catalog and profile-specific ABI/size evidence. |
| `cem-elements:build:components-runtime` | Shared component loader, worker/fallback adapters and matched native core/module files under a separate output root. |
| `@epa-wg/cem-components:build:components-runtime` | Consumer-facing bundle assembly and package verification with declarations, shared runtime and theme dependencies. |

The final target depends on the first two. Publish an explicit component-runtime
package export/subpath and generated import map; retain the existing full-runtime
exports. `cem-elements` owns reusable JavaScript behavior, loader orchestration
and native bindings. `cem-components` owns declarative UI. The component bundle
references that one shared runtime rather than privately copying another engine
per component. Installed packages and standalone bundles must resolve the same
pinned artifact cohort without pulling Studio or its command-runtime deployment.
Select one coherent profile through the entry point/import map; diagnose an
accidental full/minimal binding mixture rather than downloading both engines.

Use isolated output/cache/integrity domains for full, component-core and optional
modules. Keep common version, profile, ABI, module dependencies and hashes in the
published manifest. A full -> component -> full build must neither overwrite
another profile's files nor leave scratch inputs in its generated directory.
Keep this opt-in bundle out of the default full-demo deployment unless selected
explicitly. Any demo integration retains the existing below-25-MiB per-file and
999-file deployment guards. Target names above are proposals, not commands
available today.

## Core and optional capability boundaries

Generate a capability-to-module matrix from the component catalog, declarations,
intrinsic capability metadata and actual native call/import graphs. Include
external templates, payloads, data providers and dynamically selected modes;
static string matching alone is insufficient to prove a dependency unused.

| Layer | Planned responsibility |
| --- | --- |
| UI core | Native CEM tree/XDM values and ownership, compiled template/expression execution, required shared behaviors, DOM/patch identity, source provenance/diagnostics, style/scoping behavior, resources, cancellation and recovery primitives. |
| App import closure | Native binary artifact decoding plus only the app's required runtime formats; JSON for the first cart. Existing HTML/XHTML declaration loading, CEM-ML/CEM-QL source compilation and native CSS parsing remain dependencies of paths that still use them; prove compiled replacements before exclusion. |
| Build-only tooling | Authored-source parsers, general schema validation, template/expression/style compilation, GraphQL schema/operation validation and dependency/contract generation. Make source-driven authoring available through demand modules when the deployed app admits it. |
| Source asset packs | Optional schema bodies, formatter/colorizer templates and other declarative resources through the existing owned-pack contract. These do not each need a new WASM binary. |
| Compiled language/conversion modules | Candidates include Markdown, SCSS, additional data formats, XPath source operations, legacy/standards XSLT and specialized conversions. Move only dependencies proven separable from the UI core. |
| Tooling modules | Full Parse/Inspect/source export, language-specific formatting/coloring, command/graph execution and debugger integration when requested. Rendering diagnostics and recovery/admission guards remain core responsibilities. |

Do not trim the supported expression semantics merely to obtain a smaller
binary. Preserve common XDM operations needed by CEM-QL even when a distinct
XPath parser or tooling surface can be deferred. Audit Rust module/dependency
roots, registries, initialization and exported functions: removing a JavaScript
wrapper or relocating source assets is not proof of native code omission.
Retain small native dispatch/introspection hooks where tooling activation needs
access to core state; an external debugger cannot restore instrumentation that
was compiled out. Verify the complete tooling path before deferring its engine.

Keep all component modes available, including source-editor coloring, data-table
imports, media previews, optional template languages and external providers.
For example, text editing can remain ready while a requested language highlighter
loads; loading must preserve text, selection, composition, focus and scroll.
The capability matrix must distinguish installed, available-on-demand, loading,
ready, failed and unavailable-offline states rather than silently reducing a
component's documented behavior. Existing full-profile synchronous APIs must not
silently become asynchronous; add explicit preload/readiness contracts for the
new distribution, with shared adapters and compatibility checks.

## Native module and ownership contract

Prefer separately instantiated optional worker/owned-fallback modules and reuse
existing operation, resource and generation transports. This permits release of
an optional worker after its live leases are gone. It does not assume that Rust
crates become browser-loadable plugins by adding Cargo features, nor that a
shared-memory dynamic linker already exists.

Before moving an importer/evaluator out of core, prototype both an isolated
operation and a format whose parsed data must participate in a retained core
render. Distinct heaps cannot share pointers, handles, compiled templates or
native owner IDs. Original input bytes may be reimported/recompiled natively
where that preserves the contract; parser-produced retained data needs an
explicit versioned native CEM-tree transfer/reload contract. Preserve types,
namespace/base/source identities, source ranges, original owners and diagnostics.
Never substitute JavaScript objects, JSON AST roundtrips or rendered HTML for
that native handoff. If the handoff cannot yet preserve the contract, keep the
required parser in core until the boundary is implemented and verified.

Module manifests declare native/host ABI versions, dependencies, supported
operations and identities, hashes, and required source packs. Pin their complete
closure to the core build; reject mixed versions, corrupt bytes, missing imports
or incompatible memory/features before publication. A module requirement is
advisory metadata, not a host permission, replacement grant or native owner.

The [portable native schema contract design](cem-native-schema-capsules.md)
specifies build/export, inert verification, explicit fresh-host linking and
atomic publication for future executable/node-valued receiver contracts. Live
callbacks, preparation tokens and grants stay in-process. The existing v1
application envelope remains scalar-only; query exporters and executable
linkers remain separate implementation gates.
The native-only `CEMSC1` structural exporter/verifier implements the literal
single-schema subset and inert shared-model reload. A concrete fresh native host
can now explicitly link a literal node receiver; live invocation receipts retain
the exact context, originating attribute, selected contract and local metadata.
Shared-host acceptance keeps reference readiness, source diagnostics and directed
crossing grants independent of the artifact. Query/native implementation linking,
WASM transport and compiled-component receiver admission remain separate gates;
the deployed WASM ABI is unchanged by this native-only adapter.

Do not equate downloaded code with an active native generation. Register every
physical endpoint and its resources with the shared ownership/recovery group.
Preparation, atomic publication, cancellation and exact-owner disposal retain
existing admission and inventory rules. Unloading requires no active work,
leases or retained results; restarting a dedicated worker releases its heap.
Treat compiled-module caching separately from instances and their memories.

## Declarative demand and lifecycle

The shared `cem-elements` loader consumes a generated capability catalog and
native requirement receipts. Declaration/template language, admitted resource
identity, selected component mode or an explicit operation can request a module.
Use bounded deduplicated fetch/compile/activation, dependency ordering, integrity
checks and shared preload/readiness APIs. Applications may configure preload and
allowed module origins without writing per-component loading behavior.

Resolve requirements before committing the affected render/operation. For a late
dynamic demand, use a resumable native boundary that retains state and provenance;
do not replay side-effecting commands to discover or activate capabilities.
Ignore stale completions after source, owner, configuration or generation changes.
Failure or cancellation preserves the last committed UI and edited controls,
reports a shared structured diagnostic and permits an explicit retry. Offline
first use without cached bytes reports unavailable; it must not contact an
undeclared service, upload content or drop to a different language silently.

No full engine, optional compiler or optional source pack should be eagerly
imported by the minimal entry point, worker bootstrap, registry initialization or
service-worker installation. Cache the core shell by default; offer explicit
feature-set preload/offline installation. Activated modules and source packs
must survive offline reload and coordinated fresh-generation recovery. Demand
must stay separate from permission and package replacement authority.

Preserve entered-password/OTP recovery refusal until the user clears the value,
manual whole-Studio recovery, the 4 MiB checkpoint budget and existing deadlines.
Loading another module must not route around a privacy refusal. Do not cache
credentials or user source payloads in public deployment caches. Checkpoints
record portable module/source identities and original admitted input policy,
never live native handles. See the [credential privacy contract](committed-checkpoints.md#credential-privacy-policy-for-live-login-recovery).

## Delivery and acceptance gates

1. Use the catalog audit as a comparator, then define the shopping-cart manifest
   and prove build-time compilation, native binary reload and changing JSON data
   without source recompilation. Generate app dependency and compact input
   contracts; connect source-free artifact admission and required styles to
   shared worker/fallback registration. Measure this app's actual runtime closure.
2. Prove module ABI and native data handoff using a small optional operation and
   an importer feeding a live component, including refusal and source provenance.
3. Introduce native dependency/registry/export partitions and the distinct
   compiled-UI + JSON target. Demonstrate actual code omission and preserve
   baseline runtime checks; retain critical diagnostics/guards.
4. Add the shared loader, declarative demand, worker/fallback publication and
   readiness contracts; integrate existing source packs without duplicate engines.
5. Extract optional language/conversion/tooling modules incrementally behind the
   capability matrix. Run parity and ownership checks after each extraction.
6. Assemble clean-consumer packages/bundles and explicit offline feature sets;
   test relative/subpath URLs, import maps, cache updates and profile isolation.
7. Compare full and component profiles across the entire current catalog and
   mode matrix, dynamically requested formats, late imports and native inputs.
   No skipped capability, JavaScript parser/renderer substitute, weakened
   interaction/accessibility contract, privacy change or relaxed deadline may
   count as size progress.
8. Publish reproducible measurements and fixed CI budgets agreed from the audited
   prototype. Require smaller initial raw/gzip/Brotli WASM and report JS, source
   packs, first-use latency, peak memory, all-modules total and duplication too.
   All-modules cost can exceed the monolith; it must remain visible. Set explicit
   per-file and initial-load limits, not thresholds recalculated to fit each run.

Fixture checklist details live only in `todo.md`. Native tests precede browser
integration. Browser gates cover cold start without optional downloads, concurrent
and cancelled demand, corruption/version refusal, offline/preload behavior,
worker/fallback recovery and component/accessibility/theme equivalence. Bundle
checks prove consumers do not accidentally import both full and minimal runtimes.
Only advertise a minimum size after a measured profile passes these gates.
