//! One complete original-source and implementation identity ledger across the
//! base application's contracts and every independently verified assembly.
use super::*;
use crate::datatype_dependency_capsule::{
    remember_source, SourceAgreement, VerifiedArtifact, Work as SourceWork,
};
use crate::native_registrations::ImplementationIdentity;
use cem_ml::schema::{machine::LexicallyScopedDocument, structural_capsule::SourceIdentity};
pub(super) struct Agreement<'a> {
    sources: SourceAgreement,
    implementations: BTreeMap<String, Vec<u8>>,
    work: SourceWork<'a>,
}
impl<'a> Agreement<'a> {
    pub(super) fn new(control: &'a OperationControl) -> Self {
        Self {
            sources: BTreeMap::new(),
            implementations: BTreeMap::new(),
            work: SourceWork::new(control),
        }
    }
    fn source(
        &mut self,
        capture: &LexicallyScopedDocument,
        identity: &SourceIdentity,
    ) -> Result<(), String> {
        remember_source(&mut self.sources, capture, identity, &self.work)
    }
    fn implementation(&mut self, identity: &ImplementationIdentity) -> Result<(), String> {
        self.work.check()?;
        let bytes = encode(identity)?;
        if self
            .implementations
            .insert(identity.id.clone(), bytes.clone())
            .is_some_and(|old| old != bytes)
        {
            return Err(INVALID.into());
        }
        Ok(())
    }
    pub(super) fn application(
        &mut self,
        app: &super::super::VerifiedApplicationEnvelope,
    ) -> Result<(), String> {
        for contract in app.contracts.values() {
            match contract {
                super::super::VerifiedContract::Structural(v) => {
                    self.source(v.capture(), &v.pin().source)?
                }
                super::super::VerifiedContract::Query(v) => {
                    self.source(v.capture(), &v.pin().program.source)?
                }
                super::super::VerifiedContract::Native(v) => {
                    self.source(v.capture(), &v.pin().source)?;
                    self.implementation(v.implementation())?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn assembly(
        &mut self,
        graph: &assembly::VerifiedDatatypeModules,
    ) -> Result<(), String> {
        for id in graph.module_ids() {
            let m = graph.module(id).ok_or(INVALID)?;
            self.source(m.capture(), m.identity())?;
            for p in m.profiles().values() {
                for source in p.requirements().values() {
                    self.source(source.capture(), &source.pin().source)?;
                    self.implementation(source.implementation())?;
                }
            }
        }
        for id in graph.leaf_ids() {
            let leaf = graph.leaf(id).ok_or(INVALID)?;
            self.source(leaf.capture(), leaf.source())?;
            if let VerifiedArtifact::Native(v) = leaf {
                self.implementation(v.implementation())?;
            }
        }
        Ok(())
    }
}
