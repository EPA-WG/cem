#include "tree_sitter/parser.h"
#include <stdint.h>
#include <wctype.h>

enum TokenType { EXPRESSION_BODY, REFERENCE_BODY, QUERY_SPAN_BODY };

void *tree_sitter_cem_external_scanner_create(void) { return NULL; }
void tree_sitter_cem_external_scanner_destroy(void *payload) { (void)payload; }
unsigned tree_sitter_cem_external_scanner_serialize(void *payload, char *buffer) {
  (void)payload; (void)buffer; return 0;
}
void tree_sitter_cem_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
  (void)payload; (void)buffer; (void)length;
}

// Stateless opaque capture mirrors CemTokenizer::scan_query_brace_body.
// The host closing brace stays available to the structural grammar.
bool tree_sitter_cem_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  (void)payload;
  unsigned valid_count = 0;
  enum TokenType token = EXPRESSION_BODY;
  for (unsigned i = 0; i < 3; i++) {
    if (valid_symbols[i]) { valid_count++; token = (enum TokenType)i; }
  }
  if (valid_count != 1) return false;
  bool horizontal_prefix = true;
  while (iswspace(lexer->lookahead)) {
    if (lexer->lookahead != ' ' && lexer->lookahead != '\t') horizontal_prefix = false;
    lexer->advance(lexer, true);
  }
  if (token == EXPRESSION_BODY && horizontal_prefix && lexer->lookahead == '|') return false;

  uint32_t braces = 0, comments = 0;
  int32_t quote = 0;
  bool consumed = false, block_comment = false, line_comment = false;
  while (!lexer->eof(lexer)) {
    int32_t c = lexer->lookahead;
    if (!quote && !comments && !block_comment && !line_comment && c == '}' && braces == 0) {
      if (!consumed) return false;
      lexer->mark_end(lexer);
      lexer->result_symbol = token;
      return true;
    }
    lexer->advance(lexer, false);
    consumed = true;
    if (quote) {
      if (c == '\\' && !lexer->eof(lexer)) lexer->advance(lexer, false);
      else if (c == quote) {
        if (lexer->lookahead == quote) lexer->advance(lexer, false);
        else quote = 0;
      }
    } else if (block_comment) {
      if (c == '*' && lexer->lookahead == '/') {
        block_comment = false; lexer->advance(lexer, false);
      }
    } else if (line_comment) {
      if (c == '\n' || c == '\r') line_comment = false;
    } else if (comments) {
      if (c == '(' && lexer->lookahead == ':') {
        comments++; lexer->advance(lexer, false);
      } else if (c == ':' && lexer->lookahead == ')') {
        comments--; lexer->advance(lexer, false);
      }
    } else if (c == '/' && lexer->lookahead == '*') {
      block_comment = true; lexer->advance(lexer, false);
    } else if (c == '/' && lexer->lookahead == '/') {
      line_comment = true; lexer->advance(lexer, false);
    } else if (c == '(' && lexer->lookahead == ':') {
      comments = 1; lexer->advance(lexer, false);
    } else if (c == '\'' || c == '"') quote = c;
    else if (c == '{') braces++;
    else if (c == '}') braces--;
  }
  // A missing host delimiter, quote or comment closure must remain an error.
  return false;
}
