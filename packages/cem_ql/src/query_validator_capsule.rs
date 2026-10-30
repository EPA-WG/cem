//! Native-only, inert typed validator programs. Original declaration sources are
//! explicit host dependencies, not serialized callbacks or source-to-compile.
use crate::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    datatype_validation::{compile_function_query, value_type},
    ir::{CompiledQuery, IrTree},
    resolve::BindingId,
    selected_registry::SelectedFunctionRegistry,
    types::{NodeKind, Type},
};
use cem_ml::{
    content_cache::ContentHash,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::CemAstNode,
    schema::{
        datatype_registry::DatatypeKind,
        datatype_validation::{
            CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation,
            ScalarRepresentation, ValidationImplementation, ValueRepresentation,
        },
        structural_capsule::SourceIdentity,
    },
    source_map::SourceMapStack,
};
use serde::{Deserialize, Serialize};
use std::{cell::Cell, collections::BTreeMap, sync::Arc};
mod instructions;
pub mod declarations;
use instructions::{inspect, FunctionRequirement};

pub const MAGIC: &[u8] = b"CEMQV1\n";
pub const ABI: &str = "cem.native-validator-query/1";
const INVALID: &str = "native validator query capsule refused";
const MAX_BYTES: usize = 64 * 1024;
const MAX_RECORDS: usize = 4096;
const MAX_WORK: usize = 100_000;

#[derive(Debug, Clone)]
pub struct QueryCapsulePin {
    pub hash: String,
    pub source: SourceIdentity,
    pub registry_hash: String,
    pub declaration_hash: String,
}
pub struct BuiltValidatorQuery {
    pub bytes: Vec<u8>,
    pub pin: QueryCapsulePin,
}
/// No mutable IR, serialization implementation, execution capability or live
/// declaration owner escapes verification. Registration is a separate action.
pub struct VerifiedValidatorQuery {
    pin: QueryCapsulePin,
    binding: DeclarationBinding,
    query: CompiledQuery,
    functions: Vec<FunctionRequirement>,
    expanded_work: u64,
}
impl std::fmt::Debug for VerifiedValidatorQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VerifiedValidatorQuery(..)")
    }
}
impl VerifiedValidatorQuery {
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
    pub fn pin(&self) -> &QueryCapsulePin {
        &self.pin
    }
    pub(crate) fn link_matches(
        &self,
        contract: &DatatypeBehaviorContract,
        source: &SourceIdentity,
        registry_hash: &str,
    ) -> bool {
        source == &self.pin.source
            && registry_hash == self.pin.registry_hash
            && declaration(contract).is_ok_and(|binding| binding == self.binding)
    }
    pub(crate) fn link(
        self: &Arc<Self>,
        contract: &DatatypeBehaviorContract,
        source: &SourceIdentity,
        registry_hash: &str,
        available: &SelectedFunctionRegistry,
    ) -> Result<Arc<LinkedValidatorQuery>, String> {
        if !self.link_matches(contract, source, registry_hash) {
            return Err(INVALID.into());
        }
        let mut selected = Vec::new();
        for requirement in &self.functions {
            let entry = requirement.selected()?;
            let (module, name, arity) = entry.key();
            if available.resolve(module, name, arity) != Some(entry) {
                return Err(INVALID.into());
            }
            selected.push(entry);
        }
        Ok(Arc::new(LinkedValidatorQuery {
            program: self.clone(),
            registry: Arc::new(SelectedFunctionRegistry::new(selected)?),
        }))
    }
}
/// Only explicit registration constructs an executable native snapshot. Its
/// registry contains the required subset of implementations supplied by its host.
#[derive(Debug)]
pub(crate) struct LinkedValidatorQuery {
    program: Arc<VerifiedValidatorQuery>,
    registry: Arc<SelectedFunctionRegistry>,
}
impl LinkedValidatorQuery {
    pub(crate) fn evaluate(
        &self,
        context: &crate::api::EvaluationContext,
        control: &OperationControl,
        scope: cem_ml::operation_control::ExecutionScopeId,
    ) -> crate::eval::ItemStream {
        crate::eval::Evaluator::evaluate_verified_validator(
            &self.program.query,
            context,
            self.registry.clone(),
            self.program.expanded_work,
            control,
            scope,
        )
    }
}

