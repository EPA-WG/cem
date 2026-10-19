//! Native scalar projection preserves an importer's admitted source frames.
use super::*;

/// A parser-backed scalar keeps its provenance through queries and template
/// calls. Computed or synthetic values without origins remain ordinary atoms.
pub fn sourced_atom(value: AtomValue, source: Option<SourceMapStack>) -> Item {
    match source.filter(|source| !source.frames.is_empty()) {
        Some(source) => Item::native(SourcedAtom { value, source }),
        None => Item::Atomic(value),
    }
}

#[derive(Debug)]
struct SourcedAtom {
    value: AtomValue,
    source: SourceMapStack,
}
impl QueryItemView for SourcedAtom {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.ql.sourced-atomic"
    }
    fn identity(&self) -> String {
        format!("{:p}", self)
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Atomic
    }
    fn atom(&self) -> Option<AtomValue> {
        Some(self.value.clone())
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        Some(self.source.clone())
    }
}
