//! Explicit XPath regex subset. Never pass authored engine syntax through.
//! Only bounded compiled NFAs execute; values arrive through common atomization.
use super::*;
use regex_automata::{
    nfa::thompson::{pikevm::PikeVM as Engine, State, NFA},
    util::syntax,
    Input, PatternID,
};

type Result<T> = std::result::Result<T, XPathEvaluationError>;
const MAX_PATTERN: usize = 2048;
const MAX_INPUT: usize = 16 * 1024;
const MAX_NFA: usize = 128 * 1024;
const MAX_WORK: u64 = 16 * 1024 * 1024;
const MAX_GROUPS: usize = 32;

fn invalid(message: &str, range: XPathSourceRange) -> XPathEvaluationError {
    XPathEvaluationError::dynamic(
        "cem.xpath.regex_pattern",
        format!("err:FORX0002: {message}"),
        range,
    )
}
fn unsupported(message: &str, range: XPathSourceRange) -> XPathEvaluationError {
    XPathEvaluationError::dynamic(
        "cem.xpath.regex_unsupported",
        format!("XPath regex subset does not support {message}"),
        range,
    )
}
fn limited(message: &str, range: XPathSourceRange) -> XPathEvaluationError {
    XPathEvaluationError::dynamic("cem.xpath.regex_limit_exceeded", message, range)
}

#[derive(Default)]
struct Flags {
    dot_all: bool,
    strip_space: bool,
    literal: bool,
}
impl Flags {
    fn parse(value: &str, range: XPathSourceRange) -> Result<Self> {
        if value
            .chars()
            .any(|c| !matches!(c, 's' | 'm' | 'i' | 'x' | 'q'))
        {
            return Err(XPathEvaluationError::dynamic(
                "cem.xpath.regex_flags",
                "err:FORX0001: regex flags must contain only s, m, i, x or q",
                range,
            ));
        }
        let literal = value.contains('q');
        if value.contains('i') || (value.contains('m') && !literal) {
            return Err(unsupported(
                "i or m flags (XPath case variants and line boundaries)",
                range,
            ));
        }
        Ok(Self {
            literal,
            dot_all: value.contains('s'),
            strip_space: value.contains('x') && !literal,
        })
    }
}

fn escaped(ch: char) -> String {
    format!(r"\x{{{:X}}}", ch as u32)
}

