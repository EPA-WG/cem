//! Native program + original declaration/source dependency. Reload reconstructs
//! passive typed records and checks retained declarations, never query source.
use super::*;
use cem_ml::{
    parser::{tree::RetainedCemTree, AstNodeId},
    schema::{
        declaration_references::SchemaDeclarationNode,
        machine::LexicallyScopedDocument,
        native_source::NativeSourceSnapshot,
        value_contracts::{ContractName, ValueContractSource},
    },
};
pub const MAGIC: &[u8] = b"CEMQD1\n";
const SOURCE_ABI: &str = "cem.native-query-declaration/1";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryDeclarationPin {
    pub hash: String,
    pub program: QueryCapsulePin,
}
pub struct BuiltQueryDeclaration {
    pub bytes: Vec<u8>,
    pub pin: QueryDeclarationPin,
}
pub struct VerifiedQueryDeclaration {
    pin: QueryDeclarationPin,
    source: Arc<RetainedCemTree>,
    capture: Arc<LexicallyScopedDocument>,
    pub(crate) contract: DatatypeBehaviorContract,
    pub(crate) program: Arc<VerifiedValidatorQuery>,
}
impl VerifiedQueryDeclaration {
    pub fn pin(&self) -> &QueryDeclarationPin {
        &self.pin
    }
    pub fn source(&self) -> &Arc<RetainedCemTree> {
        &self.source
    }
    pub fn capture(&self) -> &Arc<LexicallyScopedDocument> {
        &self.capture
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    abi: String,
    ml_version: String,
    ql_version: String,
    source: NativeSourceSnapshot,
    program: Vec<u8>,
}

fn representation(name: &str) -> Result<ScalarRepresentation, String> {
    Ok(match name {
        "string" => ScalarRepresentation::String,
        "boolean" => ScalarRepresentation::Boolean,
        "integer" => ScalarRepresentation::Integer,
        "decimal" => ScalarRepresentation::Decimal,
        "double" => ScalarRepresentation::Double,
        "any-uri" => ScalarRepresentation::AnyUri,
        _ => return Err(INVALID.into()),
    })
}
fn signature(
    binding: &DeclarationBinding,
) -> Result<cem_ml::schema::datatype_validation::ValidationSignature, String> {
    let kind = match binding.kind.as_str() {
        "scalar" => DatatypeKind::Scalar,
        "lexical" => DatatypeKind::Lexical,
        "list" => DatatypeKind::List,
        "grammar" => DatatypeKind::Grammar,
        "reference" => DatatypeKind::Reference,
        "node" => DatatypeKind::Node,
        _ => return Err(INVALID.into()),
    };
    let value = if let Some(p) = binding.value.strip_prefix("scalar:") {
        ValueRepresentation::Scalar(representation(p)?)
    } else if let Some(p) = binding.value.strip_prefix("list:") {
        ValueRepresentation::List(representation(p)?)
    } else if binding.value == "nodes" {
        ValueRepresentation::Nodes
    } else {
        return Err(INVALID.into());
    };
    Ok(cem_ml::schema::datatype_validation::ValidationSignature {
        kind,
        value,
        candidate: if binding.candidate_required {
            CandidateRequirement::Required
        } else {
            CandidateRequirement::Optional
        },
        result: ResultRepresentation::Accepted(ContractName::new(
            &binding.result_namespace,
            &binding.result_name,
        )),
    })
}
fn checked_source(
    capture: &Arc<LexicallyScopedDocument>,
    program: &VerifiedValidatorQuery,
    work: &Work<'_>,
) -> Result<DatatypeBehaviorContract, String> {
    let binding = &program.binding;
    let owner = SchemaDeclarationNode::new(capture.document().clone(), binding.occurrences[0])
        .ok_or(INVALID)?;
    let behavior = SchemaDeclarationNode::new(capture.document().clone(), binding.occurrences[1])
        .ok_or(INVALID)?;
    // Exactly one inline query body may be executable syntax in this source
    // dependency; all native reference/value-node forms were refused by capture.
    let mut bodies = vec![];
    let mut roots = vec![];
    for node in &capture.document().nodes {
        work.check()?;
        if let CemAstNode::Element {
            node_id,
            expanded_name,
            ..
        } = node
        {
            if expanded_name.local_name == "$" {
                bodies.push(*node_id);
            }
            if expanded_name.local_name == "schema" {
                roots.push(*node_id);
            }
        }
    }
    if bodies != vec![binding.occurrences[3]] || roots != vec![binding.occurrences[0]] {
        return Err(INVALID.into());
    }
    let Some(CemAstNode::Document { root_children, .. }) = capture.document().root() else {
        return Err(INVALID.into());
    };
    if !root_children.contains(&binding.occurrences[0])
        || root_children
            .iter()
            .any(|id| match capture.document().get(*id) {
                Some(CemAstNode::Element { expanded_name, .. }) => {
                    *id != binding.occurrences[0]
                        && !matches!(
                            expanded_name.local_name.as_str(),
                            "@ns" | "@default" | "@doc"
                        )
                }
                Some(CemAstNode::Whitespace { .. } | CemAstNode::Comment { .. }) => false,
                Some(CemAstNode::Text { data, .. }) => !data.trim().is_empty(),
                _ => true,
            })
    {
        return Err(INVALID.into());
    }
    let source =
        ValueContractSource::new(owner, binding.namespace.clone(), binding.aliases.clone())
            .with_captured_names(capture.clone())
            .map_err(|_| INVALID)?;
    let contract = DatatypeBehaviorContract::compile(&source, &behavior, signature(binding)?)
        .map_err(|_| INVALID)?;
    if !program.link_matches(&contract, &program.pin.source, &program.pin.registry_hash) {
        return Err(INVALID.into());
    }
    work.check()?;
    Ok(contract)
}
pub fn export(
    capture: &LexicallyScopedDocument,
    contract: &DatatypeBehaviorContract,
    identity: SourceIdentity,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltQueryDeclaration, String> {
    let work = Work::new(control);
    work.check()?;
    if !Arc::ptr_eq(capture.document(), contract.owner().document()) {
        return Err(INVALID.into());
    }
    let program = super::export(contract, identity.clone(), registry_hash, control)?;
    let source = NativeSourceSnapshot::capture(capture, &identity, &mut || work.check())?;
    let capsule = Capsule {
        abi: SOURCE_ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        source,
        program: program.bytes,
    };
    let bytes = [MAGIC, &encode(&capsule)?].concat();
    let pin = QueryDeclarationPin {
        hash: hash(&bytes),
        program: program.pin,
    };
    verify(&bytes, &pin, control)?;
    Ok(BuiltQueryDeclaration { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    expected: &QueryDeclarationPin,
    control: &OperationControl,
) -> Result<VerifiedQueryDeclaration, String> {
    let work = Work::new(control);
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
        || capsule.abi != SOURCE_ABI
        || capsule.ml_version != cem_ml::VERSION
        || capsule.ql_version != crate::VERSION
    {
        return Err(INVALID.into());
    }
    let program = Arc::new(super::verify_with_work(
        &capsule.program,
        &expected.program,
        &work,
    )?);
    let capture = capsule
        .source
        .restore(&expected.program.source, &mut || work.check())?;
    let contract = checked_source(&capture, &program, &work)?;
    let source = RetainedCemTree::from_shared(
        capture.document().clone(),
        &expected.program.source.uri,
        "",
        Default::default(),
        None,
    )
    .map_err(|_| INVALID)?;
    work.check()?;
    Ok(VerifiedQueryDeclaration {
        pin: expected.clone(),
        source,
        capture,
        contract,
        program,
    })
}

/// A simple named type is an explicit validator target, not an automatically
/// compiled effective datatype. Base/facet/list-tokenizer admission stays separate.
pub(crate) fn datatype(
    source: &VerifiedQueryDeclaration,
    id: AstNodeId,
) -> Result<crate::eval::Item, String> {
    let tree = &source.source;
    let Some(CemAstNode::Element {
        attributes,
        children,
        ..
    }) = tree.ast().get(id)
    else {
        return Err(INVALID.into());
    };
    let named = |id, local: &str| {
        source
            .capture
            .expanded_name(tree.ast_owner(), id)
            .is_some_and(|name| {
                name.local_name == local
                    && name.namespace_uri == cem_ml::schema::registry::CEM_SCHEMA_URI
            })
    };
    if !named(id, "type")
        || !children.iter().all(|id| match tree.ast().get(*id) {
            Some(CemAstNode::Whitespace { .. } | CemAstNode::Comment { .. }) => true,
            Some(CemAstNode::Text { data, .. }) => data.trim().is_empty(),
            _ => false,
        })
    {
        return Err(INVALID.into());
    }
    let mut fields = BTreeMap::new();
    for id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value: Some(v),
            value_nodes,
            ..
        }) = tree.ast().get(*id)
        else {
            return Err(INVALID.into());
        };
        if !expanded_name.namespace_uri.is_empty()
            || !value_nodes.is_empty()
            || !matches!(expanded_name.local_name.as_str(), "name" | "kind")
            || fields
                .insert(expanded_name.local_name.as_str(), v.as_str())
                .is_some()
        {
            return Err(INVALID.into());
        }
    }
    if fields.get("name").is_none_or(|n| n.is_empty())
        || fields.get("kind").copied() != Some(source.program.binding.kind.as_str())
    {
        return Err(INVALID.into());
    }
    let Some(parent) = tree.source_parent(id) else {
        return Err(INVALID.into());
    };
    if !named(parent, "types")
        || tree.source_parent(parent) != Some(source.contract.owner().node_id())
    {
        return Err(INVALID.into());
    }
    crate::eval::RetainedCemNode::new(tree.clone(), id)
        .map(|n| n.query_item())
        .ok_or_else(|| INVALID.into())
}
