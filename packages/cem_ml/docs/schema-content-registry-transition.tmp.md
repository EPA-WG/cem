# Temporary Transition Note: Current Code to Schema Package Registry

Status: temporary migration plan

The target architecture is defined in
[`../../../docs/cem-ml-schema-content-registry-design.md`](../../../docs/cem-ml-schema-content-registry-design.md).
This note tracks how the current Rust code can transition toward that design.

## Current State

Current identity and conversion behavior is split across several modules:

- `cem_ml::engine::FormatIdentity` carries content type, schema, namespace, and
  base URI fields.
- `cem_ml::lifecycle::LifecycleRegistry` maps some source/target identities to
  parser/export layer formats.
- `cem_ml::transform_template::TransformTemplateAdapterRegistry` dispatches
  template compilation/execution by content type, schema, and namespace.
- `cem_ml_cli::dispatch` assembles `EngineContext`, resolver maps, transform
  template adapters, and CLI-specified source/target identity fields.
- `cem_ml_transform_cem_ql` provides executable CEMT/CEM-QL template support
  outside `cem_ml` to avoid a dependency cycle.
- HTML, CEM core, run config, transform config, native template, projection,
  and observability schema identities exist as constants or embedded artifacts,
  not as manifest-owned schema packages.

## Transition Principle

Do not replace the current runtime in one step. Introduce the target concepts as
data and registry surfaces first, then migrate existing hard-coded paths into
schema package manifests and converter edges.

## Phase 1: Registry Contracts

- Add `SchemaRegistry` and `ConversionRegistry` contracts to `cem_ml`.
- Keep `LifecycleRegistry` operating, but implement it through registered
  descriptors/edges where practical.
- Add content-type essence normalization and ambiguity diagnostics in one shared
  place.
- Register existing built-ins as in-memory descriptors:
  - CEM core;
  - classic HTML;
  - run config;
  - transform config;
  - native template;
  - DOM / AST / events semantic projection schemas with primary CEM binary
    content types and optional JSON debug views;
  - observability report events.

## Phase 2: CEMT Converter Descriptors

- Add descriptor-backed CEMT converter edge support.
- Register packaged CEMT converter assets with content type
  `application/vnd.cem.transform+cem`.
- Register existing `.cemt` templates as converter assets where they represent
  typed artifact-to-artifact transforms.
- Keep CEMT execution in `cem_ml_transform_cem_ql`.
- Let converter edges call `TransformTemplateAdapterRegistry` instead of
  duplicating template execution.

## Phase 3: Manifest Prototype

- Add a prototype `package.cem` format with schema
  `https://cem.dev/ns/schema-package/1`.
- Generate Rust registry entries from the `.cem` manifest for one schema family
  beyond the hand-registered projection schema descriptors.
- Generate CEM binary/chunk descriptor artifacts as the preferred runtime/cache
  projection once the binary format is canonized.
- Generate JSON + JSON Schema and XML + RELAX NG manifest projections as build
  artifacts, not source inputs.
- Start with `cem-element` or a projection schema because they exercise
  schema-owned content types and CEMT conversions.
- Keep generated code checked into the package only if needed for deterministic
  offline builds.

## Phase 4: Convert Existing Paths to Edges

Move these into converter edges:

- CEM source bytes -> generic CEM DOM/AST/events;
- HTML source bytes -> normalized HTML5 DOM, Rust fallback;
- XML/SVG source bytes -> normalized XML DOM, Rust fallback as needed;
- CEM DOM -> HTML, CEMT where possible and Rust serializer where needed;
- CEM DOM -> XML, Rust serializer initially;
- CEM DOM/AST/events -> canonical CEM binary/stream artifacts;
- CEM DOM/AST/events -> optional projection JSON outputs for CLI/debug/interchange;
- legacy custom-element/XSLT -> CEM template/CEM DOM.

## Phase 5: Schema Packages

- Move source schema artifacts into schema package folders.
- Move schema-specific CEMT converters next to their schema manifest.
- Keep Rust fallback hooks near the schema package, with bridge crates for
  dependency-heavy implementations.
- Generate registry tables from package manifests.

## Phase 6: CLI and Diagnostics

- Add CLI registry inspection:
  - `cem-ml registry list schemas`
  - `cem-ml registry list content-types`
  - `cem-ml registry list converters`
  - `cem-ml registry plan --from-content-type ... --to-content-type ...`
- Add transform/convert diagnostics that report selected converter path,
  implementation kind, lossiness/canonicalization, and schema owner.

## Immediate Implementation Candidate

The first useful slice is:

```text
FormatIdentity
  -> SchemaRegistry descriptor lookup
  -> ConversionRegistry direct-edge lookup
  -> CEMT converter descriptor execution through cem_ml_transform_cem_ql
```

This slice proves source identity, target identity, schema-owned content types,
and CEMT-first conversion without requiring all current conversion code to move.

Progress:

- [x] Direct CLI source validation selection resolves input content types through
      `SchemaRegistry` descriptors and requires any explicit `--schema` value to
      match the registry-owned schema before selecting a Rust fallback
      validator.
- [x] Add `ConversionRegistry` direct-edge lookup between resolved source and
      target identities.
- [x] Register CEMT converter descriptors as primary conversion edges with Rust
      fallback hooks where the CEMT edge is not yet implemented.
- [ ] Execute selected converter descriptors through CEMT template adapters,
      falling back to registered Rust hooks while planned CEMT edges are not
      executable.
