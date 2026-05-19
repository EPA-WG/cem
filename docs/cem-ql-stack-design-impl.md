# `cem-ql` Implementation Design

**Status:** Draft implementation contracts derived from
[`cem-ql-ac.md`](cem-ql-ac.md) and the high-level design in
[`cem-ql-stack-design.md`](cem-ql-stack-design.md). High-level rationale,
algorithm selection, and tier scoping live in the stack design; this
document holds the Rust module map, concrete data shapes, parser
synchronization tables, evaluator IR layout, stdlib function table,
diagnostic code table, and binary artifact byte layout.
**Primary acceptance criteria:** [`cem-ql-ac.md`](cem-ql-ac.md).
**Date:** 2026-05-19

---

## 1. Purpose And Boundary

This document specifies the implementation contracts for the `cem-ql`
crate. It is downstream of the stack design: where the stack design
says "lower the surface AST to a typed IR," this document names the
Rust enum variants, field types, and method signatures. Where the stack
design says "the evaluator is pull-based," this document names the
trait object and iterator adaptor.

When this document conflicts with the stack design, the stack design
wins until corrected. When either conflicts with the AC, the AC wins.

This document does **not** redefine host contracts. Host types
(`AstNodeId`, `ExpandedName`, `SchemaFrame`, `SourceMapStack`,
`TransformKind`, `Diagnostic`, `BufferingObserver`) are imported from
the `cem-ml` crate via the public paths fixed at
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md) §3.

---

## 2. Shared Source-Map And Diagnostic Contracts

cem-ql reuses the host source-map and diagnostic contracts unchanged.
The cem-ql layer adds:

- **`TransformKind::Query`** — appended to every produced item that is
  not a host AST node (record literal, sequence construction, computed
  atom). The frame's `FrameSpan` is the producing expression's byte
  range in the query source.
- **`TransformKind::QueryStep`** — appended to each pipeline step
  result. The frame's `FrameSpan` is the step's byte range
  (`.step_name(args)` substring).

Both variants extend the host enum in `cem-ml`; cem-ql does not define
its own source-map types.

cem-ql diagnostics use the codes in §8 below and route through
`cem_ml::report::Report` exactly like host diagnostics (AC-QE-1).
There is no parallel report tree.

---

## 3. Rust Module Map

The cem-ql crate is `cem-ql` in `packages/cem_ql` (Cargo manifest path
`packages/cem_ql/Cargo.toml`). The crate is `["rlib", "cdylib"]` so the
same source produces both the native rlib for downstream Rust callers
and the cdylib used by the WASM build (host AC-C-2 parity).

```
packages/cem_ql/
  Cargo.toml
  project.json
  src/
    lib.rs               — public re-exports, layered-contract import test
    lexer.rs             — L1: Token, TokenKind, Lexer
    parser.rs            — L2: Parser, ParseError, recovery
    parser/
      pratt.rs           — Pratt expression parser
      module.rs          — module/declare/import statements
    resolve.rs           — L3: NameResolver, BindingSet, BindingId
    resolve/
      overlay.rs         — stdlib overlay map and fingerprint
    types.rs             — L4: TypeChecker, Type lattice
    types/
      lattice.rs         — node/atom/compound/resource types
      subtype.rs         — structural subtype walk
    ir.rs                — L5: Ir node tree, IrId, lowering entry
    ir/
      lower.rs           — surface AST → IR lowering
      serialize.rs       — IR → binary artifact
      deserialize.rs     — binary artifact → IR
    eval.rs              — L6: Evaluator, Item, Stream
    eval/
      pipeline.rs        — pipeline-step iterator chains
      set_ops.rs         — union/intersect/difference/symmetric_difference
      types_runtime.rs   — runtime type checks (instance of / cast as)
    stdlib.rs            — module registry, public re-exports
    stdlib/
      sequence.rs        — cem:stdlib/sequence
      strings.rs         — cem:stdlib/strings
      numbers.rs         — cem:stdlib/numbers
      datetime.rs        — cem:stdlib/datetime
      dom.rs             — cem:stdlib/dom
      report.rs          — cem:stdlib/report
      state.rs           — cem:stdlib/state
      template.rs        — cem:stdlib/template
      cemml.rs           — cem:stdlib/cemml
      content_types.rs   — cem:stdlib/content-types (Tier B)
    artifact.rs          — compiled binary serialization (AC-QC-*)
    diagnostics.rs       — cem.ql.* code table
    api.rs               — public evaluate(), compile(), load(), parse()
  tests/
    parser_recovery.rs
    name_resolution.rs
    type_check.rs
    eval_axes.rs
    eval_set_ops.rs
    eval_pipeline.rs
    stdlib_sequence.rs
    artifact_roundtrip.rs
    xpath_parity.rs        — AC-QX-1 subset against XPath 3.1 conformance vectors
    fixtures_snapshot.rs   — Tier A query corpus against canonical fixtures
    perf_budgets.rs        — selector benchmarks shared with cem_ml:bench
```

Project metadata (`packages/cem_ql/project.json`) defines Nx targets
`build`, `test`, `lint`, `build:wasm`, `bench`, `test:xpath-parity`,
`test:fixtures` mirroring the host project layout.

### 3.1 Public API Surface (`cem_ql::api`)

```rust
/// Compile a cem-ql query module source string into a typed IR.
/// Tier A entry point. The returned `CompiledQuery` is the same shape
/// the binary artifact deserializes into.
pub fn compile(source: &str, context: &CompileContext) -> Result<CompiledQuery, CompileError>;

/// Evaluate a compiled query against a `QueryContextScope` and
/// produce a stream of items. Tier A; async surface per AC-Q-6.
pub fn evaluate(query: &CompiledQuery, ctx: &EvaluationContext) -> ItemStream;

/// Parse-only entry point for tooling (editors, formatters). Skips
/// L3..L6; produces the surface AST and any L1/L2 diagnostics.
pub fn parse(source: &str) -> ParseResult;

/// Load a compiled binary artifact by hash. Tier B (AC-QC-*).
pub fn load(hash: ContentHash, ctx: &LoadContext) -> Result<CompiledQuery, LoadError>;
```

