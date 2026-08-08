use super::XPathTokenKind;

/// CEM-owned XPath 3.1 lexical categories. Exact operator, punctuation, and
/// name spelling remains in `lexeme` so the parser can consume this lossless
/// stream without a second source-text pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum XPathLexicalTokenKind {
    IntegerLiteral,
    DecimalLiteral,
    DoubleLiteral,
    StringLiteral,
    Name,
    DelimitingName,
    BracedUriLiteral,
    Keyword,
    WordOperator,
    SymbolOperator,
    Punctuation,
    VariableSigil,
    Comment,
    Whitespace,
    Error,
}

impl XPathLexicalTokenKind {
    pub(super) fn presentation_kind(self) -> XPathTokenKind {
        match self {
            Self::IntegerLiteral | Self::DecimalLiteral | Self::DoubleLiteral => {
                XPathTokenKind::Number
            }
            Self::StringLiteral => XPathTokenKind::String,
            Self::Name | Self::DelimitingName | Self::BracedUriLiteral => XPathTokenKind::Name,
            Self::Keyword => XPathTokenKind::Keyword,
            Self::WordOperator | Self::SymbolOperator => XPathTokenKind::Operator,
            Self::Punctuation => XPathTokenKind::Punctuation,
            Self::VariableSigil => XPathTokenKind::VariableSigil,
            Self::Comment => XPathTokenKind::Comment,
            Self::Whitespace => XPathTokenKind::Whitespace,
            Self::Error => XPathTokenKind::Error,
        }
    }

    fn terminal_class(self) -> XPathTerminalClass {
        match self {
            Self::IntegerLiteral
            | Self::DecimalLiteral
            | Self::DoubleLiteral
            | Self::Name
            | Self::Keyword
            | Self::WordOperator => XPathTerminalClass::NonDelimiting,
            Self::StringLiteral
            | Self::DelimitingName
            | Self::BracedUriLiteral
            | Self::SymbolOperator
            | Self::Punctuation
            | Self::VariableSigil => XPathTerminalClass::Delimiting,
            Self::Comment | Self::Whitespace => XPathTerminalClass::Separator,
            Self::Error => XPathTerminalClass::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XPathTerminalClass {
    Delimiting,
    NonDelimiting,
    Separator,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct XPathLexicalToken<'a> {
    pub(super) kind: XPathLexicalTokenKind,
    pub(super) lexeme: &'a str,
    pub(super) start: usize,
    pub(super) end: usize,
    terminal_class: XPathTerminalClass,
}

/// Scans the W3C XPath 3.1 lexical grammar independently. Xee is intentionally
/// absent from this module and remains only a pinned differential-test oracle
/// while the CEM parser migration is in progress.
pub(super) fn xpath_lexical_tokens(source: &str) -> Vec<XPathLexicalToken<'_>> {
    let mut scanner = XPathScanner {
        source,
        cursor: 0,
        tokens: Vec::new(),
    };
    scanner.scan();
    scanner.apply_terminal_delimitation();
    scanner.tokens
}

struct XPathScanner<'a> {
    source: &'a str,
    cursor: usize,
    tokens: Vec<XPathLexicalToken<'a>>,
}

impl<'a> XPathScanner<'a> {
    fn scan(&mut self) {
        while self.cursor < self.source.len() {
            let start = self.cursor;

            if is_xml_whitespace(self.source.as_bytes()[start]) {
                let end = self.scan_whitespace(start);
                self.push(XPathLexicalTokenKind::Whitespace, start, end);
                continue;
            }

            if self.source[start..].starts_with("(:") {
                let (kind, end) = match nested_comment_end(self.source, start) {
                    Some(end) => (XPathLexicalTokenKind::Comment, end),
                    None => (XPathLexicalTokenKind::Error, self.source.len()),
                };
                self.push(kind, start, end);
                continue;
            }

            let first = self.source[start..]
                .chars()
                .next()
                .expect("cursor must point at a character boundary");
            if matches!(first, '\'' | '"') {
                let (kind, end) = self.scan_string(start, first);
                self.push(kind, start, end);
                continue;
            }

            if self.source[start..].starts_with("Q{") {
                if let Some((kind, end)) = self.scan_braced_name(start) {
                    self.push(kind, start, end);
                    continue;
                }
            }

            if first.is_ascii_digit()
                || (first == '.'
                    && self
                        .source
                        .as_bytes()
                        .get(start + 1)
                        .is_some_and(u8::is_ascii_digit))
            {
                let (kind, end) = self.scan_number(start);
                self.push(kind, start, end);
                continue;
            }

            if self.source[start..].starts_with("*:") {
                if let Some(end) = scan_ncname(self.source, start + 2) {
                    let local_word = &self.source[start + 2..end];
                    let local_kind = classify_word(local_word);
                    if can_form_eqname_component(local_word, local_kind) {
                        self.push(XPathLexicalTokenKind::DelimitingName, start, end);
                        continue;
                    }
                }
            }

            if is_ncname_start(first) {
                let (kind, end) = self.scan_name_or_word(start);
                self.push(kind, start, end);
                continue;
            }

            if let Some((kind, end)) = self.scan_symbol(start) {
                self.push(kind, start, end);
                continue;
            }

            self.push(
                XPathLexicalTokenKind::Error,
                start,
                start + first.len_utf8(),
            );
        }
    }

