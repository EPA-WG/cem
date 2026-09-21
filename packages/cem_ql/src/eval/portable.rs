//! CEM-QL adapters for the shared portable native CEM value graph.
use super::*;
use cem_ml::value::artifact::{CemValueArtifactLimits, CemValueGraph, CemValueRecord};

pub fn encode_values(
    values: &ItemStream,
    limits: &CemValueArtifactLimits,
) -> Result<Vec<u8>, String> {
    let (graph, _) = encode_graph(values, limits, true, QueryContextScope(0), &mut || Ok(()))?;
    graph.encode(limits)
}

pub fn decode_values(bytes: &[u8], limits: &CemValueArtifactLimits) -> Result<ItemStream, String> {
    let owner = xpath_values::GraphOwner::new(CemValueGraph::decode(bytes, limits)?, *limits);
    Ok(ItemStream::from_items(
        owner
            .roots
            .iter()
            .map(|&id| graph_item(owner.clone(), id))
            .collect(),
    ))
}

fn graph_item(owner: Arc<xpath_values::GraphOwner>, id: u32) -> Item {
    Item::native(GraphView { owner, id })
}

fn lexical_field(view: &NativeItemView, name: &str) -> String {
    view.field(name)
        .and_then(|v| v.first().and_then(Item::atom))
        .map(|a| crate::render::item_to_string(&Item::Atomic(a)))
        .unwrap_or_default()
}

pub(super) fn encode_graph(
    values: &ItemStream,
    limits: &CemValueArtifactLimits,
    include_parents: bool,
    scope: QueryContextScope,
    check: &mut impl FnMut() -> Result<(), String>,
) -> Result<(CemValueGraph, BTreeMap<(String, String), u32>), String> {
    struct Encoder<'a> {
        items: Vec<Item>,
        ids: BTreeMap<(String, String), u32>,
        limits: &'a CemValueArtifactLimits,
    }
    impl Encoder<'_> {
        fn add(&mut self, item: Item) -> Result<u32, String> {
            let key = item
                .view()
                .map(|v| (v.representation_id().into(), v.identity()));
            if let Some(id) = key.as_ref().and_then(|key| self.ids.get(key)) {
                return Ok(*id);
            }
            if self.items.len() >= self.limits.max_values {
                return Err("Native CEM value count limit exceeded".into());
            }
            let id = u32::try_from(self.items.len())
                .map_err(|_| "Native CEM value identity overflow")?;
            if let Some(key) = key {
                self.ids.insert(key, id);
            }
            self.items.push(item);
            Ok(id)
        }
        fn add_all(&mut self, items: Vec<Item>) -> Result<Vec<u32>, String> {
            items.into_iter().map(|item| self.add(item)).collect()
        }
    }
    let mut encoder = Encoder {
        items: Vec::new(),
        ids: BTreeMap::new(),
        limits,
    };
    let roots = encoder.add_all(values.items.clone())?;
    let mut records = Vec::new();
    while records.len() < encoder.items.len() {
        check()?;
        let item = encoder.items[records.len()].clone();
        let view = item.view();
        let record = if !view.is_some_and(|v| v.kind() == QueryItemViewKind::Node) {
            let atom = item
                .atom()
                .ok_or("Only native CEM nodes and atomic values can enter a value artifact")?;
            let datatype = view
                .map(|v| lexical_field(v, "datatype"))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| {
                    match atom {
                        AtomValue::String(_) => "string",
                        AtomValue::Integer(_) => "integer",
                        AtomValue::Decimal(_) => "decimal",
                        AtomValue::Double(_) => "double",
                        AtomValue::Boolean(_) => "boolean",
                        AtomValue::AnyUri(_) => "anyURI",
                        AtomValue::Null => "null",
                    }
                    .into()
                });
            CemValueRecord {
                lexical: crate::render::item_to_string(&Item::Atomic(atom)),
                datatype,
                source: item.source_map().unwrap_or_default(),
                ..Default::default()
            }
        } else {
            let view = view.expect("native node");
            let kind = lexical_field(view, "kind");
            let parent = if !include_parents
                || kind == "reference" && view.downcast_ref::<values::ReferenceView>().is_some()
            {
                None
            } else {
                view.parent(scope)
                    .map_err(|e| format!("Native CEM parent access denied: {e:?}"))?
                    .map(|p| encoder.add(p))
                    .transpose()?
            };
            let children = if matches!(kind.as_str(), "element" | "document") {
                view.children(scope)
                    .map_err(|e| format!("Native CEM child access denied: {e:?}"))?
                    .map(|v| v.map_err(|e| format!("Native CEM child access denied: {e:?}")))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                Vec::new()
            };
            let native_values = view.field("values");
            let native_content = kind == "attribute" && native_values.is_some();
            CemValueRecord {
                kind,
                parent,
                native_content,
                name: lexical_field(view, "name"),
                namespace: lexical_field(view, "namespace"),
                lexical: lexical_field(view, "value"),
                datatype: String::new(),
                children: encoder.add_all(children)?,
                attributes: encoder.add_all(view.field("attributes").unwrap_or_default())?,
                values: encoder.add_all(native_values.unwrap_or_default())?,
                targets: encoder.add_all(view.field("targets").unwrap_or_default())?,
                occurrence: view.field("occurrence").is_some_and(|items| {
                    items.first().and_then(Item::atom) == Some(AtomValue::Boolean(true))
                }),
                contract: view.value_contract(),
                source: view.source_map().unwrap_or_default(),
                provenance: view.provenance(),
            }
        };
        records.push(record);
    }
    if !include_parents {
        for index in 0..records.len() {
            for child in records[index]
                .children
                .clone()
                .into_iter()
                .chain(records[index].attributes.clone())
            {
                records[child as usize].parent = Some(index as u32);
            }
        }
    }
    Ok((CemValueGraph { roots, records }, encoder.ids))
}

