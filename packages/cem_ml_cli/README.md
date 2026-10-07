# cem-ml-cli

`cem-ml-cli` is the native Rust command host for the `cem-ml` binary. It applies
command-line argument, stream, exit-code, cancellation, and host-I/O policy to
the reusable services owned by the `cem-ml` library.

## Public boundary

The crate publishes the native `cem-ml` executable. Parsing, validation,
conversion, query, transform, report, and transformation-graph semantics remain
in [`cem-ml`](../cem_ml/README.md); the CEM-QL template adapter is supplied by
[`cem-ml-transform-cem-ql`](../cem_ml_transform_cem_ql/README.md).

`@epa-wg/cem-ml-cli` is a separate synchronized npm deployment for Node and
browser hosts. It does not replace this native crate or change the command
contract.

## Verification

Use Nx for native build and command verification:

```bash
yarn nx run cem_ml_cli:build
yarn nx run cem_ml_cli:test
yarn nx run cem_ml_cli:e2e
```

See the [CLI feature summary](../../docs/cem-ml-cli-contract.md) for the command
surface and the
[deployment contract](../../docs/cem-ml-deployment-contract.md) for synchronized
runtime and host ownership.

## CEM-QL query modules

Use `--query-content-type application/vnd.cem.query-expression+cem-ql` for a
standalone expression, or `application/vnd.cem.query+cem-ql` (`text/cem-ql`
alias) for a module. An omitted query schema is inferred; an explicit schema
must match the source kind. Module identities require a `module "URI"` header
and one final expression. Both inline `--query` and `--query-file` are supported.

```sh
cem-ml query catalog.xml --content-type application/xml \
  --query 'module "urn:example:links" import "cem:stdlib/url" as u u:href("https://example.test")' \
  --query-content-type application/vnd.cem.query+cem-ql --output json
```

Modules support local immutable bindings/functions and registered `cem:`
imports. External/plugin imports are rejected. `input` retains the native data
tree, and local declarations follow lexical shadowing rules. Ordered nonfatal
warnings reach the query result and report. See the
[module-query contract](../../docs/cem-ql-cli-module-query-design.md).

## Native CEM query input

CEM-QL queries accept CEM source (`application/cem`) as well as supported native
external data imports. CEM ingress parses once through the normal schema-machine
and builder path and retains the original arena and lexical snapshots. Querying
or constructing a reference to an authored reference does not evaluate its source;
a consumer supplies the context and evaluation stage separately. No authored root
or context ID is required.

Executable source bundles use explicit query flags:

```sh
cem-ml query source.cem --content-type text/cem-ml \
  --query-content-type application/vnd.cem.query-expression+cem-ql \
  --query '#()' --output cemv --out empty.cemv \
  --export-reload-bundle source.reload
cem-ml query source.reload --reload-bundle --reload-source-id 1 \
  --query-content-type application/vnd.cem.query-expression+cem-ql \
  --query 'input.children.kind' --output json
```

Reload verifies the debug source bundle and retains the decoded owner/capture;
it does not read the original source URI or parse source text again. The primary
manifest source ID defaults explicitly to 1; another ID requires
`--reload-source-id`. AST-only bundles support inert query inspection; embedding
validation admission requires capture metadata. CLI `validate`/`check` bundle
admission remains [tracked adapter work](../../docs/todo.md#next-three-reload-consumer-items).

`--output cemv` exports materialized native values as binary bytes, without a text
newline. Executable source references reject with an attributed typed diagnostic;
use source-only `--export-reload-bundle` for later evaluation. CEMV cannot replace
the executable bundle. The binary codec is debug-only. Existing textual/JSON
exports keep their compatibility projections. Neither export stores runtime
contexts, authority or pending sessions.

`#input` constructs a native reference to the input node. CLI JSON reports its
existing opaque native artifact descriptor; it does not transport the reference's
target graph. Native Rust consumers retain original owners and ordered targets.
URL/ID strings are rejected as operands rather than treated as lookups. URI
fetching belongs to the resource/import boundary. Public reference type/access
syntax and graph transport remain deferred in the reference design.

## Explicit schema package replacement

`--schema-package` loads a manifest. To replace a built-in or a package owned by
a different manifest, pass a separate, repeatable native CLI grant:

```sh
cem-ml convert input.cem --to-format cem \
  --resolver-read-map vendor://packages=/path/to/vendor \
  --schema-package vendor://packages/cem-ml/package.cem \
  --schema-package-replacement-grant '{"packageId":"cem-ml","expectedOrigin":{"kind":"builtin"},"replacementManifestUri":"vendor://packages/cem-ml/package.cem"}'
```

The grant matches the exact package ID, current origin and incoming resolved
manifest URI. `expectedOrigin` is `{"kind":"builtin"}`, `{"kind":"untracked"}`
or `{"kind":"manifest","uri":"the current resolved manifest URI"}`. Unknown
fields, unknown origin kinds and empty identities are rejected. A grant does not
load its manifest. Ordinary same-origin refreshes retain their existing behavior.

Run configs do not supply grants. Their existing permissive JSON decoder ignores
unknown grant fields, which never become authority. Virtual command arguments
attempting to supply grants fail with `cem.command.replacement_grant_host_required`;
embedding hosts set `EngineContext.schema_package_replacement_grants` directly.
Command preparation retains those host grants without widening them. JavaScript
WASM host configuration remains a subsequent API integration item.
