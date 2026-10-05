# cem-ml-transform-cem-ql

`cem-ml-transform-cem-ql` is the Rust adapter that connects CEM-ML's stable
transform-template contract to CEM-QL compilation, evaluation, and native CEMT
rendering.

## Public boundary

The crate sits above both [`cem-ml`](../cem_ml/README.md) and
[`cem-ql`](../cem_ql/README.md). CEM-ML retains transformation lifecycle,
artifact, schema, source-map, and adapter ownership; CEM-QL retains expression,
template, render-plan, and compiled-artifact ownership. Keeping the integration
in this crate prevents a dependency cycle and gives native hosts one explicit
adapter registration boundary.

The adapter supports CEM-native templates, standalone CEM-QL expression
templates, and the [strict typed XSLT 3.0 profile](../../docs/xslt-runtime-lowering.md). It is infrastructure for
hosts and embedders, not an application UI or an alternate query-language
implementation.

XSLT execution uses compiled bundles with retained native document input,
recursive named/matched templates, explicit parameters, modes, and policy-resolved
import/include closures. Legacy version/namespace shortcuts and implicit
parameters are rejected. Separate legacy conversion tools remain available.
The historical adapter type/ID names are retained for host registration;
standard XSLT media types now select this executable adapter.

## Imported document bindings

Lifecycle XML, JSON, YAML and CSV inputs and explicitly encoded JSON documents
enter through `cem_ml::import`. `input` is one retained CEM document node using
`cem_ql::eval::imported_cem_tree`; secondary labels bind the same node capability.
JSON members no longer become query fields or top-level variables, arrays are
not implicitly flattened, and XML no longer exposes an `events` query binding.
Use CEM node navigation or a declared XPath function library. Duplicate keys,
lexical values, node/source identity and native ownership survive import.

For example, a generic-data property is selected through
`seq:where(input.children.children, fn(p) => p.attributes.value == "title")`;
its scalar text is at `.children.children.value`. Explicit numeric/boolean
conversion belongs in the authored query. Named parser inspection and requested
output serialization retain their separate contracts.

## Schema package lifecycle compilation

Install `schema_packages::CemQlSchemaPackageCompiler` explicitly with
`register_cem_ql_schema_package_compiler` when a native host has the runtime
inputs needed for declaration references. Its preparation callback receives the
package/schema identities, manifest origin and retained schema source, and
returns a fresh `CemQlSchemaDeclarationHost` plus request traversal limits.
Register that source in the host, supply its current expression context and
effective policies, register target owners, and grant directed scope crossings
as required. No context root ID is needed.

Call `cem_ml::real::load_schema_package_manifest_into_context` at the chosen
lifecycle stage. Engine requests containing `schema_package_manifests` use the
same compilation hook on their private enriched context. Ordinary adapter
registration does not install this compiler; source-only loading keeps
unevaluated declaration references pending.

`EngineContext.schema_package_sources` retains the latest valid source arena
by resolved URI and byte revision. Unchanged reads reuse that allocation;
changed bytes retain a new arena. This registry caches source structure only.
Independent context snapshots share immutable source ownership and compile
against their own runtime inputs. An incomplete or failed refresh preserves the
last active schema, converters and artifacts while keeping its candidate model
inspectable. Ready candidates publish together. Replacement ownership is
checked before invoking the compiler; supplying query inputs does not grant
package replacement authority.

Preparation failures retain their diagnostics and receive a hard compilation
error if none was supplied. Invalid traversal limits and a compiler returning a
foreign schema identity also prevent publication. Native expression failures
keep their original diagnostic code and source attribution.

## Verification

Use Nx for the publishable crate gates:

```bash
yarn nx run cem_ml_transform_cem_ql:lint
yarn nx run cem_ml_transform_cem_ql:test
yarn nx run cem_ml_transform_cem_ql:build
```

The surrounding transformation contract is documented in the
[CEM-ML acceptance criteria](../../docs/cem-ml-ac.md) and the
[CEM-QL implementation design](../../docs/cem-ql-stack-design-impl.md).
