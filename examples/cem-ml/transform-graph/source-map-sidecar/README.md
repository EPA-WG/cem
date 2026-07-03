# Transform Graph Source-Map Sidecar Sample

This sample emits a linked HTML page and extracted CSS file, then writes
source-map sidecars beside both generated outputs.

Run it from the repository root:

```bash
dist/target/debug/cem-ml transform \
  --config examples/cem-ml/transform-graph/source-map-sidecar/graph.cem \
  --report-json /tmp/cem-ml-source-map-sidecar/report.json \
  --source-map-summary
```

The command prints a concise summary:

```text
source maps:
- htmlOut <- page -> /tmp/cem-ml-source-map-sidecar/page.html.map [outputSpans: 17]
- cssOut <- page -> /tmp/cem-ml-source-map-sidecar/page.css.map [outputSpans: 1]
```

Inspect the raw sidecar span counts:

```bash
node -e "const fs = require('node:fs'); for (const f of ['page.html.map','page.css.map']) { const m = JSON.parse(fs.readFileSync('/tmp/cem-ml-source-map-sidecar/' + f, 'utf8')); console.log(f + ': ' + m.outputSpans.length + ' output spans'); }"
```

The HTML sidecar keeps spans for copied HTML ranges. The generated
`<link>` element is intentionally unmapped. The CSS sidecar keeps spans
rebased from the original inline `<style>` content.
