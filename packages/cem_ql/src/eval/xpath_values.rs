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

pub(crate) type ProjectionCache = Arc<std::sync::Mutex<Option<(u32, Arc<Projection>)>>>;
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
        control: &OperationControl, execution_scope: ExecutionScopeId,
    ) -> Result<Arc<Self>, String> {
        let (graph, ids) = portable::encode_graph(&values, limits, true, scope, check)?;
        let native = CemValueXPathProjection::build_with_owner(Arc::new(graph), limits, check, |bytes, _| {
            control.charge_memory(execution_scope, bytes as u64, None)
                .map(|permit| Some(Arc::new(permit) as Arc<dyn Any + Send + Sync>)).map_err(|e| e.to_string())
        })?;
        Ok(Arc::new(Self { native, ids }))
    }
    pub(crate) fn items(&self, item: &NativeItemView, check: &mut impl FnMut() -> Result<(), String>) -> Result<Vec<XPathResultItem>, String> {
        let id = self
            .ids
            .get(&(item.representation_id().into(), item.identity()))
            .ok_or("Native XPath projection lost its source identity")?;
        self.native.items_with_check(*id, check)
    }
}

pub(crate) fn native_items(
    item: &Item,
    scope: QueryContextScope,
    control: &OperationControl, execution_scope: ExecutionScopeId,
    check: &mut impl FnMut() -> Result<(), String>,
) -> Option<Result<Vec<XPathResultItem>, String>> {
    let view = item.view()?;
    if let Some(output) = view.downcast_ref::<output::OutputView>() {
        let result = (|| {
            check()?;
            let limits = value_control::ValueControl::new(control, execution_scope, scope, CemValueArtifactLimits::default())
                .map_err(|e| e.to_string())?.limits;
            let cache = output.xpath_cache();
            let retained = cache
                .lock()
                .map_err(|_| "Native XPath cache lock failed")?
                .as_ref().filter(|(cached_scope, _)| *cached_scope == scope.0)
                .map(|(_, projection)| projection.clone());
            let projection = if let Some(projection) = retained {
                projection
            } else {
                // Only the most recently used capability scope is retained.
                // Evicted trees remain alive only through actual result owners.
                *cache.lock().map_err(|_| "Native XPath cache lock failed")? = None;
                let projection = Projection::build(
                    output.owner_values(),
                    scope,
                    &limits,
                    check, control, execution_scope,
                )?;
                let mut cached = cache.lock().map_err(|_| "Native XPath cache lock failed")?;
                if let Some((_, existing)) = cached.as_ref().filter(|(s, _)| *s == scope.0) {
                    existing.clone()
                } else {
                    *cached = Some((scope.0, projection.clone()));
                    projection
                }
            };
            let _admission = control.charge_memory(execution_scope, projection.native.accounted_bytes as u64, None).map_err(|e| e.to_string())?;
            projection.native.check_limits_with_check(&limits, check)?;
            projection.items(view, check)
        })();
        return Some(checked_projection(result, control, execution_scope));
    }
    if let Some(graph) = view.downcast_ref::<portable::GraphView>() {
        return Some(checked_projection(graph.xpath_items(control, execution_scope, check), control, execution_scope));
    }
    None
}

fn checked_projection<T>(result: Result<T, String>, control: &OperationControl, scope: ExecutionScopeId) -> Result<T, String> {
    if result.is_err() && control.check_scope(scope).is_ok() {
        let _ = control.fail_scope(scope, cem_ml::operation_control::ControlCause::InternalFailure {
            diagnostic_code: "cem.value.xpath_projection".into(),
        }, None);
    }
    result
}

#[derive(Debug)]
pub(crate) struct GraphOwner {
    pub graph: Arc<CemValueGraph>,
    pub limits: CemValueArtifactLimits,
    pub xpath: OnceLock<Arc<CemValueXPathProjection>>,
    _memory: Vec<cem_ml::operation_control::MemoryPermit>,
}
impl std::ops::Deref for GraphOwner {
    type Target = CemValueGraph;
    fn deref(&self) -> &Self::Target {
        &self.graph
    }
}
impl GraphOwner {
    pub fn new(graph: CemValueGraph, limits: CemValueArtifactLimits) -> Arc<Self> {
        Self::retained(graph, limits, Vec::new())
    }
    pub fn retained(graph: CemValueGraph, limits: CemValueArtifactLimits, memory: Vec<cem_ml::operation_control::MemoryPermit>) -> Arc<Self> {
        Arc::new(Self {
            _memory: memory,
            graph: Arc::new(graph),
            limits,
            xpath: OnceLock::new(),
        })
    }
}
