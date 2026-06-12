# `cem-ql` Stack Design

**Status:** Draft high-level design derived from [`cem-ql-ac.md`](cem-ql-ac.md) and
the host stack documents [`cem-ml-ac.md`](cem-ml-ac.md),
[`cem-ml-stack-design.md`](cem-ml-stack-design.md), and
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md). Concrete data
shapes, parser tables, evaluator IR, and Rust module ownership live in
[`cem-ql-stack-design-impl.md`](cem-ql-stack-design-impl.md).
**Primary acceptance criteria:** [`cem-ql-ac.md`](cem-ql-ac.md).
**Date:** 2026-05-19

---

## 1. Purpose And Scope

This document translates `cem-ql-ac.md` into a high-level design for the cem-ql
query language. cem-ql is the in-process selector and transformation expression
language for CEM-ML; it is **not** a general-purpose programming language and it
is **not** a browser DOM language. It reads CEM-ML AST projections (input DOM,
CEM AST, machine-state slots, report AST, template registries, source-map
stacks) and produces streamed results for templates, validators, transforms,
plugins, CLI projections, and runtime hydration rules.

This design fixes:

- the functional layer boundaries inside the cem-ql evaluator,
- the algorithm selections for each layer (lexer, parser, name resolver,
  type checker, evaluator IR, runtime),
- the Rust module topology (full module names live in the impl document),
- the source-map, diagnostic, and report responsibilities at the cem-ql layer
  versus the host layer,
- the Tier A MVP scope,
- the compiled-binary artifact layout that participates in the host's
  content-addressed cache (cem-ml-ac.md §14, cem-ql-ac.md §14), and
- open design decisions that must be resolved before implementation.

### 1.1 Acceptance Criteria Alignment Policy

`cem-ql-ac.md` is the primary decision driver. This document explains how each
AC item is satisfied, deferred, or blocked by an explicit open question; it
does not redefine AC contracts. If this design conflicts with the AC, the AC
wins until either the design is corrected or an unresolved ambiguity is
recorded in both documents.

cem-ql is **downstream of `cem-ml-ac.md`**. The host AST, scope model,
content-type set, schemas, source-map stacks, async stream APIs, scope-policy
mechanism, and report tree are owned by the host. cem-ql consumes those
contracts and adds the query-language layer on top.

### 1.2 Relation To The Host Placeholder

`cem-ml-stack-design-impl.md §3.10` defines a `ScopedQuery` shape with a
`QueryLanguage::CemQl` enum variant and a `String` body labelled "cem-ql source
per `docs/cem-ql-ac.md`". This design document is the normative resolution for
that placeholder: the `String` is parsed by the cem-ql parser defined here, the
resulting evaluator IR runs inside `QueryContextScope`, and every diagnostic
routes through the host report AST.

---

## 2. Domain Context

cem-ql is invoked in three host contexts:

1. **Template embedding (AC-T-7).** The CEM-ML template compiler extracts a
   cem-ql expression from a host-owned `{ … }` attribute span, a
   whole-expression attribute (`select="…"`, `match="…"`, `test="…"`), or an
   explicit content `$` expression node (`{$ …}` / `{$ | …}`). The compiler
   appends `TransformKind::TemplateEmbedding` to the source-map stack and hands
   plain UTF-8 cem-ql source to the cem-ql parser. cem-ql itself defines no
   embedding delimiter.
2. **Direct evaluator API.** Tools, validators, plugins, and CLI projections
   call `evaluate(query_source, context)` directly (AC-Q-6). The evaluator
   constructs the same compilation pipeline as template embedding but does not
   need the embedding source-map frame.
3. **Compiled-artifact load (AC-QC-*).** A previously compiled query binary is
   loaded from the host's content-addressed cache. Parse, name resolution, and
   type-check are skipped; evaluation resumes from the binary form.

cem-ql is **read-only with respect to the host AST** (AC-Q-2). Every
expression returns a new stream/value; no expression mutates a host node,
attribute, slot, registry entry, or scope. Mutation is owned by the Tier C
async DOM mutation API in `cem-ml-ac.md §5`.

cem-ql is **deterministic for a given input AST + scope + context** (AC-Q-4).
Iteration order, attribute order, set-operation results, and diagnostic
ordering are stable for snapshot use.

---

## 3. Pipeline Overview

The cem-ql pipeline is a strict left-to-right transform from query source to
evaluator output:

```
UTF-8 query source
  │
  ▼
[L1] Lexer            ───►  Token stream (tagged, with byte ranges)
  │
  ▼
[L2] Parser           ───►  Surface AST (cem-ql syntactic form)
  │
  ▼
[L3] Name Resolver    ───►  Resolved AST + scope binding table
  │
  ▼
[L4] Type Checker     ───►  Typed AST + schema-type bindings
  │
  ▼
[L5] IR Lowerer       ───►  Evaluator IR (typed, linear)
  │
  ▼
[L6] Evaluator        ───►  Stream<Item>     (host-attached, lazy)
```