`CompileContext` carries the active `SchemaFrame`, the resolved
overlay map, the host `Diagnostic` sink, and the source-map base. It
mirrors the host `EngineContext` shape and is constructed by the
template compiler or direct caller from host state.

`EvaluationContext` carries the active `QueryContextScope`
(`cem-ml-stack-design-impl.md §3.10`), the host scope policy, and a
`BufferingObserver` reference for diagnostic emission.

### 3.2 Layered-Contract Import Test

`packages/cem_ql/src/lib.rs::tests::layered_runtime_contract_types_are_importable`
mirrors the host test pattern: each public type named in §3.1 and the
public modules in §3 are imported and `_accept::<T>()`-ed so a release
build fails if a name is accidentally renamed or made private.

---

## 4. Layer 1 — Lexer (`cem_ql::lexer`)

### 4.1 Token Layout

```rust
pub struct Token {
    pub kind: TokenKind,
    pub range: ByteRange,                       // host primitive
    pub cooked: Option<CookedTokenPayload>,     // string value, numeric value, etc.
}

pub enum TokenKind {
    // Punctuation
    Dot, Comma, LParen, RParen, LBracket, RBracket, LBrace, RBrace,
    Pipe, Amp, Minus, Caret, Assign, Colon, ColonColon,

    // Comparison operators
    Eq, Ne, Lt, Le, Gt, Ge, EqOp, NeqOp,          // `eq`, `ne`, `lt`, `le`, `gt`, `ge`, `=`, `!=`

    // Arithmetic
    Plus, Star, DivKw, ModKw,                     // `+`, `*`, `div`, `mod`

    // Boolean
    AndKw, OrKw, NotKw,                           // `and`, `or`, `not`

    // Reserved (parse error per AC-QO-5)
    AmpAmpReserved, PipePipeReserved,

    // Path tokens
    Slash, DotDot,

    // Keywords (Tier A)
    Let, In, If, Then, Else, For, ReturnKw,
    Some, Every, Satisfies,
    Import, As, Declare, Variable, Function, Module,
    InstanceKw, OfKw, CastKw, TreatKw, IsKw,
    FnKw,

    // Identifiers and literals
    Ident,                                        // payload: name
    PrefixedName,                                 // payload: (prefix, local)
    StringLit, IntLit, DecimalLit, DoubleLit,
    BoolLit, NullLit,

    // Trivia (consumed by parser but kept for source maps)
    Whitespace, LineComment, BlockComment,

    // Lexer error
    Invalid,
}

pub enum CookedTokenPayload {
    Name(String),
    PrefixedName { prefix: String, local: String },
    StringValue(String),
    IntValue(i64),
    DecimalValue(String),    // string form; canonical-decimal parsed at L4
    DoubleValue(f64),
    BoolValue(bool),
}
```

### 4.2 Lexer Algorithm

Hand-written DFA-style scanner. State variables: current byte index,
current codepoint, current token-start index. Behaviour per character
class:

- ASCII whitespace (`U+0020`, `U+0009`, `U+000A`, `U+000D`) → `Whitespace`.
- `;` followed by `;` → `LineComment` to end-of-line.
- `(` followed by `*` → `BlockComment` to matching `*)`.
- `"` → `StringLit`; escape sequences `\n` `\t` `\r` `\\` `\"` `\u{HEX}`.
- ASCII alpha/`_` → identifier or keyword (longest-match against keyword
  table). A `:` immediately following an identifier and followed by
  another identifier produces `PrefixedName`.
- `0-9` → numeric literal; the scanner decides between `IntLit`,
  `DecimalLit`, and `DoubleLit` by character class (`.` switches to
  decimal; `e`/`E` switches to double).
- Punctuation per `TokenKind` table above.
- `&` followed by `&` → `AmpAmpReserved`; `|` followed by `|` →
  `PipePipeReserved`. Both emit `cem.ql.use_and_or` at parse time.

Lexer errors emit `cem.ql.parse_error` with the byte range of the
offending span and continue scanning at the next plausible token start.

---

## 5. Layer 2 — Parser (`cem_ql::parser`)

### 5.1 Surface AST

