# Frozen Supplier companion

The selected companion is the ordinary **Supplier pipeline 1.1.1** project at
source baseline **`ee373175`**. It shows how XML products and CSV stock become
an HTML availability report, a JSON service payload and a YAML partner feed.
It follows the required Login demonstration; it adds no Studio page or UI logic.
The Markdown folder-site example remains optional and outside this frozen route.

- [Portable project](supplier-pipeline.cem-studio.json): 12 authored resources and
  six configurations, with original identities, resolver paths and byte hashes.
- [Authored definition](example.json): source contracts, exact argv, dependencies,
  expected artifacts and the XML repair instruction.
- [Checkpoint manifest](manifest.json): original source revision, credential policy,
  ten native results, exact diagnostics and hashes for all generated artifacts.

## Open and demonstrate

Use the participant kit's printed origin and append
`/?collection=amsterdam-2026&sample=supplier-pipeline&file=readme`.
Select the project and use **Clone selected** to create an independent copy.
Keep existing saved projects and drafts. All configurations run saved inputs;
an unsaved edit does not change a result. Source IDs below are stable; their
display paths are not interchangeable with those identities.

| Checkpoint | Resource / configuration | Expected result |
| --- | --- | --- |
| Read XML | `supplier-catalog` / `catalog-source`, `validate-catalog` | Native CEM-ML structure; well-formed namespace-aware XML, zero errors/warnings. |
| Select products | `supplier-products-query` / `query-products` | `SKU-A=Café cups`, `SKU-B=Tea pots`, `SKU-C=Glass jars`, `SKU-D=Serving trays`. The query binds `supplier=urn:demo:supplier`; producer prefix spelling is independent. |
| Repair XML | `supplier-invalid-catalog` / `repair-catalog` | Two `cem.xml.parse_error` diagnostics at original XML locations. Replace `</product-broken>` with `</product>`, save and rerun: zero errors/warnings. The valid catalog stays independent. |
| Read CSV | `supplier-stock` / `stock-source` | Native retained CSV source structure, with explicit `text/csv;header=present`. |
| Build | `supplier-graph` / `availability` | `generated/availability.html`, `.json` and `.yaml`, exactly matching the saved baseline outputs. |
| Change stock | `supplier-stock`, then `availability` | Change `SKU-A,7` to `SKU-A,0`, save and build. All three formats reflect quantity 0 and out-of-stock status; other products retain their baseline results. |
| Fail the domain rule | `supplier-stock`, then `availability` | Change the value to `SKU-A,many`, save and build. CSV syntax remains valid, but native export reports two `cem.transform_runtime.export_representation_unsupported` errors containing `invalid data number`. No new output artifacts are published. |
| Restore | `supplier-stock`, then `availability` | Restore `SKU-A,7`, save and build. All three baseline outputs return. An independent project copy still has its own original stock. |

The baseline reconciles by exact product ID, independently of CSV row order:
SKU-A is available (7); SKU-B is out of stock (0); SKU-C is unknown (no stock row);
SKU-D is ambiguous (duplicate rows). Unlisted stock IDs add no catalog products.
Matching/domain rules belong to the native template; the graph connects stages.
The three exports use native format writers, without a JavaScript serializer.

## Saved outputs and fallback

The manifest links every output and its adjacent diagnostics. The useful report
checkpoints are:

- [Baseline HTML](checkpoints/availability/generated/availability.html),
  [JSON](checkpoints/availability/generated/availability.json),
  [YAML](checkpoints/availability/generated/availability.yaml).
- [Zero-stock HTML](checkpoints/availability-zero-stock/generated/availability.html),
  [JSON](checkpoints/availability-zero-stock/generated/availability.json),
  [YAML](checkpoints/availability-zero-stock/generated/availability.yaml).
- [Failed numeric export diagnostics](checkpoints/availability-invalid-stock.diagnostics.json).
  Its empty saved stdout is a command record, not a generated artifact.
- [Restored HTML](checkpoints/availability-restored-stock/generated/availability.html),
  [JSON](checkpoints/availability-restored-stock/generated/availability.json),
  [YAML](checkpoints/availability-restored-stock/generated/availability.yaml).
- [Malformed XML diagnostics](checkpoints/repair-catalog.diagnostics.json) and
  [repaired XML result](checkpoints/repair-catalog-repaired.json).

Failed native runs can leave files from a previous successful run on disk. Replay
removes the three known output files and maps before each graph run and verifies
their absence after the invalid-stock failure. It never mistakes an old artifact
for a successful failed-run result. The archive's original source bytes are kept;
mutations are confined to a disposable project directory.

## Replay and boundaries

```sh
yarn nx run @epa-wg/cem-studio:verify:supplier-presenter
```

This checks current authored inputs against the frozen project, then replays all
six configurations plus XML repair and three stock variants with the shared
native CLI. All three format outputs and original diagnostics must match byte
for byte. A temporary original-source project is removed afterward. The same
materialization, path, command and privacy helpers also serve the Login freeze;
native import, query and transformation semantics remain in the shared engine.

The HTML graph map is checked for original `templates/view.cemt` ownership and
nonempty output spans. Machine-specific map bytes are not frozen. Generated
characters from computed values do not claim direct XML/CSV character lineage;
invalid-number export errors have no original CSV-cell URI/span. The explicit
XML validation diagnostics do retain original source coordinates. Parse/Inspect
source coordinates alone do not grant Provenance-panel navigation authority.

To intentionally replace the baseline after reviewing its inputs and expected
results, run `node packages/cem-studio/scripts/supplier-presenter.mjs --write`
and repeat verification. Normal verification does not rewrite snapshots.
The test target `test:supplier-presenter` checks missing outputs, stale artifacts
claimed on a failed run, changed scenarios/archive/privacy, and altered YAML even
with a renewed checksum. Existing Login snapshots must still replay unchanged.

No credentials or browser storage are read by this tooling. Entered passwords or
OTP elsewhere still block recovery until explicitly cleared; automatic whole-
Studio recovery remains disabled, with the 4 MiB budget and five-second deadline.
