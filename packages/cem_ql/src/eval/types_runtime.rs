//! Runtime type checks for `instance of`, `cast as`, and `treat as`.

use crate::eval::{cast_item, AtomValue, EvalCtx, Item, ItemStream, QueryItemViewKind};
use crate::ir::IrId;
use crate::types::{AtomType, NodeKind, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeTypeChecker;

pub fn item_matches_type(item: &Item, ty: &Type) -> bool {
    if matches!(ty, Type::Any) {
        return true;
    }
    if let Some(atom) = item.atom() {
        return matches!(
            (atom, ty),
            (AtomValue::Null, Type::Empty)
                | (AtomValue::String(_), Type::Atom(AtomType::String))
                | (AtomValue::Integer(_), Type::Atom(AtomType::Integer))
                | (AtomValue::Decimal(_), Type::Atom(AtomType::Decimal))
                | (AtomValue::Double(_), Type::Atom(AtomType::Double))
                | (AtomValue::Boolean(_), Type::Atom(AtomType::Boolean))
                | (AtomValue::AnyUri(_), Type::Atom(AtomType::AnyUri))
        );
    }
    match (item, ty) {
        (Item::Node(_), Type::Node(NodeKind::Node | NodeKind::Element(_)))
        | (Item::Record(_), Type::Record(_))
        | (Item::Array(_), Type::Array(_))
        | (Item::Lambda(_), Type::Lambda { .. }) => true,
        (Item::Native(view), Type::Record(_)) => view.kind() == QueryItemViewKind::Record,
        (Item::Native(view), Type::Array(_)) => view.kind() == QueryItemViewKind::Array,
        (Item::Native(view), Type::Node(NodeKind::Node | NodeKind::Element(_))) => {
            view.kind() == QueryItemViewKind::Node
        }
        _ => false,
    }
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
