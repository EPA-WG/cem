# Native data view

`cem-data-view` presents bounded JSON and CSV through the shared native
`data:read` import boundary. It never parses external documents in JavaScript or
passes a serialized AST between layers. The original native input tree retains
its source provenance. The passive viewer neither edits input nor triggers a
project build or save.

| Public surface | Meaning |
|---|---|
| `value` | Original JSON or CSV source text; default empty |
| `format="json|csv"` | Explicit native import format; default JSON |
| `label` | Visible table caption or structural heading; default Data |
| `slot="label"` | Projected visible label replaces the fallback text |
| `aria-label` | Optional independent accessible name |
| `data-cem-density="super-compact"` | Inherited shared paragraph-line presentation on every input device |

Arrays become tables with a union of columns from displayed rows. Object rows
use property names; primitives and heterogeneous rows have a `#value` column.
Missing cells show `∅ missing`, JSON null shows `null`, empty strings show `""`,
and empty containers show `[]` or `{}`. String values retain quotes and a typed
marker, so the string `"null"` differs from null. Nested values use a bounded structural
projection rather than flattening away their types. CSV quoting and embedded
line breaks are handled by the native CSV importer. Strings render as text,
including text that resembles HTML.

The shared preview limits are fixed: 65,536 Unicode source characters (at most
256 KiB UTF-8), 100 rows per collection, 32 columns/properties, four nested
levels and 2,048 characters per scalar cell. Source limits are checked before
native import. Every truncation is labeled; malformed and unsupported input
produces a visible native error. Limits bound preview presentation and do not
rewrite project resources or generated artifacts.

Geometry consumes canonical D1/D2c/D5 data-row tokens. In the explicit
super-compact profile rows retain a `1lh` minimum with no extra vertical gap,
padding or border; wrapped values can grow. Coupling and generic control height
minima are zero, and the profile applies without an input media gate, including
coarse and hybrid devices. Normal geometry and the precise-input-only compact
profile retain their existing behavior. Project theme and project output themes
remain independent of the Studio chrome profile.

The component is passive: `disabled`, `readonly`, `selected`, `invalid`, sorting
and pagination are not component states. Use `cem-table` for authored table
composition and separate shared controls for commands. The viewer adds no
recovery authority. Entered Login passwords or one-time codes continue to block
checkpoint capture, and automatic Studio recovery remains disabled.

See the [property playground](../playgrounds/cem-data-view.html),
[Full examples and variation matrix](../playgrounds/cem-data-view-gallery.html),
and [automated stories](../src/components/cem-data-view/cem-data-view.stories.ts).