    fn scan_whitespace(&self, start: usize) -> usize {
        start
            + self.source.as_bytes()[start..]
                .iter()
                .take_while(|byte| is_xml_whitespace(**byte))
                .count()
    }

    fn scan_string(&self, start: usize, quote: char) -> (XPathLexicalTokenKind, usize) {
        let quote_length = quote.len_utf8();
        let mut cursor = start + quote_length;
        while cursor < self.source.len() {
            let current = self.source[cursor..]
                .chars()
                .next()
                .expect("string cursor must point at a character boundary");
            if current == quote {
                let after_quote = cursor + quote_length;
                if self.source[after_quote..].starts_with(quote) {
                    cursor = after_quote + quote_length;
                    continue;
                }
                return (XPathLexicalTokenKind::StringLiteral, after_quote);
            }
            cursor += current.len_utf8();
        }
        (XPathLexicalTokenKind::Error, self.source.len())
    }

    fn scan_braced_name(&self, start: usize) -> Option<(XPathLexicalTokenKind, usize)> {
        let uri_end = braced_uri_literal_end(self.source, start)?;
        if self.source[uri_end..].starts_with('*') {
            return Some((XPathLexicalTokenKind::DelimitingName, uri_end + 1));
        }
        if let Some(local_end) = scan_ncname(self.source, uri_end) {
            if classify_word(&self.source[uri_end..local_end]) == XPathLexicalTokenKind::Name {
                return Some((XPathLexicalTokenKind::Name, local_end));
            }
        }
        Some((XPathLexicalTokenKind::BracedUriLiteral, uri_end))
    }

    fn scan_number(&self, start: usize) -> (XPathLexicalTokenKind, usize) {
        let bytes = self.source.as_bytes();
        let mut cursor = start;
        let mut has_decimal_point = false;

        if bytes[cursor] == b'.' {
            has_decimal_point = true;
            cursor += 1;
            cursor = scan_ascii_digits(bytes, cursor);
        } else {
            cursor = scan_ascii_digits(bytes, cursor);
            if bytes.get(cursor) == Some(&b'.') {
                has_decimal_point = true;
                cursor += 1;
                cursor = scan_ascii_digits(bytes, cursor);
            }
        }

        if matches!(bytes.get(cursor), Some(b'e' | b'E')) {
            let mut exponent_end = cursor + 1;
            if matches!(bytes.get(exponent_end), Some(b'+' | b'-')) {
                exponent_end += 1;
            }
            let digits_end = scan_ascii_digits(bytes, exponent_end);
            if digits_end > exponent_end {
                return (XPathLexicalTokenKind::DoubleLiteral, digits_end);
            }
        }

        if has_decimal_point {
            (XPathLexicalTokenKind::DecimalLiteral, cursor)
        } else {
            (XPathLexicalTokenKind::IntegerLiteral, cursor)
        }
    }

