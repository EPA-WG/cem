# CEM-ML UID and Scoped CSS Design

**Status:** Accepted design requirements.
**Primary use case:** generated style-scope IDs for light-DOM custom elements.
**Related docs:** [`cem-element` design](./cem-element-design.md),
[`cem-element` WASM proposal](./cem-element-wasm-proposal.md), and
[`cem-element` scoped CSS demo](../packages/cem-elements/demo/scoped-css.html).

## 1. Problem

The legacy `<custom-element>` scoped CSS behavior uses two different scoping
strategies:

- when a declaration has a custom-element tag, CSS is prefixed by that tag name;
- when no tag is available, a generated anonymous tag/ID is used.

That makes authored tags double as scope identities. For example:

```html
<custom-element tag="dce-1">
  <style>
    dce-1 {
      button { border: 0.2rem dotted blue; }
    }
  </style>
</custom-element>
```

This is convenient for debugging but weak as an isolation model. The same produced
tag can appear in different documents, nested scopes, imported modules, tests, SSR
fragments, or independently loaded bundles. If style containment is keyed only by
`dce-1`, unrelated declarations with the same tag can collide.

The anonymous case is safer because it uses a generated identity, but generating it
with a browser UUID API is unnecessarily expensive for this purpose. A CSS scope ID
is not a security token; it only needs to be unique inside the relevant rendering
domain.

There is also a reproducibility concern. If generated IDs are based only on runtime
thread IDs plus counters, saved transform output can churn in version control when
scheduling or load order changes. If generated IDs are based only on a source hash,
the engine may need the complete template and all imports before it can allocate
IDs. That delays early streaming work and couples ID allocation to content-cache
identity.

## 2. Principle

Do not use the public tag name as the style-scope boundary.

Every declaration that needs generated identities SHOULD receive a generated
`scopeUid`, whether or not it also has a public `tag`.

The public tag remains useful for:

- `customElements.define(tag, ...)`;
- author-facing markup;
- readable diagnostics;
- devtools search and troubleshooting.

The generated UID owns:

- CSS selector containment;
- render-node namespaces when needed;
- generated anonymous tag names;
- source-map/debug correlation that must not collide across imported scopes.

When a public tag exists, it MAY be used as a readable prefix:

```text
cem-scope-dce-1-u4k9f2-p0
```

The suffix remains the identity. The prefix is only a label.

For stable generated output, identity allocation SHOULD be seeded. The seed is a
namespace supplied explicitly by the author/host or derived by the host before full
template transformation is complete. The seed is not a proof that the transformed
source is identical; it is only the stable namespace for generated IDs.

## 3. UID Seeds

`uidSeed` is the primary deterministic input for generated IDs.

Source surfaces:

- `<cem-element uid-seed="...">` for author-controlled declaration IDs;
- `<cem-element uid-seed="">` for an explicit blank seed;
- transform request metadata for CLI, SSR, tests, and build pipelines;
- host/module context defaults, such as normalized source URI plus declaration
  fragment or declaration occurrence path;
- runtime fallback when no stable seed is available.

Security/privacy advisory: seed data resides on the consumer side. Consumers and
hosts should provide `uid-seed` values that are already safe to expose in generated
markup, logs, snapshots, and diagnostics. Do not put sensitive source paths,
customer names, private route names, or other confidential material in `uid-seed`.
If privacy matters, the consumer/host should pass an opaque encoded seed such as a
bounded hash. `cem-element` and the CEM-ML engine should not be responsible for
handling, redacting, or hashing sensitive seed data.

Example:

```html
<cem-element tag="dce-1" uid-seed="demo/scoped-css/dce-1">
  <template type="text/cem-ml">
    ...
  </template>
</cem-element>
```

Seed resolution order:

1. Explicit declaration seed, for example `@uid-seed`.
2. Host-provided transform seed, for example source URI plus fragment.
3. Source hash, when the complete source is already available and no earlier seed
   exists.
4. Ephemeral runtime seed from run ID plus local occurrence counter.

Presence of `uid-seed` is significant. `uid-seed=""` is valid and is treated as an
explicit blank-string seed. It does not fall through to host-provided seeds, source
hashes, or runtime fallback seeds. Generated IDs for a blank seed omit the seed token
and rely on the deterministic occurrence path for uniqueness inside the owning
scope.