Each layer is independently testable. Each layer attaches source-map frames so
diagnostics from any later layer trace back to the original query byte range
(AC-Q-5).

**Layers L1–L5 are compile-time work.** They are deterministic, sandbox-safe,
and produce no host-AST mutations. Their output is the compiled query
artifact (AC-QC-2). L1–L5 run under the active scope's compile-time budget,
which is conservative because compile cost is paid per module per host, not
per evaluation.

**Layer L6 is evaluation.** It runs under the active scope's evaluation
budget (AC-QR-1). Bounded fields:

- max items materialized per pipeline stage,
- max recursion depth per call,
- max function-call count per evaluation,
- max captured-closure size,
- max regex backtracking budget (Tier B),
- max external-document fetches (Tier B; uses the host I/O queue).

### 3.1 Cache Skip Path

When a compiled artifact is loaded from the host cache (AC-QC-2), the pipeline
skips L1–L5 and resumes at L6 with the binary-form IR. Source-map stacks are
preserved in dev mode (AC-QC-4 / AC-QC-5) so a diagnostic emitted from a
reloaded binary is indistinguishable from a source-driven diagnostic.

### 3.2 Resource Limit Policy

cem-ql does not own a separate resource-limit policy. Every evaluation runs
inside a `QueryContextScope`
(`cem-ml-stack-design-impl.md §3.10`), and limits are inherited from the
active host scope policy. Limit breaches emit `cem.ql.budget_exceeded`
(AC-QR-2) and abort the failing evaluation; sibling evaluations under
unrelated scopes are not affected.

---

## 4. Source-Map Model

cem-ql participates in the host's origin-first source-map stack
(`cem-ml-stack-design.md §4`). Frames added by cem-ql:

- `TransformKind::Query` — appended to every produced item that is not a
  host AST node (record literal, sequence construction, computed atom)
  (AC-QD-5).
- `TransformKind::QueryStep` — appended to each pipeline step result so
  authors can trace which step produced which item.
- Reused frames: the host's `TransformKind::TemplateEmbedding` frame is
  attached by the template compiler before cem-ql sees the source; cem-ql
  inherits it without modification.

`SourceId` for cem-ql source is a fresh id assigned by the loader (direct
API), the template compiler (embedded form), or the binary loader (cache
form). The id MUST be stable across a compilation so cached binaries
reference the same source id when re-emitted.

Diagnostics from any cem-ql layer carry the query's source-map stack and
the active host scope context per `cem-ml-ac.md` AC-O-3 (AC-QE-3).

---

## 5. Layer 1 — Lexer (`cem_ql::lexer`)

### Purpose

Tokenize UTF-8 cem-ql source into a stable token stream with byte-precise
ranges. AC-QS-6 requires a stable lexical grammar that does not depend on
host content type, host whitespace policy, or host trivia preservation
policy.

### Functional Design

- Input: `&[u8]` UTF-8 query source plus a `SourceId`.
- Output: `Vec<Token>` (or streaming iterator) where each token carries kind,
  byte range, and any cooked text payload (string literal value with escapes
  resolved, numeric literal value).
- Whitespace is trivia: lexed for diagnostic spans but not surfaced as a token
  to the parser unless required by a future surface rule (e.g. record-key
  quoting clarity).
- Comments are lexed (line `;;` and block `(* … *)` to align with XPath
  conventions while staying distinct from XML/HTML/JSON markers); the parser
  ignores comment tokens but they survive to the source-map for diagnostics.

### Token Categories

| Category | Examples |
|----------|----------|
| Punctuation | `.` `,` `(` `)` `[` `]` `{` `}` `|` `&` `-` `^` `:=` `:` `::` |
| Operators | `eq` `ne` `lt` `gt` `le` `ge` `=` `!=` `+` `-` `*` `div` `mod` `and` `or` `not` |
| Reserved | `&&` `||` (parse as `cem.ql.use_and_or` per AC-QO-5) |
| Keywords | `let` `in` `if` `then` `else` `for` `return` `some` `every` `satisfies` `import` `as` `declare` `variable` `function` `module` `instance` `of` `cast` `treat` `is` `fn` |
| Identifiers | `[A-Za-z_][A-Za-z0-9_-]*` with namespace-prefix form `prefix:local` |
| Literals | string `"…"` (with `\n` `\t` `\u{…}` escapes), integer, decimal, double, boolean (`true`/`false`), `null` |
| Path tokens | `/` (path step synonym for `.`), `..` (parent step) |
| Pipeline step | leading `.` only inside a pipeline step body (AC-QS-2) |

### Tier A Scope