// The native checked contract, capture, grants and registry never go on wire.
// This fingerprint pins their source dependency and every selected occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclarationBinding {
    occurrences: [u32; 4],
    header_hash: String,
    maps: Vec<SourceMapStack>,
    namespace: String,
    aliases: BTreeMap<String, String>,
    kind: String,
    value: String,
    candidate_required: bool,
    result_namespace: String,
    result_name: String,
    body_hash: String,
    body_length: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    abi: String,
    ml_version: String,
    ql_version: String,
    registry_hash: String,
    source: SourceIdentity,
    declaration: DeclarationBinding,
    tree: IrTree,
    bindings: Vec<(BindingId, String)>,
    functions: Vec<FunctionRequirement>,
}
struct Work<'a> {
    control: &'a OperationControl,
    left: Cell<usize>,
}
impl<'a> Work<'a> {
    fn new(control: &'a OperationControl) -> Self {
        Self {
            control,
            left: Cell::new(MAX_WORK),
        }
    }
    fn check(&self) -> Result<(), String> {
        self.left
            .set(self.left.get().checked_sub(1).ok_or(INVALID)?);
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID.into())
    }
}
fn hash(bytes: &[u8]) -> String {
    ContentHash::from_blake3(bytes).header_value()
}
fn encode(value: &impl Serialize) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(value).map_err(|_| INVALID.into())
}
fn valid_hash(value: &str) -> bool {
    value.strip_prefix("cem-bin/1+blake3:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn scalar(value: ScalarRepresentation) -> &'static str {
    match value {
        ScalarRepresentation::String => "string",
        ScalarRepresentation::Boolean => "boolean",
        ScalarRepresentation::Integer => "integer",
        ScalarRepresentation::Decimal => "decimal",
        ScalarRepresentation::Double => "double",
        ScalarRepresentation::AnyUri => "any-uri",
    }
}
fn declaration(contract: &DatatypeBehaviorContract) -> Result<DeclarationBinding, String> {
    let ValidationImplementation::Query {
        function,
        body,
        source,
        body_source,
    } = contract.implementation()
    else {
        return Err(INVALID.into());
    };
    let ResultRepresentation::Accepted(result) = &contract.signature().result else {
        return Err(INVALID.into());
    };
    if [contract.behavior(), function, body_source]
        .iter()
        .any(|n| !Arc::ptr_eq(n.document(), contract.owner().document()))
        || !Arc::ptr_eq(source.schema.document(), contract.owner().document())
        || source.schema.node_id() != contract.owner().node_id()
        || body.is_empty()
        || body.len() > 16 * 1024
        || contract.owner().document().nodes.len() > 16_384
        || !contract.owner().document().diagnostics.is_empty()
        || result.namespace != cem_ml::schema::registry::CEM_SCHEMA_URI
        || result.local != "datatype-validation-result"
    {
        return Err(INVALID.into());
    }
    if let CemAstNode::Element { attributes, .. } = function.node() {
        if attributes.iter().any(|id| matches!(function.document().get(*id), Some(CemAstNode::Attribute {expanded_name, value: Some(v), ..}) if expanded_name.local_name == "deterministic" && v != "true")) {return Err(INVALID.into());}
    }
    let value = match contract.signature().value {
        ValueRepresentation::Scalar(p) => format!("scalar:{}", scalar(p)),
        ValueRepresentation::List(p) => format!("list:{}", scalar(p)),
        ValueRepresentation::Nodes => "nodes".into(),
    };
    let kind = match contract.signature().kind {
        DatatypeKind::Scalar => "scalar",
        DatatypeKind::Lexical => "lexical",
        DatatypeKind::List => "list",
        DatatypeKind::Grammar => "grammar",
        DatatypeKind::Reference => "reference",
        DatatypeKind::Node => "node",
    };
    let nodes = [contract.owner(), contract.behavior(), function, body_source];
    let mut headers = Vec::new();
    for node in nodes {
        let CemAstNode::Element {
            expanded_name,
            attributes,
            ..
        } = node.node()
        else {
            return Err(INVALID.into());
        };
        if attributes.len() > 64 {
            return Err(INVALID.into());
        }
        let mut fields = Vec::new();
        for id in attributes {
            let Some(CemAstNode::Attribute {
                expanded_name,
                value,
                value_nodes,
                ..
            }) = node.document().get(*id)
            else {
                return Err(INVALID.into());
            };
            if !value_nodes.is_empty() || value.as_ref().is_some_and(|v| v.len() > 4096) {
                return Err(INVALID.into());
            }
            fields.push((
                expanded_name.namespace_uri.clone(),
                expanded_name.local_name.clone(),
                value.clone(),
            ));
        }
        headers.push((
            expanded_name.namespace_uri.clone(),
            expanded_name.local_name.clone(),
            fields,
        ));
    }
    Ok(DeclarationBinding {
        occurrences: nodes.map(|n| n.node_id()),
        header_hash: hash(&encode(&headers)?),
        maps: nodes
            .iter()
            .map(|n| match n.node() {
                CemAstNode::Element { source, .. } => Ok(source.clone()),
                _ => Err(INVALID.to_string()),
            })
            .collect::<Result<_, _>>()?,
        namespace: source.namespace.clone(),
        aliases: source.bindings.clone(),
        kind: kind.into(),
        value,
        candidate_required: contract.signature().candidate == CandidateRequirement::Required,
        result_namespace: result.namespace.clone(),
        result_name: result.local.clone(),
        body_hash: hash(body.as_bytes()),
        body_length: body.len(),
    })
}
fn validate_declaration(
    binding: &DeclarationBinding,
    source: &SourceIdentity,
    work: &Work<'_>,
) -> Result<(), String> {
    if !valid_hash(&source.hash)
        || source.id == cem_ml::source::SourceId(0)
        || source.byte_length == 0
        || source.byte_length > 1024 * 1024
        || source.uri.len() > 4096
        || !url::Url::parse(&source.uri).is_ok_and(|u| !u.cannot_be_a_base())
        || binding.maps.len() != 4
        || binding.occurrences.iter().any(|n| *n == 0 || *n >= 16_384)
        || binding
            .occurrences
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 4
        || !valid_hash(&binding.header_hash)
        || !valid_hash(&binding.body_hash)
        || binding.body_length == 0
        || binding.body_length > 16 * 1024
        || binding.body_length > source.byte_length
        || binding.namespace != cem_ml::schema::registry::CEM_SCHEMA_URI
        || binding.result_namespace != cem_ml::schema::registry::CEM_SCHEMA_URI
        || binding.result_name != "datatype-validation-result"
        || binding.aliases.len() > 64
        || binding
            .aliases
            .iter()
            .any(|(k, v)| k.len() > 256 || v.len() > 4096)
    {
        return Err(INVALID.into());
    }
    let representation = binding
        .value
        .strip_prefix("scalar:")
        .or_else(|| binding.value.strip_prefix("list:"));
    if !matches!(
        (binding.kind.as_str(), binding.value.as_str()),
        ("node", "nodes")
    ) && !(matches!(
        representation,
        Some("string" | "boolean" | "integer" | "decimal" | "double" | "any-uri")
    ) && (binding.kind == "list" && binding.value.starts_with("list:")
        || matches!(
            binding.kind.as_str(),
            "scalar" | "lexical" | "grammar" | "reference"
        ) && binding.value.starts_with("scalar:")))
    {
        return Err(INVALID.into());
    }
    for map in &binding.maps {
        work.check()?;
        if map.frames.is_empty() || map.frames.len() > 32 {
            return Err(INVALID.into());
        }
        for frame in &map.frames {
            work.check()?;
            if frame.source_id != source.id {
                return Err(INVALID.into());
            }
            match &frame.span {
                cem_ml::source_map::FrameSpan::Single(r)
                    if r.start
                        .checked_add(r.len as u64)
                        .is_some_and(|end| end <= source.byte_length as u64) =>
                {
                    ()
                }
                _ => return Err(INVALID.into()),
            }
            match &frame.transform {
                cem_ml::source_map::TransformKind::CemTokenizer
                | cem_ml::source_map::TransformKind::EventNormalizer
                | cem_ml::source_map::TransformKind::SchemaValidation { .. }
                | cem_ml::source_map::TransformKind::CemAstBuilder => (),
                cem_ml::source_map::TransformKind::SourceBoundary { uri } if uri == &source.uri => {
                    ()
                }
                _ => return Err(INVALID.into()),
            }
        }
    }
    Ok(())
}

pub fn export(
    contract: &DatatypeBehaviorContract,
    source: SourceIdentity,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltValidatorQuery, String> {
    let work = Work::new(control);
    work.check()?;
    if !valid_hash(registry_hash) {
        return Err(INVALID.into());
    }
    let binding = declaration(contract)?;
    validate_declaration(&binding, &source, &work)?;
    let context = StandaloneExpressionContext::default()
        .with_binding(
            "value",
            StandaloneExpressionBinding::new(
                Default::default(),
                value_type(contract.signature().value),
            ),
        )
        .with_binding(
            "datatype",
            StandaloneExpressionBinding::new(Default::default(), Type::Node(NodeKind::Node)),
        )
        .with_binding(
            "candidate",
            StandaloneExpressionBinding::new(
                Default::default(),
                Type::stream(Type::Node(NodeKind::Node)),
            ),
        );
    let query = compile_function_query(contract.implementation(), &context).map_err(|_| INVALID)?;
    let mut bindings = query.policy_bindings.into_iter().collect::<Vec<_>>();
    bindings.sort_by_key(|p| p.0);
    let mut capsule = Capsule {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        registry_hash: registry_hash.into(),
        source: source.clone(),
        declaration: binding,
        tree: query.tree,
        bindings,
        functions: vec![],
    };
    capsule.functions = inspect(&capsule, &work)?.functions;
    let bytes = [MAGIC, &encode(&capsule)?].concat();
    let pin = QueryCapsulePin {
        hash: hash(&bytes),
        source,
        registry_hash: registry_hash.into(),
        declaration_hash: hash(&encode(&capsule.declaration)?),
    };
    verify(&bytes, &pin, control)?;
    Ok(BuiltValidatorQuery { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    expected: &QueryCapsulePin,
    control: &OperationControl,
) -> Result<VerifiedValidatorQuery, String> {
    let work = Work::new(control);
    verify_with_work(bytes, expected, &work)
}
fn verify_with_work(bytes: &[u8], expected: &QueryCapsulePin, work: &Work<'_>)
    -> Result<VerifiedValidatorQuery, String> {
    work.check()?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(&expected.hash)
        || hash(bytes) != expected.hash
    {
        return Err(INVALID.into());
    }
    let body = &bytes[MAGIC.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 16_384, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let capsule = Capsule::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encode(&capsule)? != body
        || capsule.abi != ABI
        || capsule.ml_version != cem_ml::VERSION
        || capsule.ql_version != crate::VERSION
        || capsule.registry_hash != expected.registry_hash
        || !valid_hash(&capsule.registry_hash)
        || capsule.source != expected.source
        || hash(&encode(&capsule.declaration)?) != expected.declaration_hash
    {
        return Err(INVALID.into());
    }
    validate_declaration(&capsule.declaration, &capsule.source, work)?;
    let inspected = inspect(&capsule, work)?;
    if inspected.functions != capsule.functions {
        return Err(INVALID.into());
    }
    let expanded_work = inspected.expanded_work;
    work.check()?;
    Ok(VerifiedValidatorQuery {
        pin: expected.clone(),
        binding: capsule.declaration,
        query: CompiledQuery {
            tree: capsule.tree,
            policy_bindings: capsule.bindings.into_iter().collect(),
            source: String::new(),
        },
        functions: inspected.functions,
        expanded_work,
    })
}

#[cfg(test)]
mod tests;