The explicit seed solves the common saved-output case: generated markup can be
stable in version control without requiring the engine to hash the full
transformation source before allocating IDs.

Important separation:

- `uidSeed` controls generated ID stability and namespace separation.
- `artifactHash` or `artifactUid` MAY still be computed from complete source,
  imports, transform version, target mode, and policy stamp for cache correctness
  and provenance.

Do not use `uidSeed` as the only cache key for compiled artifacts. If the source
changes but `uid-seed` stays the same, the generated IDs should remain stable, but
the compiled artifact cache must still invalidate by content or dependency identity.

### 3.1 Collision Behavior

`uid-seed` uniqueness is a provider contract.

Consumers and hosts that provide `uid-seed` MUST make it unique enough for the
output scope where generated IDs will coexist. The engine derives IDs
deterministically from the seed and occurrence path; it does not globally prove that
the seed is unique.

Normal ephemeral browser runtime MAY skip duplicate-ID detection. In that mode,
seed uniqueness resides entirely with the host/provider.

Validation/debug mode MUST diagnose duplicate externally visible generated IDs
within the same output scope. Build, CLI, SSR, and persisted-output modes SHOULD run
that validation by default. Externally visible generated IDs include at least:

- `scopeUid` / `data-cem-scope`;
- generated anonymous custom-element names;
- stylesheet IDs;
- hydration/render-root IDs;
- artifact IDs that are emitted into generated output.

Repeatable output MUST NOT repair duplicate seeds by appending runtime randomness,
worker indexes, or execution-order counters. If duplicates are detected in
validation/debug mode, the correct outcome is a diagnostic, not automatic
disambiguation that changes saved output.

## 4. Scoped CSS Model

Prefer attribute or class scoping over tag-name scoping.

Given a declaration for `dce-1`, the runtime should generate a scope UID and stamp it
onto the rendered host/root:

```html
<dce-1 data-cem-scope="cem-scope-dce-1-u4k9f2-p0">
  <dce-root>
    <button>Blue borders</button>
  </dce-root>
</dce-1>
```

Template-local style rules should be rewritten against the generated scope:

```css
[data-cem-scope="cem-scope-dce-1-u4k9f2-p0"] button {
  border: 0.2rem dotted blue;
}
```

For an anonymous declaration, the same rule applies:

```html
<dce-anon-1 data-cem-scope="cem-scope-u4k9f2-p0">
  <dce-root>
    <button>Green dashed border</button>
  </dce-root>
</dce-anon-1>
```

The anonymous tag may still be generated when a browser custom element name is
required, but CSS containment should use `data-cem-scope`, not the generated tag.

### 4.1 Why Attribute Scope

Attribute scoping avoids the false assumption that a tag is globally unique in all
authoring scopes. It also avoids generating invalid or awkward CSS identifiers. The
scope ID can contain a readable tag prefix and occurrence-path information while
the selector remains a straightforward attribute selector.

Class scoping is also viable:

```css
.cem-scope-dce-1-u4k9f2-p0 button { ... }
```

`data-cem-scope` is the preferred default because it is semantically explicit and
less likely to collide with author classes.

### 4.2 Scoped CSS Rewrite Contract

The default scoped CSS strategy is native nesting where it is valid:

```css
[data-cem-scope="cem-scope-dce-1-u4k9f2-p0"] {
  button {
    border: 0.2rem dotted blue;
  }
}
```

This avoids a full selector rewrite for ordinary local rules. The engine still must
scan/parse enough CSS to handle constructs that nesting does not isolate.

Before wrapping, the engine rewrites host-oriented selectors:

- `:host` becomes `&`;
- `:host(...)` becomes `&...`, for example `:host(.active)` becomes `&.active`;
- `:global` and `:global(...)` are treated like `:host` in `cem-element` scoped CSS
  and SHOULD emit a warning in debug/validation mode. This means `:global(.active)`
  becomes `&.active`, not a global escape.
- `:root` is treated like `:global` and `:host` in `cem-element` scoped CSS. It
  becomes `&` and SHOULD emit a warning in debug/validation mode.

Template-local keyframes are renamed because keyframe names are global within a
stylesheet cascade:

```css
@keyframes pulse { ... }
button { animation: pulse 1s; }
```