struct Pattern {
    chars: Vec<char>,
    at: usize,
    depth: usize,
    groups: usize,
    flags: Flags,
    range: XPathSourceRange,
}
struct ClassAtom {
    code: String,
    literal: Option<char>,
}
impl Pattern {
    fn translate(source: &str, flags: Flags, range: XPathSourceRange) -> Result<String> {
        if source.len() > MAX_PATTERN {
            return Err(limited(
                "XPath regex pattern exceeds 2048 UTF-8 bytes",
                range,
            ));
        }
        if flags.literal {
            return Ok(source.chars().map(escaped).collect());
        }
        // XPath x removes only XML whitespace outside classes, even between
        // a backslash and its escape name. It does not introduce # comments.
        let (mut in_class, mut escaped_char) = (false, false);
        let chars = source
            .chars()
            .filter(|&c| {
                if flags.strip_space && !in_class && matches!(c, ' ' | '\t' | '\r' | '\n') {
                    return false;
                }
                if escaped_char {
                    escaped_char = false;
                } else if c == '\\' {
                    escaped_char = true;
                } else if c == '[' {
                    in_class = true;
                } else if c == ']' {
                    in_class = false;
                }
                true
            })
            .collect();
        let mut parser = Self {
            chars,
            at: 0,
            depth: 0,
            groups: 0,
            flags,
            range,
        };
        let result = parser.expression()?;
        if parser.peek().is_some() {
            return Err(invalid("unmatched closing parenthesis", range));
        }
        Ok(result)
    }
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }
    fn take(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.at += 1;
        Some(c)
    }
    fn expect(&mut self, c: char) -> Result<()> {
        if self.take() == Some(c) {
            Ok(())
        } else {
            Err(invalid("incomplete regex construct", self.range))
        }
    }
    fn expression(&mut self) -> Result<String> {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if c == ')' {
                break;
            }
            if c == '|' {
                self.at += 1;
                out.push('|');
                continue;
            }
            out.push_str(&self.atom()?);
            if matches!(self.peek(), Some('?' | '*' | '+' | '{')) {
                if self.peek() == Some('{') {
                    self.at += 1;
                    let min = self.number()?;
                    out.push_str(&format!("{{{min}"));
                    if self.peek() == Some(',') {
                        self.at += 1;
                        out.push(',');
                        if self.peek() != Some('}') {
                            let max = self.number()?;
                            if max < min {
                                return Err(invalid(
                                    "repetition upper bound is smaller than its lower bound",
                                    self.range,
                                ));
                            }
                            out.push_str(&max.to_string());
                        }
                    }
                    self.expect('}')?;
                    out.push('}');
                } else {
                    out.push(self.take().unwrap());
                }
                if self.peek() == Some('?') {
                    self.at += 1;
                    out.push('?');
                }
            }
        }
        Ok(out)
    }
    fn number(&mut self) -> Result<usize> {
        let start = self.at;
        let mut value = 0usize;
        while let Some(c @ '0'..='9') = self.peek() {
            self.at += 1;
            value = value
                .saturating_mul(10)
                .saturating_add(c as usize - '0' as usize);
            if value > 1024 {
                return Err(limited("XPath regex repetition exceeds 1024", self.range));
            }
        }
        if start == self.at {
            return Err(invalid("repetition requires ASCII digits", self.range));
        }
        Ok(value)
    }
    fn atom(&mut self) -> Result<String> {
        let c = self.take().unwrap();
        Ok(match c {
            '(' => {
                self.depth += 1;
                if self.depth > 32 {
                    return Err(limited("XPath regex nesting exceeds 32", self.range));
                }
                let prefix = if self.peek() == Some('?') {
                    self.at += 1;
                    self.expect(':')?;
                    "(?:"
                } else {
                    self.groups += 1;
                    if self.groups > MAX_GROUPS {
                        return Err(limited(
                            "XPath regex has more than 32 capture groups",
                            self.range,
                        ));
                    }
                    "("
                };
                let body = self.expression()?;
                self.expect(')')?;
                self.depth -= 1;
                format!("{prefix}{body})")
            }
            '[' => self.class()?,
            '\\' => self.escape(false)?.code,
            '.' => if self.flags.dot_all {
                "(?s:.)"
            } else {
                r"[^\r\n]"
            }
            .into(),
            '^' => r"\A".into(),
            '$' => r"\z".into(),
            ']' | '{' | '}' | '*' | '+' | '?' => {
                return Err(invalid("unexpected regex metacharacter", self.range))
            }
            _ => escaped(c),
        })
    }
    fn class(&mut self) -> Result<String> {
        let mut out = String::from("[");
        if self.peek() == Some('^') {
            self.at += 1;
            out.push('^');
        }
        let mut count = 0;
        while self.peek() != Some(']') {
            if self.peek() == Some('-') && self.chars.get(self.at + 1) == Some(&'[') {
                return Err(unsupported("character-class subtraction", self.range));
            }
            let first_dash = self.peek() == Some('-');
            let first = self.class_atom()?;
            if first_dash && count > 0 && self.peek() != Some(']') {
                return Err(invalid(
                    "an unescaped hyphen must be first or last in a character class",
                    self.range,
                ));
            }
            if self.peek() == Some('-') && self.chars.get(self.at + 1) != Some(&']') {
                self.at += 1;
                if self.peek() == Some('[') {
                    return Err(unsupported("character-class subtraction", self.range));
                }
                let last_dash = self.peek() == Some('-');
                let last = self.class_atom()?;
                match (first.literal, last.literal) {
                    (Some(a), Some(b)) if a <= b && !first_dash && !last_dash => {
                        out.push_str(&first.code);
                        out.push('-');
                        out.push_str(&last.code);
                    }
                    _ => return Err(invalid("invalid character range", self.range)),
                }
            } else {
                out.push_str(&first.code);
            }
            count += 1;
        }
        if count == 0 {
            return Err(invalid("empty character class", self.range));
        }
        self.expect(']')?;
        out.push(']');
        Ok(out)
    }
    fn class_atom(&mut self) -> Result<ClassAtom> {
        match self.take() {
            Some('\\') => self.escape(true),
            Some('[' | ']') | None => {
                Err(invalid("invalid or unclosed character class", self.range))
            }
            Some(c) => Ok(ClassAtom {
                code: escaped(c),
                literal: Some(c),
            }),
        }
    }
    fn escape(&mut self, in_class: bool) -> Result<ClassAtom> {
        let c = self
            .take()
            .ok_or_else(|| invalid("trailing backslash", self.range))?;
        let literal = match c {
            'n' => Some('\n'),
            'r' => Some('\r'),
            't' => Some('\t'),
            '\\' | '|' | '.' | '?' | '*' | '+' | '(' | ')' | '{' | '}' | '$' | '-' | '[' | ']'
            | '^' => Some(c),
            _ => None,
        };
        if let Some(c) = literal {
            return Ok(ClassAtom {
                code: escaped(c),
                literal,
            });
        }
        let code = match c {
            's' => r"[\x09\x0A\x0D\x20]".into(),
            'S' => r"[^\x09\x0A\x0D\x20]".into(),
            'd' => r"\p{Nd}".into(),
            'D' => r"\P{Nd}".into(),
            'w' => r"[^\p{P}\p{Z}\p{C}]".into(),
            'W' => r"[\p{P}\p{Z}\p{C}]".into(),
            'i' | 'I' | 'c' | 'C' => return Err(unsupported("XML name escapes", self.range)),
            '1'..='9' if !in_class => {
                return Err(unsupported("pattern backreferences", self.range))
            }
            'p' | 'P' => {
                self.expect('{')?;
                let start = self.at;
                while self.peek().is_some_and(|c| c != '}') {
                    self.at += 1;
                }
                let name: String = self.chars[start..self.at].iter().collect();
                self.expect('}')?;
                if name.starts_with("Is") {
                    return Err(unsupported("Unicode block escapes", self.range));
                }
                // Surrogates are not Unicode scalar values or XPath string
                // characters. The positive category is therefore empty.
                if name == "Cs" {
                    return Ok(ClassAtom {
                        code: if c == 'p' {
                            r"[\x00&&[^\x00]]"
                        } else {
                            r"[\s\S]"
                        }
                        .into(),
                        literal: None,
                    });
                }
                if !matches!(
                    name.as_str(),
                    "L" | "Lu"
                        | "Ll"
                        | "Lt"
                        | "Lm"
                        | "Lo"
                        | "M"
                        | "Mn"
                        | "Mc"
                        | "Me"
                        | "N"
                        | "Nd"
                        | "Nl"
                        | "No"
                        | "P"
                        | "Pc"
                        | "Pd"
                        | "Ps"
                        | "Pe"
                        | "Pi"
                        | "Pf"
                        | "Po"
                        | "Z"
                        | "Zs"
                        | "Zl"
                        | "Zp"
                        | "S"
                        | "Sm"
                        | "Sc"
                        | "Sk"
                        | "So"
                        | "C"
                        | "Cc"
                        | "Cf"
                        | "Co"
                        | "Cn"
                        | "Cs"
                ) {
                    return Err(invalid("unknown Unicode general category", self.range));
                }
                format!(r"\{c}{{{name}}}")
            }
            _ => {
                return Err(invalid(
                    "escape is not part of the XPath regex syntax",
                    self.range,
                ))
            }
        };
        Ok(ClassAtom {
            code,
            literal: None,
        })
    }
}

