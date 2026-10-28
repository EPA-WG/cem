# Module-map deployments

Open this ordinary project from **Content types and schemas → Schema authoring**
or **Queries and transformations**, then clone it to edit. It compares the native
paired deployment-map versions using owned files, explicit graph dependencies and
exact exported artifacts. Studio reuses its shared editor, commands, artifact
tree, structured data view and sandboxed HTML preview.

## Authored identity reference

These are resource identities, including expected artifacts. The configurations
below determine their supported uses. An identity alone does not grant an
operation.

| Resource role | Content type | Schema |
| --- | --- | --- |
| Transformation graphs | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` |
| Data and expected JSON artifacts | `application/json` | `https://cem.dev/ns/data/json/1` |
| Paired deployment maps v1 | `application/vnd.cem.module-map+json` | `https://cem.dev/ns/data/module-map/1` |
| Paired deployment maps v2 | `application/vnd.cem.module-map+json` | `https://cem.dev/ns/data/module-map/2` |
| Paired deployment maps v3 | `application/vnd.cem.module-map+json` | `https://cem.dev/ns/data/module-map/3` |
| Opaque XHTML deployment resources | `application/xhtml+xml` | `https://cem.dev/ns/data/xhtml/1` |
| Stylesheets and expected CSS artifacts | `text/css` | `https://cem.dev/ns/data/css/1` |
| HTML source and expected artifacts | `text/html` | `https://cem.dev/ns/data/html/1` |
| Opaque JavaScript deployment modules | `text/javascript` | No registered source schema |

## Source contracts and configurations

Every version uses `application/vnd.cem.module-map+json` with its explicit schema:

| Version | Schema | Demonstrated deployment contract |
| --- | --- | --- |
| v1 | `https://cem.dev/ns/data/module-map/1` | Exact bare JavaScript imports copied to app-relative targets. |
| v2 | `https://cem.dev/ns/data/module-map/2` | The same import contract plus an explicit CSS deployment resource. The native version also supports declared JS/WASM resources. |
| v3 | `https://cem.dev/ns/data/module-map/3` | Typed JS/JSON imports, a declared bare module edge rewritten to a relative output URL, and an opaque XHTML resource. |

| Version | Validation | Inspection | Formatting | Syntax repair | Build |
| --- | --- | --- | --- | --- | --- |
| v1 | `validate-v1` | `inspect-v1` | `format-v1` | `repair-v1` | `build-v1` |
| v2 | `validate-v2` | `inspect-v2` | `format-v2` | `repair-v2` | `build-v2` |
| v3 | `validate-v3` | `inspect-v3` | `format-v3` | `repair-v3` | `build-v3` |

For each `N` in `1`, `2`, `3`:

- `validate-vN` validates **JSON source syntax and selected-version semantics**. It
  enforces its selected v1/v2/v3 schema, exact bare identities, version-specific
  import/resource kinds and declared module-edge targets. No assets are read.
- `inspect-vN` shows a generic imported CEM source tree and original ranges, not a
  typed module-map AST or an executable plan.
- `format-vN` applies the native tabular JSON formatter with an exact fixture.
- `repair-vN` restores a missing outer object brace after an original native JSON
  diagnostic. Its EOF locations are checked independently for all three buffers.
- `build-vN` performs real native graph lowering and execution. This is where the
  paired deployment contract is enforced.

`validate-graph` and `inspect-graph` cover the v3 graph source separately from
reading its maps or running deployment. JSON-backed Parse and Inspect use the
retained native data import and preserve the selected map schema/MIME, without
retokenizing JSON as CEM. These operations refuse the same selected-version
semantic errors. Pair matching, output containment and resource reads remain
separate deployment-plan checks. Standalone CEM-ML Parse/Inspect presentations now publish native source-owned
references to Studio's Provenance selector. Those references bind the admitted
original map bytes and native ranges to the published artifact. Printed fields
in an explicit JSON export remain data and cannot grant navigation or reads.

