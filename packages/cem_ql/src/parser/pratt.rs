//! Pratt binding powers for CEM-QL expressions.

use crate::lexer::TokenKind;

use super::{BinaryOp, SetOp};

pub(crate) const PREC_OR: u8 = 1;
pub(crate) const PREC_AND: u8 = 2;
pub(crate) const PREC_NOT: u8 = 3;
const PREC_COMPARE: u8 = 4;
const PREC_SET_UNION: u8 = 5;
const PREC_SET_INTERSECT: u8 = 6;
const PREC_ADD: u8 = 7;
const PREC_MUL: u8 = 8;
pub(crate) const PREC_UNARY_MINUS: u8 = 9;
pub(crate) const PREC_TYPE: u8 = 10;
pub(crate) const PREC_DOT: u8 = 11;
pub(crate) const PREC_PATH: u8 = 12;
pub(crate) const PREC_CALL: u8 = 13;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InfixOperator {
    Binary(BinaryOp),
    Set(SetOp),
}

pub(crate) fn infix_operator(kind: TokenKind) -> Option<(u8, InfixOperator)> {
    let op = match kind {
        TokenKind::OrKw => (PREC_OR, InfixOperator::Binary(BinaryOp::Or)),
        TokenKind::AndKw => (PREC_AND, InfixOperator::Binary(BinaryOp::And)),
        TokenKind::Eq => (PREC_COMPARE, InfixOperator::Binary(BinaryOp::Eq)),
        TokenKind::Ne => (PREC_COMPARE, InfixOperator::Binary(BinaryOp::Ne)),
        TokenKind::Lt => (PREC_COMPARE, InfixOperator::Binary(BinaryOp::Lt)),
        TokenKind::Le => (PREC_COMPARE, InfixOperator::Binary(BinaryOp::Le)),
        TokenKind::Gt => (PREC_COMPARE, InfixOperator::Binary(BinaryOp::Gt)),
        TokenKind::Ge => (PREC_COMPARE, InfixOperator::Binary(BinaryOp::Ge)),
        TokenKind::EqOp => (PREC_COMPARE, InfixOperator::Binary(BinaryOp::EqOp)),
        TokenKind::NeqOp => (PREC_COMPARE, InfixOperator::Binary(BinaryOp::NeqOp)),
        TokenKind::IsKw => (PREC_COMPARE, InfixOperator::Binary(BinaryOp::Is)),
        TokenKind::Pipe => (PREC_SET_UNION, InfixOperator::Set(SetOp::Union)),
        TokenKind::Caret => (
            PREC_SET_UNION,
            InfixOperator::Set(SetOp::SymmetricDifference),
        ),
        TokenKind::Amp => (PREC_SET_INTERSECT, InfixOperator::Set(SetOp::Intersect)),
        TokenKind::Plus => (PREC_ADD, InfixOperator::Binary(BinaryOp::Plus)),
        TokenKind::Minus => (PREC_ADD, InfixOperator::Binary(BinaryOp::Minus)),
        TokenKind::Star => (PREC_MUL, InfixOperator::Binary(BinaryOp::Star)),
        TokenKind::DivKw => (PREC_MUL, InfixOperator::Binary(BinaryOp::Div)),
        TokenKind::ModKw => (PREC_MUL, InfixOperator::Binary(BinaryOp::Mod)),
        _ => return None,
    };
    Some(op)
}