enum Replacement<'a> {
    Literal(&'a str),
    Capture(usize),
}
fn replacement<'a>(
    source: &'a str,
    literal: bool,
    groups: usize,
    range: XPathSourceRange,
) -> Result<Vec<Replacement<'a>>> {
    if literal {
        return Ok(vec![Replacement::Literal(source)]);
    }
    let mut parts = Vec::new();
    let mut at = 0;
    while at < source.len() {
        match source.as_bytes()[at] {
            b'\\' => {
                let next = at + 1;
                if !matches!(source.as_bytes().get(next), Some(b'\\' | b'$')) {
                    return Err(XPathEvaluationError::dynamic(
                        "cem.xpath.regex_replacement",
                        "err:FORX0004: replacement backslash must escape a dollar or backslash",
                        range,
                    ));
                }
                parts.push(Replacement::Literal(&source[next..next + 1]));
                at += 2;
            }
            b'$' => {
                at += 1;
                let start = at;
                while source.as_bytes().get(at).is_some_and(u8::is_ascii_digit) {
                    at += 1;
                }
                if at == start {
                    return Err(XPathEvaluationError::dynamic(
                        "cem.xpath.regex_replacement",
                        "err:FORX0004: replacement dollar must be followed by ASCII digits",
                        range,
                    ));
                }
                // Take the longest numeric prefix naming a capture, or one
                // digit when none does. Leading zeros never overflow.
                let mut value = 0usize;
                let mut end = start;
                for (i, digit) in source.as_bytes()[start..at].iter().enumerate() {
                    value = value
                        .saturating_mul(10)
                        .saturating_add((digit - b'0') as usize);
                    if value <= groups || value <= 9 {
                        end = start + i + 1;
                    } else {
                        break;
                    }
                }
                let capture = source[start..end].parse::<usize>().unwrap_or(usize::MAX);
                // Once a single-digit nonexistent group is reached, subsequent
                // digits are literal, as in $23 with fewer than 23 captures.
                parts.push(Replacement::Capture(capture));
                if end < at {
                    parts.push(Replacement::Literal(&source[end..at]));
                }
            }
            _ => {
                let start = at;
                while at < source.len() && !matches!(source.as_bytes()[at], b'$' | b'\\') {
                    at += 1;
                }
                parts.push(Replacement::Literal(&source[start..at]));
            }
        }
    }
    Ok(parts)
}