All token categories above are Tier A. Tier B adds regex literal escapes
inside string literals, FLWOR-related keywords (`where`, `order`, `by`), and
the `try`/`catch` keywords.

---

## 6. Layer 2 — Parser (`cem_ql::parser`)

### Purpose

Build a surface AST from the token stream. The parser is hand-written
recursive descent with Pratt-style operator precedence for the expression
sub-grammar.

### Algorithm Selection

- **Hand-written recursive descent** for module-level forms (`module`,
  `import`, `declare`), statements, and pipeline composition. Recursive
  descent gives precise error spans, easy recovery synchronization on
  statement and step boundaries, and predictable performance.
- **Pratt parsing** for the expression grammar: arithmetic, comparison,
  boolean, set operators (`|` `&` `-` `^`), pipeline operator (`.`), path
  operator (`/`). Pratt parsing is the standard choice for operator-precedence
  expressions and integrates cleanly with recursive descent.
- **No parser generator.** cem-ql grammar is small and Pratt + recursive
  descent is the lowest-overhead choice. A future Tier C grammar extension
  (XQuery 3.1 parity) MAY adopt LALR if the manual grammar becomes
  unmanageable; that decision is deferred.

### Error Recovery

The parser recovers at three synchronization points:

1. **Statement boundary** — `declare`, `import`, top-level `module`.
2. **Pipeline step boundary** — between `.step` invocations.
3. **Bracket boundary** — `)`, `]`, `}` close the current group.

Recovery emits `cem.ql.parse_error` and continues so multiple syntax errors
surface in one compile pass (AC-QE-1).

### Tier A Scope

The full surface syntax in AC-QS-1..AC-QS-6 is Tier A. Tier B adds:

- comprehension sugar (AC-QO-7),
- `try`/`catch` (AC-QE-2),
- FLWOR with `where`/`order by` (AC-QX-4, AC-QO-6),
- XSLT-style include/import precedence syntax (AC-QI-7),
- regex literal escapes inside strings.

Tier C: full XQuery 3.1 surface where it does not duplicate Tier A/B helpers
(AC-QX-0 functional-not-syntactic-parity).

---

## 7. Layer 3 — Name Resolver (`cem_ql::resolve`)

### Purpose

Resolve every name reference in the surface AST to a binding: a local
lexical variable, a query-module declaration, a host scope binding, a
schema-derived type, a template reference, a machine-state slot key, or a
stdlib name. Resolution order is fixed by AC-QV-3:

```
local lexical
  → query module (declarations + non-cem: imports)
    → for each ancestor host scope, innermost first:
          { scope's bindings ∪ scope's stdlib overlay }
      → platform stdlib defaults
```

### Algorithm Selection

- **Lexical scope chain** materialized as a stack of `BindingSet`s. The
  resolver walks the chain on each unresolved reference; resolution is
  source-position-aware so a re-declaration shadows from its source point
  forward but earlier uses keep their original binding (AC-QV-4).
- **Per-scope stdlib overlay.** Each host scope contributes an optional
  overlay map `(module-uri, name) → binding`. Overlays are part of the
  binding set, not a parallel resolution path; AC-QV-3's nearest-binding
  rule subsumes overlays automatically.
- **Reserved-scheme guard.** `cem:` and `urn:cem:` are reserved (AC-QI-2);
  scope policies whose grant set lists either scheme fail at policy load
  with `cem.ql.reserved_scheme`. The resolver enforces this on import
  resolution; the scope-policy loader enforces it on policy install.

### Outputs

- Resolved AST: every name reference carries a `BindingId`.
- Resolution-trace events for AC-QV-8 verification (every name resolution
  emits a structured event recording resolved scope id, declaration site,
  and resolution rule).
- Diagnostics: `cem.ql.unknown_variable`, `cem.ql.unknown_function`,
  `cem.ql.import_denied`, `cem.ql.import_unresolved`,
  `cem.ql.reserved_scheme`.

### Tier A Scope

The full AC-QV-3 resolution chain is Tier A, including stdlib overlays.
Tier B adds:

- XSLT-style include/import precedence (AC-QI-7),
- `urn:cem:` plugin-registered modules (depends on plugin runtime per
  `cem-ml-ac.md` AC-PL-*).

---

## 8. Layer 4 — Type Checker (`cem_ql::types`)

### Purpose

Statically check types at query compile time when both sides are known and
fall back to runtime checks otherwise (AC-QT-3). The lattice is fixed by
AC-QT-1:

- **node types**: `node`, `element(QName)`, `attribute(QName)`, `text()`,
  `comment()`, `processing-instruction()`, `document-node()`.
- **schema-declared element types** derived from the active schemas'
  `StructuralSchemaIr`. Identity is **scope-relative** (AC-QT-4): the
  same lexical name `Button` may resolve to different schema types in
  different host scopes.
