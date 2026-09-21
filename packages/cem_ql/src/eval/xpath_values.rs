//! Bridge constructed/portable values through CEM-ML's native XPath projection.
use super::*;
use cem_ml::{
    validation::xpath::XPathResultItem,
    value::{
        artifact::{CemValueArtifactLimits, CemValueGraph},
        xpath::CemValueXPathProjection,
    },
};
use std::sync::OnceLock;

pub(crate) type ProjectionCache = Arc<std::sync::Mutex<BTreeMap<u32, Arc<Projection>>>>;
#[derive(Debug)]
pub(crate) struct Projection {
    native: CemValueXPathProjection,
    ids: BTreeMap<(String, String), u32>,
}
impl Projection {
    pub(crate) fn build(
        values: ItemStream,
        scope: QueryContextScope,
        limits: &CemValueArtifactLimits,
        check: &mut impl FnMut() -> Result<(), String>,
    ) -> Result<Arc<Self>, String> {
        let (graph, ids) = portable::encode_graph(&values, limits, true, scope, check)?;
        let native = CemValueXPathProjection::build(Arc::new(graph), limits, check)?;
        Ok(Arc::new(Self { native, ids }))
    }
    pub(crate) fn items(&self, item: &NativeItemView) -> Result<Vec<XPathResultItem>, String> {
        let id = self
            .ids
            .get(&(item.representation_id().into(), item.identity()))
            .ok_or("Native XPath projection lost its source identity")?;
        self.native.items(*id)
    }
}

pub(crate) fn native_items(
    item: &Item,
    scope: QueryContextScope,
    check: &mut impl FnMut() -> Result<(), String>,
) -> Option<Result<Vec<XPathResultItem>, String>> {
    let view = item.view()?;
    if let Some(output) = view.downcast_ref::<output::OutputView>() {
        return Some((|| {
            check()?;
            let cache = output.xpath_cache();
            let retained = cache
                .lock()
                .map_err(|_| "Native XPath cache lock failed")?
                .get(&scope.0)
                .cloned();
            let projection = if let Some(projection) = retained {
                projection
            } else {
                let projection = Projection::build(
                    output.owner_values(),
                    scope,
                    &CemValueArtifactLimits::default(),
                    check,
                )?;
                cache
                    .lock()
                    .map_err(|_| "Native XPath cache lock failed")?
                    .entry(scope.0)
                    .or_insert(projection)
                    .clone()
            };
            projection.items(view)
        })());
    }
    if let Some(graph) = view.downcast_ref::<portable::GraphView>() {
        return Some(graph.xpath_items(check));
    }
    None
}

#[derive(Debug)]
pub(crate) struct GraphOwner {
    pub graph: Arc<CemValueGraph>,
    pub limits: CemValueArtifactLimits,
    pub xpath: OnceLock<Arc<CemValueXPathProjection>>,
}
impl std::ops::Deref for GraphOwner {
    type Target = CemValueGraph;
    fn deref(&self) -> &Self::Target {
        &self.graph
    }
}
impl GraphOwner {
    pub fn new(graph: CemValueGraph, limits: CemValueArtifactLimits) -> Arc<Self> {
        Arc::new(Self {
            graph: Arc::new(graph),
            limits,
            xpath: OnceLock::new(),
        })
    }
}
