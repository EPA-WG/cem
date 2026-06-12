//! Layer 1: byte-precise CEM-QL lexer.

use cem_ml::source::ByteRange;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub range: ByteRange,
    pub cooked: Option<CookedTokenPayload>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Dot,
    Comma,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Pipe,
    Amp,
    Minus,
    Caret,
    Assign,
    Colon,
    ColonColon,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    EqOp,
    NeqOp,
    Plus,
    Star,
    DivKw,
    ModKw,
    AndKw,
    OrKw,
    NotKw,
    AmpAmpReserved,
    PipePipeReserved,
    Coalesce,
    Slash,
    DotDot,
    Let,
    In,
    If,
    Then,
    Else,
    For,
    ReturnKw,
    Some,
    Every,
    Satisfies,
    Import,
    As,
    Declare,
    Variable,
    Function,
    Module,
    InstanceKw,
    OfKw,
    CastKw,
    TreatKw,
    IsKw,
    FnKw,
    Ident,
    PrefixedName,
    StringLit,
    IntLit,
    DecimalLit,
    DoubleLit,
    BoolLit,
    NullLit,
    Whitespace,
    LineComment,
    BlockComment,
    Invalid,
    EndOfInput,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CookedTokenPayload {
    Name(String),
    PrefixedName { prefix: String, local: String },
    StringValue(String),
    IntValue(i64),
    DecimalValue(String),
    DoubleValue(f64),
    BoolValue(bool),
}