- **atom types**: XPath-equivalent `string`, `xs:integer`, `xs:decimal`,
  `xs:double`, `xs:boolean`, `xs:date`, `xs:dateTime`, `xs:duration`,
  `xs:anyURI`.
- **compound types**: `record(k1: T1, …)`, `array(T)`, `stream(T)`,
  `lambda(args …) -> T`.
- **resource types**: `resource(content-type, schema?)`.

### Algorithm Selection

- **Bidirectional type-checking** (inference for elimination forms,
  checking for introduction forms). This is the standard low-cost choice
  for a structural type system with subtyping (schema-derived types).
- **Subtype check via structural traversal.** The schema IR already
  encodes content models as a finite tree; subtyping reduces to a
  structural walk with name-equivalence on `ExpandedName`. No general
  algebraic-subtyping engine is required at Tier A.
- **No implicit coercion** (AC-QO-3, AC-QO-8). The type checker rejects
  cross-type comparisons that XPath 3.1 would auto-promote and emits
  `cem.ql.cross_type_compare`; the explicit conversion functions
  (`double(.)`, `decimal(.)`, `integer(.)`, `string(.)`, `nfc(.)`,
  `to_utc(.)`) are the only path to uniform-type streams.

### Strict-Default Failure Contract

The type checker emits `cem.ql.type_error` (and the static resolution codes
`cem.ql.unknown_type`, `cem.ql.unknown_function`, `cem.ql.unknown_variable`)
at `error` severity by default (AC-QT-3). A static failure means no
evaluator IR is produced; the compiled artifact is not emitted to the cache
and `evaluate(...)` rejects with the same diagnostic.

A **development/debugging CLI profile** (a named scope-policy preset
shipped by the host CLI) MAY relax this contract by remapping the codes
above to `warning`. Under that profile the compiler still emits the
diagnostic but produces evaluator IR; runtime type failures substitute the
empty stream for the failing sub-expression and continue evaluating
siblings. The profile MUST be opt-in and MUST NOT be the default for
non-interactive callers (templates, validators, build pipelines,
server-side hosts).

### Tier A Scope

The full lattice in AC-QT-1, `instance of`/`cast as`/`treat as`/`is`, and
scope-relative schema-type identity are Tier A. Tier B adds attribute-group
record types (AC-QT-5). Tier B MAY emit TS/Rust type stubs for query
modules (AC-QT-6).

---

## 9. Layer 5 — IR Lowerer (`cem_ql::ir`)

### Purpose

Lower the typed AST into a linear evaluator IR. The IR is the binary
artifact serialized to the host cache (AC-QC-2) and the input to the
evaluator.

### IR Shape (High Level)

The IR is a forest of typed instructions. Each instruction node carries:

- opcode (axis step, pipeline step, function call, set operator,
  literal, binding reference, etc.),
- output type,
- source-map frame,
- operand references into the same forest.

The IR is **not a stack machine**. It is a tree-shaped instruction graph
that the evaluator walks lazily. Concrete shape (Rust enum variants and
field layouts) lives in
[`cem-ql-stack-design-impl.md`](cem-ql-stack-design-impl.md).

### Lowering Rules

- **Pipeline `.`** lowers to a `Pipeline { source, steps }` node; each
  step is itself an IR node. The lowering preserves AC-QP-4 laziness:
  the IR does not materialize intermediate streams; the evaluator
  pulls one item at a time.
- **Set operators** lower to dedicated `Union`, `Intersect`, `Difference`,
  `SymmetricDifference` nodes with the strict-typed identity rule baked
  in (AC-QO-3). The IR does not lower set operators to comprehensions;
  evaluator-side specialization preserves AC-QO-4 streaming semantics.
- **`let`** lowers to a `Let { name, value, body }` node. Bindings are
  immutable.
- **Lambdas** lower to closure values carrying a captured binding
  environment. Capture rules per AC-QV-6 detach host-AST references at
  lowering time when the closure may outlive the host scope.
- **Stdlib calls** lower to `StdlibCall { module, name, args }` with the
  call site's overlay fingerprint captured in the surrounding artifact's
  policy stamps (AC-QC-3).

### Tier A Scope

Lowering for every Tier A surface form is Tier A. Tier B lowers `try`/`catch`,
FLWOR with `where`/`order by`, and comprehension sugar.

---

## 10. Layer 6 — Evaluator (`cem_ql::eval`)

### Purpose

Walk the IR and produce a `Stream<Item>`. Streams are **lazy** (AC-QP-4):
a pipeline step does not consume more of its input than its output
requires. Short-circuit forms (`.first`, `.exists`, `.empty`, `if/then/else`)
stop iteration as soon as the answer is known (AC-QP-5).

### Algorithm Selection