`repair-semantic-v1`, `repair-semantic-v2` and `repair-semantic-v3` exercise
syntactically valid maps containing a forbidden `./invalid` identity. The native
`cem.module_map.specifier_invalid` diagnostic points to its original JSON key;
repairing that identity restores the exact corresponding valid map. Explicit
schema selection is authoritative; MIME-only requests select v1, and ordinary
JSON never acquires module-map behavior from its `$schema` property. Input-root
alias maps and ordinary browser prefix/scoped import maps retain their separate
contracts.

## Relative paths, maps and artifacts

`graphs/vN.cem` explicitly imports `../pages/vN.html` and references the paired
`../maps/source-vN.json` / `../maps/target-vN.json`. Source resource paths resolve
relative to the source map. The HTML's authored import values match that map.
Target URLs start with `./` and resolve beside `generated/vN/index.html`.

- v1 exports the HTML entry and `assets/caption.js`.
- v2 additionally exports the declared `assets/site.css` resource.
- v3 exports HTML, `assets/app.js`, `assets/metadata.json` and
  `content/fragment.xhtml`. Its exact `@demo/metadata` edge becomes
  `./metadata.json` in the deployed app module.

The JavaScript fixtures only export data; they implement no Studio or component
UI behavior. Assets remain explicit opaque payloads except the bounded v3 edge
rewrite. No package-export selection, directory scanning, transitive import
traversal, undeclared copying or JavaScript execution is inferred. XHTML is copied
byte-for-byte without registering components or discovering dependencies.

The exported HTML retains its real browser import map. Studio's existing artifact
preview removes scripts, including import maps, and keeps its sandbox/CSP. The
visible static body is a preview of the output; it does not execute deployed
modules. No preview policy or execution grant is weakened for the example.

## Native provenance and publication

These rewrite-only outputs have **no output spans or `.map` sidecars**. Provenance
is the real native `moduleAssetManifest`: specifier, source-map-document URI,
source URI, target, output destination, content type, source/output byte lengths
and SHA-256. `sourceMapUri` names the deployment JSON document, not a debug sidecar.
The native aggregate hash is host-neutral and stable for unchanged resources.
The v3 app's source and output hashes differ because its declared edge changes;
opaque JSON, CSS, JS and XHTML copies retain their exact source bytes.

CLI hosts can inspect this metadata with `--report-json <report-file>`; that
host-only report destination is not part of the portable project command. The
shared worker publishes the same native manifest with its graph result. No
application code fabricates character lineage or converts an AST through JSON.

These paired deployment maps are distinct from the CLI input-root-scope
`--module-map` alias option. Neither kind grants filesystem or network access.
The CLI's local file reader and explicit resolver-read maps are host policy;
Studio supplies only the declared owned project resources. Missing map/resource
or resolver grants refuse planning rather than fetching undeclared dependencies.

## Failure, repair and saved copies

The three `failures/syntax-vN.json` buffers omit the outer closing brace. Their
`repair-vN` configurations report `cem.json.parse_error`; append `}` and a newline
to restore the exact valid source. Catalog originals remain unchanged.

The separate `failures/source-vN.json` files are syntactically valid but violate
real deployment contracts. Standalone validation now refuses the v1 trailing-
slash identity (`cem.module_map.specifier_invalid`), the v2 resource kind and
the v3 embedded version. Replacing a saved source map with one still makes
`build-vN` fail during native planning:

- v1 reports `cem.module_map.entries_mismatch` for unequal import keys before
  checking the invalid trailing-slash identity; pair confirmation retains its
  original planning priority.
- v2 reports `cem.module_map.resource_type_unsupported` for a JSON resource.
- v3 reports `cem.module_map.schema_unsupported` for its unsupported embedded version.

The CLI exits 2 without publishing artifacts. Studio retains the last good build.
Restore the valid source map and rebuild. This separation demonstrates why a
schema identity or successful syntax validation alone is not execution support.

Change the v3 target map's `./assets/metadata.json` to `./assets/catalog.json`.
Native deployment changes the artifact name, HTML import map and the app module's
relative edge to `./catalog.json`. Change `Café modules` in `modules/metadata.json`
to update the exact JSON artifact independently. A second clone can choose another
target while the first project and read-only catalog remain unchanged. Saved maps,
resources, repairs and native rebuilds work after an offline reload.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. User projects and
drafts are not cleared; the sample adds no application UI JavaScript or parser.
