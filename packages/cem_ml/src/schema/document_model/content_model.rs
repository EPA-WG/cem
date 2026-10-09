//! Syntax consumer for shipped content-model strings. Names remain lexical;
//! parsing grants no schema lookup or document matching authority.
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
pub struct GrammarLimits {
    pub max_bytes: usize,
    pub max_tokens: usize,
    pub max_depth: usize,
}
impl Default for GrammarLimits {
    fn default() -> Self {
        Self {
            max_bytes: 1_048_576,
            max_tokens: 100_000,
            max_depth: 64,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum GrammarError<E> {
    Invalid { span: Range<usize> },
    Limit(&'static str),
    Interrupted(E),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Name,
    Star,
    Plus,
    Question,
    Open,
    Close,
    Choice,
}
#[derive(Debug)]
struct Token {
    kind: Kind,
    span: Range<usize>,
}

/// Empty input denotes an empty model. Otherwise: alternatives of whitespace
/// sequences; a term is a QName, `*`, or parenthesized model, optionally followed
/// immediately by one `?`, `*`, or `+` (except the standalone wildcard).
/// Offsets always refer to decoded UTF-8 input. Caller limits also apply to
/// malformed input; parsing uses an explicit stack instead of Rust recursion.
pub fn validate_with_check<E>(
    text: &str,
    limits: GrammarLimits,
    check: &mut impl FnMut() -> Result<(), E>,
) -> Result<(), GrammarError<E>> {
    check().map_err(GrammarError::Interrupted)?;
    if text.len() > limits.max_bytes {
        return Err(GrammarError::Limit("bytes"));
    }
    let mut tokens = Vec::new();
    let mut start = None;
    let mut push = |kind, span| {
        if tokens.len() >= limits.max_tokens {
            return Err(GrammarError::Limit("tokens"));
        }
        tokens.push(Token { kind, span });
        Ok(())
    };
    for (index, (offset, ch)) in text.char_indices().enumerate() {
        if index % 64 == 0 {
            check().map_err(GrammarError::Interrupted)?;
        }
        let kind = match ch {
            '*' => Some(Kind::Star),
            '+' => Some(Kind::Plus),
            '?' => Some(Kind::Question),
            '(' => Some(Kind::Open),
            ')' => Some(Kind::Close),
            '|' => Some(Kind::Choice),
            _ => None,
        };
        if ch.is_whitespace() || kind.is_some() {
            if let Some(begin) = start.take() {
                push(Kind::Name, begin..offset)?;
            }
            if let Some(kind) = kind {
                push(kind, offset..offset + ch.len_utf8())?;
            }
        } else if start.is_none() {
            start = Some(offset);
        }
    }
    if let Some(begin) = start {
        push(Kind::Name, begin..text.len())?;
    }
    // Each frame tracks whether a complete term precedes the next token and
    // whether that immediately preceding term may receive a single postfix.
    #[derive(Default)]
    struct Frame {
        term: bool,
        postfix: bool,
        end: usize,
    }
    let mut stack = vec![Frame::default()];
    for (index, token) in tokens.iter().enumerate() {
        if index % 64 == 0 {
            check().map_err(GrammarError::Interrupted)?;
        }
        let invalid = || GrammarError::Invalid {
            span: token.span.clone(),
        };
        let frame = stack.last_mut().unwrap();
        match token.kind {
            Kind::Name | Kind::Open => {
                if frame.term && frame.end == token.span.start {
                    return Err(invalid());
                }
                if token.kind == Kind::Name {
                    if !super::is_cem_qualified_name(&text[token.span.clone()]) {
                        return Err(invalid());
                    }
                    frame.term = true;
                    frame.postfix = true;
                    frame.end = token.span.end;
                } else {
                    if stack.len() > limits.max_depth {
                        return Err(GrammarError::Limit("depth"));
                    }
                    stack.push(Frame::default());
                }
            }
            Kind::Star if !frame.term || frame.end != token.span.start => {
                frame.term = true;
                frame.postfix = false;
                frame.end = token.span.end;
            }
            Kind::Star | Kind::Plus | Kind::Question => {
                if !frame.term || !frame.postfix || frame.end != token.span.start {
                    return Err(invalid());
                }
                frame.postfix = false;
                frame.end = token.span.end;
            }
            Kind::Choice => {
                if !frame.term {
                    return Err(invalid());
                }
                frame.term = false;
                frame.postfix = false;
            }
            Kind::Close => {
                if !frame.term || stack.len() == 1 {
                    return Err(invalid());
                }
                stack.pop();
                let parent = stack.last_mut().unwrap();
                parent.term = true;
                parent.postfix = true;
                parent.end = token.span.end;
            }
        }
    }
    check().map_err(GrammarError::Interrupted)?;
    if stack.len() != 1 || (!tokens.is_empty() && !stack[0].term) {
        return Err(GrammarError::Invalid {
            span: text.len()..text.len(),
        });
    }
    Ok(())
}
