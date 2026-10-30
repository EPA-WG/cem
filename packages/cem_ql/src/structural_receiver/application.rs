//! Native staging of a verified package against one actual host context.
//! This slot does not publish components, activate modules or grant recovery.
use super::*;
use crate::{
    application_envelope::{DependencyKey, VerifiedApplicationEnvelope},
    datatype_results::DatatypeResultAdapter,
    render::{TemplateAttributeValue, TemplateNode},
    selected_registry::SelectedFunctionRegistry,
};
use cem_ml::schema::attribute_datatypes::{
    AttributeDatatypeContextLease, CompiledAttributeDatatype,
};
mod occurrences;
const INVALID: &str = "native application assembly refused";

/// Supplied by the trusted fresh embedding, never reconstructed from public IDs.
pub struct ComponentReceiverSource {
    pub schema_uri: String,
    pub source: Arc<RetainedCemTree>,
    pub attributes: BTreeMap<String, AstNodeId>,
}
/// Exact existing native link operations. Callbacks/registrations must already
/// exist on the concrete host; this record cannot install or serialize them.
pub enum ContractBinding {
    Query {
        scope: DeclarationScope,
        datatype: AstNodeId,
        functions: SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
    },
    NativeValidator {
        scope: DeclarationScope,
        datatype: AstNodeId,
        adapter: DatatypeResultAdapter,
    },
    NativeTokenizer {
        scope: DeclarationScope,
    },
    NativeOperation {
        scope: DeclarationScope,
    },
    NativeSourceOperation {
        scope: DeclarationScope,
        adapter: NativeResultAdapter,
    },
}
enum LinkedContract {
    Query(FreshQueryBinding),
    Tokenizer(FreshNativeTokenizer),
    Operation(FreshNativeOperation),
}
impl LinkedContract {
    fn live(&self, host: &FreshStructuralHost) -> bool {
        match self {
            Self::Query(value) => value.live(host),
            Self::Tokenizer(value) => value.is_live(),
            Self::Operation(value) => value.is_live(),
        }
    }
    fn retire(&self) {
        match self {
            Self::Query(value) => value.retire(),
            Self::Tokenizer(value) => value.retire(),
            Self::Operation(value) => value.retire(),
        }
    }
}
pub struct LinkedApplication {
    envelope: Arc<VerifiedApplicationEnvelope>,
    context: AttributeDatatypeContextLease,
    models: BTreeMap<String, SchemaDocumentModel>,
    contracts: BTreeMap<String, LinkedContract>,
    node_guards: Vec<Arc<StructuralNodeReceiver>>,
}
impl LinkedApplication {
    fn check(&self, host: &FreshStructuralHost) -> Result<(), String> {
        host.check().map_err(|_| INVALID)?;
        if !self.context.matches(&host.context)
            || self.envelope.pin().registry_hash != host.registry_hash
            || self.contracts.values().any(|value| !value.live(host))
            || self
                .node_guards
                .iter()
                .any(|guard| !guard.is_available(&host.context))
        {
            return Err(INVALID.into());
        }
        Ok(())
    }
    pub fn envelope(&self) -> &Arc<VerifiedApplicationEnvelope> {
        &self.envelope
    }
    pub fn model(
        &self,
        host: &FreshStructuralHost,
        tag: &str,
    ) -> Result<&SchemaDocumentModel, String> {
        self.check(host)?;
        self.models.get(tag).ok_or_else(|| INVALID.into())
    }
    pub fn query(
        &self,
        host: &FreshStructuralHost,
        id: &str,
    ) -> Result<&FreshQueryBinding, String> {
        self.check(host)?;
        match self.contracts.get(id) {
            Some(LinkedContract::Query(value)) => Ok(value),
            _ => Err(INVALID.into()),
        }
    }
    pub fn tokenizer(
        &self,
        host: &FreshStructuralHost,
        id: &str,
    ) -> Result<&FreshNativeTokenizer, String> {
        self.check(host)?;
        match self.contracts.get(id) {
            Some(LinkedContract::Tokenizer(value)) => Ok(value),
            _ => Err(INVALID.into()),
        }
    }
    pub fn operation(
        &self,
        host: &FreshStructuralHost,
        id: &str,
    ) -> Result<&FreshNativeOperation, String> {
        self.check(host)?;
        match self.contracts.get(id) {
            Some(LinkedContract::Operation(value)) => Ok(value),
            _ => Err(INVALID.into()),
        }
    }
}
impl Drop for LinkedApplication {
    fn drop(&mut self) {
        for guard in &self.node_guards {
            guard.retire_preparations();
        }
        for value in self.contracts.values() {
            value.retire();
        }
    }
}
#[derive(Default)]
pub struct ApplicationSlot {
    current: Option<LinkedApplication>,
}
impl ApplicationSlot {
    /// One synchronous in-process replacement, after all staging and context
    /// checks. Physical generation/runtime publication remains a separate gate.
    pub fn publish(
        &mut self,
        host: &FreshStructuralHost,
        staged: LinkedApplication,
    ) -> Result<(), String> {
        staged.check(host)?;
        let retired = self.current.replace(staged);
        drop(retired);
        Ok(())
    }
    pub fn current(&self, host: &FreshStructuralHost) -> Result<&LinkedApplication, String> {
        let current = self.current.as_ref().ok_or(INVALID)?;
        current.check(host)?;
        Ok(current)
    }
    pub fn clear(&mut self) {
        self.current = None;
    }
}
pub fn stage(
    host: &FreshStructuralHost,
    envelope: Arc<VerifiedApplicationEnvelope>,
    mut sources: BTreeMap<String, ComponentReceiverSource>,
    mut bindings: BTreeMap<String, ContractBinding>,
) -> Result<LinkedApplication, String> {
    host.check().map_err(|_| INVALID)?;
    if envelope.pin().registry_hash != host.registry_hash
        || sources.len() > 32
        || bindings.len() > 64
    {
        return Err(INVALID.into());
    }
    occurrences::bounds(&sources)?;
    // Staged ownership immediately ensures that every partial failure retires
    // linked proxies/models; no host registry or previous slot is mutated.
    let mut staged = LinkedApplication {
        context: host.context.lease(),
        envelope,
        models: BTreeMap::new(),
        contracts: BTreeMap::new(),
        node_guards: Vec::new(),
    };
    for key in staged.envelope.dependency_order() {
        host.check().map_err(|_| INVALID)?;
        let DependencyKey::Contract(id) = key else {
            continue;
        };
        let linked = if let Some(query) = staged.envelope.query(id) {
            let Some(ContractBinding::Query {
                scope,
                datatype,
                functions,
                adapter,
            }) = bindings.remove(id)
            else {
                return Err(INVALID.into());
            };
            LinkedContract::Query(host.link_query(
                query.clone(),
                scope,
                datatype,
                &functions,
                adapter,
            )?)
        } else if let Some(native) = staged.envelope.native(id) {
            let value = native.clone();
            match bindings.remove(id).ok_or(INVALID)? {
                ContractBinding::NativeValidator {
                    scope,
                    datatype,
                    adapter,
                } => LinkedContract::Query(
                    host.link_native_validator(value, scope, datatype, adapter)
                        .map_err(|_| INVALID)?,
                ),
                ContractBinding::NativeTokenizer { scope } => LinkedContract::Tokenizer(
                    host.link_native_tokenizer(value, scope)
                        .map_err(|_| INVALID)?,
                ),
                ContractBinding::NativeOperation { scope } => LinkedContract::Operation(
                    host.link_native_operation(value, scope)
                        .map_err(|_| INVALID)?,
                ),
                ContractBinding::NativeSourceOperation { scope, adapter } => {
                    LinkedContract::Operation(
                        host.link_native_source_operation(value, scope, adapter)
                            .map_err(|_| INVALID)?,
                    )
                }
                _ => return Err(INVALID.into()),
            }
        } else {
            continue;
        };
        staged.contracts.insert(id.clone(), linked);
    }
    if !bindings.is_empty() {
        return Err(INVALID.into());
    }
    for key in staged.envelope.dependency_order() {
        let DependencyKey::Component(tag) = key else {
            continue;
        };
        host.check().map_err(|_| INVALID)?;
        let expected = occurrences::expected(&staged.envelope, tag, &host.control)?;
        if expected.is_empty() {
            if sources.contains_key(tag) {
                return Err(INVALID.into());
            }
            continue;
        }
        let source = sources.remove(tag).ok_or(INVALID)?;
        if source.attributes.keys().cloned().collect::<BTreeSet<_>>()
            != expected.keys().cloned().collect()
        {
            return Err(INVALID.into());
        }
        let mut combined: Option<SchemaDocumentModel> = None;
        for (name, fields) in expected {
            host.check().map_err(|_| INVALID)?;
            let occurrence = source.attributes[&name];
            occurrences::matches(&source.source, occurrence, &fields)?;
            let requirement = staged
                .envelope
                .receiver(tag, &fields["type"])
                .ok_or(INVALID)?;
            let capsule = staged
                .envelope
                .structural(&requirement.contract)
                .ok_or(INVALID)?;
            let (mut model, guard) = host.link_receiver_with_guard(
                capsule.clone(),
                &source.schema_uri,
                source.source.clone(),
                occurrence,
            )?;
            staged.node_guards.push(guard);
            if model.attributes.keys().cloned().collect::<BTreeSet<_>>()
                != source.attributes.keys().cloned().collect()
            {
                return Err(INVALID.into());
            }
            if let Some(previous) = &mut combined {
                previous
                    .attribute_datatypes
                    .append(&mut model.attribute_datatypes);
            } else {
                combined = Some(model);
            }
        }
        staged.models.insert(tag.clone(), combined.ok_or(INVALID)?);
    }
    if !sources.is_empty() {
        return Err(INVALID.into());
    }
    staged.check(host)?;
    Ok(staged)
}
