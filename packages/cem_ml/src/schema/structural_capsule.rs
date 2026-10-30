//! Native-only structural schema build/verification boundary. No executable
//! callbacks, runtime grants, source parsing on reload or component admission.
use super::document_model::{AttributeModel, ElementModel, FieldContract, SchemaDocumentModel};
use crate::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{document::CemDocument, AstNodeId, CemAstNode, ExpandedName},
    schema::machine::{LexicalReloadMetadata, LexicallyScopedDocument},
    source::SourceId,
    source_map::SourceMapStack,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

mod source;
mod wire;
use source::{declarations, field, nodes, restore};
use wire::preflight;

pub const MAGIC: &[u8] = b"CEMSC1\n";
pub const ABI: &str = "cem.native-structural/1";
const INVALID: &str = "native structural schema capsule refused";
// A deliberately smaller first subset of the design's aggregate ceiling.
const MAX_BYTES: usize = 64 * 1024;
const MAX_RECORDS: usize = 16_384;
const MAX_WORK: usize = 100_000;

/// The linker accepts the same bounded literal source vocabulary, with node
/// cardinality fields. This inspects retained syntax; no parser/evaluator runs.
pub fn validate_literal_receiver_source(
    document: &CemDocument,
    schema_uri: &str,
    control: &OperationControl,
) -> Result<(), String> {
    if !uri(schema_uri)
        || document.nodes.len() > MAX_RECORDS
        || !document.typed_preludes.is_empty()
        || !document.diagnostics.is_empty()
    {
        return Err(INVALID.into());
    }
    source::receiver_declarations(document, schema_uri, &Work::new(control))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub uri: String,
    pub hash: String,
    pub byte_length: usize,
    pub id: SourceId,
}
impl SourceIdentity {
    /// Trusted build input, tied to the original capture by its caller.
    pub fn new(uri: &str, id: SourceId, bytes: &[u8]) -> Self {
        Self {
            uri: uri.into(),
            hash: hash(bytes),
            byte_length: bytes.len(),
            id,
        }
    }
}
#[derive(Debug, Clone)]
pub struct CapsulePin {
    pub hash: String,
    pub schema_uri: String,
    pub source: SourceIdentity,
    pub registry_hash: String,
}
pub struct BuiltStructuralCapsule {
    pub bytes: Vec<u8>,
    pub pin: CapsulePin,
}
pub struct VerifiedStructuralCapsule {
    pin: CapsulePin,
    model: SchemaDocumentModel,
    source: Arc<crate::parser::tree::RetainedCemTree>,
    capture: LexicallyScopedDocument,
}
impl VerifiedStructuralCapsule {
    pub fn pin(&self) -> &CapsulePin {
        &self.pin
    }
    pub fn model(&self) -> &SchemaDocumentModel {
        &self.model
    }
    pub fn source(&self) -> &Arc<crate::parser::tree::RetainedCemTree> {
        &self.source
    }
    pub fn capture(&self) -> &LexicallyScopedDocument {
        &self.capture
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceNode {
    kind: String,
    name: Option<ExpandedName>,
    attributes: Vec<AstNodeId>,
    children: Vec<AstNodeId>,
    value: Option<String>,
    boundary: bool,
    source: SourceMapStack,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Count {
    occurrence: AstNodeId,
    name: String,
    exact: Option<String>,
    min: Option<String>,
    max: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Element {
    occurrence: AstNodeId,
    name: String,
    required: BTreeSet<String>,
    optional: BTreeSet<String>,
    children: BTreeSet<String>,
    counts: Vec<Count>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attribute {
    occurrence: AstNodeId,
    model: AttributeModel,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Dependency {
    from: AstNodeId,
    to: AstNodeId,
    hash: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    abi: String,
    engine_version: String,
    registry_hash: String,
    schema_uri: String,
    source: SourceIdentity,
    nodes: Vec<SourceNode>,
    lexical: LexicalReloadMetadata,
    elements: Vec<Element>,
    attributes: Vec<Attribute>,
    dependencies: Vec<Dependency>,
}
fn hash(bytes: &[u8]) -> String {
    crate::content_cache::ContentHash::from_blake3(bytes).header_value()
}
struct Work<'a> {
    control: &'a OperationControl,
    remaining: std::cell::Cell<usize>,
}
impl<'a> Work<'a> {
    fn new(control: &'a OperationControl) -> Self {
        Self {
            control,
            remaining: std::cell::Cell::new(MAX_WORK),
        }
    }
}
fn check(work: &Work<'_>) -> Result<(), String> {
    let remaining = work.remaining.get().checked_sub(1).ok_or(INVALID)?;
    work.remaining.set(remaining);
    work.control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| INVALID.into())
}
fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(value).map_err(|_| INVALID.into())
}
fn valid_hash(value: &str) -> bool {
    value.strip_prefix("cem-bin/1+blake3:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn uri(value: &str) -> bool {
    value.len() <= 4096 && url::Url::parse(value).is_ok_and(|u| !u.cannot_be_a_base())
}
fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_alphabetic()
                || b == b'_'
                || (i > 0 && (b.is_ascii_digit() || matches!(b, b'-' | b'.')))
        })
}
fn scalar(model: &AttributeModel) -> bool {
    if !model.value_type.as_deref().is_some_and(|v| {
        matches!(
            v.strip_prefix("schema:").unwrap_or(v),
            "string" | "integer" | "decimal" | "number" | "boolean"
        )
    }) {
        return false;
    }
    let mut rest = model.clone();
    rest.name.clear();
    rest.value_type = None;
    rest.source_map = Default::default();
    rest.allowed_values.clear();
    rest.min_inclusive = None;
    rest.max_inclusive = None;
    rest.min_exclusive = None;
    rest.max_exclusive = None;
    rest.min_length = None;
    rest.max_length = None;
    rest.length = None;
    rest.total_digits = None;
    rest.fraction_digits = None;
    rest.pattern = None;
    rest.white_space = None;
    rest == Default::default()
}
fn count_field(
    count: &Count,
    target: &str,
    source: SourceMapStack,
) -> Result<FieldContract, String> {
    let number = |v: &Option<String>| -> Result<Option<usize>, String> {
        v.as_ref()
            .map(|s| {
                s.parse::<usize>()
                    .ok()
                    .filter(|n| *n <= MAX_WORK)
                    .ok_or_else(|| INVALID.to_owned())
            })
            .transpose()
    };
    let (exact, min, max) = (
        number(&count.exact)?,
        number(&count.min)?,
        number(&count.max)?,
    );
    if (exact.is_none() && min.is_none() && max.is_none())
        || min.zip(max).is_some_and(|(a, b)| a > b)
        || exact.is_some_and(|n| min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m))
    {
        return Err(INVALID.into());
    }
    Ok(FieldContract {
        name: count.name.clone(),
        target: target.into(),
        exact_total_children: count.exact.clone(),
        min_total_children: count.min.clone(),
        max_total_children: count.max.clone(),
        source_map: source,
        ..Default::default()
    })
}
fn model(
    capsule: &Capsule,
    document: &CemDocument,
    control: &Work<'_>,
) -> Result<SchemaDocumentModel, String> {
    let mut result = SchemaDocumentModel {
        schema_uri: capsule.schema_uri.clone(),
        ..Default::default()
    };
    let roles = declarations(document, &capsule.schema_uri, control)?;
    let mut used = BTreeSet::new();
    if capsule.elements.is_empty() || capsule.elements.len() + capsule.attributes.len() > 64 {
        return Err(INVALID.into());
    }
    for attribute in &capsule.attributes {
        check(control)?;
        let id = attribute.occurrence;
        if roles.get(&id).map(String::as_str) != Some("attribute")
            || !used.insert(id)
            || field(document, id, "name").as_deref() != Some(&attribute.model.name)
            || !scalar(&attribute.model)
            || attribute.model.source_map
                != *super::document_model::source_stack_for_node(document.get(id).ok_or(INVALID)?)
            || result
                .attributes
                .insert(attribute.model.name.clone(), attribute.model.clone())
                .is_some()
        {
            return Err(INVALID.into());
        }
    }
    for element in &capsule.elements {
        let id = element.occurrence;
        if roles.get(&id).map(String::as_str) != Some("element")
            || !used.insert(id)
            || field(document, id, "name").as_deref() != Some(&element.name)
            || !name(&element.name)
            || element.counts.len() > 16
            || !element
                .required
                .iter()
                .chain(&element.optional)
                .chain(&element.children)
                .all(|v| name(v))
        {
            return Err(INVALID.into());
        }
        let mut fields = vec![];
        for count in &element.counts {
            check(control)?;
            if roles.get(&count.occurrence).map(String::as_str) != Some("field-contract")
                || !used.insert(count.occurrence)
                || field(document, count.occurrence, "name").as_deref() != Some(&count.name)
                || field(document, count.occurrence, "target").as_deref() != Some(&element.name)
            {
                return Err(INVALID.into());
            }
            fields.push(count_field(
                count,
                &element.name,
                super::document_model::source_stack_for_node(
                    document.get(count.occurrence).ok_or(INVALID)?,
                )
                .clone(),
            )?);
        }
        let value = ElementModel {
            name: element.name.clone(),
            required_attributes: element.required.clone(),
            optional_attributes: element.optional.clone(),
            child_elements: element.children.clone(),
            field_contracts: fields,
            ..Default::default()
        };
        if result
            .elements
            .insert(element.name.clone(), value)
            .is_some()
        {
            return Err(INVALID.into());
        }
    }
    if used != roles.keys().copied().collect() {
        return Err(INVALID.into());
    }
    Ok(result)
}
fn dependencies(capsule: &Capsule, control: &Work<'_>) -> Result<Vec<Dependency>, String> {
    let mut result = vec![];
    let mut referenced = BTreeSet::new();
    for element in &capsule.elements {
        check(control)?;
        for name in element.required.union(&element.optional) {
            let target = capsule
                .attributes
                .iter()
                .find(|a| &a.model.name == name)
                .ok_or(INVALID)?;
            referenced.insert(target.occurrence);
            result.push(Dependency {
                from: element.occurrence,
                to: target.occurrence,
                hash: hash(&encoded(target)?),
            });
        }
        for name in &element.children {
            let target = capsule
                .elements
                .iter()
                .find(|e| &e.name == name)
                .ok_or(INVALID)?;
            result.push(Dependency {
                from: element.occurrence,
                to: target.occurrence,
                hash: hash(&encoded(target)?),
            });
        }
    }
    if referenced != capsule.attributes.iter().map(|a| a.occurrence).collect()
        || result.len() > MAX_RECORDS
    {
        return Err(INVALID.into());
    }
    // All exported elements are explicit roots. Bound expanded work as well as
    // depth, so repeated dependency paths cannot multiply verification work.
    let mut work = 0;
    for root in &capsule.elements {
        let mut pending = vec![(root.occurrence, vec![])];
        while let Some((id, mut path)) = pending.pop() {
            check(control)?;
            work += 1;
            if work > MAX_WORK || path.len() >= 64 || path.contains(&id) {
                return Err(INVALID.into());
            }
            path.push(id);
            pending.extend(
                result
                    .iter()
                    .filter(|d| d.from == id)
                    .map(|d| (d.to, path.clone())),
            );
        }
    }
    result.sort();
    Ok(result)
}