struct Compiled {
    engine: Engine,
    weight: u64,
    charged: u64,
}
impl Compiled {
    fn compile(
        pattern: &str,
        flags: Flags,
        runtime: &mut XPathEvaluationRuntime,
        range: XPathSourceRange,
    ) -> Result<Self> {
        let translated = Pattern::translate(pattern, flags, range)?;
        runtime.charge_work(translated.len() as u64, range)?;
        runtime.force(range)?;
        let engine = Engine::builder()
            .syntax(syntax::Config::new().nest_limit(128))
            .thompson(NFA::config().nfa_size_limit(Some(MAX_NFA)))
            .build(&translated)
            .map_err(|error| {
                if error.size_limit().is_some() {
                    limited("XPath regex compiled NFA exceeds 128 KiB", range)
                } else {
                    invalid("regex could not be compiled in the supported subset", range)
                }
            })?;
        runtime.force(range)?;
        // Count states and outgoing edges, independently of pointer/heap
        // sizes, so native and WASM charge the same search work.
        let weight = engine
            .get_nfa()
            .states()
            .iter()
            .map(|state| {
                1 + match state {
                    State::Sparse(trans) => trans.transitions.len(),
                    State::Dense(_) => 256,
                    State::Union { alternates } => alternates.len(),
                    State::BinaryUnion { .. } => 2,
                    _ => 1,
                } as u64
            })
            .sum();
        Ok(Self {
            engine,
            weight,
            charged: 0,
        })
    }
    fn groups(&self) -> usize {
        self.engine
            .get_nfa()
            .group_info()
            .group_len(PatternID::ZERO)
    }
    fn before_search(
        &mut self,
        bytes: usize,
        captures: bool,
        runtime: &mut XPathEvaluationRuntime,
        range: XPathSourceRange,
    ) -> Result<()> {
        // Charge each remaining suffix before searching. Repeated searches can
        // be quadratic; a linear-input-only charge would hide that work.
        let slots = if captures {
            self.groups().max(1) * 2
        } else {
            2
        };
        let work = self
            .weight
            .saturating_mul(bytes as u64 + 1)
            .saturating_mul(slots as u64);
        self.charged = self.charged.saturating_add(work);
        if self.charged > MAX_WORK {
            return Err(limited(
                "XPath regex search exceeds its 16 Mi work ceiling",
                range,
            ));
        }
        runtime.charge_work(work, range)?;
        runtime.force(range)
    }
}