#[derive(Debug, Clone)]
pub struct GraphView {
    pub(crate) owner: Arc<xpath_values::GraphOwner>,
    pub(crate) id: u32,
}
impl GraphView {
    pub(crate) fn xpath_items(
        &self,
        check: &mut impl FnMut() -> Result<(), String>,
    ) -> Result<Vec<cem_ml::validation::xpath::XPathResultItem>, String> {
        check()?;
        if self.owner.xpath.get().is_none() {
            let projection = cem_ml::value::xpath::CemValueXPathProjection::build(
                self.owner.graph.clone(),
                &self.owner.limits,
                check,
            )?;
            let _ = self.owner.xpath.set(Arc::new(projection));
        }
        self.owner
            .xpath
            .get()
            .expect("initialized projection")
            .items(self.id)
    }

    pub fn record(&self) -> &CemValueRecord {
        &self.owner.records[self.id as usize]
    }
    pub fn item(&self, id: u32) -> Item {
        graph_item(self.owner.clone(), id)
    }
}
impl QueryItemView for GraphView {
    fn provenance(&self) -> Option<cem_ml::value::artifact::CemValueProvenance> {
        self.record().provenance.clone()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.native-value/1"
    }
    fn identity(&self) -> String {
        format!("cem:value:{:p}:{}", Arc::as_ptr(&self.owner), self.id)
    }
    fn kind(&self) -> QueryItemViewKind {
        if self.record().kind == "atomic" {
            QueryItemViewKind::Atomic
        } else {
            QueryItemViewKind::Node
        }
    }
    fn atom(&self) -> Option<AtomValue> {
        let record = self.record();
        if record.kind != "atomic" {
            return None;
        }
        Some(match record.datatype.as_str() {
            "integer" => record
                .lexical
                .parse()
                .map(AtomValue::Integer)
                .unwrap_or_else(|_| AtomValue::Decimal(record.lexical.clone())),
            "decimal" | "number" => AtomValue::Decimal(record.lexical.clone()),
            "double" => AtomValue::Double(record.lexical.parse().ok()?),
            "boolean" => AtomValue::Boolean(record.lexical == "true"),
            "null" => AtomValue::Null,
            "anyURI" => AtomValue::AnyUri(record.lexical.clone()),
            _ => AtomValue::String(record.lexical.clone()),
        })
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let record = self.record();
        if name == "occurrence" && record.kind == "reference" {
            return Some(vec![Item::Atomic(AtomValue::Boolean(record.occurrence))]);
        }
        let value = match name {
            "id" => self.identity(),
            "kind" => record.kind.clone(),
            "datatype" => record.datatype.clone(),
            "name"
                if matches!(
                    record.kind.as_str(),
                    "element" | "attribute" | "processing-instruction"
                ) =>
            {
                record.name.clone()
            }
            "namespace" if matches!(record.kind.as_str(), "element" | "attribute") => {
                record.namespace.clone()
            }
            "value" | "data" => record.lexical.clone(),
            "children" => return Some(record.children.iter().map(|&id| self.item(id)).collect()),
            "attributes" => {
                return Some(record.attributes.iter().map(|&id| self.item(id)).collect())
            }
            "values" if record.has_native_content() => {
                return Some(record.values.iter().map(|&id| self.item(id)).collect())
            }
            "targets" => return Some(record.targets.iter().map(|&id| self.item(id)).collect()),
            _ => return None,
        };
        Some(vec![Item::Atomic(AtomValue::String(value))])
    }
    fn parent(&self, _: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        Ok(self.record().parent.map(|id| self.item(id)))
    }
    fn children(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(
            self.record().children.iter().map(|&id| Ok(self.item(id))),
        ))
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        Some(self.record().source.clone())
    }
    fn value_contract(&self) -> Option<cem_ml::schema::document_model::AttributeValueContract> {
        self.record().contract.clone()
    }
    fn text_fragments(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(GraphText {
            graph: &self.owner,
            first: Some((self.id, true)),
            pending: Vec::new(),
        }))
    }
}

