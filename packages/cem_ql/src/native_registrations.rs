//! Exact native implementation requirements. Wire records carry inert source and
//! compatibility metadata; only an explicit fresh host supplies implementations.
use crate::{
    datatype_conversion::ConversionSignature, datatype_preparation::PreparationSignature,
    datatype_serialization::ListSerializationSignature,
};
use cem_ml::{
    content_cache::ContentHash,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{tree::RetainedCemTree, CemAstNode},
    schema::{
        datatype_registry::{DatatypeKind, DatatypeKindSource, DatatypeRegistry, DatatypeSource},
        datatype_validation::{
            DatatypeBehaviorContract, ResultRepresentation, ScalarRepresentation,
            ValidationImplementation, ValidationSignature, ValueRepresentation,
        },
        declaration_references::SchemaDeclarationNode,
        document_model::attribute_facets::FacetFamily,
        machine::LexicallyScopedDocument,
        native_source::NativeSourceSnapshot,
        registry::CEM_SCHEMA_URI,
        structural_capsule::SourceIdentity,
        value_contracts::ValueContractSource,
    },
};
use serde::{Deserialize, Serialize};
use std::{cell::Cell, collections::BTreeMap, sync::Arc};
pub const MAGIC: &[u8] = b"CEMNR1\n";
const ABI: &str = "cem.native-registration/1";
const INVALID: &str = "native implementation requirement refused";
const MAX_BYTES: usize = 64 * 1024;
mod source_profiles;
pub(crate) use source_profiles::SourceContract;
pub use source_profiles::{export_source_operation, SourceProfile};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeSignature {
    Validator(ValidationSignature),
    TokenizerSpans,
    Preparation(PreparationSignature),
    Conversion(ConversionSignature),
    Equality(ScalarRepresentation),
    ListSerializer(ListSerializationSignature),
    Constant(ScalarRepresentation),
    FacetProfile(FacetFamily),
    ListItems(ScalarRepresentation),
    SourceConversion(ConversionSignature),
    SourceEquality(ScalarRepresentation),
    SourceConstant(ScalarRepresentation),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImplementationIdentity {
    pub id: String,
    pub abi: String,
    pub module_hash: String,
    pub export: String,
    pub signature: NativeSignature,
}
impl ImplementationIdentity {
    pub fn constant_interpreter(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        representation: ScalarRepresentation,
    ) -> Self {
        Self::operation(
            id,
            module_hash,
            export,
            "cem.native-constant-interpreter/1",
            NativeSignature::Constant(representation),
        )
    }
    pub fn facet_profile(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        family: FacetFamily,
    ) -> Self {
        Self::operation(
            id,
            module_hash,
            export,
            "cem.shared-facet-profile/1",
            NativeSignature::FacetProfile(family),
        )
    }
    pub fn list_items(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        item: ScalarRepresentation,
    ) -> Self {
        Self::operation(
            id,
            module_hash,
            export,
            "cem.shared-list-item-preparation/1",
            NativeSignature::ListItems(item),
        )
    }
    pub fn source_conversion(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        signature: ConversionSignature,
    ) -> Self {
        Self::operation(
            id,
            module_hash,
            export,
            "cem.native-source-converter/1",
            NativeSignature::SourceConversion(signature),
        )
    }
    pub fn source_equality(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        representation: ScalarRepresentation,
    ) -> Self {
        Self::operation(
            id,
            module_hash,
            export,
            "cem.native-source-equality/1",
            NativeSignature::SourceEquality(representation),
        )
    }
    pub fn source_constant(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        representation: ScalarRepresentation,
    ) -> Self {
        Self::operation(
            id,
            module_hash,
            export,
            "cem.native-source-constant/1",
            NativeSignature::SourceConstant(representation),
        )
    }
    fn operation(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        abi: &str,
        signature: NativeSignature,
    ) -> Self {
        Self {
            id: id.into(),
            abi: abi.into(),
            module_hash: module_hash.into(),
            export: export.into(),
            signature,
        }
    }
    pub fn preparation(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        signature: PreparationSignature,
    ) -> Self {
        Self::operation(
            id,
            module_hash,
            export,
            "cem.native-lexical-preparer/1",
            NativeSignature::Preparation(signature),
        )
    }
    pub fn conversion(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        signature: ConversionSignature,
    ) -> Self {
        Self::operation(
            id,
            module_hash,
            export,
            "cem.native-datatype-converter/1",
            NativeSignature::Conversion(signature),
        )
    }
    pub fn equality(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        representation: ScalarRepresentation,
    ) -> Self {
        Self::operation(
            id,
            module_hash,
            export,
            "cem.native-scalar-equality/1",
            NativeSignature::Equality(representation),
        )
    }
    pub fn list_serializer(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        signature: ListSerializationSignature,
    ) -> Self {
        Self::operation(
            id,
            module_hash,
            export,
            "cem.native-list-serializer/1",
            NativeSignature::ListSerializer(signature),
        )
    }
    pub fn validator(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
        signature: ValidationSignature,
    ) -> Self {
        Self {
            id: id.into(),
            abi: "cem.native-datatype-validator/1".into(),
            module_hash: module_hash.into(),
            export: export.into(),
            signature: NativeSignature::Validator(signature),
        }
    }
    pub fn tokenizer(
        id: impl Into<String>,
        module_hash: impl Into<String>,
        export: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            abi: "cem.native-tokenizer-spans/1".into(),
            module_hash: module_hash.into(),
            export: export.into(),
            signature: NativeSignature::TokenizerSpans,
        }
    }
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.id.trim().is_empty()
            || self.id.len() > 4096
            || self.id.chars().any(char::is_control)
            || !valid_hash(&self.module_hash)
            || self.export.is_empty()
            || self.export.len() > 256
            || !self
                .export
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
        {
            return Err(INVALID.into());
        }
        match &self.signature {
            NativeSignature::Validator(sig)
                if self.abi == "cem.native-datatype-validator/1"
                    && matches!(&sig.result,ResultRepresentation::Accepted(name) if name.namespace==CEM_SCHEMA_URI && name.local=="datatype-validation-result") =>
            {
                ()
            }
            NativeSignature::TokenizerSpans if self.abi == "cem.native-tokenizer-spans/1" => (),
            NativeSignature::Preparation(sig)
                if self.abi == "cem.native-lexical-preparer/1"
                    && !matches!(sig.kind, DatatypeKind::Node | DatatypeKind::List)
                    && matches!(sig.output, ValueRepresentation::Scalar(_)) =>
            {
                ()
            }
            NativeSignature::Conversion(sig)
                if self.abi == "cem.native-datatype-converter/1" && sig.check().is_ok() =>
            {
                ()
            }
            NativeSignature::Equality(_) if self.abi == "cem.native-scalar-equality/1" => (),
            NativeSignature::ListSerializer(_) if self.abi == "cem.native-list-serializer/1" => (),
            NativeSignature::Constant(_) if self.abi == "cem.native-constant-interpreter/1" => (),
            NativeSignature::FacetProfile(_) if self.abi == "cem.shared-facet-profile/1" => (),
            NativeSignature::ListItems(_) if self.abi == "cem.shared-list-item-preparation/1" => (),
            NativeSignature::SourceConversion(sig)
                if self.abi == "cem.native-source-converter/1" && sig.check().is_ok() =>
            {
                ()
            }
            NativeSignature::SourceEquality(_) if self.abi == "cem.native-source-equality/1" => (),
            NativeSignature::SourceConstant(_) if self.abi == "cem.native-source-constant/1" => (),
            _ => return Err(INVALID.into()),
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenizerMode {
    Ready,
    CheckedReplacement,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreparationMode {
    Ready,
    CheckedReplacement,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationMode {
    Preparation(PreparationMode),
    Conversion,
    Equality,
    ListSerializer,
    Constant,
    FacetProfile(FacetMode),
    ListItems(PreparationMode),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FacetMode {
    Ready,
    CheckedReplacement,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeLinkError {
    Unavailable,
    IdentityMismatch,
    InvalidSource,
    Expired,
    Duplicate,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRequirementPin {
    pub hash: String,
    pub source: SourceIdentity,
    pub registry_hash: String,
}
pub struct BuiltNativeRequirement {
    pub bytes: Vec<u8>,
    pub pin: NativeRequirementPin,
}
pub(crate) enum Requirement {
    Validator(DatatypeBehaviorContract),
    Tokenizer(DatatypeSource, TokenizerMode),
    Operation(DatatypeSource, OperationMode),
    SourceOperation(DatatypeSource, SourceContract),
}
pub struct VerifiedNativeRequirement {
    pin: NativeRequirementPin,
    source: Arc<RetainedCemTree>,
    capture: Arc<LexicallyScopedDocument>,
    identity: ImplementationIdentity,
    source_snapshot_hash: String,
    selection: Selection,
    pub(crate) requirement: Requirement,
}
impl VerifiedNativeRequirement {
    /// Explicitly map occurrences into an independently verified identical
    /// source arena. Equal names/URIs alone never authorize an allocation swap.
    pub fn share_source(&self, source: &Self, control: &OperationControl) -> Result<Self, String> {
        let work = Work::new(control);
        work.check()?;
        if self.pin.source != source.pin.source
            || self.pin.registry_hash != source.pin.registry_hash
            || self.source_snapshot_hash != source.source_snapshot_hash
        {
            return Err(INVALID.into());
        }
        let requirement = restore_requirement(
            &source.capture,
            &self.identity,
            self.selection.clone(),
            &work,
        )?;
        work.check()?;
        Ok(Self {
            pin: self.pin.clone(),
            source: source.source.clone(),
            capture: source.capture.clone(),
            identity: self.identity.clone(),
            source_snapshot_hash: self.source_snapshot_hash.clone(),
            selection: self.selection.clone(),
            requirement,
        })
    }
    pub fn pin(&self) -> &NativeRequirementPin {
        &self.pin
    }
    pub fn source(&self) -> &Arc<RetainedCemTree> {
        &self.source
    }
    pub fn capture(&self) -> &Arc<LexicallyScopedDocument> {
        &self.capture
    }
    pub fn implementation(&self) -> &ImplementationIdentity {
        &self.identity
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Selection {
    Validator {
        scope: u32,
        behavior: u32,
    },
    Tokenizer {
        scope: u32,
        declaration: u32,
        mode: TokenizerMode,
    },
    Operation {
        scope: u32,
        declaration: u32,
        mode: OperationMode,
    },
    SourceOperation {
        scope: u32,
        declaration: u32,
        behavior: u32,
        profile: SourceProfile,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    abi: String,
    ml_version: String,
    ql_version: String,
    registry_hash: String,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    selection: Selection,
    source: NativeSourceSnapshot,
}
fn hash(bytes: &[u8]) -> String {
    ContentHash::from_blake3(bytes).header_value()
}
fn valid_hash(value: &str) -> bool {
    value.strip_prefix("cem-bin/1+blake3:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn encoded(value: &impl Serialize) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(value).map_err(|_| INVALID.into())
}
struct Work<'a> {
    control: &'a OperationControl,
    left: Cell<usize>,
}
impl<'a> Work<'a> {
    fn new(control: &'a OperationControl) -> Self {
        Self {
            control,
            left: Cell::new(100_000),
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
fn member(
    capture: &LexicallyScopedDocument,
    scope: u32,
    target: u32,
    collection: &str,
    kind: &str,
) -> bool {
    let document = capture.document();
    let named = |id, local: &str| {
        capture
            .expanded_name(document, id)
            .is_some_and(|n| n.namespace_uri == CEM_SCHEMA_URI && n.local_name == local)
    };
    if !named(scope, "schema") || !named(target, kind) {
        return false;
    }
    let Some(CemAstNode::Document { root_children, .. }) = document.root() else {
        return false;
    };
    if !root_children.contains(&scope) {
        return false;
    }
    let Some(CemAstNode::Element { children, .. }) = document.get(scope) else {
        return false;
    };
    children.iter().any(|id|named(*id,collection)&&matches!(document.get(*id),Some(CemAstNode::Element{children,..}) if children.contains(&target)))
}
pub(crate) fn datatype(
    capture: &LexicallyScopedDocument,
    scope: u32,
    id: u32,
) -> Result<DatatypeSource, String> {
    if !member(capture, scope, id, "types", "type") {
        return Err(INVALID.into());
    }
    let scope = SchemaDeclarationNode::new(capture.document().clone(), scope).ok_or(INVALID)?;
    let declaration = SchemaDeclarationNode::new(capture.document().clone(), id).ok_or(INVALID)?;
    let mut registry = DatatypeRegistry::default();
    registry
        .insert(scope.clone(), declaration.clone())
        .map_err(|_| INVALID)?;
    registry.get_by_declaration(&declaration).ok_or(INVALID)?;
    let CemAstNode::Element { attributes, .. } = declaration.node() else {
        return Err(INVALID.into());
    };
    let name = attributes
        .iter()
        .find_map(|id| match declaration.document().get(*id) {
            Some(CemAstNode::Attribute {
                expanded_name,
                value: Some(v),
                ..
            }) if expanded_name.local_name == "name" => Some(v.as_str()),
            _ => None,
        })
        .ok_or(INVALID)?;
    registry.source(&scope, name).ok_or_else(|| INVALID.into())
}
fn validator_source(
    capture: &Arc<LexicallyScopedDocument>,
    scope: u32,
    behavior: u32,
    work: &Work<'_>,
) -> Result<ValueContractSource, String> {
    let document = capture.document();
    let mut aliases = BTreeMap::new();
    let mut pending = vec![behavior];
    while let Some(id) = pending.pop() {
        work.check()?;
        if let Some(CemAstNode::Element {
            attributes,
            children,
            ..
        }) = document.get(id)
        {
            pending.extend(children);
            for attribute in attributes {
                work.check()?;
                let Some(CemAstNode::Attribute {
                    expanded_name,
                    value: Some(value),
                    ..
                }) = document.get(*attribute)
                else {
                    continue;
                };
                if expanded_name.local_name != "type" {
                    continue;
                }
                if let Some((prefix, _)) = value.split_once(':') {
                    let namespace = capture
                        .attribute_namespaces(document, *attribute)
                        .and_then(|snapshot| snapshot.namespaces.binding(prefix))
                        .map(|binding| binding.namespace_uri.clone())
                        .ok_or(INVALID)?;
                    if aliases
                        .insert(prefix.to_string(), namespace.clone())
                        .is_some_and(|old| old != namespace)
                        || aliases.len() > 64
                    {
                        return Err(INVALID.into());
                    }
                }
            }
        }
    }
    ValueContractSource::new(
        SchemaDeclarationNode::new(document.clone(), scope).ok_or(INVALID)?,
        CEM_SCHEMA_URI,
        aliases,
    )
    .with_captured_names(capture.clone())
    .map_err(|_| INVALID.into())
}
fn check_tokenizer(source: &DatatypeSource, mode: TokenizerMode) -> Result<(), String> {
    let plan = source.plan();
    if !plan.issues.is_empty()
        || !matches!(
            plan.kind,
            DatatypeKindSource::Explicit(DatatypeKind::List) | DatatypeKindSource::Inherited
        )
        || (mode == TokenizerMode::Ready
            && (source.attribute("list-base").is_some()
                || plan.kind != DatatypeKindSource::Explicit(DatatypeKind::List)))
        || (mode == TokenizerMode::CheckedReplacement && source.attribute("list-base").is_none())
    {
        return Err(INVALID.into());
    }
    Ok(())
}
fn check_operation(
    source: &DatatypeSource,
    implementation: &ImplementationIdentity,
    mode: OperationMode,
) -> Result<(), String> {
    let plan = source.plan();
    let kind = match (&implementation.signature, mode) {
        (NativeSignature::Preparation(sig), OperationMode::Preparation(mode)) => {
            let inherited = source.attribute("base").is_some();
            let requires_base = mode == PreparationMode::CheckedReplacement;
            if inherited != requires_base {
                return Err(INVALID.into());
            }
            Some(sig.kind)
        }
        (NativeSignature::Conversion(sig), OperationMode::Conversion) => Some(sig.kind),
        (NativeSignature::Equality(_), OperationMode::Equality)
        | (NativeSignature::Constant(_), OperationMode::Constant) => {
            if matches!(
                plan.kind,
                DatatypeKindSource::Explicit(DatatypeKind::List | DatatypeKind::Node)
            ) {
                return Err(INVALID.into());
            }
            None
        }
        (NativeSignature::ListSerializer(_), OperationMode::ListSerializer) => {
            Some(DatatypeKind::List)
        }
        (NativeSignature::ListItems(_), OperationMode::ListItems(mode)) => {
            check_tokenizer(
                source,
                match mode {
                    PreparationMode::Ready => TokenizerMode::Ready,
                    PreparationMode::CheckedReplacement => TokenizerMode::CheckedReplacement,
                },
            )?;
            Some(DatatypeKind::List)
        }
        (NativeSignature::FacetProfile(family), OperationMode::FacetProfile(mode)) => {
            let kind = match family.representation() {
                ValueRepresentation::Scalar(_) => None,
                ValueRepresentation::List(_) => Some(DatatypeKind::List),
                ValueRepresentation::Nodes => Some(DatatypeKind::Node),
            };
            let inherited = if kind == Some(DatatypeKind::List) {
                source.attribute("list-base").is_some()
            } else {
                source.attribute("base").is_some()
            };
            if inherited != (mode == FacetMode::CheckedReplacement)
                || (kind.is_none()
                    && matches!(
                        plan.kind,
                        DatatypeKindSource::Explicit(DatatypeKind::List | DatatypeKind::Node)
                    ))
            {
                return Err(INVALID.into());
            }
            kind
        }
        _ => return Err(INVALID.into()),
    };
    if !plan.issues.is_empty()
        || (source.attribute("list-base").is_some() && kind != Some(DatatypeKind::List))
        || match plan.kind {
            DatatypeKindSource::Inherited => false,
            DatatypeKindSource::Explicit(actual) => kind.is_some_and(|expected| actual != expected),
            _ => true,
        }
    {
        return Err(INVALID.into());
    }
    Ok(())
}
fn build(
    capture: &LexicallyScopedDocument,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    selection: Selection,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltNativeRequirement, String> {
    let work = Work::new(control);
    work.check()?;
    implementation.check()?;
    if !valid_hash(registry_hash) {
        return Err(INVALID.into());
    }
    let source = NativeSourceSnapshot::capture(capture, &identity, &mut || work.check())?;
    let capsule = Capsule {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        registry_hash: registry_hash.into(),
        identity: identity.clone(),
        implementation,
        selection,
        source,
    };
    let bytes = [MAGIC, &encoded(&capsule)?].concat();
    let pin = NativeRequirementPin {
        hash: hash(&bytes),
        source: identity,
        registry_hash: registry_hash.into(),
    };
    verify(&bytes, &pin, control)?;
    Ok(BuiltNativeRequirement { bytes, pin })
}
pub fn export_validator(
    capture: &LexicallyScopedDocument,
    contract: &DatatypeBehaviorContract,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltNativeRequirement, String> {
    if !Arc::ptr_eq(capture.document(), contract.owner().document())
        || !matches!(contract.implementation(),ValidationImplementation::Native(id) if *id==implementation.id)
        || implementation.signature != NativeSignature::Validator(contract.signature().clone())
    {
        return Err(INVALID.into());
    }
    build(
        capture,
        identity,
        implementation,
        Selection::Validator {
            scope: contract.owner().node_id(),
            behavior: contract.behavior().node_id(),
        },
        registry_hash,
        control,
    )
}
pub fn export_tokenizer(
    capture: &LexicallyScopedDocument,
    source: &DatatypeSource,
    mode: TokenizerMode,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltNativeRequirement, String> {
    if !Arc::ptr_eq(capture.document(), source.declaration().document())
        || implementation.signature != NativeSignature::TokenizerSpans
    {
        return Err(INVALID.into());
    }
    check_tokenizer(source, mode)?;
    build(
        capture,
        identity,
        implementation,
        Selection::Tokenizer {
            scope: source.scope().node_id(),
            declaration: source.declaration().node_id(),
            mode,
        },
        registry_hash,
        control,
    )
}
pub fn export_operation(
    capture: &LexicallyScopedDocument,
    source: &DatatypeSource,
    mode: OperationMode,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltNativeRequirement, String> {
    if !Arc::ptr_eq(capture.document(), source.declaration().document()) {
        return Err(INVALID.into());
    }
    check_operation(source, &implementation, mode)?;
    build(
        capture,
        identity,
        implementation,
        Selection::Operation {
            scope: source.scope().node_id(),
            declaration: source.declaration().node_id(),
            mode,
        },
        registry_hash,
        control,
    )
}
fn restore_requirement(
    capture: &Arc<LexicallyScopedDocument>,
    implementation: &ImplementationIdentity,
    selection: Selection,
    work: &Work<'_>,
) -> Result<Requirement, String> {
    let requirement = match selection {
        Selection::Validator { scope, behavior } => {
            if !member(capture, scope, behavior, "behaviors", "behavior") {
                return Err(INVALID.into());
            }
            let NativeSignature::Validator(signature) = &implementation.signature else {
                return Err(INVALID.into());
            };
            let source = validator_source(capture, scope, behavior, work)?;
            let contract = DatatypeBehaviorContract::compile(
                &source,
                &SchemaDeclarationNode::new(capture.document().clone(), behavior).ok_or(INVALID)?,
                signature.clone(),
            )
            .map_err(|_| INVALID)?;
            if !matches!(contract.implementation(),ValidationImplementation::Native(id) if *id==implementation.id)
            {
                return Err(INVALID.into());
            }
            Requirement::Validator(contract)
        }
        Selection::Tokenizer {
            scope,
            declaration,
            mode,
        } => {
            if implementation.signature != NativeSignature::TokenizerSpans {
                return Err(INVALID.into());
            }
            let source = datatype(capture, scope, declaration)?;
            check_tokenizer(&source, mode)?;
            Requirement::Tokenizer(source, mode)
        }
        Selection::Operation {
            scope,
            declaration,
            mode,
        } => {
            let source = datatype(capture, scope, declaration)?;
            check_operation(&source, implementation, mode)?;
            Requirement::Operation(source, mode)
        }
        Selection::SourceOperation {
            scope,
            declaration,
            behavior,
            profile,
        } => {
            let source = datatype(capture, scope, declaration)?;
            let contract = source_profiles::restore(
                capture,
                &source,
                behavior,
                profile,
                implementation,
                work,
            )?;
            Requirement::SourceOperation(source, contract)
        }
    };
    Ok(requirement)
}

pub fn verify(
    bytes: &[u8],
    pin: &NativeRequirementPin,
    control: &OperationControl,
) -> Result<VerifiedNativeRequirement, String> {
    let work = Work::new(control);
    work.check()?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(&pin.hash)
        || hash(bytes) != pin.hash
    {
        return Err(INVALID.into());
    }
    let body = &bytes[MAGIC.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 16_384, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let capsule = Capsule::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encoded(&capsule)? != body
        || capsule.abi != ABI
        || capsule.ml_version != cem_ml::VERSION
        || capsule.ql_version != crate::VERSION
        || capsule.registry_hash != pin.registry_hash
        || !valid_hash(&capsule.registry_hash)
        || capsule.identity != pin.source
    {
        return Err(INVALID.into());
    }
    capsule.implementation.check()?;
    let capture = capsule.source.restore(&pin.source, &mut || work.check())?;
    for node in &capture.document().nodes {
        work.check()?;
        if matches!(node,CemAstNode::Element{expanded_name,..} if expanded_name.local_name=="$") {
            return Err(INVALID.into());
        }
    }
    let source_snapshot_hash = hash(&encoded(&capsule.source)?);
    let requirement = restore_requirement(
        &capture,
        &capsule.implementation,
        capsule.selection.clone(),
        &work,
    )?;
    let source = RetainedCemTree::from_shared(
        capture.document().clone(),
        &pin.source.uri,
        "",
        Default::default(),
        None,
    )
    .map_err(|_| INVALID)?;
    work.check()?;
    Ok(VerifiedNativeRequirement {
        pin: pin.clone(),
        source,
        capture,
        identity: capsule.implementation,
        source_snapshot_hash,
        selection: capsule.selection,
        requirement,
    })
}
