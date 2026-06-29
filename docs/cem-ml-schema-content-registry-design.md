# CEM-ML Schema Content Registry Design

Status: target architecture

This design chooses a manifest-driven schema package architecture for CEM-ML
content identity, schema ownership, and content-to-content transformation. It
optimizes for long-term schema evolution rather than the current implementation
shape.

## Decision

CEM-ML schemas are distributed as schema packages. A schema package owns:

- schema URL and version identity;
- public content types;
- namespace identities, when applicable;
- schema source artifacts;
- CEMT converters;
- optional Rust fallback converter hooks;
- fixtures and conformance tests;
- generated registry metadata for host runtimes.

CEMT (`.cemt`, CEM-ML transform template) is the primary conversion mechanism.
Rust is a fallback for parser recovery, serialization, performance-critical
streaming, host integration, graph-heavy algorithms, and bootstrapping.

## Package Shape

Each schema family has a package directory. Each major schema version has its
own source folder.

```text
schema-packages/
  cem-element/
    v1/
      package.cem
      schema/
        cem-element.cem
        cem-element.rnc
      converters/
        cem-dom-to-cem-element-declarations.cemt
        cem-element-to-dce-declarations.cemt
      rust/
        fallback.rs
      fixtures/
        declarations.input.json
        declarations.output.json
      tests/
        conformance.cem
```

Generated Rust/TypeScript and manifest projection artifacts may be emitted into
runtime package trees, but the source of truth is the schema package.

## Manifest

`package.cem` is the canonical registry source. It is CEM-ML with its own
schema:

```text
content type: application/vnd.cem.schema-package+cem
schema: https://cem.dev/ns/schema-package/1
```

The schema-package manifest schema is itself versioned and distributed as a
schema package. Bootstrap runtimes may consume generated registry projections,
but authored package metadata remains CEM-ML.

```cem
{@schema="https://cem.dev/ns/schema-package/1" |
  {package @id="cem-element" @version="1.0.0" |
    {schema
      @uri="https://cem.dev/ns/cem-element/1"
      @source="schema/cem-element.cem"
    }

    {content-type @value="application/vnd.cem.element+cem-bin"}
    {content-type @value="application/vnd.cem.element.declarations+cem-bin"}
    {content-type @value="application/vnd.cem.element+json" @alias=true}
    {content-type @value="application/vnd.cem.element.declarations+json" @alias=true}

    {namespace @uri="https://cem.dev/ns/cem-element/1"}

    {converter
      @id="cem-dom-to-cem-element-declarations-v1"
      @implementation="cemt"
      @template="converters/cem-dom-to-cem-element-declarations.cemt"
      @template-content-type="application/vnd.cem.transform+cem"
      @template-schema="https://cem.dev/ns/template/cem-native/1"
      @streamable=true
      @lossiness="canonicalizing"
      @implicit=true
    |
      {from @content-type="application/vnd.cem.dom+cem-bin"}
      {to @content-type="application/vnd.cem.element.declarations+cem-bin"}
    }

    {converter
      @id="html5-recovery-v1"
      @implementation="rust"
      @rust-symbol="Html5RecoveryConverter"
      @streamable=true
      @lossiness="recovery"
      @implicit=true
    |
      {from @content-type="text/html"}
      {to @content-type="application/vnd.html5.dom+cem-bin"}
    }
  }
}
```

The manifest is also the source for generated registry code, CLI inspection
output, docs, and conformance matrices.

## Identity Model

Content type is the public identity for source and target artifacts. Schema URL
is the authority that defines the content type contract. Namespace is a
secondary matching signal for XML/HTML-derived surfaces.

```text
source identity = content type + optional schema URL + optional namespace set
target identity = content type + optional schema URL + optional namespace set
```

If exactly one schema package claims a content type, that content type resolves
to the package's schema URL. If more than one package claims a content type, the
caller must provide a schema URL or the planner rejects the identity as
ambiguous.

## Converter Graph

The runtime maintains a directed graph of converter edges.

```text
source identity -> converter edge -> target identity
```

Every converter edge declares:

- source content type/schema;
- target content type/schema;
- implementation kind: `cemt` or `rust`;
- streamability;
- lossiness or canonicalization behavior;
- source-map behavior;
- diagnostics contract;
- whether implicit planning may use it.

The planner may build a multi-step path when there is no direct edge.

## CEMT Primary, Rust Fallback

CEMT is preferred whenever a conversion is expressible as a declarative
schema-to-schema transformation. This keeps transformation behavior inspectable,
portable, and versioned with the schema package.

Rust fallback is appropriate for:

- incomplete or non-normalized HTML recovery into an HTML5 normalized DOM;
- byte-correct HTML/XML serialization;
- high-throughput streaming;
- browser, WASM, or host parser integration;
- graph algorithms or global mutation passes that are not a good CEMT fit;
- bootstrapping the runtime before CEMT execution is available.