#[derive(Debug, Clone)]
pub struct Lexer<'src> {
    pub source: &'src str,
    cursor: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { source, cursor: 0 }
    }

    pub fn scan_all(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let done = token.kind == TokenKind::EndOfInput;
            tokens.push(token);
            if done {
                return tokens;
            }
        }
    }

    pub fn next_token(&mut self) -> Token {
        let start = self.cursor;
        let Some(ch) = self.peek_char() else {
            return self.token(TokenKind::EndOfInput, start, start, None);
        };

        if is_whitespace(ch) {
            return self.scan_whitespace(start);
        }
        if self.starts_with(";;") {
            return self.scan_line_comment(start);
        }
        if self.starts_with("(*") {
            return self.scan_block_comment(start);
        }
        if ch == '"' {
            return self.scan_string(start);
        }
        if ch.is_ascii_digit() {
            return self.scan_number(start);
        }
        if is_ident_start(ch) {
            return self.scan_identifier_or_keyword(start);
        }

        self.advance_char();
        match ch {
            '.' if self.consume_if('.') => self.token(TokenKind::DotDot, start, self.cursor, None),
            '.' => self.token(TokenKind::Dot, start, self.cursor, None),
            ',' => self.token(TokenKind::Comma, start, self.cursor, None),
            '(' => self.token(TokenKind::LParen, start, self.cursor, None),
            ')' => self.token(TokenKind::RParen, start, self.cursor, None),
            '[' => self.token(TokenKind::LBracket, start, self.cursor, None),
            ']' => self.token(TokenKind::RBracket, start, self.cursor, None),
            '{' => self.token(TokenKind::LBrace, start, self.cursor, None),
            '}' => self.token(TokenKind::RBrace, start, self.cursor, None),
            '|' if self.consume_if('|') => {
                self.token(TokenKind::PipePipeReserved, start, self.cursor, None)
            }
            '|' => self.token(TokenKind::Pipe, start, self.cursor, None),
            '&' if self.consume_if('&') => {
                self.token(TokenKind::AmpAmpReserved, start, self.cursor, None)
            }
            '&' => self.token(TokenKind::Amp, start, self.cursor, None),
            '?' if self.consume_if('?') => self.token(TokenKind::Coalesce, start, self.cursor, None),
            '-' => self.token(TokenKind::Minus, start, self.cursor, None),
            '^' => self.token(TokenKind::Caret, start, self.cursor, None),
            ':' if self.consume_if('=') => self.token(TokenKind::Assign, start, self.cursor, None),
            ':' if self.consume_if(':') => {
                self.token(TokenKind::ColonColon, start, self.cursor, None)
            }
            ':' => self.token(TokenKind::Colon, start, self.cursor, None),
            '=' => self.token(TokenKind::EqOp, start, self.cursor, None),
            '!' if self.consume_if('=') => self.token(TokenKind::NeqOp, start, self.cursor, None),
            '<' if self.consume_if('=') => self.token(TokenKind::Le, start, self.cursor, None),
            '<' => self.token(TokenKind::Lt, start, self.cursor, None),
            '>' if self.consume_if('=') => self.token(TokenKind::Ge, start, self.cursor, None),
            '>' => self.token(TokenKind::Gt, start, self.cursor, None),
            '+' => self.token(TokenKind::Plus, start, self.cursor, None),
            '*' => self.token(TokenKind::Star, start, self.cursor, None),
            '/' => self.token(TokenKind::Slash, start, self.cursor, None),
            _ => self.token(TokenKind::Invalid, start, self.cursor, None),
        }
    }

    fn scan_whitespace(&mut self, start: usize) -> Token {
        while self.peek_char().is_some_and(is_whitespace) {
            self.advance_char();
        }
        self.token(TokenKind::Whitespace, start, self.cursor, None)
    }

    fn scan_line_comment(&mut self, start: usize) -> Token {
        self.cursor += ";;".len();
        while let Some(ch) = self.peek_char() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            self.advance_char();
        }
        self.token(TokenKind::LineComment, start, self.cursor, None)
    }

    fn scan_block_comment(&mut self, start: usize) -> Token {
        self.cursor += "(*".len();
        while self.cursor < self.source.len() {
            if self.starts_with("*)") {
                self.cursor += "*)".len();
                return self.token(TokenKind::BlockComment, start, self.cursor, None);
            }
            self.advance_char();
        }
        self.token(TokenKind::Invalid, start, self.cursor, None)
    }

    fn scan_string(&mut self, start: usize) -> Token {
        self.advance_char(); // opening quote
        let mut value = String::new();
        let mut invalid = false;
        while let Some(ch) = self.peek_char() {
            match ch {
                '"' => {
                    self.advance_char();
                    let kind = if invalid {
                        TokenKind::Invalid
                    } else {
                        TokenKind::StringLit
                    };
                    let cooked = (!invalid).then_some(CookedTokenPayload::StringValue(value));
                    return self.token(kind, start, self.cursor, cooked);
                }
                '\\' => {
                    self.advance_char();
                    match self.scan_escape() {
                        Some(escaped) => value.push(escaped),
                        None => invalid = true,
                    }
                }
                _ => {
                    self.advance_char();
                    value.push(ch);
                }
            }
        }
        self.token(TokenKind::Invalid, start, self.cursor, None)
    }

    fn scan_escape(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.advance_char();
        match ch {
            'n' => Some('\n'),
            't' => Some('\t'),
            'r' => Some('\r'),
            '\\' => Some('\\'),
            '"' => Some('"'),
            'u' if self.consume_if('{') => self.scan_unicode_escape(),
            _ => None,
        }
    }

    fn scan_unicode_escape(&mut self) -> Option<char> {
        let start = self.cursor;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_hexdigit()) {
            self.advance_char();
        }
        if self.cursor == start || !self.consume_if('}') {
            return None;
        }
        let value = u32::from_str_radix(&self.source[start..self.cursor - 1], 16).ok()?;
        char::from_u32(value)
    }

    fn scan_number(&mut self, start: usize) -> Token {
        self.consume_ascii_digits();
        let mut kind = TokenKind::IntLit;

        if self.peek_char() == Some('.')
            && self.peek_next_char().is_some_and(|ch| ch.is_ascii_digit())
        {
            kind = TokenKind::DecimalLit;
            self.advance_char();
            self.consume_ascii_digits();
        }

        if self.peek_char().is_some_and(|ch| ch == 'e' || ch == 'E') {
            kind = TokenKind::DoubleLit;
            self.advance_char();
            if self.peek_char().is_some_and(|ch| ch == '+' || ch == '-') {
                self.advance_char();
            }
            let exponent_start = self.cursor;
            self.consume_ascii_digits();
            if self.cursor == exponent_start {
                return self.token(TokenKind::Invalid, start, self.cursor, None);
            }
        }

        let lexeme = &self.source[start..self.cursor];
        let cooked = match kind {
            TokenKind::IntLit => lexeme.parse::<i64>().ok().map(CookedTokenPayload::IntValue),
            TokenKind::DecimalLit => Some(CookedTokenPayload::DecimalValue(lexeme.to_owned())),
            TokenKind::DoubleLit => lexeme
                .parse::<f64>()
                .ok()
                .map(CookedTokenPayload::DoubleValue),
            _ => None,
        };
        let final_kind = if cooked.is_some() {
            kind
        } else {
            TokenKind::Invalid
        };
        self.token(final_kind, start, self.cursor, cooked)
    }

    fn scan_identifier_or_keyword(&mut self, start: usize) -> Token {
        let first = self.scan_ident_text();
        if self.peek_char() == Some(':') && self.peek_next_char().is_some_and(is_ident_start) {
            self.advance_char();
            let local = self.scan_ident_text();
            return self.token(
                TokenKind::PrefixedName,
                start,
                self.cursor,
                Some(CookedTokenPayload::PrefixedName {
                    prefix: first,
                    local,
                }),
            );
        }

        match keyword_kind(&first) {
            Some((kind, cooked)) => self.token(kind, start, self.cursor, cooked),
            None => self.token(
                TokenKind::Ident,
                start,
                self.cursor,
                Some(CookedTokenPayload::Name(first)),
            ),
        }
    }

    fn scan_ident_text(&mut self) -> String {
        let start = self.cursor;
        self.advance_char();
        while self.peek_char().is_some_and(is_ident_continue) {
            self.advance_char();
        }
        self.source[start..self.cursor].to_owned()
    }

    fn consume_ascii_digits(&mut self) {
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance_char();
        }
    }

    fn token(
        &self,
        kind: TokenKind,
        start: usize,
        end: usize,
        cooked: Option<CookedTokenPayload>,
    ) -> Token {
        Token {
            kind,
            range: ByteRange::new(start as u64, (end - start).try_into().unwrap_or(u32::MAX)),
            cooked,
        }
    }

    fn starts_with(&self, pat: &str) -> bool {
        self.source[self.cursor..].starts_with(pat)
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut chars = self.source[self.cursor..].chars();
        chars.next()?;
        chars.next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }

    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.advance_char();
            true
        } else {
            false
        }
    }
}