```rust
pub enum SurfaceNode {
    Module(ModuleDecl),
    Import(ImportDecl),
    DeclareVariable(VariableDecl),
    DeclareFunction(FunctionDecl),
    Expression(Expression),
}

pub struct ModuleDecl { pub uri: String, pub range: ByteRange }

pub struct ImportDecl {
    pub uri: String,
    pub alias: Option<String>,
    pub range: ByteRange,
}

pub struct VariableDecl {
    pub name: QName,
    pub value: Expression,
    pub range: ByteRange,
}

pub struct FunctionDecl {
    pub name: QName,
    pub params: Vec<FunctionParam>,
    pub body: Expression,
    pub range: ByteRange,
}

pub struct FunctionParam {
    pub name: QName,
    pub type_annotation: Option<TypeExpr>,
}

pub enum Expression {
    Literal(LiteralValue, ByteRange),
    Name(QName, ByteRange),
    LeadingDot(ByteRange),                            // valid only inside pipeline step body
    Path { steps: Vec<PathStep>, range: ByteRange },  // `/`-separated axis steps
    Pipeline { source: Box<Expression>, steps: Vec<PipelineStep>, range: ByteRange },
    BinaryOp { op: BinaryOp, lhs: Box<Expression>, rhs: Box<Expression>, range: ByteRange },
    UnaryOp { op: UnaryOp, operand: Box<Expression>, range: ByteRange },
    SetOp { op: SetOp, lhs: Box<Expression>, rhs: Box<Expression>, range: ByteRange },
    If { cond: Box<Expression>, then_branch: Box<Expression>, else_branch: Box<Expression>, range: ByteRange },
    Let { name: QName, value: Box<Expression>, body: Box<Expression>, range: ByteRange },
    For { var: QName, source: Box<Expression>, body: Box<Expression>, range: ByteRange },
    Quantified { kind: QuantifierKind, var: QName, source: Box<Expression>, predicate: Box<Expression>, range: ByteRange },
    Record { entries: Vec<RecordEntry>, range: ByteRange },
    Sequence { items: Vec<Expression>, range: ByteRange },
    Lambda { params: Vec<FunctionParam>, body: Box<Expression>, range: ByteRange },
    Call { callee: Box<Expression>, args: Vec<Expression>, range: ByteRange },
    InstanceOf { value: Box<Expression>, ty: TypeExpr, range: ByteRange },
    CastAs { value: Box<Expression>, ty: TypeExpr, range: ByteRange },
    TreatAs { value: Box<Expression>, ty: TypeExpr, range: ByteRange },
    Is { lhs: Box<Expression>, rhs: Box<Expression>, range: ByteRange },
}

pub enum PathStep {
    Axis { axis: Axis, name_test: NameTest, predicates: Vec<Expression> },
    Parent(ByteRange),
    Self_(ByteRange),
}

pub enum PipelineStep {
    Named { name: QName, args: Vec<Expression>, range: ByteRange },
    Lambda { lambda: Expression, range: ByteRange },
}

pub enum SetOp { Union, Intersect, Difference, SymmetricDifference }
pub enum BinaryOp { Eq, Ne, Lt, Le, Gt, Ge, EqOp, NeqOp, Plus, Minus, Star, Div, Mod, And, Or }
pub enum UnaryOp { Negate, Not }
pub enum QuantifierKind { Some, Every }

pub struct QName { pub prefix: Option<String>, pub local: String }
pub struct NameTest { pub prefix: Option<String>, pub local: Option<String> }   // `*`, `prefix:*`, `*:local`
pub enum Axis { Self_, Child, Parent, Descendants, DescendantsOrSelf, Ancestors, AncestorsOrSelf, FollowingSibling, PrecedingSibling, Attributes }
```

### 5.2 Pratt Operator Precedence

From lowest to highest:

| Level | Operator(s) | Associativity |
|-------|-------------|---------------|
| 1     | `or`        | left          |
| 2     | `and`       | left          |
| 3     | `not` (unary) | prefix      |
| 4     | `eq` `ne` `lt` `le` `gt` `ge` `=` `!=` `is` | left |
| 5     | `|` `^`     | left          |
| 6     | `&`         | left          |
| 7     | `+` `-` (binary) | left     |
| 8     | `*` `div` `mod` | left      |
| 9     | unary `-`   | prefix        |
| 10    | `instance of` `cast as` `treat as` | left |
| 11    | `.` (pipeline / step) | left |
| 12    | `/` (path step) | left      |
| 13    | call `(...)`, index `[...]`, member access | left |

Reserved `&&` and `||` parse to a stub that emits
`cem.ql.use_and_or` and is then dropped from the surface AST so the
type checker does not see them.

### 5.3 Recovery Synchronization

The parser's `synchronize()` helper consumes tokens until it finds one
of the following anchors:

1. Top-level keyword (`module`, `import`, `declare`).
2. Statement terminator (newline followed by a top-level keyword;
   cem-ql has no `;` statement terminator at Tier A).
3. Closing bracket matching the innermost unclosed group (`)`, `]`,
   `}`).
4. Pipeline step boundary (a `.` followed by an identifier at the same
   precedence level).

Each recovery point emits `cem.ql.parse_error` with the byte range of
the consumed tokens.

---

## 6. Layer 3 — Name Resolver (`cem_ql::resolve`)

### 6.1 Binding Set

```rust
pub struct BindingSet {
    pub scope_id: ScopeId,                                  // host ScopeId
    pub variables: HashMap<QName, BindingId>,
    pub functions: HashMap<(QName, Arity), BindingId>,
    pub types: HashMap<QName, SchemaTypeId>,                // schema-derived
    pub namespaces: NsContext,                              // host primitive
    pub templates: HashMap<QName, TemplateRefId>,           // host primitive
    pub state_slots: HashMap<String, StateSlotId>,          // host primitive
    pub overlay: StdlibOverlay,
}

pub struct StdlibOverlay {
    pub map: HashMap<(ModuleUri, QName), BindingId>,
    pub fingerprint: OverlayFingerprint,                    // hashed for AC-QC-3 stamps
}
```

`BindingId` is an opaque index into a per-compile `BindingTable`. Each
binding entry records:

- the declaring source-map stack,
- the binding kind (`Variable`, `Function`, `StdlibFunction`,
  `OverlayBinding`, `SchemaType`, `TemplateRef`, `StateSlot`),
- the resolved value (function body, schema-type id, overlay
  implementation, etc.).

### 6.2 Resolution Algorithm

```rust
fn resolve(name: &QName, sites: &[BindingSet]) -> Resolution {
    // sites is innermost-first: local lexical, then query-module, then
    // host scopes innermost-first, then platform stdlib defaults.
    for site in sites {
        if let Some(b) = site.lookup(name) {
            emit_resolution_trace(name, &b, site);   // AC-QV-8
            return Resolution::Resolved(b);
        }
    }
    emit_diagnostic(unknown_code(name), name);
    Resolution::Unknown
}
```

