// CEM-ML tree-sitter grammar (skeleton).
//
// Source-of-truth lexical grammar: ../lexical.ebnf
// Rust tokenizer mirror: ../../src/tokenizer/cem.rs
//
// Tier A scope: the production-level grammar that any editor parse
// implementation should treat as authoritative. The Tier A goal is
// editor highlighting + structural folding + best-effort error
// recovery, not byte-identical parity with the Rust tokenizer. A
// parity round-trip test (every `examples/cem-ml/*.cem` fixture parses
// in both engines into structurally-equivalent trees) is a follow-up
// once the tree-sitter scanner ships alongside the parser-enabled
// milestone (`cem-ml-cli-plan.md` Phase 11).

module.exports = grammar({
  name: 'cem',

  extras: $ => [
    /\s+/,
    $.line_comment,
    $.block_comment,
  ],

  rules: {
    document: $ => seq(
      repeat($._directive),
      repeat($._item),
    ),

    _item: $ => choice(
      $.node,
      $.expression_node,
      $.anonymous_scope,
      $._directive,
      $.rich_content,
      $.text,
    ),

    node: $ => seq(
      '{',
      field('name', $.qname),
      repeat($.attribute),
      optional($.content_boundary),
      repeat($._item),
      '}',
    ),

    expression_node: $ => seq(
      '{',
      '$',
      optional($.content_boundary),
      field('body', $.expression_body),
      '}',
    ),

    anonymous_scope: $ => seq(
      '{',
      repeat1($.attribute),
      optional($.content_boundary),
      repeat($._item),
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

    bare_value: $ => /[A-Za-z0-9_\-./]+/,
    quoted_string: $ => choice(
      seq('"', /[^"]*/, '"'),
      seq("'", /[^']*/, "'"),
    ),

    cem_ql_span: $ => seq(
      '{',
      // Tier A: opaque body, balanced braces; cem-ql parsing is a
      // separate grammar that lands with the cem-ql crate.
      repeat(choice(/[^{}]+/, $.cem_ql_span)),
      '}',
    ),

    expression_body: $ => repeat1(choice(/[^{}]+/, $.cem_ql_span)),

    content_boundary: $ => choice('|', '▷'),

    qname: $ => /[A-Za-z_][A-Za-z0-9_\-]*(:[A-Za-z_][A-Za-z0-9_\-]*)?/,

    _directive: $ => seq(
      '@',
      field('directive_name', /(doc|ns|default|schema)\b/),
      field('body', /[^\n]*/),
    ),

    line_comment: $ => /\/\/[^\n]*/,
    block_comment: $ => seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/'),

    rich_content: $ => seq(
      '```',
      repeat(/[^`]|`[^`]|``[^`]/),
      '```',
    ),

    text: $ => /[^{}`@]+/,
  },
});
