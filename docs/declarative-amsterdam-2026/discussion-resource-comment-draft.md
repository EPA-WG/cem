# Discussion #24 resource comment — draft

Do not post yet. The owner-deployed `ab74c3a7` version now matches the verified
local inventory and the public playground passes editing/Render, but Studio
still misses the original cold-start readiness deadline. Complete the public
Login/Supplier acceptance before removing this notice and posting. No comment
has been sent.

The presentation uses ordinary Studio projects, resources and configurations.
Clone a sample before editing. The slide numbers follow the recorded native
deck order; stable IDs keep the mapping traceable when that order changes.

| Slide | Stable slide ID | Demonstration | Project resource and configuration |
| --- | --- | --- | --- |
| 03 | `p2` | read | [login: readme](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=readme&build=read-edit) — `read-edit` |
| 08 | `p7` | edit | [login: login-cem](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-cem&build=validate) — `validate` |
| 10 | `p9` | fail | [login: login-invalid](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-invalid&build=validation-repair) — `validation-repair` |
| 11 | `p10` | repair | [login: login-invalid](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-invalid&build=validation-repair) — `validation-repair` |
| 12 | `p11` | inspect | [login: login-cem](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-cem&build=inspect) — `inspect` |
| 13 | `p12` | format | [login: login-cem](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-cem&build=format-pretty) — `format-pretty`, `format-tabular`, `source-colors` |
| 14 | `p13` | compare | [login: login-html](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-html&build=semantics-html) — `semantics-html`, `semantics-cem`, `html-conversion` |
| 15 | `p14` | companion pipeline overview | [Supplier transformation graph](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=supplier-pipeline&file=supplier-graph&build=availability) — `availability` |
| 16 | `p15` | query | [login: login-cem](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-cem&build=semantics-html-cemql) — `semantics-html-cemql`, `semantics-cem-cemql` |
| 16 | `p15` | expression | [login: login-expression](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-expression&build=expression-html) — `expression-html`, `expression-cem` |
| 17 | `p16` | transform | [login: login-view-graph](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-view-graph&build=render-graph) — `render-graph`, `render-html`, `render-cem` |
| 18 | `p17` | source provenance | [Login template and generated output](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-view-template&build=render-graph) — `render-graph` |
| 19 | `p18` | live | [login: login-live-view](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-live-view&build=live-view) — `live-view` |
| 30 | `p24` | supplier | [supplier-pipeline: supplier-stock](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=supplier-pipeline&file=supplier-stock&build=stock-source) — `stock-source`, `availability` |

Use checked-in or disposable example data. Do not enter real credentials.
Entered passwords or one-time codes block recovery until explicitly cleared;
they must never be exported.

The [participant resource index](participant-resources.md) provides the checked
setup and frozen reference results. The [demo verification record](demo-verification.md)
distinguishes completed local acceptance from pending public acceptance.
