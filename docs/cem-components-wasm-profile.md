# Minimal component runtime WASM profile

Status: planned separate build and distribution; no target or runtime switch is
implemented by this document. Requested 2026-10-28. Product sequencing belongs to
[roadmap phase 3.1.3](../roadmap.md#313-minimal-component-runtime-wasm-profile);
implementation steps belong to the [execution checklist](todo.md#component-application-wasm-profile-separate-target).

The goal is a smaller initial WASM download for live `cem-element` applications
using `@epa-wg/cem-elements` and `@epa-wg/cem-components`. Every current component
capability remains available through the core or a module loaded when needed.
Static HTML output alone does not meet this live-component contract. Keep the
current full-runtime build, CLI, Studio defaults and package entry points intact.

## Baseline and evidence

The component runtime currently vendors the CEM-QL binary, which depends on the
common CEM-ML engine. The native binding facade imports that generated runtime
statically. Default Cargo features include debug control, and the common crate
includes multiple language implementations. Moving schema/template source bodies
into asset packs does not by itself remove those compiled implementations.

The current release artifact after `9c74b08c`, measured on 2026-10-28:

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
| UI core | Native CEM tree/XDM values and ownership, canonical CEM-ML declarations/templates, component-used CEM-QL evaluation, DOM/patch identity, source provenance/diagnostics, native CSS/scoping/selectors, resources, cancellation and recovery primitives. |
| Core import closure | HTML/XHTML declaration loading and JSON/XML or other imports required by baseline component flows. Determine the exact closure by fixtures; keep required functionality in core until its modular handoff is proven. |
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

1. Inventory the current full runtime and every component/mode's native closure;
   add baseline network, size, latency and heap/owner measurements first.
2. Prove module ABI and native data handoff using a small optional operation and
   an importer feeding a live component, including refusal and source provenance.
3. Introduce native dependency/registry/export partitions and the distinct core
   target. Demonstrate actual code omission; retain critical diagnostics/guards.
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
