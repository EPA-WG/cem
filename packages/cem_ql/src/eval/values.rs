//! Native retained references and explicit text extraction.
use super::*;
use cem_ml::value::CemReference;

#[derive(Debug, Clone)]
pub struct ReferenceView(pub CemReference<Item>);

pub fn reference(values: Vec<Item>) -> Item {
    if let [item] = values.as_slice() {
        if reference_values(item).is_some()
            || item
                .view()
                .and_then(|v| v.field("kind"))
                .is_some_and(|kind| {
                    kind.first().and_then(Item::atom) == Some(AtomValue::String("reference".into()))
                })
        {
            return item.clone();
        }
    }
    Item::native(ReferenceView(CemReference::new(values)))
}

pub fn reference_values(item: &Item) -> Option<&[Item]> {
    Some(item.view()?.downcast_ref::<ReferenceView>()?.0.values())
}

impl QueryItemView for ReferenceView {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.reference"
    }
    fn identity(&self) -> String {
        self.0.identity()
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        match name {
            "kind" => Some(vec![Item::Atomic(AtomValue::String("reference".into()))]),
            "targets" => Some(self.0.values().to_vec()),
            "id" => Some(vec![Item::Atomic(AtomValue::String(self.identity()))]),
            _ => None,
        }
    }
}

pub(crate) fn text(args: Vec<ItemStream>, ctx: &mut EvalCtx<'_>, source: IrId) -> ItemStream {
    let input = if let Some(input) = args.into_iter().next() {
        input.items
    } else if let Some(item) = ctx.current_items.last() {
        vec![item.clone()]
    } else {
        return ctx.fail_diagnostic(
            source,
            crate::diagnostics::DiagnosticCode("cem.ql.context_missing"),
            "dom:text() requires an active template or query focus",
            "missing context",
        );
    };
    let mut value = String::new();
    let mut pending = vec![input.as_slice().iter()];
    while let Some(items) = pending.last_mut() {
        let Some(item) = items.next() else {
            pending.pop();
            continue;
        };
        if let Err(error) = ctx.charge(BudgetAxis::XPathWorkUnits, 1, source) {
            return error;
        }
        if let Some(targets) = reference_values(item) {
            pending.push(targets.iter());
            continue;
        }
        let query_scope = ctx.query_scope;
        let mut append = |fragment: &str| -> Result<(), ItemStream> {
            ctx.charge(BudgetAxis::XPathWorkUnits, 1, source)?;
            ctx.charge(BudgetAxis::XPathTextBytes, fragment.len() as u64, source)?;
            value.push_str(fragment);
            Ok(())
        };
        if let Some(view) = item.view().filter(|v| v.kind() == QueryItemViewKind::Node) {
            let fragments = view.text_fragments(query_scope);
            let fragments = match fragments {
                Ok(f) => f,
                Err(e) => return super::pipeline::node_access_error(e, ctx, source),
            };
            for fragment in fragments {
                let fragment = match fragment {
                    Ok(f) => f,
                    Err(e) => return super::pipeline::node_access_error(e, ctx, source),
                };
                if let Err(error) = append(fragment) {
                    return error;
                }
            }
        } else if let Some(atom) = item.atom() {
            let lexical = crate::render::item_to_string(&Item::Atomic(atom));
            if let Err(error) = append(&lexical) {
                return error;
            }
        } else {
            return ctx.type_error(source, "dom:text requires nodes or atomic values");
        }
    }
    match ctx.force_safe_point(source) {
        Ok(()) => ItemStream::once(Item::Atomic(AtomValue::String(value))),
        Err(error) => error,
    }
}

pub(crate) fn construct(
    input: ItemStream,
    deep: bool,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
) -> ItemStream {
    let mut result = ItemStream::empty();
    for item in &input.items {
        match clone_item(item, deep, ctx, source) {
            Ok(items) => result.items.extend(items),
            Err(error) => return error,
        }
    }
    result
}

fn clone_item(
    item: &Item,
    deep: bool,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
) -> Result<Vec<Item>, ItemStream> {
    use crate::render::RenderPlanNode;
    ctx.charge_items(1, source)?;
    if let Some(targets) = reference_values(item) {
        let mut result = Vec::new();
        for item in targets {
            result.extend(clone_item(item, deep, ctx, source)?);
        }
        return Ok(result);
    }
    if item.atom().is_some()
        && !item
            .view()
            .is_some_and(|v| v.kind() == QueryItemViewKind::Node)
    {
        return if deep {
            Ok(vec![item.clone()])
        } else {
            Err(ctx.type_error(source, "dom:element requires element nodes"))
        };
    }
    if deep {
        return portable::clone_native(item, ctx, source);
    }
    let view = item
        .view()
        .filter(|view| view.kind() == QueryItemViewKind::Node)
        .ok_or_else(|| ctx.type_error(source, "DOM construction requires native nodes"))?;
    // Capability check precedes resolving any retained native owner.
    drop(
        view.children(ctx.query_scope)
            .map_err(|error| super::pipeline::node_access_error(error, ctx, source))?,
    );
    let mut nodes = crate::render::expand_reference(&CemReference::new(vec![item.clone()]));
    if !deep {
        if nodes.len() != 1 {
            return Err(ctx.type_error(source, "dom:element requires element nodes"));
        }
        let RenderPlanNode::Element {
            attributes,
            children,
            ..
        } = &mut nodes[0]
        else {
            return Err(ctx.type_error(source, "dom:element requires element nodes"));
        };
        attributes.clear();
        children.clear();
    }
    Ok(output::output_nodes(nodes).items)
}