- **Pull-based stream model.** The evaluator returns iterators that the
  host (or another evaluator step) pulls. Pull-based evaluation is the
  natural fit for XPath-style pipelines and aligns with the host's
  async stream API (AC-Q-6, `cem-ml-ac.md` AC-A-1).
- **Tail-call elimination on pipeline chains.** AC-QL-6 requires that a
  long `.`-chain not grow the stack proportional to chain length. The
  evaluator represents pipelines as iterator chains, not recursive
  calls; tail position is automatically flat.
- **Streaming set operators.** `|` streams both operands; `&`, `-`, `^`
  buffer the right operand bounded by the host's scope-policy memory
  cap (AC-QO-4). The buffer size and overflow behaviour are policy
  fields, not evaluator constants.
- **Bounded recursion.** The evaluator counts call depth and function
  invocations against the active scope policy; a breach emits
  `cem.ql.budget_exceeded` (AC-QR-2) and aborts the failing
  evaluation. Siblings under unrelated scopes are unaffected.

### Item Production And Source Maps

Every produced item carries a source-map stack (AC-Q-5):

- **Host AST nodes** preserve the host's stack untouched.
- **Computed atoms, records, arrays** carry the originating expression's
  stack plus a `TransformKind::Query` frame.
- **Pipeline-step intermediates** carry the step's source range as a
  `TransformKind::QueryStep` frame.

### Scope Violation

Any node, slot, or resource access outside the active `QueryContextScope`
emits `cem.ql.scope_violation` (AC-Q-3). The evaluator never silently
skips an out-of-scope access; the diagnostic surfaces and the failing
expression returns an empty stream.

---

## 11. Stdlib Module Layout

Stdlib modules are baked into the host crate and reachable via
`cem:stdlib/<topic>` (AC-QI-2 platform tier, AC-QI-3). Per-scope
**overlay** maps may shadow specific names within a subtree without
changing which module body the URI loads (AC-QV-3, AC-QV-7).

### Tier A Modules

| URI | Tier A scope |
|-----|--------------|
| `cem:stdlib/sequence` | Pipeline step helpers (`map`, `where`, `flat_map`, `take`, `drop`, `first`, `last`, `nth`, `peek`) plus function aliases for the four set operators (`union`, `intersect`, `difference`, `symmetric_difference`). |
| `cem:stdlib/strings` | Codepoint iteration, length, slicing, casing, formatting. |
| `cem:stdlib/numbers` | Math, formatting, conversion (`double`, `decimal`, `integer`, `string`). |
| `cem:stdlib/datetime` | `xs:date`/`xs:dateTime` helpers, `to_utc`. |
| `cem:stdlib/dom` | Function-form host AST helpers (axes, attribute access, reference resolution). |
| `cem:stdlib/report` | Diagnostic emit and severity helpers. |
| `cem:stdlib/state` | Read-side machine-state slot helpers. |
| `cem:stdlib/template` | Template-registry lookup helpers. |
| `cem:stdlib/cemml` | Read CEM-ML canonical content from in-memory strings. |

### Tier B Modules

| URI | Tier B scope |
|-----|--------------|
| `cem:stdlib/sequence` | Full AC-QO-6 helper family (`group_by`, `count_by`, `partition`, `zip`, `chunked`, `windowed`, `sliding`, `take_while`, `drop_while`, `sorted`, `reversed`, `reduce`, `fold`, `scan`, `any`, `all`, `none`, `min`, `max`, `sum`, `avg`). |
| `cem:stdlib/strings` | Regex (`matches`, `replace`, `split`), Unicode normalization (`nfc`, `nfd`). |
| `cem:stdlib/content-types` | Canonical media-type identifiers and the default `read()` preference list per AC-QA-1.1. |

Tier C MAY add domain-specific modules under `cem:stdlib/` for features
that depend on Tier C host runtime (NVDL dispatch, query-time hydration
rule generation, etc.).

### Overlay Semantics

A scope's overlay map re-binds an existing `cem:stdlib/<topic>` name to a
host- or plugin-supplied implementation for that scope and its descendants.
Overlay-introduced bindings MUST match the platform signature (AC-QT-3);
mismatched overlays fail compile. Overlay state is captured in the binary
artifact's policy stamps (AC-QC-3) so a binary compiled under one overlay
is reused only when the loading scope's resolved overlay matches.

---

## 12. Cost Model

cem-ql costs are inherited from the host scope policy; cem-ql does not
own its own budget knobs. The cost model below explains how cem-ql
**charges** against those budgets.