`BindingSet::lookup` checks variables, functions, types, templates,
state-slots, and overlay in that order. The first hit wins. Overlay
entries take precedence over platform stdlib defaults at the same site
because overlays are stored on the same `BindingSet`; outer scopes'
overlays apply only if no inner-scope binding matches.

### 6.3 Reserved-Scheme Enforcement

`ImportDecl::uri` is checked against the reserved-scheme set
(`cem:`, `urn:cem:`) before resolution:

- `cem:` URIs always resolve to platform stdlib; scope-policy grants
  that list `cem:` in their grant source fail at policy load with
  `cem.ql.reserved_scheme`.
- `urn:cem:` URIs resolve via the host plugin registry (Tier B). Lack
  of a registration emits `cem.ql.import_unresolved`. A scope policy
  that lists `urn:cem:` in its grant source fails at policy load with
  `cem.ql.reserved_scheme`.
- Other schemes (`https:`, `http:`, `file:`, plugin-registered
  transports) require an active scope-policy grant per AC-QI-4; an
  unwhitelisted URI emits `cem.ql.import_denied`.

---

## 7. Layer 4 — Type Checker (`cem_ql::types`)

### 7.1 Type Lattice

```rust
pub enum Type {
    Node(NodeKind),                                 // node, element(QName), attribute(QName), text(), comment(), pi(), document-node()
    SchemaElement(SchemaTypeId),                    // scope-relative, AC-QT-4
    Atom(AtomType),
    Record(Vec<(String, Type)>),
    Array(Box<Type>),
    Stream(Box<Type>),
    Lambda { params: Vec<Type>, ret: Box<Type> },
    Resource { content_type: ContentType, schema: Option<SchemaTypeId> },
    Any,                                            // top — used only for unresolved bindings during error recovery
    Empty,                                          // bottom — used for empty-stream literals
}

pub enum AtomType { String, Integer, Decimal, Double, Boolean, Date, DateTime, Duration, AnyUri }
pub enum NodeKind { Node, Element(QName), Attribute(QName), Text, Comment, ProcessingInstruction, DocumentNode }
```

### 7.2 Bidirectional Inference

```rust
trait TypeCheck {
    fn infer(&self, ctx: &mut TyCtx) -> Type;
    fn check(&self, expected: &Type, ctx: &mut TyCtx) -> bool;
}
```

- `infer` for elimination forms (variable references, function calls,
  pipeline steps, axis steps).
- `check` for introduction forms (literals, record/array/sequence
  construction, lambda bodies against an expected `Lambda` type).

Subtype check (`Type::is_subtype_of`) walks structurally:

- `Node` subtypes its more-specific `NodeKind` variants.
- `SchemaElement(a)` subtypes `SchemaElement(b)` iff schema IR records
  a structural-subtype relation. `SchemaElement(_)` subtypes
  `Node(NodeKind::Element(_))` when the schema element name matches.
- `Atom`, `Record`, `Array`, `Stream`, `Lambda`, `Resource` follow
  structural-equality subtyping with the strict-typed-identity rule
  from AC-QO-3 (no implicit promotion across atom types).
- `Empty` subtypes every type; `Any` is a supertype of every type and
  appears only during error recovery.

### 7.3 Cross-Type Comparison Warning

`Expression::BinaryOp` with comparison op and statically known
operand types triggers AC-QO-8:

- Same atom type → check valid.
- Different atom types → emit `cem.ql.cross_type_compare` at warning
  severity; the resulting expression has type `Atom(Boolean)` and
  evaluates to `false` at runtime.
- One side is `Stream` / `Array` / `Record` and the other is a
  scalar atom → same warning.

The warning is silenced under the dev/debug CLI profile (§7.4).

### 7.4 Strict-Default Failure Profile

The default `TyConfig` stamps the following codes at `error` severity:

- `cem.ql.type_error`
- `cem.ql.unknown_type`
- `cem.ql.unknown_function`
- `cem.ql.unknown_variable`

A `TyConfig::dev_profile()` constructor remaps the same codes to
`warning` for the opt-in dev/debug profile. Selecting the dev profile
is the host CLI's responsibility (e.g. `cem-ml-cli ... --profile=dev`);
cem-ql does not infer it from the environment.

---

## 8. Diagnostic Code Table

Every cem-ql code MUST appear in this table. New codes added during
implementation MUST update both this table and AC-QE-1.

| Code | Default severity | Emitting layer | Notes |
|------|------------------|----------------|-------|
| `cem.ql.parse_error` | error | L1 / L2 | Lexer or parser failed; range = offending tokens. |
| `cem.ql.use_and_or` | error | L1 | `&&` / `||` reserved; suggest `and`/`or`. |
| `cem.ql.type_error` | error | L4 / L6 | Static failure prevents IR emission; runtime failure aborts evaluation. |
| `cem.ql.unknown_type` | error | L4 | Type name not in active schema. |
| `cem.ql.unknown_function` | error | L3 | Function name not in resolution chain. |
| `cem.ql.unknown_variable` | error | L3 | Variable name not in resolution chain. |
| `cem.ql.scope_violation` | error | L6 | Access outside `QueryContextScope`. |
| `cem.ql.unresolved_reference` | warning | L6 | Reference slot (id/for/aria-*) unresolved; scope policy may raise to error. |
| `cem.ql.cross_type_compare` | warning | L4 | Cross-atom-type comparison; silenced under dev profile. |
| `cem.ql.import_denied` | warning | L3 | Scope policy denied network-scheme import. |
| `cem.ql.import_unresolved` | error | L3 | `urn:cem:` URI not registered. |
| `cem.ql.reserved_scheme` | error | policy load | Scope policy attempted to grant `cem:` / `urn:cem:`. |
| `cem.ql.read_denied` | error | L6 | `read()` URI denied by scope policy. |
| `cem.ql.read_unsatisfiable` | error | L6 | `read()` wire content type has no transform to any resolved `accepts` entry. |
| `cem.ql.read_dynamic_accepts` | warning | L4 | `read()` `accepts` argument dynamic; binary stamps as wildcard. |
| `cem.ql.aborted` | info | L6 | Evaluation aborted via `AbortSignal`. |
| `cem.ql.budget_exceeded` | error | L6 | Scope-policy budget breached; carries limit name. |
| `cem.ql.closure_detached` | info | L5 / L6 | Closure capture detached host-AST refs; information lost only if a captured reference is no longer reachable. |
| `cem.ql.policy_accessor_failed` | error | L6 | Policy-supplied `resource` accessor returned an error. |