Both CEMT and Rust converters expose the same registry contract. The planner
does not expose separate user semantics for them.

## Planning Rules

Explicit rules:

- CLI/config source identity fields create hard source constraints:
  `--data-content-type`, `--data-schema`, `--content-type`, `--schema`.
- CLI/config target identity fields create hard target constraints:
  `--to-content-type`, `--to-schema`.
- Template identity fields create hard template constraints:
  `--template-content-type`, `--template-schema`.
- A run graph may specify a converter ID to choose a specific edge.
- Explicit converter selection may use `explicit_only` edges.

Implicit rules:

- Use only registered implicit edges.
- Prefer CEMT over Rust when both are equivalent in identity, cost, and
  capability. Remember, CEMT can be compile-able and optimized for platform by vendor implementation.
- Prefer exact schema URL match over content type alone.
- Prefer content type essence match over namespace-only match.
- Prefer direct edge over multi-edge path.
- Prefer lower total cost, then stable converter ID sort order.
- Reject equal-cost ambiguous paths.
- Do not cross lossy edges implicitly unless the target content type was
  explicitly requested.
- Rust may be the implicit canonical edge when the schema package declares it,
  such as HTML5 recovery.

## Runtime Registries

The target runtime exposes:

```rust
pub struct EngineContext {
    pub schema_registry: SchemaRegistry,
    pub converter_registry: ConversionRegistry,
    pub template_adapter_registry: TransformTemplateAdapterRegistry,
    // existing execution fields...
}
```

`SchemaRegistry` answers:

```text
schema URI -> schema descriptor
content type -> schema descriptor candidates
namespace -> schema descriptor candidates
```

`ConversionRegistry` answers:

```text
source identity + target identity -> planned converter path
```

`TransformTemplateAdapterRegistry` remains specialized for template
compilation/execution. CEMT converter edges call into it, but generic converter
planning does not become template-specific.

## Examples

Projection artifacts are binary-first. `+json` forms are debug/interchange
views, not the canonical runtime transport. Native CLI, Rust, WASM workers, and
server/edge hosts should consume typed structures or CEM binary chunks directly
when possible, preserving the same binary representation across cache,
transport, query, and converter boundaries.
The native projection route surface exposes sealed chunk streams; one stream can
be multicast to several sinks with shared immutable bytes, and route execution
can remain deterministic or use the parallel runtime path without changing the
chunk format.

The first semantic projection packages are `cem-dom-projection/v1`,
`cem-ast-projection/v1`, and `cem-events-projection/v1`. They own
`https://cem.dev/ns/projection/dom/1`, `https://cem.dev/ns/projection/ast/1`,
and `https://cem.dev/ns/projection/events/1` respectively, with
`application/vnd.cem.dom+cem-bin`, `application/vnd.cem.ast+cem-bin`, and
`application/vnd.cem.events+cem-bin` as primary content types. Their `+json`
content types are debug/interchange views.

HTML recovery:

```text
text/html
  -> application/vnd.html5.dom+cem-bin
```

This is a Rust fallback edge because tolerant HTML5 recovery is parser behavior.
The normalized DOM result can then flow through CEMT converters.

Generic CEM DOM to DCE runtime artifacts:

```text
application/vnd.cem.dom+cem-bin
  -> application/vnd.cem.element.declarations+cem-bin
  -> application/vnd.cem.dce.declarations+cem-bin
  -> application/vnd.cem.dce.instances+cem-bin
  -> application/vnd.cem.dce.instances+cem-bin; profile=data-islands
  -> text/html
```

The declaration, instance, and data-island propagation stages are CEMT-first.
Final HTML serialization may be CEMT or Rust depending on serializer
requirements.

## Versioning

Use one source folder per major schema version:

```text
cem-element/v1/
cem-element/v2/
```

Minor and patch versions stay in the same major folder unless behavior must run
side by side in one runtime. The full SemVer is recorded in the manifest.

## Generated Outputs

Schema packages may generate:

- Rust registry tables;
- TypeScript declarations;
- CEM binary registry manifests or chunk descriptors;
- JSON registry manifests;
- JSON Schema for the JSON registry manifest;
- XML registry manifests;
- RELAX NG XML/compact schemas for the XML registry manifest;
- CLI documentation;
- conformance reports.

Generated output is not authoritative. The schema package manifest and source
artifacts are authoritative. Binary CEM artifacts are the preferred generated
runtime/cache form once canonized. JSON/JSON Schema and XML/RELAX NG outputs are
distribution artifacts for consumers and tooling that cannot or should not parse
CEM-ML or CEM binary artifacts directly.

## CLI Inspection

The CLI should expose registry introspection:

```text
cem-ml registry list schemas
cem-ml registry list content-types
cem-ml registry list converters
cem-ml registry plan --from-content-type ... --to-content-type ...
```

Planning output should show every selected edge, implementation kind, schema
owner, source-map behavior, and lossiness/canonicalization policy.
