//! Source assertions across the entire composition. Complete native snapshot
//! agreement stays with CEMAPPD3; declarations/scalars do not fabricate one.
use super::*;
pub(super) struct Claims<'a, 'b> {
    sources: BTreeMap<String, String>,
    work: &'a Work<'b>,
}
impl<'a, 'b> Claims<'a, 'b> {
    pub(super) fn new(work: &'a Work<'b>) -> Self {
        Self {
            sources: BTreeMap::new(),
            work,
        }
    }
    pub(super) fn remember(&mut self, uri: &str, hash: &str) -> Result<(), String> {
        self.work.check()?;
        if self
            .sources
            .insert(uri.into(), hash.into())
            .is_some_and(|old| old != hash)
        {
            return Err(INVALID.into());
        }
        Ok(())
    }
    pub(super) fn parent(
        &mut self,
        parent: &v3::VerifiedApplicationEnvelope,
    ) -> Result<(), String> {
        for value in parent.application().contracts.values() {
            let source = match value {
                super::super::VerifiedContract::Structural(v) => &v.pin().source,
                super::super::VerifiedContract::Query(v) => &v.pin().program.source,
                super::super::VerifiedContract::Native(v) => &v.pin().source,
            };
            self.remember(&source.uri, &source.hash)?;
        }
        for id in parent.assembly_ids() {
            let graph = parent.assembly(id).ok_or(INVALID)?;
            for id in graph.module_ids() {
                let module = graph.module(id).ok_or(INVALID)?;
                let source = module.identity();
                self.remember(&source.uri, &source.hash)?;
                for profile in module.profiles().values() {
                    for requirement in profile.requirements().values() {
                        let source = &requirement.pin().source;
                        self.remember(&source.uri, &source.hash)?;
                    }
                }
            }
            for id in graph.leaf_ids() {
                let source = graph.leaf(id).ok_or(INVALID)?.source();
                self.remember(&source.uri, &source.hash)?;
            }
        }
        Ok(())
    }
}
