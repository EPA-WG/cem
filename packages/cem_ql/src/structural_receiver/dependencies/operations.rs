//! Exact reachable CEMOS1 owners; metadata never supplies executable authority.
use super::*;
use crate::{
    datatype_compilation::{DatatypeImplementation, DatatypeImplementations},
    native_registrations::profiles::OperationRole,
    structural_receiver::{FreshNativeOperation, FreshNativeTokenizer},
};
use std::sync::Weak;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DatatypeAssemblyOperationKey {
    pub module: String,
    pub datatype: AstNodeId,
    pub role: OperationRole,
}
pub enum DatatypeAssemblyOperationBinding {
    Operation(Arc<FreshNativeOperation>),
    Tokenizer(Arc<FreshNativeTokenizer>),
}
pub(super) enum Owner {
    Operation(Weak<FreshNativeOperation>),
    Tokenizer(Weak<FreshNativeTokenizer>),
}
impl Owner {
    pub fn is_live(&self, host: &FreshStructuralHost) -> bool {
        match self {
            Self::Operation(o) => o.upgrade().is_some_and(|o| o.is_live_on(host)),
            Self::Tokenizer(o) => o.upgrade().is_some_and(|o| o.is_live_on(host)),
        }
    }
}
pub(super) fn install(
    host: &FreshStructuralHost,
    input: InputRef<'_>,
    root: DeclarationScope,
    scopes: &BTreeMap<String, DeclarationScope>,
    reachable: &BTreeSet<String>,
    bindings: Option<&BTreeMap<DatatypeAssemblyOperationKey, DatatypeAssemblyOperationBinding>>,
    primitives: &mut [DatatypeImplementation],
    registry: &mut DatatypeImplementations,
    limit: usize,
) -> Result<Vec<Owner>, String> {
    let InputRef::Assembly(graph) = input else {
        return if bindings.is_none() {
            Ok(vec![])
        } else {
            Err(REFUSED.into())
        };
    };
    let bindings = bindings.ok_or(REFUSED)?;
    let mut expected = BTreeMap::new();
    let mut remaining = limit;
    for id in graph.module_ids() {
        for (dtype, p) in graph.module(id).ok_or(REFUSED)?.profiles() {
            remaining = remaining.checked_sub(1).ok_or(REFUSED)?;
            host.check().map_err(|_| REFUSED)?;
            if !reachable.contains(&p.source().declaration().identity()) {
                continue;
            }
            for (role, r) in p.requirements() {
                remaining = remaining.checked_sub(1).ok_or(REFUSED)?;
                expected.insert(
                    DatatypeAssemblyOperationKey {
                        module: id.into(),
                        datatype: *dtype,
                        role: *role,
                    },
                    (p, r),
                );
            }
        }
    }
    if bindings.len() != expected.len() || expected.keys().any(|key| !bindings.contains_key(key)) {
        return Err(REFUSED.into());
    }
    let mut owners = vec![];
    for (key, (p, r)) in expected {
        host.check().map_err(|_| REFUSED)?;
        let scope = if key.module == graph.pin().root {
            root
        } else {
            *scopes.get(&key.module).ok_or(REFUSED)?
        };
        match (&key.role, &bindings[&key]) {
            (OperationRole::Tokenizer, DatatypeAssemblyOperationBinding::Tokenizer(o))
                if o.matches_dependency(r, host, scope) =>
            {
                let primitive = primitives
                    .iter_mut()
                    .find(|i| {
                        i.source.declaration().identity() == p.source().declaration().identity()
                    })
                    .ok_or(REFUSED)?;
                o.install(primitive).map_err(|_| REFUSED)?;
                owners.push(Owner::Tokenizer(Arc::downgrade(o)));
            }
            (role, DatatypeAssemblyOperationBinding::Operation(o))
                if *role != OperationRole::Tokenizer && o.matches_dependency(r, host, scope) =>
            {
                o.install(p.source(), registry).map_err(|_| REFUSED)?;
                owners.push(Owner::Operation(Arc::downgrade(o)));
            }
            _ => return Err(REFUSED.into()),
        }
    }
    Ok(owners)
}
pub(super) fn check_shapes(
    input: InputRef<'_>,
    reachable: &BTreeSet<String>,
    compilation: &cem_ml::schema::datatype_contracts::DatatypeCompilation,
) -> Result<(), String> {
    use cem_ml::schema::datatype_contracts::CompiledDatatypeContract;
    let InputRef::Assembly(g) = input else {
        return Ok(());
    };
    for id in g.module_ids() {
        for p in g.module(id).ok_or(REFUSED)?.profiles().values() {
            if !reachable.contains(&p.source().declaration().identity()) {
                continue;
            }
            let actual = compilation
                .contracts
                .iter()
                .filter_map(|c| {
                    c.as_any()
                        .downcast_ref::<crate::datatype_compilation::ExecutableDatatype>()
                })
                .find(|c| {
                    c.source().declaration().identity() == p.source().declaration().identity()
                })
                .ok_or(REFUSED)?;
            if actual.kind() != p.kind() || actual.representation() != p.representation() {
                return Err(REFUSED.into());
            }
        }
    }
    Ok(())
}