---

## 9. Layer 5 — IR Lowerer (`cem_ql::ir`)

### 9.1 IR Node

```rust
pub struct IrTree {
    pub nodes: Vec<IrNode>,                 // index = IrId
    pub root: IrId,
    pub source_maps: Vec<SourceMapStack>,   // parallel to nodes
    pub types: Vec<Type>,                   // parallel to nodes
}

pub type IrId = u32;

pub enum IrNode {
    // Literals
    LitString(String),
    LitInt(i64),
    LitDecimal(String),
    LitDouble(f64),
    LitBool(bool),
    LitNull,

    // References
    LocalVar(BindingId),
    FunctionRef(BindingId),
    SchemaType(SchemaTypeId),
    TemplateRef(TemplateRefId),
    StateSlot(StateSlotId),

    // Constructors
    Record(Vec<(String, IrId)>),                  // keys are quoted-string literals
    Array(Vec<IrId>),
    Sequence(Vec<IrId>),
    Lambda { params: Vec<(BindingId, Type)>, body: IrId, captures: Vec<BindingId> },

    // Path / axes
    AxisStep { axis: Axis, name_test: NameTest, predicates: Vec<IrId> },
    Parent,
    Self_,
    Reference,                                    // .target on a node carrying id/for/aria-* slot

    // Pipeline
    Pipeline { source: IrId, steps: Vec<IrStep> },
    LeadingDot,                                   // current-item placeholder inside a pipeline step body

    // Calls
    Call { callee: IrId, args: Vec<IrId> },
    StdlibCall { module: ModuleUri, name: QName, args: Vec<IrId> },

    // Operators
    BinaryOp { op: BinaryOp, lhs: IrId, rhs: IrId },
    UnaryOp { op: UnaryOp, operand: IrId },
    SetOp { op: SetOp, lhs: IrId, rhs: IrId },

    // Control flow
    If { cond: IrId, then_branch: IrId, else_branch: IrId },
    Let { name: BindingId, value: IrId, body: IrId },
    For { var: BindingId, source: IrId, body: IrId },
    Quantified { kind: QuantifierKind, var: BindingId, source: IrId, predicate: IrId },

    // Type forms
    InstanceOf { value: IrId, ty: Type },
    CastAs { value: IrId, ty: Type },
    TreatAs { value: IrId, ty: Type },
    Is { lhs: IrId, rhs: IrId },
}

pub enum IrStep {
    Named { name: QName, args: Vec<IrId> },
    NamedStdlib { module: ModuleUri, name: QName, args: Vec<IrId> },
    Lambda(IrId),
}
```

### 9.2 Lowering Rules

- `Expression::Path { steps }` lowers to a chain of `AxisStep` nodes
  threaded through `Pipeline { source: <first step>, steps: <rest> }`.
- `Expression::Pipeline { source, steps }` lowers source to an IR
  expression and each step to an `IrStep`. Named steps whose resolution
  points to a stdlib function become `NamedStdlib`; user-defined steps
  become `Named` with the resolved `BindingId` carried alongside (in a
  parallel `IrTree::resolutions` table not shown above).
- `Expression::SetOp` lowers to `SetOp { op, lhs, rhs }`. The evaluator
  specializes by `op` so streaming behaviour matches AC-QO-4.
- `Expression::Lambda` lowers the body in a child binding scope and
  captures the lexical environment. AC-QV-6 closure-detachment runs at
  lowering: any captured `BindingId` whose binding holds a host AST
  reference and whose lifetime may exceed the host scope is rewritten
  into a copy of the captured value; a `cem.ql.closure_detached`
  diagnostic is emitted if information is lost (e.g. a captured
  attribute occurrence collapses to its `(name, value)` snapshot).
- `Expression::Call` to a stdlib function lowers to `StdlibCall`; calls
  to user-declared functions lower to `Call` with a `FunctionRef`
  callee.

The lowering pass produces no host-AST mutations and no I/O.

### 9.3 Source-Map Frames

Every `IrNode` carries a `SourceMapStack` in `IrTree::source_maps[id]`.
The lowerer constructs the stack as:

```
[host frames inherited from the template embedding, if any]
+ TransformKind::Query { range: expression byte range }
```

Pipeline steps add a `TransformKind::QueryStep { range: step byte range }`
frame.

---

## 10. Layer 6 — Evaluator (`cem_ql::eval`)

### 10.1 Item And Stream Shapes

```rust
pub enum Item {
    Node(AstNodeId, SourceMapStack),
    Attribute(AstNodeId, ExpandedName, SourceMapStack),
    Text(String, SourceMapStack),
    Atom(AtomValue, SourceMapStack),
    Record(BTreeMap<String, ItemStream>, SourceMapStack),
    Array(Vec<Item>, SourceMapStack),
    Lambda(LambdaValue, SourceMapStack),
    StateSlot(StateSlotId, SourceMapStack),
    Resource(ResourceHandle, SourceMapStack),
}

pub enum AtomValue {
    String(String),
    Integer(i64),
    Decimal(String),     // canonical lexical form per AC-QO-3
    Double(f64),         // NaN normalized to canonical NaN per AC-QO-3
    Boolean(bool),
    Date(DateValue),
    DateTime(DateTimeValue),
    Duration(DurationValue),
    AnyUri(String),
    Null,
}

pub trait ItemStream {
    fn next_item(&mut self) -> Poll<Option<Result<Item, EvalError>>>;
}
```