becomes:

```css
@keyframes pulse-cem-scope-dce-1-u4k9f2-p0 { ... }
[data-cem-scope="cem-scope-dce-1-u4k9f2-p0"] {
  button { animation: pulse-cem-scope-dce-1-u4k9f2-p0 1s; }
}
```

The engine must rewrite both `animation-name` and shorthand `animation` references
that name rewritten keyframes.

`@import` support is phased out for scoped CSS. A browser `@import` cannot be
insulated by a nesting wrapper because it loads rules into the stylesheet cascade.
For now, scoped CSS MUST suppress `@import` and emit a warning. It MUST NOT leave a
native global `@import` in scoped output unless the author opted into an explicit
global style mode outside this scoped-CSS contract.

Other constructs that nesting does not fully isolate:

- `@font-face` defines global font-family names. Support requires renaming the
  family and rewriting `font-family` references inside the scoped CSS. Otherwise
  diagnose.
- `@property` registers custom properties globally. Diagnose unless a future scoped
  property strategy is defined.
- `@counter-style`, `@font-palette-values`, and other named global registries require
  name rewriting plus reference rewriting, or diagnostics.
- `@page` and other page/document-level rules cannot be scoped to a component and
  should be rejected in scoped CSS.
- `@namespace` must be handled before selector parsing and cannot simply be nested.
  The engine may support it as parser metadata; otherwise diagnose.
- `html` and `body` selectors are not checked specially by this scoped-CSS contract.
- Grouping at-rules that contain style rules, such as `@media`, `@supports`,
  `@container`, `@layer`, and `@starting-style`, may remain inside the nesting
  wrapper only when the target CSS engine supports nested grouping rules. For
  persisted output targeting broad compatibility, the transform SHOULD lower them
  to equivalent prefixed selectors or emit a diagnostic.

Unsupported selectors or at-rules MUST NOT silently leak global CSS in
debug/validation or persisted-output mode. The engine should emit diagnostics rather
than partially scope a rule whose behavior it cannot preserve.

## 5. UID Format

Recommended shape:

```text
cem-{kind}-{debug-prefix?}-u{seed}-p{occurrence-path}
```

Examples:

```text
cem-scope-dce-1-u4k9f2-p0
cem-node-cem-button-u4k9f2-p0-2
cem-anon-u4k9f2-p7
```

Fields:

| Field | Purpose |
| --- | --- |
| `cem` | Reserved CEM-generated namespace. |
| `kind` | `scope`, `node`, `anon`, `artifact`, or another bounded identity kind. |
| `debug-prefix` | Optional sanitized public tag or declaration name. Not part of the uniqueness guarantee. |
| `seed` | Stable encoded `uidSeed` namespace. Explicit/host seeds are preferred; source hash or runtime seed is a fallback. |
| `occurrence-path` | Deterministic source/AST occurrence path, with path segments encoded in base 36. |

Sanitization rules:

- lowercase ASCII;
- replace characters outside `[a-z0-9_-]` with `-`;
- collapse repeated `-`;
- truncate debug prefixes to a bounded length, for example 48 bytes;
- if a CSS class is emitted instead of an attribute value, ensure the final class
  token starts with a letter.

Seed values supplied to the engine SHOULD already be bounded and public-safe. The
engine may normalize seed text for CSS identifier syntax, but privacy-preserving
encoding or hashing is the consumer/host responsibility, not a CEM-ML engine
requirement.

## 6. JS Host Algorithm

Browser JS hosts should not call `crypto.randomUUID()` for routine style scopes.
Use a seed plus runtime-local occurrence counters for ephemeral browser-only output.

```ts
interface CemUidAllocator {
  seed: string;
  next(kind: string, debugPrefix?: string): string;
}
```

Single-threaded browser runtime:

```ts
let runCounter = 0;

function createUidAllocator(uidSeed?: string): CemUidAllocator {
  const seed = encodeSeed(uidSeed ?? `runtime-${runCounter++}`);
  let occurrenceCounter = 0;
  return {
    seed,
    next(kind, debugPrefix) {
      occurrenceCounter += 1;
      const prefix = debugPrefix ? `-${sanitize(debugPrefix)}` : '';
      return `cem-${kind}${prefix}-u${seed}-p${occurrenceCounter.toString(36)}`;
    },
  };
}
```

