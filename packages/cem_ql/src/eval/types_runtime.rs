//! Runtime type checks for `instance of`, `cast as`, and `treat as`.

use crate::eval::{cast_item, AtomValue, EvalCtx, Item, ItemStream};
use crate::ir::IrId;
use crate::types::{AtomType, NodeKind, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeTypeChecker;

pub fn item_matches_type(item: &Item, ty: &Type) -> bool {
    matches!(
        (item, ty),
        (_, Type::Any)
            | (Item::Atomic(AtomValue::Null), Type::Empty)
            | (
                Item::Node(_),
                Type::Node(NodeKind::Node | NodeKind::Element(_))
            )
            | (
                Item::Atomic(AtomValue::String(_)),
                Type::Atom(AtomType::String)
            )
            | (
                Item::Atomic(AtomValue::Integer(_)),
                Type::Atom(AtomType::Integer)
            )
            | (
                Item::Atomic(AtomValue::Decimal(_)),
                Type::Atom(AtomType::Decimal)
            )
            | (
                Item::Atomic(AtomValue::Double(_)),
                Type::Atom(AtomType::Double)
            )
            | (
                Item::Atomic(AtomValue::Boolean(_)),
                Type::Atom(AtomType::Boolean)
            )
            | (
                Item::Atomic(AtomValue::AnyUri(_)),
                Type::Atom(AtomType::AnyUri)
            )
            | (Item::Record(_), Type::Record(_))
            | (Item::Array(_), Type::Array(_))
            | (Item::Lambda(_), Type::Lambda { .. })
    )
}

pub(crate) fn cast_stream(
    mut stream: ItemStream,
    ty: &Type,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
) -> ItemStream {
    let mut out = ItemStream::empty();
    out.diagnostics.append(&mut stream.diagnostics);
    out.error = stream.error.take();
    for item in stream.items {
        let Some(item) = cast_item(&item, ty) else {
            let err = ctx.type_error(source, "cast failed");
            out.extend_diagnostics(err);
            return out;
        };
        out.items.push(item);
    }
    out
}

pub(crate) fn treat_stream(
    mut stream: ItemStream,
    ty: &Type,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
) -> ItemStream {
    let mut out = ItemStream::empty();
    out.diagnostics.append(&mut stream.diagnostics);
    out.error = stream.error.take();
    for item in stream.items {
        if !item_matches_type(&item, ty) {
            let err = ctx.type_error(source, "treat as failed");
            out.extend_diagnostics(err);
            return out;
        }
        out.items.push(item);
    }
    out
}