`ItemStream` is a pull-based async iterator. Tier A in-memory queries
resolve synchronously through the Poll surface (no actual `await`); the
trait is async-shaped per AC-Q-6 so Tier B `read()` callers fit without
an API split.

### 10.2 Pipeline Evaluation

```rust
fn eval_pipeline(source: IrId, steps: &[IrStep], ctx: &mut EvalCtx) -> Box<dyn ItemStream> {
    let mut stream = eval(source, ctx);
    for step in steps {
        stream = apply_step(stream, step, ctx);
    }
    stream
}
```

Each `apply_step` adapter returns a new stream that pulls from its
upstream one item at a time. The adapters are:

- `Named { name, args }` — resolve `name`, invoke the function with the
  pulled item as the implicit first argument and `args` as the rest.
- `NamedStdlib { module, name, args }` — direct stdlib dispatch.
- `Lambda(ir)` — invoke the lambda with the pulled item as the only
  argument.

Short-circuit forms wrap the upstream with adapters that stop pulling
once their answer is decided (`.first`, `.exists`, `.empty`, `if`).

### 10.3 Set-Operator Evaluation

| Op | Algorithm |
|----|-----------|
| `Union` | Stream LHS and RHS in document order; emit each item once. Identity per AC-QO-3 backs the dedup set. The dedup set is bounded by the scope policy's `max items per pipeline stage`; an overflow emits `cem.ql.budget_exceeded`. |
| `Intersect` | Materialize the RHS up to the scope-policy bound; stream LHS and emit items present in the RHS set. |
| `Difference` | Materialize the RHS up to the scope-policy bound; stream LHS and emit items absent from the RHS set. |
| `SymmetricDifference` | Materialize the RHS; stream LHS and emit items absent from the RHS, then drain remaining RHS items absent from the (also-materialized) LHS dedup set. |

For node operands, document order is preserved by relying on the host
event-emit order. For atom operands, order is "LHS items in their
source order, then new RHS items in their source order" per AC-QO-2.

### 10.4 Budget Charging

Every adapter increments a counter on the active `EvalCtx`:

```rust
fn charge(ctx: &mut EvalCtx, axis: BudgetAxis, amount: u64) -> Result<(), EvalError> {
    let new = ctx.counters[axis] + amount;
    if new > ctx.limits[axis] {
        emit_diagnostic("cem.ql.budget_exceeded", ctx, axis);
        return Err(EvalError::BudgetExceeded(axis));
    }
    ctx.counters[axis] = new;
    Ok(())
}
```

`BudgetAxis` is the AC-QR-1 set: `ItemsPerStage`, `CallDepth`,
`FunctionCalls`, `ClosureSize`, `RegexBacktrack` (Tier B),
`ExternalFetches` (Tier B). Limits are read from the active scope
policy on `EvalCtx` construction.

---

## 11. Stdlib Function Table

Each stdlib module is a Rust module under `cem_ql::stdlib::`. Functions
are registered in a static table at module init time:

```rust
pub struct StdlibFunction {
    pub module: ModuleUri,           // e.g. "cem:stdlib/sequence"
    pub name: QName,                 // e.g. "seq:map"
    pub signature: FunctionSignature,
    pub implementation: StdlibImpl,
    pub tier: Tier,
}

pub enum StdlibImpl {
    Native(fn(&mut EvalCtx, Vec<ItemStream>) -> Box<dyn ItemStream>),
    Macro(MacroLowering),       // for forms that need IR-level rewriting (e.g. short-circuit `.first`)
}
```

A condensed Tier A function inventory (the full table lives next to the
implementation as a regenerated reference; this section names the
public-API floor):

### 11.1 `cem:stdlib/sequence`

| Function | Signature | Tier |
|----------|-----------|------|
| `seq:map`         | `(stream<T>, fn(T) -> U) -> stream<U>` | A |
| `seq:where`       | `(stream<T>, fn(T) -> boolean) -> stream<T>` | A |
| `seq:flat_map`    | `(stream<T>, fn(T) -> stream<U>) -> stream<U>` | A |
| `seq:take`        | `(stream<T>, integer) -> stream<T>` | A |
| `seq:drop`        | `(stream<T>, integer) -> stream<T>` | A |
| `seq:first`       | `(stream<T>) -> stream<T>` | A |
| `seq:last`        | `(stream<T>) -> stream<T>` | A |
| `seq:nth`         | `(stream<T>, integer) -> stream<T>` | A |
| `seq:peek`        | `(stream<T>, fn(T) -> ()) -> stream<T>` | A |
| `seq:union`       | `(stream<T>, stream<T>) -> stream<T>` | A — function alias for `|` |
| `seq:intersect`   | `(stream<T>, stream<T>) -> stream<T>` | A — alias for `&` |
| `seq:difference`  | `(stream<T>, stream<T>) -> stream<T>` | A — alias for `-` |
| `seq:symmetric_difference` | `(stream<T>, stream<T>) -> stream<T>` | A — alias for `^` |
| AC-QO-6 family    | `group_by`, `count_by`, `partition`, `zip`, `chunked`, `windowed`, `sliding`, `take_while`, `drop_while`, `sorted`, `reversed`, `reduce`, `fold`, `scan`, `any`, `all`, `none`, `min`, `max`, `sum`, `avg` | B |