    fn scan_name_or_word(&self, start: usize) -> (XPathLexicalTokenKind, usize) {
        let name_end = scan_ncname(self.source, start)
            .expect("name scanning requires an NCName start character");
        let word = &self.source[start..name_end];
        let word_kind = classify_word(word);
        if !can_form_eqname_component(word, word_kind) {
            return (word_kind, name_end);
        }

        if self.source[name_end..].starts_with(":*") {
            return (XPathLexicalTokenKind::Name, name_end + 2);
        }
        if self.source[name_end..].starts_with(':') {
            let local_start = name_end + 1;
            if let Some(local_end) = scan_ncname(self.source, local_start) {
                let local_word = &self.source[local_start..local_end];
                let local_kind = classify_word(local_word);
                if can_form_eqname_component(local_word, local_kind) {
                    return (XPathLexicalTokenKind::Name, local_end);
                }
            }
        }
        (word_kind, name_end)
    }

    fn scan_symbol(&self, start: usize) -> Option<(XPathLexicalTokenKind, usize)> {
        let rest = &self.source[start..];
        for symbol in [
            "!=", "//", "<<", "<=", "=>", ">=", ">>", "||", "::", ":=", "*:", ":*", "..",
        ] {
            if rest.starts_with(symbol) {
                let kind = if matches!(
                    symbol,
                    "!=" | "//" | "<<" | "<=" | "=>" | ">=" | ">>" | "||" | ":="
                ) {
                    XPathLexicalTokenKind::SymbolOperator
                } else {
                    XPathLexicalTokenKind::Punctuation
                };
                return Some((kind, start + symbol.len()));
            }
        }

        let symbol = rest.chars().next()?;
        let kind = match symbol {
            '!' | '*' | '+' | '-' | '/' | '<' | '=' | '>' | '|' => {
                XPathLexicalTokenKind::SymbolOperator
            }
            '$' => XPathLexicalTokenKind::VariableSigil,
            '#' | '(' | ')' | ',' | '.' | ':' | '?' | '@' | '[' | ']' | '{' | '}' => {
                XPathLexicalTokenKind::Punctuation
            }
            _ => return None,
        };
        Some((kind, start + symbol.len_utf8()))
    }

    fn push(&mut self, kind: XPathLexicalTokenKind, start: usize, end: usize) {
        debug_assert_eq!(self.cursor, start);
        debug_assert!(start < end && end <= self.source.len());
        self.tokens.push(XPathLexicalToken {
            kind,
            lexeme: &self.source[start..end],
            start,
            end,
            terminal_class: kind.terminal_class(),
        });
        self.cursor = end;
    }

    fn apply_terminal_delimitation(&mut self) {
        for index in 0..self.tokens.len().saturating_sub(1) {
            let current = self.tokens[index];
            let next = self.tokens[index + 1];
            let decimal_before_dot = matches!(
                current.kind,
                XPathLexicalTokenKind::DecimalLiteral | XPathLexicalTokenKind::DoubleLiteral
            ) && next.kind == XPathLexicalTokenKind::Punctuation
                && next.lexeme == ".";
            let adjacent_non_delimiting = current.terminal_class
                == XPathTerminalClass::NonDelimiting
                && next.terminal_class == XPathTerminalClass::NonDelimiting;
            if decimal_before_dot || adjacent_non_delimiting {
                self.tokens[index].kind = XPathLexicalTokenKind::Error;
            }
        }
    }
}

fn is_xml_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn scan_ascii_digits(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start;
    while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
        cursor += 1;
    }
    cursor
}

fn nested_comment_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start + 2;
    let mut depth = 1usize;
    while cursor + 1 < bytes.len() {
        match &bytes[cursor..cursor + 2] {
            b"(:" => {
                depth += 1;
                cursor += 2;
            }
            b":)" => {
                depth -= 1;
                cursor += 2;
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => cursor += 1,
        }
    }
    None
}