This is enough for the ordinary DOM runtime because JavaScript execution in one
agent is single-threaded. With an explicit or host-provided seed, this allocator is
stable only when allocation order is stable. Saved generated output and SSR output
MUST use the occurrence-path planner in §7, not runtime execution order. Without a
stable seed, this allocator is still unique enough for ephemeral browser-only
rendering.

## 7. WASM And Worker-Pool Algorithm

The CEM-ML engine and `cem-element` WASM path must account for worker-pool execution.
UID generation should avoid cross-thread synchronization, but repeatable output takes
priority over using dynamic worker IDs.

Recommended model:

1. The host supplies or derives a `uid_seed` before work starts.
2. The transform planner assigns each deterministic output scope an occurrence path
   before parallel work emits public IDs.
3. Each worker receives the stable seed plus its assigned occurrence path.
4. A public UID is `(uid_seed, deterministic_occurrence, kind, debug_prefix)`.

Rust sketch:

```rust
pub struct UidPlanner {
    seed: String,
}

impl UidPlanner {
    pub fn uid_for(
        &self,
        kind: &str,
        debug_prefix: Option<&str>,
        occurrence_path: &OccurrencePath,
    ) -> String {
        format_uid(kind, debug_prefix, &self.seed, occurrence_path)
    }
}
```

No atomic increment is required on the hot path when the planner assigns each task a
stable occurrence path. The generated ID does not include
`worker_index`, because worker scheduling is dynamic and would make saved transform
output non-repeatable.

Normative public-ID algorithm:

- occurrence paths are assigned by source/AST pre-order before parallel render work
  starts;
- sibling indexes are counted among generated UID-producing nodes of the same stable
  parent scope;
- declaration-level seeds are combined with stable child occurrence paths, for
  example `uidSeed + declarationPath + occurrencePath`.

Counter ranges MAY be used internally as an optimization only when the public IDs
are equivalent to the occurrence-path algorithm. Counter range reservation must not
change saved generated output, SSR output, or hydration identities.

`worker_index` MAY be used only for ephemeral fallback identities when no stable seed
or deterministic occurrence plan exists and dynamic output is acceptable. Such IDs
MUST NOT be used for saved generated markup, SSR hydration contracts, or versioned
artifacts that need repeatable output.

If the engine executes work outside a fixed worker pool, use one of these fallbacks:

- allocate deterministic occurrence paths before parallel execution;
- allocate a temporary worker/shard index only for ephemeral runtime IDs;
- run a single-threaded allocator for deterministic CLI modes.

### 7.1 Early Allocation Before Source Load

`uidSeed` allows ID allocation before the transformation source is fully loaded. This
matters for streaming template acquisition and import graphs:

- CSS scope IDs can be allocated when the declaration is discovered.
- Placeholder render roots can carry stable IDs before remote template fetch
  completes.
- Worker tasks can allocate render-node IDs from stable ranges without waiting for a
  whole-template content hash.

When no explicit or host seed exists, the engine may fall back to a source hash once
the source is complete. That is deterministic but not early.

## 8. Runtime Vs Persisted Output

The UID and scoped-CSS contract distinguishes ephemeral browser rendering from
persisted transform output.

**Ephemeral browser rendering** is live runtime output that is not saved as an
artifact. Examples include local demos, client-side component upgrade, app runtime
rendering, and development previews.

**Persisted transform output** is output that becomes an artifact or hydration
contract. Examples include generated HTML committed to version control, static-site
build output, SSR HTML, fixture snapshots, compiled CSS, and CDN-published generated
pages.