struct GraphText<'a> {
    graph: &'a CemValueGraph,
    first: Option<(u32, bool)>,
    pending: Vec<(std::slice::Iter<'a, u32>, bool)>,
}
impl<'a> Iterator for GraphText<'a> {
    type Item = Result<&'a str, QueryNodeAccessError>;
    fn next(&mut self) -> Option<Self::Item> {
        let (id, root) = if let Some(first) = self.first.take() {
            first
        } else {
            loop {
                let (items, root) = self.pending.last_mut()?;
                if let Some(&id) = items.next() {
                    break (id, *root);
                }
                self.pending.pop();
            }
        };
        let record = &self.graph.records[id as usize];
        let value = match record.kind.as_str() {
            "element" | "document" => {
                self.pending.push((record.children.iter(), false));
                ""
            }
            "reference" => {
                self.pending.push((record.targets.iter(), root));
                ""
            }
            "attribute" if record.has_native_content() => {
                self.pending.push((record.values.iter(), true));
                ""
            }
            "text" | "whitespace" | "cdata" | "raw-text" | "atomic" => &record.lexical,
            "attribute" | "comment" | "processing-instruction" if root => &record.lexical,
            _ => "",
        };
        Some(Ok(value))
    }
}

/// Explicit clone uses the shared value graph without serializing or re-importing.
/// Source parents outside the cloned subtree are not retained.
pub(super) fn clone_native(
    item: &Item,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
) -> Result<Vec<Item>, ItemStream> {
    let limits = CemValueArtifactLimits {
        max_values: ctx
            .limits
            .get(&BudgetAxis::ItemsPerStage)
            .copied()
            .unwrap_or(100_000)
            .min(100_000) as usize,
        ..Default::default()
    };
    let scope = ctx.query_scope;
    let mut failure = None;
    let graph = encode_graph(
        &ItemStream::once(item.clone()),
        &limits,
        false,
        scope,
        &mut || {
            if let Err(error) = ctx.charge_items(1, source) {
                failure = Some(error);
                return Err("Native clone cancelled or limited".into());
            }
            Ok(())
        },
    );
    if let Some(error) = failure {
        return Err(error);
    }
    let (graph, _) = graph.map_err(|error| {
        ctx.fail_diagnostic(
            source,
            crate::diagnostics::TYPE_ERROR,
            error,
            "native clone failed",
        )
    })?;
    graph.validate(&limits).map_err(|error| {
        ctx.fail_diagnostic(
            source,
            crate::diagnostics::TYPE_ERROR,
            error,
            "native clone failed",
        )
    })?;
    let owner = xpath_values::GraphOwner::new(graph, limits);
    Ok(owner
        .roots
        .iter()
        .map(|&id| graph_item(owner.clone(), id))
        .collect())
}
