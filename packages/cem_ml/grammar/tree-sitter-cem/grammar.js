// CEM-ML tree-sitter grammar — Tier A.
//
// Source-of-truth lexical grammar: ../lexical.ebnf
// Rust tokenizer mirror: ../../src/tokenizer/cem.rs
// Document syntax reference: ../../../../docs/cem-ml-syntax.md
//
// Tier A goal: editor highlighting + structural folding + best-effort
// error recovery, and a parity round-trip against the Rust tokenizer
// for every canonical `examples/cem-ml/*.cem` fixture. The parity
// projection lives in `packages/cem_ml/tests/tree_sitter_parity.rs`.

module.exports = grammar({
  name: 'cem',

  extras: $ => [
    /[ \t\r\n]+/,
    $.line_comment,
    $.block_comment,
  ],

  word: $ => $._name_token,

  conflicts: $ => [],

  rules: {
    document: $ => seq(
      repeat($.directive),
      repeat($._item),
    ),

    // Document directive: `@name body...` to end-of-line. Tier A
    // canonical names are `doc`, `ns`, `default`, `schema`.
    directive: $ => seq(
      $._directive_head,
      field('body', optional($._directive_body)),
      optional(/\r?\n/),
    ),

    _directive_head: $ => token(seq('@', choice('doc', 'ns', 'default', 'schema'))),
    _directive_body: $ => token.immediate(/[^\r\n]+/),

    _item: $ => choice(
      $.expression_node,
      $.node,
      $.anonymous_scope,
      $.rich_content,
      $.text,
    ),

    // `{name @attr=value | content}` — the canonical CEM-ML scope.
    node: $ => seq(
      '{',
      field('name', $.qname),
      repeat($.attribute),
      optional($.content_boundary),
      repeat($._item),
      '}',
    ),

    // `{@attr=value | content}` — a scope whose schema is given by the
    // attributes (no element name). Disambiguated by lookahead: the
    // first non-whitespace character after `{` is `@`.
    anonymous_scope: $ => seq(
      '{',
      repeat1($.attribute),
      optional($.content_boundary),
      repeat($._item),
      '}',
    ),

    // `{$ ... }` — expression node. Body is delegated to a future
    // cem-ql parser; Tier A captures it as an opaque balanced span.
    expression_node: $ => seq(
      '{',
      '$',
      optional($.content_boundary),
      field('body', $.expression_body),
      '}',
    ),

    attribute: $ => seq(
      '@',
      field('name', $.qname),
      optional(seq(
        '=',
        field('value', $._attribute_value),
      )),
    ),

    _attribute_value: $ => choice(
      $.bare_value,
      $.quoted_string,
      $.cem_ql_span,
    ),

    bare_value: $ => /[A-Za-z0-9_\-.\/:]+/,

    quoted_string: $ => choice(
      seq('"', repeat(token.immediate(/[^"]+/)), token.immediate('"')),
      seq("'", repeat(token.immediate(/[^']+/)), token.immediate("'")),
    ),

    // cem-ql attribute span — opaque, balanced braces. The cem-ql
    // grammar lands with the cem-ql crate.
    cem_ql_span: $ => seq(
      '{',
      repeat(choice(/[^{}]+/, $.cem_ql_span)),
      '}',
    ),

    expression_body: $ => repeat1(choice(/[^{}]+/, $.cem_ql_span)),

    content_boundary: $ => choice('|', '▷'),

    // Qualified name: either `prefix:local` or `local` only.
    qname: $ => /[A-Za-z_][A-Za-z0-9_-]*(:[A-Za-z_][A-Za-z0-9_-]*)?/,

    line_comment: $ => token(seq('//', /[^\r\n]*/)),

    // C-style /* ... */ block comment, allowing `*` inside as long as
    // it isn't followed by `/`.
    block_comment: $ => token(seq(
      '/*',
      /[^*]*\*+([^/*][^*]*\*+)*/,
      '/',
    )),

    rich_content: $ => token(seq(
      '```',
      /([^`]|`[^`]|``[^`])*/,
      '```',
    )),

    // Free-form text content: anything that isn't a structural sigil.
    // The first character excludes `@`, `/`, and `` ` `` so attributes,
    // directives, comments, and rich-content enclosures win at the
    // start of an item position. Subsequent characters allow `@`, `/`,
    // and single `` ` `` so prose like `alex@example.test`,
    // `https://example/path`, or inline `` `code` `` markers parse as
    // one contiguous text run, matching the Rust tokenizer's behaviour
    // in `scan_content_text` (only `/*` interrupts a text run; `//`
    // and bare backticks stay in text).
    text: $ => token(prec(-1, /[^{}@`\/][^{}]*/)),

    // Internal token used as the `word` for keyword discrimination.
    _name_token: $ => /[A-Za-z_][A-Za-z0-9_-]*/,
  },
});