| Concern | Ephemeral browser rendering | Persisted/build/SSR output |
| --- | --- | --- |
| ID stability | Best effort unless `uid-seed` or a host seed is available. | Required. |
| Public UID algorithm | MAY use runtime allocation for fallback dynamic IDs. | MUST use occurrence path. |
| Worker/thread indexes in public IDs | MAY appear only in fallback dynamic IDs. | MUST NOT appear. |
| Runtime randomness in public IDs | MAY appear only in fallback dynamic IDs. | MUST NOT appear. |
| Duplicate generated-ID detection | MAY be skipped by default. | SHOULD run by default in validation/debug. |
| `uid-seed` uniqueness | Host/provider responsibility. | Host/provider responsibility, validated when checks run. |
| Unsupported scoped CSS | MAY warn and suppress/drop unsafe rules. | MUST diagnose; SHOULD reject or fail validation when output would leak global CSS. |
| `@import` in scoped CSS | MUST suppress with warning. | MUST suppress with warning; SHOULD fail validation if persisted output requires strict CSS completeness. |
| Output diff stability | Not required. | Required. |
| Hydration identity | Only relevant when hydrating retained DOM. | Required for SSR/hydration output. |
| Artifact cache identity | Optional. | SHOULD include content/dependency identity, not only `uid-seed`. |

Rules:

- Normal ephemeral browser runtime MAY use dynamic fallback seeds when no explicit
  `uid-seed`, host seed, or source hash is available.
- Normal ephemeral browser runtime MAY skip duplicate generated-ID detection.
- Normal ephemeral browser runtime MAY suppress unsupported scoped CSS with warnings.
- Persisted transform output and SSR output MUST use stable `uid-seed`,
  host-provided seeds, or source hashes.
- Persisted transform output and SSR output MUST use occurrence-path public IDs, not
  runtime execution order.
- Persisted transform output and SSR output MUST NOT include worker indexes, runtime
  randomness, or execution-order counters in public IDs.
- Persisted transform output and SSR output SHOULD run validation/debug checks by
  default.

## 9. SSR And Hydration

SSR output and browser live-render output are expected to be identical for the same
template, engine version, scoped-CSS mode, UID seed, occurrence path algorithm, and
input data.

The only expected differences are values intentionally supplied as dynamic input
data. For example, if a timer value is bound into the data island, the rendered
timer text is moment-specific. Without such dynamic input differences, server and
browser output should match.

This is a trust and integrity contract between delivery and rendering tiers. The
project should validate it with SSR unit tests and in-browser tests that compare the
same fixtures across server render and client render. Once those tests pass for a
given engine/version/configuration, normal hydration can rely on SSR output without
per-instance generated UUID revalidation.

Rules:

- Server-rendered output MUST include the generated `data-cem-scope` values and
  template artifact identity.
- The client hydration path MUST reuse existing scope IDs from retained SSR DOM.
- The client MUST NOT regenerate CSS scope IDs or other generated UUIDs for retained
  SSR DOM.
- The client MAY verify hydration metadata in debug/validation mode, but normal
  hydration should not require per-instance UUID revalidation after SSR/browser
  parity is established by tests.
- The client MAY generate new IDs only when there is no retained SSR DOM, when it is
  rendering client-only content, or when it intentionally discards the server DOM and
  performs a full client render.
- For deterministic tests and fixtures, the host SHOULD provide an explicit
  `uidSeed`.
- A produced element's `connectedCallback` MUST NOT trigger a DOM update when the
  element body is already produced by hydration.
- Hydration detection should be based on runtime-owned evidence such as the retained
  render root plus an instance data island (`template[data-cem-island="instance"]`)
  produced by SSR or an earlier render.
- The runtime MUST NOT trust author-supplied data-island markup as authoritative
  hydration evidence. If an author-authored data island is possible at parse time,
  the runtime should validate ownership metadata or recreate the island before
  treating it as hydrated output.
- Data-island and template state may be retriggered by events. If an event does not
  change the data-island state, the browser DOM MUST remain unchanged.
- Even when the WASM virtual render tree is recomputed, the DOM sync routine should
  leave the browser DOM untouched when it finds no rendered-tree difference.

## 10. Diagnostics

Diagnostics should distinguish public names from generated identities:

```json
{
  "tag": "dce-1",
  "uidSeed": "demo/scoped-css/dce-1",
  "scopeUid": "cem-scope-dce-1-u4k9f2-p0",
  "message": "scoped CSS rewritten against generated scope UID"
}
```

This keeps troubleshooting readable without making the tag the collision boundary.

## 11. Acceptance Criteria

1. Two declarations with the same public `tag` in separate CEM scopes do not share a
   CSS scope UID.
2. Two anonymous declarations receive unique CSS scope UIDs without calling a browser
   UUID API.
3. A declaration with `tag="dce-1"` may produce a readable UID containing `dce-1`,
   but style containment uses the generated UID.