| Operation | Charge |
|-----------|--------|
| Axis step (Tier A axes) | 1 item-materialization per visited node, bounded by AC-QR-1 max items per pipeline stage. |
| Pipeline `.` | 1 step-result-materialization per produced item. |
| Set operator | `|`: 1 materialization per output item, no buffering. `&` `-` `^`: 1 materialization per output item plus a right-operand buffer bounded by the scope policy. |
| `let` / `for` | 1 binding allocation per iteration. |
| Lambda | 1 closure allocation at definition; 1 invocation charge per call. |
| Stdlib call | per-function charge published in the stdlib reference table (impl doc). |
| `read(uri, accepts?)` (Tier B) | 1 host I/O queue slot (AC-A-6) plus the transform-graph cost named by the resolved `accepts` entry. |
| Regex (Tier B) | 1 backtracking-budget unit per character considered, bounded by AC-QR-1 max regex backtracking. |

Compile-time cost (L1–L5) is separate. The compile budget is conservative
because compile cost is paid once per module per host; the host MAY enforce
a compile-time soft cap (default 100 ms per module on a developer-class
machine) but a compile-budget breach is not Tier A AC-graded.

---

## 13. Compiled Artifact Layout

cem-ql participates in the host's shared content-addressed cache and
transport protocol (cem-ml-ac.md §14, cem-ql-ac.md §14). The compiled
artifact carries:

| Field | Description |
|-------|-------------|
| Header | `cem-bin/1+blake3` scheme tag, content-type `cem-ql/1`, hash. |
| Module identity | declared `module` URI plus source URI (AC-QI-6). |
| IR | typed evaluator IR per §9. |
| Schema bindings | resolved schema-type bindings (or rebindable stubs for scope-relative re-resolution per AC-QT-4). |
| Import closure | resolved import URIs and their hashes. |
| Source-map sidecar | dev-mode only (AC-QC-4 / AC-CC-4); referenced by sidecar hash per AC-CC-5. |
| Policy stamps | declared imports, declared `read()` `accepts` lists (one per call site), declared external resources, resolved stdlib overlay fingerprint (AC-QC-3). |

The artifact MUST be reproducible across hosts whose schemas, imports, and
overlays fingerprint identically (AC-QC-1).

Stamping rules for `read()` follow the AC-QA-1 input forms (AC-QC-3):
omitted form stamps the floor marker; header-string form parses and stamps
the canonical preference list at compile time; collection form normalizes
through the alias table and stamps in caller order. Dynamically computed
`accepts` entries emit `cem.ql.read_dynamic_accepts` and stamp as
wildcard, deferring resolution to load-time.

A binary whose stamps the active scope policy cannot satisfy MUST fail
with `cem.cc.policy_mismatch` and fall back to source when available.

---

## 14. Rust Module Map (Summary)

Concrete module ownership lives in
[`cem-ql-stack-design-impl.md`](cem-ql-stack-design-impl.md) §3. The
high-level shape:

```
cem_ql/
  lexer/        — L1
  parser/       — L2
  resolve/      — L3
  types/        — L4
  ir/           — L5
  eval/         — L6
  stdlib/       — platform stdlib modules
  artifact/     — compiled binary serialization
  diagnostics/  — cem.ql.* code table (mirrored from host diagnostics)
  api/          — public evaluate(), compile(), load() surface
```

The cem-ql crate is a **library crate** (`rlib` + `cdylib`) in the same
workspace as `cem-ml`. The CLI does not gain a separate cem-ql binary;
queries are reachable through `cem-ml-cli` (as template-embedded queries
or via a `query` sub-command). Public API consumers are templates,
validators, plugins, and host crate code; there is no out-of-process
boundary inside cem-ql.

---

## 15. Tier A Scope

A Tier A `cem-ql` release ships:

| Area | Tier A includes | Tier A excludes |
|------|-----------------|-----------------|
| Surface syntax | AC-QS-1..AC-QS-6 (dot pipelines, leading-dot in step body, record literals with quoted keys, `let … in`, module form). | Comprehension sugar, FLWOR, `try/catch`, bare-identifier record keys. |
| Axes (AC-QD-1) | `self`, `child`, `parent`, `descendants`, `descendants-or-self`, `ancestors`, `ancestors-or-self`, `following-sibling`, `preceding-sibling`, `attributes`. | `following`, `preceding`, `namespace`. |
| DOM access | Reference resolution (AC-QD-4), source-map preservation (AC-QD-5), tainted-subtree visibility (AC-QD-6). | Machine-state slots and template-registry entries as first-class items (AC-QD-7). |
| XPath functional parity | AC-QX-1 subset (axes, predicates, sequence construction, comparisons, arithmetic, boolean, `if/then/else`, `for…return`, `some/every…satisfies`, built-in function library subset per AC-QF-2). | XPath 3.1 maps/arrays (Tier B), regex, `try/catch`. |
| Set operators | `|` `&` `-` `^` with strict-typed identity (AC-QO-1..AC-QO-5). | Collection-helper function family (Tier B, AC-QO-6); comprehensions (AC-QO-7). |
| Pipeline | `.`-chain, named-function steps, lambda steps, laziness, short-circuit (AC-QP-1..AC-QP-5). | None — pipeline is fully Tier A. |
| Scope inheritance | Full AC-QV-3 chain with stdlib overlays; AC-QV-4 source-position-aware shadowing; AC-QV-5 XSLT precedence; AC-QV-6 closure detachment; AC-QV-8 resolution trace. | XSLT-style `include` precedence syntax (Tier B, AC-QI-7); plugin-registered `urn:cem:` modules (Tier B, depends on plugin runtime). |
| Type system | Full lattice (AC-QT-1), `instance of`/`cast as`/`treat as`/`is`, scope-relative schema-type identity, strict default failure. | Attribute-group record types (AC-QT-5, Tier B); TS/Rust type stubs (AC-QT-6, Tier B). |
| Stdlib | The Tier A modules in §11. | Tier B / Tier C modules in §11. |
| External data | None at Tier A. | `read()` (AC-QA-1) is Tier B. |
| Imports | Platform `cem:` imports always available; `urn:cem:` and network schemes Tier B. | All AC-QI-4 user modules. |
| Diagnostics | All Tier A codes in AC-QE-1; host report AST routing (AC-QE-3); bubble-to-boundary (AC-QE-4). | Surface `try/catch` keyword (Tier B). |
| Resource limits | AC-QR-1 limit set (items, recursion, function calls, closure size); AC-QR-2 `cem.ql.budget_exceeded`; AC-QR-3 no `eval`; AC-QR-4 untrusted input. | Tier B regex / external fetch limits. |
| Compiled artifact | None at Tier A. | All of §13 is Tier B. |

---

## 16. Algorithm Selection Summary

| Layer     | Problem                                | Algorithm                                        | Reason |
|-----------|----------------------------------------|--------------------------------------------------|--------|
| L1        | Tokenization                           | Hand-written DFA-style lexer                     | Small grammar, byte-precise spans, no generator dependency |
| L2        | Module-level forms                     | Hand-written recursive descent                   | Statement-boundary recovery, precise diagnostics |
| L2        | Expression operators                   | Pratt parsing                                    | Standard low-overhead choice for operator-precedence grammars |
| L3        | Name resolution                        | Lexical scope chain with per-scope overlay maps  | AC-QV-3 nearest-binding rule subsumes overlays uniformly |
| L4        | Type checking                          | Bidirectional inference + structural subtype walk | Low cost for a structural type system with schema-derived subtypes |
| L4        | Cross-type comparison                  | Reject + diagnostic (`cem.ql.cross_type_compare`) | AC-QO-3 / AC-QO-8 strict-typed identity |
| L5        | IR shape                               | Tree-shaped typed instruction graph              | Natural fit for pull-based evaluation; serializes cleanly to binary form |
| L6        | Evaluation                             | Pull-based iterator chains                       | AC-QP-4 laziness; AC-QL-6 stack-flat pipelines; integrates with host async streams |
| L6        | Set operators                          | Streamed `|`; bounded right-buffer for `& - ^`   | AC-QO-4 streaming requirement; bound owned by scope policy |
| L6        | Recursion / call budget                | Per-evaluation counters charged against scope policy | AC-QR-1 / AC-QR-2 |
| Artifact  | Binary form                            | Tree-shaped IR serialized with policy stamps     | Participates in cem-ml-ac.md §14 cache; AC-QC-1..AC-QC-7 |

---

## 17. Performance Budgets And Verification

`cem-ql` adopts the host's AC-N-* perf model
(`cem-ml-stack-design.md §17`). Verification entry points:

- `yarn nx run cem_ql:test` — unit tests for parser, type checker,
  evaluator, and stdlib (AC verification plan §13.1).
- `yarn nx run cem_ql:test:xpath-parity` — XPath 3.1 conformance subset
  per AC-QX-1 / §4.1 (AC verification plan §13.2).
- `yarn nx run cem_ql:test:fixtures` — every Tier A query the CEM
  templates need to transform canonical fixtures; output snapshots match
  the host transform snapshots (AC verification plan §13.3).
- `yarn nx run cem_ql:bench` — selector benchmarks sharing the host's
  `cem_ml:bench` budget per AC-QR-5. Selector + transform end-to-end
  stays under the host's 150 ms Tier A budget when run together.

The cem-ql benchmark suite is built on the same
`cem_ml::benchmark::BenchmarkBudget` infrastructure as the host (no
parallel implementation). Tolerance and skip env vars
(`CEM_ML_PERF_TOLERANCE`, `CEM_ML_PERF_SKIP`) cover cem-ql automatically;
a future cem-ql-specific budget MAY be added as a sibling constructor
once Tier A is implemented and measured.

---

## 18. Compatibility And Distribution