pub(super) fn evaluate(
    function: XPathNativeFunction,
    expression: &XPathExpressionAst,
    arguments: &[XPathExpressionNode],
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    let mut values = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        let items = xpath_evaluate_expression_node(expression, argument, focus, bindings, runtime)?;
        values.push(text::string_argument(
            &items,
            index == 0,
            argument.source_range,
            runtime,
        )?);
    }
    let input = &values[0];
    if input.len() > MAX_INPUT {
        return Err(limited(
            "XPath regex input exceeds 16 KiB",
            arguments[0].source_range,
        ));
    }
    let flag_index = if function == XPathNativeFunction::Replace {
        3
    } else {
        2
    };
    if values
        .get(flag_index)
        .is_some_and(|value| value.len() > MAX_PATTERN)
        || (function == XPathNativeFunction::Replace && values[2].len() > MAX_INPUT)
    {
        return Err(limited(
            "XPath regex flags exceed 2048 bytes or replacement exceeds 16 KiB",
            range,
        ));
    }
    let flags = Flags::parse(
        values.get(flag_index).map_or("", String::as_str),
        arguments
            .get(flag_index)
            .map_or(range, |arg| arg.source_range),
    )?;
    let literal = flags.literal;
    let pattern_range = arguments[1].source_range;
    let mut compiled = Compiled::compile(&values[1], flags, runtime, pattern_range)?;
    let mut cache = compiled.engine.create_cache();
    if function == XPathNativeFunction::Matches {
        compiled.before_search(input.len(), false, runtime, range)?;
        let matched = compiled.engine.is_match(&mut cache, input);
        runtime.force(range)?;
        return Ok(vec![xpath_boolean_result_item(expression, range, matched)]);
    }
    compiled.before_search(0, false, runtime, pattern_range)?;
    let empty_match = compiled.engine.is_match(&mut cache, "");
    runtime.force(pattern_range)?;
    if empty_match {
        return Err(XPathEvaluationError::dynamic(
            "cem.xpath.regex_empty_match",
            "err:FORX0003: regex must not match the empty string",
            pattern_range,
        ));
    }
    let parts = if function == XPathNativeFunction::Replace {
        replacement(
            &values[2],
            literal,
            compiled.groups() - 1,
            arguments[2].source_range,
        )?
    } else {
        Vec::new()
    };
    if function == XPathNativeFunction::Tokenize && input.is_empty() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    let (mut offset, mut output, mut bytes) = (0, String::new(), 0usize);
    let mut captures = compiled.engine.create_captures();
    loop {
        compiled.before_search(
            input.len() - offset,
            function == XPathNativeFunction::Replace,
            runtime,
            range,
        )?;
        let search = Input::new(input).range(offset..);
        let found = if function == XPathNativeFunction::Replace {
            compiled.engine.search(&mut cache, &search, &mut captures);
            captures.get_match()
        } else {
            compiled.engine.find(&mut cache, search)
        };
        runtime.force(range)?;
        let Some(matched) = found else {
            break;
        };
        // The supported grammar has no context-dependent empty lookarounds.
        // Empty patterns were rejected above; still guard progress explicitly.
        if matched.start() == matched.end() {
            return Err(invalid("unexpected empty regex match", pattern_range));
        }
        if function == XPathNativeFunction::Tokenize {
            append_token(
                expression,
                &input[offset..matched.start()],
                &mut items,
                &mut bytes,
                runtime,
                range,
            )?;
        } else {
            runtime.append_text(&mut output, &input[offset..matched.start()], range)?;
            for part in &parts {
                runtime.poll(range)?;
                let value = match part {
                    Replacement::Literal(value) => value,
                    Replacement::Capture(index) => captures
                        .get_group(*index)
                        .map_or("", |span| &input[span.start..span.end]),
                };
                runtime.append_text(&mut output, value, range)?;
            }
        }
        offset = matched.end();
    }
    if function == XPathNativeFunction::Tokenize {
        append_token(
            expression,
            &input[offset..],
            &mut items,
            &mut bytes,
            runtime,
            range,
        )?;
        Ok(items)
    } else {
        runtime.append_text(&mut output, &input[offset..], range)?;
        Ok(vec![xpath_string_result_item(expression, range, output)])
    }
}
fn append_token(
    expression: &XPathExpressionAst,
    value: &str,
    items: &mut Vec<XPathResultItem>,
    bytes: &mut usize,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<()> {
    runtime.enforce_sequence_items(items.len() + 1, range)?;
    *bytes = bytes.saturating_add(value.len());
    runtime.check_text_size(*bytes, range)?;
    items.push(xpath_string_result_item(
        expression,
        range,
        runtime.copy_text(value, range)?,
    ));
    Ok(())
}