/// Conservative retained accounting, separate from wire bytes. Charge the
/// complete source owner to every root; shared dependencies count once per
/// consuming root and once in the aggregate, never once per path.
fn account(capsule: &Capsule, control: &Work<'_>) -> Result<(), String> {
    let source_cost = encoded(&capsule.nodes)?.len()
        + 3 * encoded(&capsule.lexical)?.len()
        + capsule.nodes.len() * std::mem::size_of::<CemAstNode>()
        + capsule
            .nodes
            .iter()
            .map(|n| {
                (n.attributes.len() + n.children.len()) * std::mem::size_of::<AstNodeId>()
                    + n.source.frames.len()
                        * std::mem::size_of::<crate::source_map::SourceMapFrame>()
                    + n.source
                        .frames
                        .iter()
                        .map(|f| match &f.span {
                            crate::source_map::FrameSpan::Multi(r) => {
                                r.len() * std::mem::size_of::<crate::source::ByteRange>()
                            }
                            _ => 0,
                        })
                        .sum::<usize>()
            })
            .sum::<usize>();
    let mut costs = BTreeMap::new();
    for e in &capsule.elements {
        costs.insert(
            e.occurrence,
            encoded(e)?.len()
                + std::mem::size_of::<ElementModel>()
                + e.counts.len() * std::mem::size_of::<FieldContract>(),
        );
    }
    for a in &capsule.attributes {
        let value = super::document_model::AttributeValueContract {
            model: a.model.clone(),
            ..Default::default()
        };
        costs.insert(
            a.occurrence,
            value.accounted_bytes() + std::mem::size_of::<AttributeModel>(),
        );
    }
    if source_cost + costs.values().sum::<usize>() > 1024 * 1024 {
        return Err(INVALID.into());
    }
    for root in &capsule.elements {
        let mut seen = BTreeSet::new();
        let mut pending = vec![root.occurrence];
        let mut charged = source_cost;
        while let Some(id) = pending.pop() {
            check(control)?;
            if !seen.insert(id) {
                continue;
            }
            charged += costs.get(&id).ok_or(INVALID)?;
            if charged > 64 * 1024 {
                return Err(INVALID.into());
            }
            pending.extend(
                capsule
                    .dependencies
                    .iter()
                    .filter(|d| d.from == id)
                    .map(|d| d.to),
            );
        }
    }
    Ok(())
}

