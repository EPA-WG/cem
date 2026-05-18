# CEM-ML Editor Grammar Assets

This directory ships the editor-side grammar artifacts that mirror the
Rust tokenizer in [`../src/tokenizer/cem.rs`](../src/tokenizer/cem.rs).

| File                          | Purpose                                                                              |
| ----------------------------- | ------------------------------------------------------------------------------------ |
| `lexical.ebnf`                | Machine-readable lexical grammar (Tier A). Source of truth for editor grammars.      |
| `cem-ml.tmLanguage.json`      | TextMate / VS Code syntax-highlighting grammar.                                      |
| `tree-sitter-cem/grammar.js`  | Tree-sitter skeleton grammar for editor parse trees.                                 |

## Synchronization

The Rust tokenizer is the canonical implementation. Two synchronization
gates apply:

1. **Token-kind cross-reference.**
   `lexical.ebnf` ends with a comment block listing every
   `SchemaTokenKind` variant. The Rust unit test
   `tokenizer::cem::tests::grammar_token_kinds_match_lexical_grammar`
   fails if a token kind is added in Rust but not listed in the EBNF.
2. **Tree-sitter parity (follow-up).** A round-trip test that parses
   every `examples/cem-ml/*.cem` fixture in both engines and compares
   the node graphs lands alongside the parser-enabled milestone in
   `cem-ml-cli-plan.md` Phase 11.

## Tree-Sitter Build Instructions

The grammar skeleton is intentionally minimal; building the actual
parser requires the tree-sitter CLI:

```bash
cd packages/cem_ml/grammar/tree-sitter-cem
npx tree-sitter generate   # writes parser.c
npx tree-sitter test       # runs the corpus tests (none today)
```

Tier A does not bundle the generated parser or scanner; we ship only
the grammar source so editor extensions can build it themselves.

## TextMate Coverage

The TextMate grammar covers:

- Nodes (`{name ...}`) with element-name highlighting.
- Attributes (`@name=value`), namespaced (`@cem:action=primary`) and
  unprefixed.
- Content boundary (`|` / `▷`).
- Expression nodes (`{$ ...}`) — body delegated to the `source.cem-ql`
  scope so consumers can layer the cem-ql grammar on top.
- Rich-content enclosures (triple backticks).
- Line and block comments.
- Directives (`@doc`, `@ns`, `@default`, `@schema`).
- Quoted strings and cem-ql AVT spans in attribute values.

## Adding New Token Kinds

When adding a `SchemaTokenKind` variant:

1. Update `lexical.ebnf`'s production set and its token-kind cross
   reference block.
2. Update `tree-sitter-cem/grammar.js`.
3. Update `cem-ml.tmLanguage.json` if the surface form is visually
   distinct.
4. Re-run `cargo test -p cem-ml grammar_token_kinds_match_lexical_grammar`.
