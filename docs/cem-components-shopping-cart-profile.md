# Shopping-cart application deployment profile

Status: native reference fixture and build/runtime tests, 2026-10-29. A compiled
browser bundle, GraphQL transport/validation and reduced WASM remain pending.
This is the first concrete application case for the
[component WASM profile](cem-components-wasm-profile.md#application-build-and-deployment-boundary).

The app shows cart products, quantities, server totals and service errors. Its
build validates and compiles the authored UI and fixed GraphQL operations. Its
deployed client needs reactive rendering and JSON communication; unrelated
content formats, schemas and authoring tools can be loaded if requested later.

## Reference inputs and required behavior

The [fixture README](../packages/cem_ql/fixtures/component-application/shopping-cart/README.md)
links every input. The [reference manifest](../packages/cem_ql/fixtures/component-application/shopping-cart/profile.json)
declares one host binding, five shared components, JSON runtime data, optional
formats and fixed operation inputs. It is a test-consumed reference contract,
not the final production module manifest or an installed build target.

| Stage | Requirements |
| --- | --- |
| App build | CEM-ML/CEM-QL and declaration parsing, native template/expression compilation, style compilation/validation, GraphQL schema/operation and static-data validation, exact dependency/ABI/source hash records, generated variable/response guards. |
| Initial client | Compiled native render/evaluation, JSON decoding/encoding, required action/form behaviors, live DOM updates, app input checks, resources/cancellation and ownership/recovery guards. CSS parsing remains required by the current artifact wire format. |
| Optional demand | Additional data formats, dynamically admitted source templates, editor highlighting, Parse/Inspect, language converters and general schema tooling. |

Predefined `Cart`, `SetCartLineQuantity` and `RemoveCartLine` operations use the
same response shape through mutation aliases. Their JSON envelope carries data
and optional errors; a partial result remains visible alongside its error. The
protocol basis is [GraphQL over HTTP](https://graphql.org/learn/serving-over-http/).
The partial-error case nulls the nullable shipping-estimate field while retaining
the cart's non-null lines and totals.
The server returns integer minor-unit amounts and formatted totals. Stock,
prices, quantity limits and authorization are server decisions; generated client
checks support interaction without replacing those decisions.

## Native boundary proof

`template_artifact::application_profile_tests` compiles the cart at build time
and reloads its binary IR with the build's source identity and binding policy.
The reload/render path receives no original view source. Compiler profiling has
a positive build-stage control and asserts no template/query compilation during
runtime reload and repeated rendering. Both dev and prod source-map modes are
covered. JSON API documents are imported natively and retain their source owner;
they do not cross a JSON AST or JavaScript object reconstruction boundary.
The evaluator still assembles its general function registry during this path;
that runtime dependency is distinct from source compilation and must also be
partitioned before the minimal binary can exclude unused implementations.

The response sequence checks initial quantities, changed quantities and an added
line, line removal, empty cart, partial data/errors and failed requests. Product
text remains escaped. The fixture compares component references in compiled IR
with its declared roster, builds/reloads those canonical shared declarations,
and checks retained native CSS. Corrupt artifacts, different source hashes,
incompatible bindings/compiler versions and malformed JSON are refused.

Reproduce from the repository root with the Rust toolchain on `PATH`:

```bash
cargo test --locked --offline -p cem-ql --lib template_artifact::application_profile_tests -- --nocapture
cargo test --locked --offline -p cem-ql --test template_artifact_reload
```

The four app-boundary tests and nine existing artifact reload tests pass. This
native proof does not establish browser recovery, code omission or size savings;
those are delivery gates below. Full runtime defaults and production code paths
are unchanged by these test fixtures.

## Remaining delivery boundaries

- The current browser template-artifact adapter still takes source text. Add a
  shared, manifest-pinned source-hash admission path and compiled registration
  for the complete declaration closure, including metadata and runtime behaviors.
- Stylesheet v1 reload imports CSS source. Prove compiled style/scoping admission
  with original identity and URL bases before removing its parser dependency.
- Generate/validate the app manifest and API contracts during the app build.
  The illustrative GraphQL files currently have no schema/operation validator in
  the fixture runner, and a manifest alone proves no native code omission.
- Bind real quantity/remove actions through shared declarative resources and
  GraphQL transport. The native render fixture sends no requests and has no
  interactive action bindings. Register a normal Studio example project with
  working build/preview only when those behaviors exist.
- Partition native dependencies/exports/registries and measure the separate
  compiled-UI + JSON distribution. Verify optional module acquisition, offline
  behavior and worker/fallback recovery with the same app state and semantics.

Existing full builds/defaults stay intact. Entered passwords/OTPs continue to
block recovery until cleared; no credential is stored in an app manifest or
public cache. The 4 MiB checkpoint limit and current deadlines are unchanged.