fn braced_uri_literal_end(source: &str, start: usize) -> Option<usize> {
    let mut cursor = start + 2;
    while cursor < source.len() {
        let current = source[cursor..].chars().next()?;
        match current {
            '{' => return None,
            '}' => return Some(cursor + 1),
            _ => cursor += current.len_utf8(),
        }
    }
    None
}

fn scan_ncname(source: &str, start: usize) -> Option<usize> {
    let first = source.get(start..)?.chars().next()?;
    if !is_ncname_start(first) {
        return None;
    }
    let mut cursor = start + first.len_utf8();
    while cursor < source.len() {
        let current = source[cursor..]
            .chars()
            .next()
            .expect("NCName cursor must point at a character boundary");
        if !is_ncname_char(current) {
            break;
        }
        cursor += current.len_utf8();
    }
    Some(cursor)
}

pub(super) fn is_ncname(source: &str) -> bool {
    !source.is_empty() && scan_ncname(source, 0) == Some(source.len())
}

fn is_ncname_start(character: char) -> bool {
    matches!(
        character,
        'A'..='Z'
            | '_'
            | 'a'..='z'
            | '\u{00c0}'..='\u{00d6}'
            | '\u{00d8}'..='\u{00f6}'
            | '\u{00f8}'..='\u{02ff}'
            | '\u{0370}'..='\u{037d}'
            | '\u{037f}'..='\u{1fff}'
            | '\u{200c}'..='\u{200d}'
            | '\u{2070}'..='\u{218f}'
            | '\u{2c00}'..='\u{2fef}'
            | '\u{3001}'..='\u{d7ff}'
            | '\u{f900}'..='\u{fdcf}'
            | '\u{fdf0}'..='\u{fffd}'
            | '\u{10000}'..='\u{effff}'
    )
}

fn is_ncname_char(character: char) -> bool {
    is_ncname_start(character)
        || matches!(
            character,
            '-' | '.'
                | '0'..='9'
                | '\u{00b7}'
                | '\u{0300}'..='\u{036f}'
                | '\u{203f}'..='\u{2040}'
        )
}

fn classify_word(word: &str) -> XPathLexicalTokenKind {
    if matches!(
        word,
        "and"
            | "div"
            | "eq"
            | "except"
            | "ge"
            | "gt"
            | "idiv"
            | "intersect"
            | "is"
            | "le"
            | "lt"
            | "mod"
            | "ne"
            | "or"
            | "to"
            | "union"
    ) {
        return XPathLexicalTokenKind::WordOperator;
    }
    if matches!(
        word,
        "ancestor"
            | "ancestor-or-self"
            | "array"
            | "as"
            | "attribute"
            | "cast"
            | "castable"
            | "child"
            | "comment"
            | "descendant"
            | "descendant-or-self"
            | "document-node"
            | "element"
            | "else"
            | "empty-sequence"
            | "every"
            | "following"
            | "following-sibling"
            | "for"
            | "function"
            | "if"
            | "in"
            | "instance"
            | "item"
            | "let"
            | "map"
            | "namespace"
            | "namespace-node"
            | "node"
            | "of"
            | "parent"
            | "preceding"
            | "preceding-sibling"
            | "processing-instruction"
            | "return"
            | "satisfies"
            | "schema-attribute"
            | "schema-element"
            | "self"
            | "some"
            | "switch"
            | "text"
            | "then"
            | "treat"
            | "typeswitch"
    ) {
        XPathLexicalTokenKind::Keyword
    } else {
        XPathLexicalTokenKind::Name
    }
}

fn can_form_eqname_component(word: &str, kind: XPathLexicalTokenKind) -> bool {
    kind == XPathLexicalTokenKind::Name
        || matches!(
            word,
            "array"
                | "attribute"
                | "comment"
                | "document-node"
                | "element"
                | "empty-sequence"
                | "function"
                | "if"
                | "item"
                | "map"
                | "namespace-node"
                | "node"
                | "processing-instruction"
                | "schema-attribute"
                | "schema-element"
                | "switch"
                | "text"
                | "typeswitch"
        )
}