### 11.2 `cem:stdlib/strings`

| Function | Signature | Tier |
|----------|-----------|------|
| `str:length`      | `(string) -> integer` | A |
| `str:codepoints`  | `(string) -> stream<integer>` | A |
| `str:lower`       | `(string) -> string` | A |
| `str:upper`       | `(string) -> string` | A |
| `str:slice`       | `(string, integer, integer?) -> string` | A |
| `str:concat`      | `(stream<string>, string?) -> string` | A |
| `str:contains` `str:starts_with` `str:ends_with` | `(string, string) -> boolean` | A |
| `str:nfc` `str:nfd` `str:matches` `str:replace` `str:split` | regex / normalization | B |

### 11.3 `cem:stdlib/numbers`

| Function | Signature | Tier |
|----------|-----------|------|
| `num:double` `num:decimal` `num:integer` `num:string` | atomic conversions | A |
| `num:abs` `num:floor` `num:ceil` `num:round` | math | A |
| `num:format` | `(number, string) -> string` | A |

### 11.4 `cem:stdlib/datetime`

| Function | Signature | Tier |
|----------|-----------|------|
| `dt:to_utc`        | `(dateTime) -> dateTime` | A |
| `dt:components`    | `(dateTime) -> record(year, month, day, hour, minute, second, tz)` | A |
| `dt:format`        | `(dateTime, string) -> string` | A |

### 11.5 `cem:stdlib/dom`

| Function | Signature | Tier |
|----------|-----------|------|
| `dom:children`     | `(node) -> stream<node>` | A |
| `dom:descendants`  | `(node) -> stream<node>` | A |
| `dom:parent`       | `(node) -> stream<node>` | A |
| `dom:attribute`    | `(node, QName) -> stream<attribute>` | A |
| `dom:resolve_ref`  | `(node) -> stream<node>` | A — follows id/for/aria-* per AC-QD-4 |
| `dom:tainted`      | `(node) -> boolean` | A — AC-QD-6 |

### 11.6 `cem:stdlib/report`

| Function | Signature | Tier |
|----------|-----------|------|
| `report:emit`      | `(string, string, severity?) -> ()` | A — emits a diagnostic with the given code and message |
| `report:severity_floor` | `(severity) -> ()` | A — scope-local severity floor |

### 11.7 `cem:stdlib/state`

| Function | Signature | Tier |
|----------|-----------|------|
| `state:read`       | `(string) -> stream<atom>` | A — read-only access to a machine-state slot |
| `state:keys`       | `() -> stream<string>` | A |

### 11.8 `cem:stdlib/template`

| Function | Signature | Tier |
|----------|-----------|------|
| `tpl:lookup`       | `(QName) -> stream<template-ref>` | A |
| `tpl:names`        | `() -> stream<QName>` | A |

### 11.9 `cem:stdlib/cemml`

| Function | Signature | Tier |
|----------|-----------|------|
| `cemml:parse`      | `(string) -> stream<node>` | A — parse in-memory CEM-ML canonical source |
| `cemml:format`     | `(node) -> string` | A |

### 11.10 `cem:stdlib/content-types` (Tier B)

| Function | Signature | Tier |
|----------|-----------|------|
| `ct:html` `ct:xml` `ct:svg` `ct:mathml` `ct:css` `ct:scss` `ct:json` `ct:yaml` `ct:csv` `ct:js` `ct:ts` `ct:cemml` `ct:floor` | canonical identifiers | B |
| `ct:default_accepts` | `() -> array<string>` | B — the AC-QA-1.1 floor list |

---

## 12. Compiled Artifact (`cem_ql::artifact`)

### 12.1 Binary Layout

The compiled artifact is the shared `cem-bin/1+blake3` container from
`cem-ml-ac.md` §14, with `content-type = cem-ql/1`. Logical sections:

| Section | Bytes | Notes |
|---------|-------|-------|
| Header | fixed | `cem-bin/1` scheme tag + content-type + hash. |
| Module identity | varint-prefixed strings | declared `module` URI, source URI. |
| IR table | length-prefixed | `IrNode` array; opcode-prefixed records. |
| Source-map indices | length-prefixed | for dev binaries only; references sidecar by AC-CC-5 hash. |
| Type table | length-prefixed | `Type` array; schema-type entries are rebindable stubs (AC-QT-4). |
| Schema bindings | length-prefixed | resolved schema-type IDs, in declaration order, with the resolution scope's fingerprint. |
| Import closure | length-prefixed | each entry: URI + content hash. |
| Policy stamps | length-prefixed | declared imports, declared `read()` `accepts` lists (one per call site), declared external resources, resolved stdlib overlay fingerprint. |

Format details (varint encoding, opcode numbering, byte-stability rules)
follow the host §14 binary form contract; cem-ql does not invent new
encoding rules.

### 12.2 Serialization

```rust
pub fn serialize(query: &CompiledQuery, mode: ArtifactMode) -> Vec<u8>;
pub fn deserialize(bytes: &[u8], ctx: &LoadContext) -> Result<CompiledQuery, LoadError>;

pub enum ArtifactMode { Dev, Prod }
```

`Dev` mode preserves the full source-map sidecar reference (AC-CC-4).
`Prod` mode omits source-map indices to reduce size; runtime diagnostics
still carry the active scope's source-map frame but lose the query-side
origin.

### 12.3 Policy-Stamp Mismatch

`deserialize` re-resolves schema-type stubs against the loading scope's
schema bindings (AC-QT-4). It also re-fingerprints the loading scope's
stdlib overlay and compares it against the stamp:

- Match → load proceeds.
- Mismatch → emit `cem.cc.policy_mismatch` (host code reused) and fall
  back to source when available; if no source, the loader reports an
  unrecoverable load error.

