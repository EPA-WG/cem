//! One installation-wide guard for all retained roots. It owns no compilation,
//! callback, source scope or host; every root observes peer retirement/readiness.
use super::*;
use crate::datatype_module_capsule::v2::VerifiedDatatypeModules;
use cem_ml::{parser::AstNodeId, schema::attribute_datatypes::AttributeDatatypeContextLease};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Weak,
};
struct Binding {
    graph: Weak<VerifiedDatatypeModules>,
    compilation: Weak<FreshDependencyDatatypeCompilation>,
    datatype: AstNodeId,
}
pub(super) struct Installation {
    context: AttributeDatatypeContextLease,
    application: Weak<super::super::LinkedApplication>,
    bindings: Vec<Binding>,
    active: AtomicBool,
}
impl Installation {
    pub(super) fn new(
        host: &FreshStructuralHost,
        envelope: &VerifiedApplicationEnvelope,
        application: &Arc<super::super::LinkedApplication>,
        owners: &BTreeMap<String, Arc<FreshDependencyDatatypeCompilation>>,
    ) -> Result<Self, String> {
        let mut bindings = vec![];
        for root in envelope.roots().values() {
            let graph = envelope.assembly_owner(&root.assembly).ok_or(INVALID)?;
            let owner = owners.get(&root.assembly).ok_or(INVALID)?;
            bindings.push(Binding {
                graph: Arc::downgrade(graph),
                compilation: Arc::downgrade(owner),
                datatype: root.datatype,
            });
        }
        let result = Self {
            context: host.context.lease(),
            application: Arc::downgrade(application),
            bindings,
            active: AtomicBool::new(true),
        };
        result.check(host)?;
        Ok(result)
    }
    pub(super) fn check(&self, host: &FreshStructuralHost) -> Result<(), String> {
        if !self.active.load(Ordering::Acquire) || !self.context.matches(&host.context) {
            return Err(INVALID.into());
        }
        self.application.upgrade().ok_or(INVALID)?.check(host)?;
        for binding in &self.bindings {
            let graph = binding.graph.upgrade().ok_or(INVALID)?;
            let owner = binding.compilation.upgrade().ok_or(INVALID)?;
            if !owner.matches_assembly_root(host, &graph, binding.datatype) {
                return Err(INVALID.into());
            }
        }
        Ok(())
    }
    pub(super) fn retire(&self) {
        self.active.store(false, Ordering::Release);
    }
}