4. WASM worker-pool rendering can allocate UIDs without per-ID synchronization by
   using stable seeds plus deterministic occurrence paths.
5. SSR output and client hydration preserve server-generated scope UIDs.
6. Scoped CSS uses a `[data-cem-scope="..."] { ... }` nesting wrapper where native
   nesting safely scopes the authored CSS.
7. An explicit `uid-seed` produces stable generated IDs without requiring the full
   transformation source to be loaded or hashed.
8. `uid-seed` is not used as the only compiled-artifact cache key when source,
   imports, transform version, target mode, or policy stamp can change.
9. Worker/thread indexes are used only for ephemeral fallback IDs where dynamic
   output is acceptable.
10. `uid-seed=""` is a valid explicit blank seed and does not fall through to any
    derived seed source.
11. Saved generated output and SSR output use occurrence path as the normative public
    UID occurrence algorithm.
12. Normal ephemeral runtime may skip duplicate generated-ID detection, leaving
    `uid-seed` uniqueness to the host/provider.
13. Validation/debug mode diagnoses duplicate externally visible generated IDs within
    the same output scope.
14. Repeatable output does not repair duplicate seeds with runtime randomness,
    worker indexes, or execution-order counters.
15. Scoped CSS rewrites `:host` to `&`; treats `:global` and `:root` as `:host`
    with a debug/validation warning; and renames scoped `@keyframes` plus animation
    references.
16. Scoped CSS suppresses `@import` with a warning for now.
17. Unsupported scoped CSS constructs diagnose rather than silently leaking global
    CSS in debug/validation or persisted-output mode.
18. Ephemeral browser rendering may use dynamic fallback seeds and skip duplicate-ID
    validation when output is not persisted.
19. Persisted transform output and SSR output use stable seeds or source hashes,
    occurrence-path public IDs, and validation/debug checks by default.
20. Persisted transform output and SSR output do not include worker indexes, runtime
    randomness, or execution-order counters in public IDs.
21. SSR output and browser live-render output are identical for the same static
    inputs, except for explicitly dynamic input data.
22. SSR-vs-browser parity is validated by SSR unit tests and in-browser tests before
    normal hydration relies on retained SSR DOM.
23. Hydration reuses existing `data-cem-scope` and generated IDs from retained SSR
    DOM and does not regenerate them during `connectedCallback`.
24. `connectedCallback` does not update the DOM for produced elements whose body is
    already produced by hydration.
25. Runtime-owned data islands may signal hydrated output, but author-supplied data
    islands are not trusted as authoritative hydration evidence.
26. Event-triggered rerendering leaves the browser DOM unchanged when the data island
    and rendered output do not change.
27. The project includes unit, validation, browser, and SSR tests covering repeatable
    UID generation, parallel scheduling stability, scoped CSS isolation,
    duplicate-ID diagnostics, SSR/browser parity, hydration no-op behavior, and
    unsupported scoped-CSS diagnostics.

## 12. Test Matrix

The requirements should be protected by executable gates. The test matrix is part of
the acceptance contract, not only implementation guidance.

| Area | Test Type | What To Prove |
| --- | --- | --- |
| Repeated builds | Unit/fixture snapshot | Same input produces byte-identical public IDs and generated output across runs. |
| Occurrence path | Unit | Public IDs derive from source/AST occurrence path, not execution order. |
| Parallel scheduling | WASM/worker integration | Worker scheduling order does not change public IDs. |
| Same tag in separate scopes | Unit/integration | Public tag is not the CSS scope identity; generated `scopeUid` differs. |
| Same `uid-seed` collision | Validation/debug | Duplicate externally visible generated IDs are diagnosed in the same output scope. |
| Blank seed | Unit | `uid-seed=""` is valid, does not fall through, and uses occurrence path. |
| Runtime fallback | Browser/runtime | Ephemeral runtime can use dynamic fallback seeds without persisted-output guarantees. |
| SSR/browser parity | SSR unit + browser | Same static inputs produce identical server-rendered and browser-rendered DOM. |
| Dynamic data exception | SSR unit + browser | Known dynamic binding may differ only in that bound value. |
| Hydration no-op | Browser | Client reuses server `data-cem-scope`; `connectedCallback` does not rewrite hydrated DOM. |
| Event no-op rerender | Browser | Event retrigger that does not change data leaves browser DOM unchanged. |
| Scoped CSS nesting | Unit/browser | Local rules are wrapped with `[data-cem-scope] { ... }` and do not style outside nodes. |
| `:host`, `:global`, `:root` | Validation/unit | `:host` rewrites to `&`; `:global` and `:root` rewrite to `&` and warn. |
| Keyframes | Unit/browser | `@keyframes` names and `animation` references are renamed consistently. |
| `@import` | Validation/unit | Scoped CSS suppresses `@import` with warning. |
| Unsupported CSS | Validation/unit | Unsupported scoped-CSS constructs diagnose rather than leaking global CSS. |
| Security advisory examples | Docs/lint fixture | Public docs/examples do not encourage raw private paths in `uid-seed`; engine does not own hashing. |