cem-ql ships as part of the same crate boundary as `cem-ml`:

- **Crate boundary.** cem-ql lives in `packages/cem_ql` (Rust crate
  `cem-ql`). It is publishable independently but is normally consumed by
  `cem-ml` via path dependency in the workspace and version dependency
  in published releases.
- **Browser / Node parity.** cem-ql compiles to `wasm32-unknown-unknown`
  through the same build path as cem-ml. The public API is identical
  across browser and Node per host AC-C-1.
- **Crate surface.** Public modules `lexer`, `parser`, `resolve`, `types`,
  `ir`, `eval`, `stdlib`, `artifact`, `diagnostics`, `api` form the
  semver contract. Breaking changes in any of those module paths are
  semver-major events.
- **CLI surface.** No separate `cem-ql` binary at Tier A. The
  `cem-ml-cli` `query` sub-command (Tier B) is the user-facing entry
  point.
- **No JavaScript wrapper.** Per host AC-C-3, future JS wrappers
  consume the Rust-owned contract through the WASM artifact. The
  deprecated TypeScript implementation MUST NOT be reintroduced.

Release checks for cem-ql follow the host pattern
(`cem-ml-stack-design.md §18.4`): lint, test, WASM build, bench, browser/
Node smoke, and manifest checks. cem-ql adds:

- **XPath parity check** (`cem_ql:test:xpath-parity`) as a release gate
  for Tier A and later.
- **Fixture snapshot check** (`cem_ql:test:fixtures`) as a release gate;
  snapshot drift forces an explicit AC-traceable change.

---

## 19. Open Ambiguities

No open ambiguity entries. AC items pending implementation are tracked in
§21 (Appendix: AC Alignment Follow-Up).

---

## 20. Critical Review Questions And Concerns

This section will collect unresolved issues found by reviewing this
design against `cem-ql-ac.md` and the host stack documents. None
currently. Reviewers SHOULD add items here before opening implementation
PRs that would change the contracts above.

---

## 21. Appendix: AC Alignment Follow-Up

The table below maps each AC section in `cem-ql-ac.md` to its design
home in this document. AC items missing a check are not closeable
(AC-QV-V-* verification plan, §13 in the AC document).

| AC section | Topic | Design home |
|------------|-------|-------------|
| §0 (AC-Q-1..AC-Q-7) | Cross-cutting | §2 Domain Context, §3 Pipeline Overview |
| §1 (AC-QL-1..AC-QL-6) | Language model | §2, §10 Evaluator |
| §2 (AC-QS-1..AC-QS-6) | Surface syntax | §5 Lexer, §6 Parser |
| §3 (AC-QD-1..AC-QD-7) | DOM access | §10 Evaluator, §11 Stdlib (`cem:stdlib/dom`) |
| §4 (AC-QX-0..AC-QX-6, §4.1) | XPath functional parity | §16 Algorithm Selection (rows for axes, comparisons), §11 Stdlib, §17 Verification (xpath-parity script) |
| §5 (AC-QO-1..AC-QO-8) | Stream/set operations | §9 IR (`Union`/`Intersect`/`Difference`/`SymmetricDifference`), §10 Evaluator (streaming set semantics) |
| §6 (AC-QP-1..AC-QP-5) | Pipeline composition | §6 Parser (Pratt for `.`), §10 Evaluator (pull-based iterators, short-circuit) |
| §7 (AC-QV-1..AC-QV-8) | Variables/functions/scope | §7 Name Resolver |
| §8 (AC-QT-1..AC-QT-6) | Type system | §8 Type Checker |
| §9 (AC-QA-1..AC-QA-5, §AC-QA-1.1) | Async/external data | §11 Stdlib (`cem:stdlib/content-types` Tier B), §12 Cost Model |
| §10 (AC-QI-1..AC-QI-7) | Imports/modules/stdlib | §7 Name Resolver (reserved scheme), §11 Stdlib |
| §11 (AC-QE-1..AC-QE-4) | Errors/diagnostics | §4 Source-Map Model, §6 Parser (recovery), §10 Evaluator (scope violation, bubble-to-boundary) |
| §12 (AC-QR-1..AC-QR-5) | Resource limits | §3.2 Resource Limit Policy, §12 Cost Model, §17 Performance Budgets |
| §13 (verification plan) | Verification | §17 Performance Budgets And Verification |
| §14 (AC-QC-1..AC-QC-7, AC-QC-V-1..V-2) | Compiled artifact / cache protocol | §13 Compiled Artifact Layout |
| §15 (open questions) | Open questions | §19 Open Ambiguities (none) |

*End of design document. Implementation contracts (concrete Rust shapes,
parser tables, evaluator IR, stdlib function tables) live in
[`cem-ql-stack-design-impl.md`](cem-ql-stack-design-impl.md).*