Re-resolved schema-type IDs that fail to resolve emit
`cem.ql.unknown_type` exactly as on a fresh compile.

---

## 13. Tier Status

```
Tier A:
  - Layers L1..L6 complete for the surface forms in AC-QS-1..AC-QS-6
  - Axes in AC-QD-1 Tier A set
  - XPath 3.1 subset per AC-QX-1
  - Set operators with strict-typed identity (AC-QO-1..AC-QO-5)
  - Pipeline composition (AC-QP-*)
  - Full scope chain with overlay support (AC-QV-3..AC-QV-8)
  - Type lattice and bidirectional inference (AC-QT-1..AC-QT-4)
  - Stdlib Tier A modules (§11)
  - Diagnostic codes Tier A (§8)
  - Verification: cem_ql:test, cem_ql:test:xpath-parity,
    cem_ql:test:fixtures, cem_ql:bench

Tier B:
  - try/catch surface keyword (AC-QE-2)
  - FLWOR with where/order by (AC-QX-4)
  - Comprehension sugar (AC-QO-7)
  - AC-QO-6 collection helper family
  - Regex (AC-QX-1 / strings)
  - read(uri, accepts?) (AC-QA-1..AC-QA-5)
  - urn:cem: plugin-registered modules (AC-QI-2)
  - Scope-policy gated user modules (AC-QI-4)
  - Compiled artifact / cache participation (AC-QC-1..AC-QC-7)
  - Attribute-group record types (AC-QT-5)
  - TS/Rust type stubs (AC-QT-6)
  - cem:stdlib/content-types (§11.10)

Tier C:
  - Full XQuery 3.1 surface where it does not duplicate Tier A/B helpers
  - NVDL-style schema dispatch (AC-QX-* extensions)
  - XPath 4.0 candidate function library (AC-QX-5)
  - Binary AST consumption inside queries
  - Query-time hydration rule generation
  - Query-emitted DOM patch plans
```

---

## 14. Implementation Ownership Rules

1. **No host AST mutation.** Every cem-ql evaluator function MUST
   return a new `Item` rather than mutating its inputs. Mutation belongs
   to the host's Tier C DOM mutation API.
2. **No I/O at compile time.** L1–L5 are pure functions of source +
   schema fingerprint + overlay fingerprint. `read()` is an L6 form.
3. **No `eval` or dynamic-source compilation.** AC-QR-3 / AC-QC-6. The
   only path from string to executable IR is `cem_ql::api::compile`,
   which runs the full pipeline including type-check.
4. **All diagnostics route through `cem-ml::report`.** cem-ql owns the
   code table (§8) but never its own report tree.
5. **All resource limits inherited from the host scope policy.** cem-ql
   does not introduce a parallel budget knob.
6. **Public-module path stability.** Every name in §3 is part of the
   crate's semver contract; renames are semver-major events.

---

## 15. Appendix: AC Implementation Follow-Up

| AC item | Implementation home | Status |
|---------|--------------------|--------|
| AC-Q-1..AC-Q-7 | §2 (cross-cutting source-map / diagnostic / scope contract) | Designed; implementation pending. |
| AC-QL-1..AC-QL-6 | §10 (Item, ItemStream, lazy pipeline) | Designed. |
| AC-QS-1..AC-QS-6 | §4 (Lexer), §5 (Parser, surface AST) | Designed. |
| AC-QD-1..AC-QD-6 | §5 (AxisStep), §11.5 (dom stdlib) | Designed. AC-QD-7 deferred to Tier B (§13). |
| AC-QX-0..AC-QX-2, AC-QX-6 | §5 / §7 / §10 / §11 | Designed. AC-QX-3..AC-QX-5 Tier B/C (§13). |
| AC-QO-1..AC-QO-5, AC-QO-8 | §5 (SetOp), §10.3 (set-operator evaluation), §7.3 (cross-type compare) | Designed. AC-QO-6/AC-QO-7 Tier B (§13). |
| AC-QP-1..AC-QP-5 | §5 (Pipeline), §10.2 (pipeline evaluation), §10.4 (short-circuit) | Designed. |
| AC-QV-1..AC-QV-8 | §6 (Name Resolver, BindingSet, overlay) | Designed. |
| AC-QT-1..AC-QT-4 | §7 (Type lattice, bidirectional inference, strict-default profile) | Designed. AC-QT-5/AC-QT-6 Tier B (§13). |
| AC-QA-1..AC-QA-5 | §11.10 (content-types stdlib), §10 (`read()` adapter) | Tier B (§13). |
| AC-QI-1..AC-QI-3, AC-QI-5..AC-QI-6 | §6.3 (reserved-scheme), §11 (stdlib registry) | Designed. AC-QI-4/AC-QI-7 Tier B (§13). |
| AC-QE-1..AC-QE-4 | §8 (code table), §2 (routing), §5 (recovery), §10 (scope-violation) | Designed. |
| AC-QR-1..AC-QR-5 | §10.4 (budget charging), §17 (Performance) in stack design | Designed. |
| §13 verification plan | tests/ layout in §3, Nx targets in §3 | Targets owned by `project.json`; test files listed in §3. |
| AC-QC-1..AC-QC-7, AC-QC-V-* | §12 (Compiled Artifact) | Tier B (§13). |
| AC-QV-V-*, AC-QO-V-*, AC-QI-V-*, AC-QA-V-* | tests/ files (`name_resolution.rs`, `eval_set_ops.rs`, `parser_recovery.rs`, etc.) | Test scaffolding pending implementation. |

*End of implementation design. Concrete code shapes here MUST be
re-validated against the AC and the stack design before each
implementation phase begins. Resolved deviations should be folded back
into the relevant document so the AC, the stack design, and this file
stay in lockstep.*