Test layering:

- Unit tests cover seed precedence, blank seed behavior, occurrence paths, UID
  formatting, and CSS rewrite decisions.
- Validation tests cover duplicate IDs, `@import`, unsupported CSS, `:global`, and
  `:root` diagnostics.
- Browser tests cover scoped CSS isolation, hydration no-op behavior, and event
  rerender no-op behavior.
- SSR tests compare server output and browser output for shared fixtures.

## 13. Adopted Design

Adopt generated UID scope identity for all declarations. Use the public tag only as
an optional debug prefix. Add `uid-seed` as the explicit stable namespace override
for generated IDs. For scoped CSS, stamp `data-cem-scope` on the produced render
host/root and rewrite local style rules to target that generated scope.

This gives `cem-element` and the CEM-ML engine one identity model for anonymous
declarations, public custom-element tags, browser runtime output, SSR output, and
parallel WASM processing, while preserving stable saved transform output when hosts
provide deterministic seeds.

## 14. Resolved Design Questions

1. [x] Normative seed rules — resolved
   Seed precedence is defined in §3. `uid-seed=""` is valid and treated as an
   explicit blank-string seed.

2. [x] Deterministic occurrence algorithm — resolved
   Occurrence path is the normative public UID algorithm for saved generated output
   and SSR output. Counter ranges are allowed only as an internal optimization when
   public IDs remain equivalent to occurrence-path output.

3. [x] Collision behavior — resolved
   `uid-seed` uniqueness is a host/provider contract in normal ephemeral runtime.
   Validation/debug mode diagnoses duplicate externally visible generated IDs in the
   same output scope. Repeatable output does not auto-repair duplicates with dynamic
   disambiguators.

4. [x] CSS rewrite contract — resolved
   Native nesting is the default scoping strategy where it works. `:host` rewrites
   to `&`; `:global` and `:root` are treated as `:host` with debug/validation
   warning; keyframes and animation references are renamed; `@import` is suppressed
   with warning for now; `html` and `body` are not checked specially; unsupported
   constructs diagnose rather than leak global CSS in debug/validation or persisted
   output.

5. [x] Runtime vs build output — resolved
   Ephemeral browser rendering may use dynamic fallback seeds, skip duplicate-ID
   validation, and warn/drop unsupported scoped CSS. Persisted build/SSR output uses
   stable seeds or source hashes, occurrence-path public IDs, and validation/debug
   checks by default.

6. [x] Hydration contract — resolved
   SSR and browser live render are expected to produce identical output for the same
   static inputs. That parity is validated by SSR unit and in-browser tests. Normal
   hydration reuses retained SSR `data-cem-scope` and generated IDs without
   per-instance UUID revalidation; `connectedCallback` does not update DOM that is
   already produced by hydration.

7. [x] Security/privacy — resolved as consumer advisory
   Seed data resides on the consumer side. Consumers/hosts should provide bounded,
   public-safe, optionally encoded `uid-seed` values. `cem-element` and the CEM-ML
   engine do not handle, redact, or hash sensitive seed data as part of this design.

8. [x] Test matrix — resolved
   Unit, validation, browser, and SSR tests cover repeatable UID generation,
   occurrence-path stability, parallel scheduling, same-tag separate scopes,
   same-seed collision diagnostics, blank seed behavior, scoped CSS isolation,
   SSR/browser parity, hydration no-op, event no-op rerendering, and unsupported CSS
   diagnostics.