fn keyword_kind(raw: &str) -> Option<(TokenKind, Option<CookedTokenPayload>)> {
    let kind = match raw {
        "eq" => TokenKind::Eq,
        "ne" => TokenKind::Ne,
        "lt" => TokenKind::Lt,
        "le" => TokenKind::Le,
        "gt" => TokenKind::Gt,
        "ge" => TokenKind::Ge,
        "div" => TokenKind::DivKw,
        "mod" => TokenKind::ModKw,
        "and" => TokenKind::AndKw,
        "or" => TokenKind::OrKw,
        "not" => TokenKind::NotKw,
        "let" => TokenKind::Let,
        "in" => TokenKind::In,
        "if" => TokenKind::If,
        "then" => TokenKind::Then,
        "else" => TokenKind::Else,
        "for" => TokenKind::For,
        "return" => TokenKind::ReturnKw,
        "some" => TokenKind::Some,
        "every" => TokenKind::Every,
        "satisfies" => TokenKind::Satisfies,
        "import" => TokenKind::Import,
        "as" => TokenKind::As,
        "declare" => TokenKind::Declare,
        "variable" => TokenKind::Variable,
        "function" => TokenKind::Function,
        "module" => TokenKind::Module,
        "instance" => TokenKind::InstanceKw,
        "of" => TokenKind::OfKw,
        "cast" => TokenKind::CastKw,
        "treat" => TokenKind::TreatKw,
        "is" => TokenKind::IsKw,
        "fn" => TokenKind::FnKw,
        "null" => TokenKind::NullLit,
        "true" => {
            return Some((
                TokenKind::BoolLit,
                Some(CookedTokenPayload::BoolValue(true)),
            ))
        }
        "false" => {
            return Some((
                TokenKind::BoolLit,
                Some(CookedTokenPayload::BoolValue(false)),
            ))
        }
        _ => return None,
    };
    Some((kind, None))
}

fn is_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r')
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}