pub fn export(
    capture: &LexicallyScopedDocument,
    source: SourceIdentity,
    schema_uri: &str,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltStructuralCapsule, String> {
    let work = Work::new(control);
    let control = &work;
    check(control)?;
    if !uri(schema_uri) || !valid_hash(registry_hash) {
        return Err(INVALID.into());
    }
    let nodes = nodes(capture.document())?;
    let mut document = restore(&nodes, &source, control)?;
    document.format_identity = capture.document().format_identity.clone();
    declarations(&document, schema_uri, control)?;
    let retained = crate::parser::tree::RetainedCemTree::from_shared(
        capture.document().clone(),
        &source.uri,
        "",
        Default::default(),
        None,
    )
    .map_err(|_| INVALID)?;
    let compiled =
        super::document_model::compile_schema_document_model_from_retained(schema_uri, &retained);
    if !compiled.is_ready_for_structural_validation()
        || !compiled.compile_diagnostics.is_empty()
        || !compiled.behaviors.is_empty()
        || !compiled.constraints.is_empty()
        || !compiled.diagnostics.is_empty()
        || !compiled.diagnostic_behaviors.is_empty()
        || compiled.function_bindings.is_some()
        || compiled.datatype_compilation.is_some()
        || !compiled.attribute_datatypes.is_empty()
    {
        return Err(INVALID.into());
    }
    let lexical =
        LexicalReloadMetadata::export(capture, *blake3::hash(&encoded(&nodes)?).as_bytes());
    let mut capsule = Capsule {
        abi: ABI.into(),
        engine_version: crate::VERSION.into(),
        registry_hash: registry_hash.into(),
        schema_uri: schema_uri.into(),
        source,
        nodes,
        lexical,
        elements: vec![],
        attributes: vec![],
        dependencies: vec![],
    };
    let roles = declarations(capture.document(), schema_uri, control)?;
    for (&id, role) in &roles {
        let name = field(capture.document(), id, "name").ok_or(INVALID)?;
        if role == "attribute" {
            capsule.attributes.push(Attribute {
                occurrence: id,
                model: compiled.attributes.get(&name).ok_or(INVALID)?.clone(),
            });
        } else if role == "element" {
            let value = compiled.elements.get(&name).ok_or(INVALID)?;
            let mut counts = vec![];
            for contract in &value.field_contracts {
                let occurrence = roles
                    .iter()
                    .find_map(|(&id, role)| {
                        (role == "field-contract"
                            && field(capture.document(), id, "name").as_deref()
                                == Some(&contract.name))
                        .then_some(id)
                    })
                    .ok_or(INVALID)?;
                let count = Count {
                    occurrence,
                    name: contract.name.clone(),
                    exact: contract.exact_total_children.clone(),
                    min: contract.min_total_children.clone(),
                    max: contract.max_total_children.clone(),
                };
                if count_field(&count, &name, contract.source_map.clone())? != *contract {
                    return Err(INVALID.into());
                }
                counts.push(count);
            }
            capsule.elements.push(Element {
                occurrence: id,
                name,
                required: value.required_attributes.clone(),
                optional: value.optional_attributes.clone(),
                children: value.child_elements.clone(),
                counts,
            });
        }
    }
    // Ensure the portable projection loses no selected native model policy.
    let projected = model(&capsule, capture.document(), control)?;
    if projected.elements != compiled.elements || projected.attributes != compiled.attributes {
        return Err(INVALID.into());
    }
    capsule.dependencies = dependencies(&capsule, control)?;
    let bytes = [MAGIC, &encoded(&capsule)?].concat();
    let pin = CapsulePin {
        hash: hash(&bytes),
        schema_uri: schema_uri.into(),
        source: capsule.source.clone(),
        registry_hash: registry_hash.into(),
    };
    verify(&bytes, &pin, control.control)?;
    Ok(BuiltStructuralCapsule { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    expected: &CapsulePin,
    control: &OperationControl,
) -> Result<VerifiedStructuralCapsule, String> {
    let work = Work::new(control);
    let control = &work;
    check(control)?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(&expected.hash)
        || hash(bytes) != expected.hash
    {
        return Err(INVALID.into());
    }
    let body = &bytes[MAGIC.len()..];
    preflight(body, control)?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let capsule = Capsule::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encoded(&capsule)? != body
        || capsule.abi != ABI
        || capsule.engine_version != crate::VERSION
        || capsule.registry_hash != expected.registry_hash
        || !valid_hash(&capsule.registry_hash)
        || capsule.schema_uri != expected.schema_uri
        || !uri(&capsule.schema_uri)
        || capsule.source != expected.source
    {
        return Err(INVALID.into());
    }
    let mut document = restore(&capsule.nodes, &capsule.source, control)?;
    document.format_identity = capsule.lexical.format_identity.clone();
    if capsule.lexical.version != 2
        || !capsule.lexical.typed_preludes.is_empty()
        || !capsule.lexical.occurrences.is_empty()
        || !capsule.lexical.diagnostics.is_empty()
        || !capsule.lexical.pending_namespace_declarations.is_empty()
        || !capsule.lexical.pending_namespace_names.is_empty()
        || !capsule.lexical.pending_namespace_bindings.is_empty()
    {
        return Err(INVALID.into());
    }
    capsule
        .lexical
        .validate(
            &document,
            *blake3::hash(&encoded(&capsule.nodes)?).as_bytes(),
        )
        .map_err(|_| INVALID)?;
    let model = model(&capsule, &document, control)?;
    if capsule.dependencies != dependencies(&capsule, control)? {
        return Err(INVALID.into());
    }
    account(&capsule, control)?;
    let owner = Arc::new(document);
    let capture = capsule.lexical.restore(owner.clone());
    let source = crate::parser::tree::RetainedCemTree::from_shared(
        owner,
        &capsule.source.uri,
        "",
        Default::default(),
        None,
    )
    .map_err(|_| INVALID)?;
    check(control)?;
    Ok(VerifiedStructuralCapsule {
        pin: expected.clone(),
        model,
        source,
        capture,
    })
}

#[cfg(test)]
mod tests;
